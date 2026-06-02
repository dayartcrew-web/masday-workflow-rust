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

/// API defaults
pub mod api_defaults {
    pub const DEFAULT_API_PORT: u16 = 30101;
    pub const DEFAULT_API_HOST: &str = "127.0.0.1";
}
