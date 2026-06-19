//! Tracked fire-and-forget PostgreSQL sync spawns (C2.12).
//!
//! Every SQLite (local/stdio) mutation that mirrors to PostgreSQL does so via a
//! `tokio::spawn` fire-and-forget task (`direct.rs` → `direct_pg::*`). Under the
//! `#[tokio::main]` entry points (`run_stdio`/`run_local`), when stdin hits EOF the
//! JSON-RPC loop returns and the runtime is dropped shortly after — which
//! **aborts** any spawned sync still in flight. The final sync(s) of a session
//! (typically the task/workflow status flip on completion) then never reach PG,
//! so the dashboard — which reads PG — keeps showing a stale/stuck state.
//!
//! Fix: route those spawns through [`spawn`] here, which registers each handle in
//! a global [`JoinSet`]. [`drain`] is called once at shutdown (after
//! `JsonRpcServer::run` returns, before the runtime drops) to best-effort flush
//! the pending syncs under a bounded timeout, then abort whatever is left so the
//! process never hangs.
//!
//! Safety properties:
//! - `drain` `mem::take`s the `JoinSet` out of the `Mutex` first, so it never
//!   holds a `std::sync::Mutex` guard across an `.await` (which can deadlock the
//!   runtime).
//! - The tracked tasks are pure PostgreSQL (`direct_pg::*`); they never touch the
//!   SQLite `Mutex`, so draining cannot deadlock the SQLite path.
//! - `drain` is a no-op when nothing is pending (the common case): an empty
//!   `JoinSet` yields `None` immediately, so the bounded timeout is never waited.

use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use once_cell::sync::Lazy;
use tokio::task::JoinSet;

/// Global registry of in-flight PG-sync tasks.
static SYNC_SET: Lazy<Mutex<JoinSet<()>>> = Lazy::new(|| Mutex::new(JoinSet::new()));

/// Spawn a fire-and-forget PG sync, tracked so it can be drained at shutdown.
///
/// Mirrors `tokio::spawn` for `Output = ()` futures, but registers the handle in
/// the global [`JoinSet`] instead of detaching it anonymously. Spawns happen on
/// the currently-running tokio runtime, exactly like `tokio::spawn`.
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    SYNC_SET.lock().expect("SYNC_SET poisoned").spawn(future);
}

/// Best-effort drain of all pending PG-sync tasks, bounded by `overall`.
///
/// Call once at shutdown (after the JSON-RPC loop ends, before the runtime
/// drops). Awaits completion of every tracked task but gives up after `overall`,
/// aborting any stragglers so the process exits promptly. Returns immediately
/// when nothing is pending.
pub async fn drain(overall: Duration) {
    // Detach the current JoinSet so we can await completions WITHOUT holding the
    // std Mutex (a std Mutex guard held across `.await` can deadlock the tokio
    // runtime). `JoinSet: Default` → `mem::take` swaps in a fresh empty set,
    // leaving room for any straggling late spawn to land harmlessly.
    let mut set: JoinSet<()> = std::mem::take(&mut *SYNC_SET.lock().expect("SYNC_SET poisoned"));

    // Each task already carries its own internal timeout; tasks run concurrently,
    // so this normally completes well under `overall`. The outer timeout is the
    // hard shutdown cap.
    let _ = tokio::time::timeout(overall, async { while set.join_next().await.is_some() {} }).await;

    // Cancel anything still in flight so we never exceed `overall`.
    set.abort_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // The global SYNC_SET is shared across parallel tests. Serialize drain/spawn
    // tests so one test's `mem::take` can't steal another's tasks (same lesson
    // as the registry parallel-test flake — see registry-test-global-path-flake).
    // tokio::sync::Mutex (not std) so the guard may be held across `.await`
    // without tripping clippy::await_holding_lock or risking a runtime deadlock.
    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn drain_completes_a_pending_sync() {
        let _guard = TEST_LOCK.lock().await;
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&flag);
        spawn(async move {
            flag_clone.store(true, Ordering::SeqCst);
        });
        drain(Duration::from_secs(2)).await;
        assert!(
            flag.load(Ordering::SeqCst),
            "drain should let the sync finish"
        );
    }

    #[tokio::test]
    async fn drain_is_bounded_and_aborts_stragglers() {
        let _guard = TEST_LOCK.lock().await;
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);
        // A sync that would outlive the drain budget.
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            finished_clone.store(true, Ordering::SeqCst);
        });
        let start = tokio::time::Instant::now();
        drain(Duration::from_millis(150)).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "drain must return near its budget, not wait for the task; took {:?}",
            elapsed
        );
        // Yield a few times so a non-aborted task could flag if it were running.
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            !finished.load(Ordering::SeqCst),
            "straggler must have been aborted, not completed"
        );
    }

    #[tokio::test]
    async fn drain_returns_promptly_when_empty() {
        let _guard = TEST_LOCK.lock().await;
        // Ensure the set starts empty (a prior test's drain leaves it empty).
        let start = tokio::time::Instant::now();
        drain(Duration::from_secs(5)).await;
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "empty drain must be near-instant; took {:?}",
            start.elapsed()
        );
    }
}
