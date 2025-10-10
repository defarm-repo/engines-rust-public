/// Storage Factory
/// Creates the appropriate storage backend based on environment configuration
use std::sync::{Arc, Mutex};
use std::env;

use crate::storage::{StorageBackend, InMemoryStorage};
use crate::postgres_storage::PostgresStorage;

pub enum StorageType {
    InMemory(Arc<Mutex<InMemoryStorage>>),
    Postgres(Arc<Mutex<PostgresStorage>>),
}

impl StorageType {
    pub fn as_backend(&self) -> &dyn StorageBackend {
        match self {
            StorageType::InMemory(storage) => {
                // This is a bit tricky - we need to return a reference to something
                // that implements StorageBackend. The Arc<Mutex<InMemoryStorage>>
                // implements StorageBackend, so we can return a reference to it.
                storage as &dyn StorageBackend
            }
            StorageType::Postgres(storage) => {
                storage as &dyn StorageBackend
            }
        }
    }
}

/// Create storage backend based on DATABASE_URL environment variable
/// - If DATABASE_URL is set: Use PostgreSQL
/// - If DATABASE_URL is not set: Use In-Memory storage (for development)
pub async fn create_storage() -> Result<StorageType, Box<dyn std::error::Error>> {
    match env::var("DATABASE_URL") {
        Ok(database_url) => {
            tracing::info!("🗄️  Using PostgreSQL storage: {}",
                database_url.split('@').last().unwrap_or("database"));

            let postgres = PostgresStorage::new(&database_url).await?;

            tracing::info!("✅ PostgreSQL connection established");

            Ok(StorageType::Postgres(Arc::new(Mutex::new(postgres))))
        }
        Err(_) => {
            tracing::info!("🗄️  Using In-Memory storage (development mode)");
            tracing::warn!("⚠️  Data will not persist between restarts");
            tracing::info!("💡 Set DATABASE_URL environment variable to use PostgreSQL");

            Ok(StorageType::InMemory(Arc::new(Mutex::new(InMemoryStorage::new()))))
        }
    }
}

/// Run database migrations (PostgreSQL only)
pub async fn run_migrations(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use refinery::embed_migrations;

    embed_migrations!("migrations");

    let (client, connection) = tokio_postgres::connect(database_url, tokio_postgres::NoTls).await?;

    // Spawn the connection to run in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("PostgreSQL connection error: {}", e);
        }
    });

    // Run migrations
    tracing::info!("🔄 Running database migrations...");

    let mut client_wrapper = refinery::postgres::Client::new(client);
    migrations::runner().run_async(&mut client_wrapper).await?;

    tracing::info!("✅ Database migrations complete");

    Ok(())
}
