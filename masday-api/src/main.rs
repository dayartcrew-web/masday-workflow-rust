//! masday-api server entry point

use axum::Router;
use std::net::SocketAddr;
use tokio::signal;

use masday_api::{build_router, AppState};
use masday_db::pool::init_pool_with_retry;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("masday_api=debug,tower_http=debug")
        .init();

    tracing::info!("Starting masday-api server");

    let pool = init_pool_with_retry(3)
        .await
        .expect("Failed to create database pool");

    let state = AppState::new(pool);
    let app: Router = build_router(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("masday-api listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Received shutdown signal");
}
