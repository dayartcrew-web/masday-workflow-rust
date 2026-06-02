//! Application constants

/// Workflow status string constants
pub mod workflow_status {
    pub const INIT: &str = "INIT";
    pub const ANALYZE: &str = "ANALYZE";
    pub const PLAN: &str = "PLAN";
    pub const EXECUTE: &str = "EXECUTE";
    pub const VERIFY: &str = "VERIFY";
    pub const FIX: &str = "FIX";
    pub const DONE: &str = "DONE";
    pub const FAILED: &str = "FAILED";
    pub const PAUSED: &str = "PAUSED";
}

/// Task status string constants
pub mod task_status {
    pub const PENDING: &str = "PENDING";
    pub const RUNNING: &str = "RUNNING";
    pub const DONE: &str = "DONE";
    pub const FAILED: &str = "FAILED";
}

/// Default limits and thresholds
pub mod limits {
    pub const MAX_RETRIES: u32 = 3;
    pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
    pub const MAX_CONCURRENT_TASKS: usize = 10;
}

/// Database connection pool configuration
pub mod pool {
    pub const MAX_POOL_SIZE: u32 = 20;
    pub const IDLE_TIMEOUT_SECS: u64 = 30;
    pub const CONNECT_TIMEOUT_SECS: u64 = 2;
    pub const WAIT_TIMEOUT_SECS: u64 = 5;
}

/// Memory scoring weights
pub mod memory_scoring {
    pub const SIMILARITY_WEIGHT: f64 = 0.6;
    pub const IMPORTANCE_WEIGHT: f64 = 0.2;
    pub const RECENCY_WEIGHT: f64 = 0.1;
    pub const USAGE_WEIGHT: f64 = 0.1;
}

/// Workflow defaults
pub mod workflow_defaults {
    pub const DEFAULT_MAX_REWORK_ATTEMPTS: u32 = 2;
    pub const REMINDER_CHECK_INTERVAL_SECS: u64 = 900;
}

/// Port defaults (centralized — read from env with these as fallbacks)
pub mod ports {
    pub const API_PORT: u16 = 30101;
    pub const POSTGRES_PORT: u16 = 54341;
    pub const REDIS_PORT: u16 = 63791;
    pub const API_HOST: &str = "127.0.0.1";

    /// Read API port from MASDAY_API_PORT env, falling back to default.
    pub fn api_port() -> u16 {
        std::env::var("MASDAY_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(API_PORT)
    }

    /// Read PostgreSQL port from MASDAY_DB_PORT env, falling back to default.
    pub fn postgres_port() -> u16 {
        std::env::var("MASDAY_DB_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(POSTGRES_PORT)
    }

    /// Read Redis port from MASDAY_REDIS_PORT env, falling back to default.
    pub fn redis_port() -> u16 {
        std::env::var("MASDAY_REDIS_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(REDIS_PORT)
    }

    /// Default API base URL (e.g. "http://localhost:30101")
    pub fn api_base_url() -> String {
        format!("http://localhost:{}", api_port())
    }
}

/// API defaults (re-exports from ports for backward compat)
pub mod api_defaults {
    pub use super::ports::API_PORT as DEFAULT_API_PORT;
    pub use super::ports::API_HOST as DEFAULT_API_HOST;
}
