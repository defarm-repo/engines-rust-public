//! Blocking helper for bridging sync storage with async runtime
//!
//! This module provides utilities to safely call synchronous storage operations
//! from async contexts using tokio::task::spawn_blocking.

/// Execute a blocking operation on Tokio's blocking thread pool
///
/// This is the correct way to call synchronous storage operations from async code.
/// It moves the blocking work off the async runtime's worker threads, preventing
/// the runtime from being blocked.
///
/// # Example
/// ```no_run
/// let item = blocking(|| storage.get_item(&id)).await?;
/// ```
pub async fn blocking<F, T, E>(f: F) -> Result<T, BlockingError<E>>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| BlockingError::JoinError(e.to_string()))?
        .map_err(BlockingError::OperationError)
}

/// Errors that can occur when executing blocking operations
#[derive(Debug)]
pub enum BlockingError<E> {
    /// The tokio task failed to join (task panicked or was cancelled)
    JoinError(String),
    /// The blocking operation itself returned an error
    OperationError(E),
}

impl<E: std::fmt::Display> std::fmt::Display for BlockingError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockingError::JoinError(e) => write!(f, "Blocking task join error: {}", e),
            BlockingError::OperationError(e) => write!(f, "Operation error: {}", e),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for BlockingError<E> {}
