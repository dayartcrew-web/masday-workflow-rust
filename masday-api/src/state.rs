//! Shared application state

use axum::extract::FromRef;
use masday_db::pool::DbPool;
use masday_mcp::handler::McpHandler;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Manages SSE sessions for MCP HTTP transport
#[derive(Clone)]
pub struct McpSessionManager {
    sessions: Arc<DashMap<String, broadcast::Sender<String>>>,
}

impl McpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Create a new session, returns (session_id, receiver)
    pub fn create_session(&self) -> (String, broadcast::Receiver<String>) {
        let session_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = broadcast::channel(64);
        self.sessions.insert(session_id.clone(), tx);
        (session_id, rx)
    }

    /// Send a message to a session. Returns false if session not found.
    pub fn send_to_session(&self, session_id: &str, message: &str) -> bool {
        if let Some(tx) = self.sessions.get(session_id) {
            tx.send(message.to_string()).is_ok()
        } else {
            false
        }
    }

    /// Remove a session
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Clean up stale sessions (those with no active receivers)
    pub fn cleanup_stale(&self) {
        self.sessions.retain(|_, tx| tx.receiver_count() > 0);
    }
}

/// Application state shared across all route handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub mcp_handler: McpHandler,
    pub mcp_sessions: McpSessionManager,
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for McpHandler {
    fn from_ref(state: &AppState) -> Self {
        state.mcp_handler.clone()
    }
}

impl FromRef<AppState> for McpSessionManager {
    fn from_ref(state: &AppState) -> Self {
        state.mcp_sessions.clone()
    }
}

impl AppState {
    pub fn new(pool: DbPool, mcp_handler: McpHandler) -> Self {
        Self {
            pool,
            mcp_handler,
            mcp_sessions: McpSessionManager::new(),
        }
    }
}
