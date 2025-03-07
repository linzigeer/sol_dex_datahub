use std::{net::SocketAddr, time::Instant};

use anyhow::Result;
use axum::{Router, extract::DefaultBodyLimit, routing::post};
use tokio::net::TcpListener;
use tower_http::{decompression::RequestDecompressionLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry, fmt::Layer, layer::SubscriberExt};

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(env_filter).with(
        Layer::default()
            .with_writer(std::io::stdout)
            .with_ansi(false),
    );

    tracing::subscriber::set_global_default(subscriber)?;

    let app = Router::new()
        .route("/webhook", post(webhook))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 300))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new());

    let listen_on = "0.0.0.0:9999";
    let listener = TcpListener::bind(listen_on).await?;

    info!("fake webhhok server started, listen on: {}", listen_on);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

pub async fn webhook(req_body: String) -> Result<(), String> {
    let start = Instant::now();
    let body_len = req_body.len();
    let mut body_start_len = 50;
    if body_len < 50 {
        body_start_len = body_len;
    }

    let body_start = &req_body[0..body_start_len];
    info!("request body is start with: {}", body_start);
    let elapsed = start.elapsed().as_millis();
    info!("process webook take {elapsed} ms");

    Ok(())
}
