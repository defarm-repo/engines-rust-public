use std::env;
/// Storage Factory
/// Creates the appropriate storage backend based on environment configuration
use std::sync::{Arc, Mutex};

use crate::storage::{InMemoryStorage, StorageBackend};
// TEMPORARILY DISABLED: PostgreSQL implementation needs type fixes
// use crate::postgres_storage::PostgresStorage;

pub enum StorageType {
    InMemory(Arc<Mutex<InMemoryStorage>>),
    // TEMPORARILY DISABLED: PostgreSQL implementation needs type fixes
    // Postgres(Arc<Mutex<PostgresStorage>>),
}

impl StorageType {
    pub fn as_backend(&self) -> &dyn StorageBackend {
        match self {
            StorageType::InMemory(storage) => {
                // This is a bit tricky - we need to return a reference to something
                // that implements StorageBackend. The Arc<Mutex<InMemoryStorage>>
                // implements StorageBackend, so we can return a reference to it.
                storage as &dyn StorageBackend
            } // TEMPORARILY DISABLED: PostgreSQL implementation needs type fixes
              // StorageType::Postgres(storage) => {
              //     storage as &dyn StorageBackend
              // }
        }
    }
}

/// Create storage backend based on DATABASE_URL environment variable
/// - If DATABASE_URL is set: Log warning that PostgreSQL is temporarily disabled
/// - Always use In-Memory storage for now
pub async fn create_storage() -> Result<StorageType, Box<dyn std::error::Error>> {
    if env::var("DATABASE_URL").is_ok() {
        tracing::warn!("⚠️  DATABASE_URL detected but PostgreSQL is temporarily disabled");
        tracing::warn!("⚠️  Using In-Memory storage instead");
        tracing::info!("💡 PostgreSQL will be re-enabled after fixing type mismatches");
    } else {
        tracing::info!("🗄️  Using In-Memory storage (development mode)");
    }

    tracing::warn!("⚠️  Data will not persist between restarts");
    tracing::info!("💡 PostgreSQL support coming soon");

    Ok(StorageType::InMemory(Arc::new(Mutex::new(
        InMemoryStorage::new(),
    ))))
}

/// Run database migrations (PostgreSQL only)
/// TEMPORARILY DISABLED - PostgreSQL implementation needs type fixes
pub async fn run_migrations(_database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    tracing::warn!("⚠️  Database migrations temporarily disabled");
    tracing::info!("💡 PostgreSQL support will be re-enabled after fixing type mismatches");
    Ok(())
}
