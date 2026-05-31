//! Shared application state

use axum::extract::FromRef;
use masday_db::pool::DbPool;

/// Application state shared across all route handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}
