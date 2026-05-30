//! Memory business logic (4-layer memory, scoring, BM25)

use masday_core::{AppError, Result};
use uuid::Uuid;

/// Memory service
pub struct MemoryService;

impl MemoryService {
    pub fn new() -> Self {
        Self
    }

    pub async fn store(&self, content: serde_json::Value, importance: f64) -> Result<Uuid> {
        // Placeholder implementation
        Ok(Uuid::new_v4())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        // Placeholder implementation
        Ok(Vec::new())
    }

    pub async fn recall_recent(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        // Placeholder implementation
        Ok(Vec::new())
    }
}
