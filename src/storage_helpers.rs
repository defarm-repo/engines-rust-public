/// Storage lock helpers to prevent worker thread blocking
use crate::storage::StorageBackend;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

const STORAGE_LOCK_TRY_MS: u64 = 50;
const STORAGE_LOCK_SPIN_MS: u64 = 5;

#[derive(Debug)]
pub enum StorageLockError {
    Timeout,
    Other(String),
}

impl std::fmt::Display for StorageLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageLockError::Timeout => write!(f, "Storage lock timeout"),
            StorageLockError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for StorageLockError {}

/// Safely acquire storage lock with timeout to prevent worker thread blocking
/// Generic over storage types that implement StorageBackend
pub fn with_storage<S, T, F>(
    storage: &Arc<Mutex<S>>,
    label: &str,
    f: F,
) -> Result<T, StorageLockError>
where
    S: StorageBackend,
    F: FnOnce(&S) -> Result<T, Box<dyn std::error::Error>>,
{
    let start = Instant::now();
    info!(%label, "attempting to acquire storage lock");

    // Try quick lock first, then spin with small sleeps to avoid blocking Tokio worker
    for attempt in 0..(STORAGE_LOCK_TRY_MS / STORAGE_LOCK_SPIN_MS) {
        match storage.try_lock() {
            Ok(guard) => {
                let acquire_ms = start.elapsed().as_millis();
                info!(%label, acquire_ms, "storage lock acquired");

                let op_start = Instant::now();
                let res = f(&*guard);
                let op_ms = op_start.elapsed().as_millis();

                info!(%label, op_ms, total_ms = %(start.elapsed().as_millis()), "storage operation complete");

                return res.map_err(|e| StorageLockError::Other(e.to_string()));
            }
            Err(_) => {
                if attempt % 5 == 0 {
                    info!(%label, attempt, "storage lock contention, retrying...");
                }
                std::thread::sleep(Duration::from_millis(STORAGE_LOCK_SPIN_MS));
            }
        }
    }

    let waited_ms = start.elapsed().as_millis();
    warn!(%label, waited_ms, "storage lock timeout - returning 503");
    Err(StorageLockError::Timeout)
}

/// Generic version for any mutex-protected data (not just StorageBackend)
/// Used for internal state like workspaces, circuits cache, etc.
pub fn with_lock<T, R, F>(mutex: &Arc<Mutex<T>>, label: &str, f: F) -> Result<R, StorageLockError>
where
    F: FnOnce(&T) -> Result<R, Box<dyn std::error::Error>>,
{
    let start = Instant::now();
    info!(%label, "attempting to acquire lock");

    // Try quick lock first, then spin with small sleeps to avoid blocking Tokio worker
    for attempt in 0..(STORAGE_LOCK_TRY_MS / STORAGE_LOCK_SPIN_MS) {
        match mutex.try_lock() {
            Ok(guard) => {
                let acquire_ms = start.elapsed().as_millis();
                info!(%label, acquire_ms, "lock acquired");

                let op_start = Instant::now();
                let res = f(&*guard);
                let op_ms = op_start.elapsed().as_millis();

                info!(%label, op_ms, total_ms = %(start.elapsed().as_millis()), "operation complete");

                return res.map_err(|e| StorageLockError::Other(e.to_string()));
            }
            Err(_) => {
                if attempt % 5 == 0 {
                    info!(%label, attempt, "lock contention, retrying...");
                }
                std::thread::sleep(Duration::from_millis(STORAGE_LOCK_SPIN_MS));
            }
        }
    }

    let waited_ms = start.elapsed().as_millis();
    warn!(%label, waited_ms, "lock timeout - returning 503");
    Err(StorageLockError::Timeout)
}

/// Generic version for mutable operations on any mutex-protected data
pub fn with_lock_mut<T, R, F>(
    mutex: &Arc<Mutex<T>>,
    label: &str,
    f: F,
) -> Result<R, StorageLockError>
where
    F: FnOnce(&mut T) -> Result<R, Box<dyn std::error::Error>>,
{
    let start = Instant::now();
    info!(%label, "attempting to acquire lock (mut)");

    // Try quick lock first, then spin with small sleeps to avoid blocking Tokio worker
    for attempt in 0..(STORAGE_LOCK_TRY_MS / STORAGE_LOCK_SPIN_MS) {
        match mutex.try_lock() {
            Ok(mut guard) => {
                let acquire_ms = start.elapsed().as_millis();
                info!(%label, acquire_ms, "lock acquired (mut)");

                let op_start = Instant::now();
                let res = f(&mut *guard);
                let op_ms = op_start.elapsed().as_millis();

                info!(%label, op_ms, total_ms = %(start.elapsed().as_millis()), "operation complete (mut)");

                return res.map_err(|e| StorageLockError::Other(e.to_string()));
            }
            Err(_) => {
                if attempt % 5 == 0 {
                    info!(%label, attempt, "lock contention (mut), retrying...");
                }
                std::thread::sleep(Duration::from_millis(STORAGE_LOCK_SPIN_MS));
            }
        }
    }

    let waited_ms = start.elapsed().as_millis();
    warn!(%label, waited_ms, "lock timeout (mut) - returning 503");
    Err(StorageLockError::Timeout)
}
