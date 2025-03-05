use std::{net::SocketAddr, ops::ControlFlow, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{
        ConnectInfo, Query, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, info, warn};

use crate::{
    cache::{self, DexEvent},
    web::error::WebAppError,
};

use crate::web::WebAppContext;

#[derive(Deserialize)]
pub struct WsQuery {
    ticket: String,
}

pub async fn ws_handler(
    Query(WsQuery { ticket }): Query<WsQuery>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(WebAppContext {
        ws_connected,
        ws_broadcast,
        redis_client,
        ..
    }): State<WebAppContext>,
) -> impl IntoResponse {
    let mut guard = ws_connected.write().await;
    if *guard {
        return WebAppError::other("already have connected client").into_response();
    } else {
        *guard = true;
    }
    drop(guard);
    // this can be used to auth
    if ticket != "123" {
        return WebAppError::unauth("no auth websocket").into_response();
    }

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            addr,
            ws_broadcast.clone(),
            ws_connected.clone(),
            redis_client.clone(),
        )
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    ws_topic: broadcast::Sender<Utf8Bytes>,
    ws_connected: Arc<RwLock<bool>>,
    redis_client: Arc<redis::Client>,
) {
    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket
        .send(Message::Ping(Bytes::from_static(&[1])))
        .await
        .is_ok()
    {
        info!("Pinged {who}...");
    } else {
        info!("Could not send ping {who}!");
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    }

    // receive single message from a client (we can either receive or send with socket).
    // this will likely be the Pong for our Ping or a hello message from client.
    // waiting for message from a client will block this task, but will not block other client's
    // connections.
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            info!("client {who} abruptly disconnected");
            return;
        }
    }

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    let mut rx = ws_topic.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            info!("rx received msg: {msg}, {who}");
            if msg.trim() == "subscribe_dex_trades" {
                loop {
                    match get_dex_events(redis_client.clone()).await {
                        Ok(events) if !events.is_empty() => {
                            info!("send {} trades to client: {who}", events.len());
                            match serde_json::to_string(&events) {
                                Ok(msg) => {
                                    if sender.send(Message::text(msg)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    warn!("error serialize dex event from redis: {err}");
                                }
                            };
                        }
                        Ok(_) => {}
                        Err(err) => {
                            warn!("{who} get trades error: {err}");
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // print message and break if instructed to do so
            match process_message(msg, who) {
                ControlFlow::Continue(msg) => {
                    if let Message::Text(m) = msg {
                        if ws_topic.send(m).is_err() {
                            break;
                        }
                    }
                }
                ControlFlow::Break(_) => {
                    break;
                }
            }
        }
    });

    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(_) => println!("******> sent task to {who} finished"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(_) => println!("======> {who} recv task finished"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    info!("Websocket context {who} destroyed");
    let mut guard = ws_connected.write().await;
    *guard = false;
    drop(guard);
}

async fn get_dex_events(redis_client: Arc<redis::Client>) -> Result<Vec<DexEvent>> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    cache::take_dex_evts(&mut conn).await
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), Message> {
    match msg {
        Message::Text(t) => {
            debug!(">>> {who} sent str: {t:?}");
            ControlFlow::Continue(Message::Text(t))
        }
        Message::Binary(d) => {
            debug!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
            ControlFlow::Continue(Message::Binary(d))
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                info!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                info!(">>> {who} somehow sent close message without CloseFrame");
            }
            ControlFlow::Break(())
        }

        Message::Pong(v) => {
            debug!(">>> receive pong with {v:?} from {who}");
            ControlFlow::Continue(Message::Ping(v))
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            debug!(">>> receive ping with {v:?} from {who}");
            ControlFlow::Continue(Message::Pong(v))
        }
    }
}
