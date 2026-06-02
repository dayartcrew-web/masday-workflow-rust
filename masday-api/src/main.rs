//! masday-api server entry point

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("masday_api=debug,tower_http=debug")
        .init();

    tracing::info!("Starting masday-api server");

    let pool = masday_db::pool::init_pool_with_retry(3)
        .await
        .expect("Failed to create database pool");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(30101);

    masday_api::run(pool, port).await.expect("API server failed");
}
