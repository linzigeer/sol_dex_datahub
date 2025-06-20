mod context;
pub mod controller;
mod error;
pub mod extractor;

use std::net::SocketAddr;

use anyhow::Result;
pub use context::*;
use controller::{home, metrics, qn_stream};
pub use error::*;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

pub async fn start(context: WebAppContext, listen_on: &str) -> Result<()> {
    let app = Router::new()
        .route("/", get(home::index))
        .route("/metrics", get(metrics::check_health))
        .route("/sol_dex_stream", post(qn_stream::sol_dex_stream))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 300))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(context);
    let listener = TcpListener::bind(listen_on).await?;

    info!("web server started, listen on: {}", listen_on);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
