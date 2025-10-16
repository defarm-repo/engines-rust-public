/// PostgreSQL Persistence Layer with Retry Logic and Lazy Initialization
///
/// This module provides:
/// - Connection pooling with automatic retry
/// - Lazy initialization (won't block server startup)
/// - Migration execution with timeout handling
/// - Circuit breaker pattern for failed connections
/// - Background sync from in-memory to PostgreSQL
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};

use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::{NoTls, Row};
use uuid::Uuid;

use crate::types::*;

/// PostgreSQL persistence manager with circuit breaker
#[derive(Clone)]
pub struct PostgresPersistence {
    pool: Option<Pool>,
    database_url: String,
    connection_state: Arc<tokio::sync::RwLock<ConnectionState>>,
    queue_tx: mpsc::Sender<PersistJob>,
    metrics: Arc<PersistMetrics>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionState {
    Connected,
    Connecting,
    Failed,
    #[allow(dead_code)]
    CircuitOpen,
}

const PERSIST_QUEUE_CAPACITY: usize = 512;

#[derive(Debug, Default)]
struct PersistMetrics {
    total_attempts: AtomicU64,
    total_successes: AtomicU64,
    total_failures: AtomicU64,
    total_retries: AtomicU64,
}

impl PersistMetrics {
    fn snapshot(&self) -> PersistMetricsSnapshot {
        PersistMetricsSnapshot {
            total_attempts: self.total_attempts.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_retries: self.total_retries.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PersistMetricsSnapshot {
    pub total_attempts: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_retries: u64,
}

#[derive(Clone)]
enum PersistCommand {
    Circuit(Circuit),
    User(UserAccount),
    Item(Item),
    Event(Event),
    LidMapping { local_id: Uuid, dfid: String },
    CircuitOperation(CircuitOperation),
    Activity(Activity),
    StorageRecord { dfid: String, record: StorageRecord },
    AdapterConfig(AdapterConfig),
    WebhookConfig(WebhookConfig),
}

struct PersistJob {
    operation: &'static str,
    command: Arc<PersistCommand>,
    response: oneshot::Sender<Result<(), String>>,
}

impl PostgresPersistence {
    /// Create a new PostgreSQL persistence layer
    /// This does NOT connect immediately - connection is lazy
    pub fn new(database_url: String) -> Self {
        let (queue_tx, queue_rx) = mpsc::channel(PERSIST_QUEUE_CAPACITY);
        let persistence = Self {
            pool: None,
            database_url,
            connection_state: Arc::new(tokio::sync::RwLock::new(ConnectionState::Connecting)),
            queue_tx: queue_tx.clone(),
            metrics: Arc::new(PersistMetrics::default()),
        };

        if tokio::runtime::Handle::try_current().is_ok() {
            let worker = persistence.clone();
            tokio::spawn(async move {
                worker.queue_worker(queue_rx).await;
            });
        } else {
            tracing::warn!(
                "Tokio runtime not initialized; persistence queue worker not started. \
                 Persistence operations will run inline if needed."
            );
        }

        persistence
    }

    /// Initialize the connection pool with retry logic
    /// This can be called in the background without blocking server startup
    pub async fn connect(&mut self) -> Result<(), String> {
        tracing::info!("🗄️  Attempting to connect to PostgreSQL...");

        let max_retries = 5;
        let mut retry_delay = Duration::from_secs(1);

        for attempt in 1..=max_retries {
            match self.try_connect().await {
                Ok(pool) => {
                    self.pool = Some(pool);
                    *self.connection_state.write().await = ConnectionState::Connected;
                    tracing::info!(
                        "✅ PostgreSQL connected successfully on attempt {}",
                        attempt
                    );

                    // Run migrations
                    if let Err(e) = self.run_migrations().await {
                        tracing::error!("❌ Migration failed: {}", e);
                        return Err(format!("Migration failed: {e}"));
                    }

                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️  PostgreSQL connection attempt {} failed: {}",
                        attempt,
                        e
                    );

                    if attempt < max_retries {
                        tracing::info!("🔄 Retrying in {:?}...", retry_delay);
                        sleep(retry_delay).await;
                        retry_delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        *self.connection_state.write().await = ConnectionState::Failed;
        Err(format!(
            "Failed to connect to PostgreSQL after {max_retries} attempts"
        ))
    }

    /// Try to establish a connection pool
    async fn try_connect(&self) -> Result<Pool, String> {
        let config = self
            .database_url
            .parse::<tokio_postgres::Config>()
            .map_err(|e| format!("Invalid database URL: {e}"))?;

        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, manager_config);

        let pool = Pool::builder(manager)
            .max_size(16)
            .wait_timeout(Some(Duration::from_secs(5)))
            .create_timeout(Some(Duration::from_secs(10)))
            .recycle_timeout(Some(Duration::from_secs(5)))
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| format!("Failed to create pool: {e}"))?;

        // Test the connection
        let client = timeout(Duration::from_secs(5), pool.get())
            .await
            .map_err(|_| "Connection timeout".to_string())?
            .map_err(|e| format!("Failed to get test connection: {e}"))?;

        // Verify connection works
        client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| format!("Connection test failed: {e}"))?;

        Ok(pool)
    }

    /// Run database migrations with timeout
    pub async fn run_migrations(&self) -> Result<(), String> {
        tracing::info!("🔄 Running database migrations...");

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| "PostgreSQL not connected".to_string())?;

        let client = timeout(Duration::from_secs(10), pool.get())
            .await
            .map_err(|_| "Migration connection timeout".to_string())?
            .map_err(|e| format!("Failed to get connection for migration: {e}"))?;

        // Read migration file
        let migration_sql = include_str!("../migrations/V1__initial_schema.sql");

        // Execute migration with timeout
        match timeout(Duration::from_secs(30), client.batch_execute(migration_sql)).await {
            Ok(Ok(_)) => {
                tracing::info!("✅ Database migrations completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                // Check if error is "already exists" which is okay
                if e.to_string().contains("already exists") {
                    tracing::info!("ℹ️  Database schema already exists, skipping migration");
                    Ok(())
                } else {
                    tracing::error!("❌ Migration failed: {}", e);
                    Err(format!("Migration failed: {e}"))
                }
            }
            Err(_) => {
                tracing::error!("❌ Migration timed out after 30 seconds");
                Err("Migration timeout".to_string())
            }
        }
    }

    /// Check if PostgreSQL is connected and operational
    pub async fn is_connected(&self) -> bool {
        let state = *self.connection_state.read().await;
        state == ConnectionState::Connected && self.pool.is_some()
    }

    /// Get connection status for health checks
    pub async fn get_status(&self) -> (String, String) {
        let state = *self.connection_state.read().await;

        let status = match state {
            ConnectionState::Connected => "connected",
            ConnectionState::Connecting => "connecting",
            ConnectionState::Failed => "failed",
            ConnectionState::CircuitOpen => "circuit_open",
        };

        let message = match state {
            ConnectionState::Connected => "PostgreSQL operational",
            ConnectionState::Connecting => "Connecting to PostgreSQL...",
            ConnectionState::Failed => "PostgreSQL connection failed",
            ConnectionState::CircuitOpen => "PostgreSQL circuit breaker open - too many failures",
        };

        (status.to_string(), message.to_string())
    }

    /// Get a database client from the pool with timeout
    async fn get_client(&self) -> Result<deadpool_postgres::Client, String> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| "PostgreSQL not connected".to_string())?;

        timeout(Duration::from_secs(5), pool.get())
            .await
            .map_err(|_| "Connection pool timeout".to_string())?
            .map_err(|e| format!("Failed to get connection: {e}"))
    }

    async fn execute_with_retry<F, Fut, T>(&self, operation: &str, mut task: F) -> Result<T, String>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, String>> + Send,
    {
        const MAX_ATTEMPTS: u8 = 3;
        let mut attempt: u8 = 0;
        let mut last_error: Option<String> = None;

        self.metrics.total_attempts.fetch_add(1, Ordering::Relaxed);

        while attempt < MAX_ATTEMPTS {
            attempt += 1;

            match task().await {
                Ok(result) => {
                    self.metrics.total_successes.fetch_add(1, Ordering::Relaxed);
                    if attempt > 1 {
                        self.metrics
                            .total_retries
                            .fetch_add((attempt as u64) - 1, Ordering::Relaxed);
                        tracing::info!("✅ {} succeeded on attempt {}", operation, attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    last_error = Some(err.clone());

                    if attempt >= MAX_ATTEMPTS {
                        self.metrics.total_failures.fetch_add(1, Ordering::Relaxed);
                        tracing::error!(
                            "❌ {} failed after {} attempts: {}",
                            operation,
                            attempt,
                            err
                        );
                        return Err(err);
                    }

                    self.metrics.total_retries.fetch_add(1, Ordering::Relaxed);

                    let backoff = Duration::from_millis((attempt as u64).pow(2) * 100);
                    tracing::warn!(
                        "⚠️  {} attempt {} failed: {}. Retrying in {:?}",
                        operation,
                        attempt,
                        err,
                        backoff
                    );
                    sleep(backoff).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Unknown persistence error".to_string()))
    }

    pub fn metrics_snapshot(&self) -> PersistMetricsSnapshot {
        self.metrics.snapshot()
    }

    async fn enqueue_persist(
        &self,
        operation: &'static str,
        command: PersistCommand,
    ) -> Result<(), String> {
        let (response_tx, response_rx) = oneshot::channel();
        let command_arc = Arc::new(command);
        let job = PersistJob {
            operation,
            command: command_arc.clone(),
            response: response_tx,
        };

        if self.queue_tx.send(job).await.is_err() {
            tracing::warn!(
                "Persistence queue unavailable for {} — executing inline",
                operation
            );
            let persistence_clone = self.clone();
            return self
                .execute_with_retry(operation, move || {
                    let this = persistence_clone.clone();
                    let command = command_arc.clone();
                    async move { this.process_command_once(command.as_ref()).await }
                })
                .await;
        }

        match response_rx.await {
            Ok(result) => result,
            Err(_) => {
                tracing::error!(
                    "Persistence worker dropped response channel for {}",
                    operation
                );
                Err("Persistence worker unavailable".to_string())
            }
        }
    }

    async fn process_command_once(&self, command: &PersistCommand) -> Result<(), String> {
        match command {
            PersistCommand::Circuit(circuit) => self.persist_circuit_once(circuit).await,
            PersistCommand::User(user) => self.persist_user_once(user).await,
            PersistCommand::Item(item) => self.persist_item_once(item).await,
            PersistCommand::Event(event) => self.persist_event_once(event).await,
            PersistCommand::LidMapping { local_id, dfid } => {
                self.persist_lid_dfid_mapping_once(local_id, dfid).await
            }
            PersistCommand::CircuitOperation(operation) => {
                self.persist_circuit_operation_once(operation).await
            }
            PersistCommand::Activity(activity) => self.persist_activity_once(activity).await,
            PersistCommand::StorageRecord { dfid, record } => {
                self.persist_storage_record_once(dfid, record).await
            }
            PersistCommand::AdapterConfig(config) => self.persist_adapter_config_once(config).await,
            PersistCommand::WebhookConfig(webhook) => {
                self.persist_webhook_config_once(webhook).await
            }
        }
    }

    async fn queue_worker(self, mut rx: mpsc::Receiver<PersistJob>) {
        while let Some(job) = rx.recv().await {
            let PersistJob {
                operation,
                command,
                response,
            } = job;

            let this = self.clone();
            let command_clone = command.clone();

            let result = self
                .execute_with_retry(operation, move || {
                    let this = this.clone();
                    let command = command_clone.clone();
                    async move { this.process_command_once(command.as_ref()).await }
                })
                .await;

            if let Err(ref err) = result {
                tracing::error!(
                    "❌ Persistence operation {} failed after retries: {}",
                    operation,
                    err
                );
            }

            if response.send(result).is_err() {
                tracing::warn!(
                    "Persistence response receiver dropped for operation {}",
                    operation
                );
            }
        }

        tracing::warn!("Persistence queue worker terminated");
    }
    /// Persist a circuit to PostgreSQL
    pub async fn persist_circuit(&self, circuit: &Circuit) -> Result<(), String> {
        self.enqueue_persist("persist_circuit", PersistCommand::Circuit(circuit.clone()))
            .await
    }

    async fn persist_circuit_once(&self, circuit: &Circuit) -> Result<(), String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let permissions_json = serde_json::to_value(&circuit.permissions)
            .map_err(|e| format!("Failed to serialize permissions: {e}"))?;

        let alias_config_json = circuit
            .alias_config
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| format!("Failed to serialize alias_config: {e}"))?;

        let adapter_config_json = circuit
            .adapter_config
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| format!("Failed to serialize adapter_config: {e}"))?;

        let public_settings_json = circuit
            .public_settings
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| format!("Failed to serialize public_settings: {e}"))?;

        let post_action_json = circuit
            .post_action_settings
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| format!("Failed to serialize post_action_settings: {e}"))?;

        let status_str = match circuit.status {
            CircuitStatus::Active => "Active",
            CircuitStatus::Inactive => "Inactive",
            CircuitStatus::Archived => "Archived",
        };

        client
            .execute(
                "INSERT INTO circuits (
                circuit_id, name, description, owner_id, status,
                created_at_ts, last_modified_ts, permissions,
                alias_config, adapter_config, public_settings, post_action_settings
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (circuit_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                status = EXCLUDED.status,
                last_modified_ts = EXCLUDED.last_modified_ts,
                permissions = EXCLUDED.permissions,
                alias_config = EXCLUDED.alias_config,
                adapter_config = EXCLUDED.adapter_config,
                public_settings = EXCLUDED.public_settings,
                post_action_settings = EXCLUDED.post_action_settings",
                &[
                    &circuit.circuit_id,
                    &circuit.name,
                    &circuit.description,
                    &circuit.owner_id,
                    &status_str,
                    &circuit.created_timestamp.timestamp(),
                    &circuit.last_modified.timestamp(),
                    &permissions_json,
                    &alias_config_json,
                    &adapter_config_json,
                    &public_settings_json,
                    &post_action_json,
                ],
            )
            .await
            .map_err(|e| format!("Failed to persist circuit: {e}"))?;

        tracing::debug!("✅ Persisted circuit {} to PostgreSQL", circuit.circuit_id);
        Ok(())
    }

    /// Load all circuits from PostgreSQL on startup
    pub async fn load_circuits(&self) -> Result<Vec<Circuit>, String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let rows = client
            .query(
                "SELECT circuit_id, name, description, owner_id, status,
                    created_at_ts, last_modified_ts, permissions,
                    alias_config, adapter_config, public_settings, post_action_settings
             FROM circuits
             WHERE status != 'Archived'
             ORDER BY created_at_ts DESC",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to load circuits: {e}"))?;

        let mut circuits = Vec::new();
        for row in rows {
            match self.row_to_circuit(&row) {
                Ok(circuit) => circuits.push(circuit),
                Err(e) => tracing::warn!("⚠️  Failed to parse circuit: {}", e),
            }
        }

        tracing::info!("✅ Loaded {} circuits from PostgreSQL", circuits.len());
        Ok(circuits)
    }

    fn row_to_circuit(&self, row: &Row) -> Result<Circuit, String> {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Active" => CircuitStatus::Active,
            "Inactive" => CircuitStatus::Inactive,
            "Archived" => CircuitStatus::Archived,
            _ => return Err(format!("Invalid circuit status: {status_str}")),
        };

        let permissions_json: serde_json::Value = row.get("permissions");
        let permissions = serde_json::from_value(permissions_json)
            .map_err(|e| format!("Failed to parse permissions: {e}"))?;

        let alias_config: Option<serde_json::Value> = row.get("alias_config");
        let alias_config = alias_config
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| format!("Failed to parse alias_config: {e}"))?;

        let adapter_config: Option<serde_json::Value> = row.get("adapter_config");
        let adapter_config = adapter_config
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| format!("Failed to parse adapter_config: {e}"))?;

        let public_settings: Option<serde_json::Value> = row.get("public_settings");
        let public_settings = public_settings
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| format!("Failed to parse public_settings: {e}"))?;

        let post_action_settings: Option<serde_json::Value> = row.get("post_action_settings");
        let post_action_settings = post_action_settings
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| format!("Failed to parse post_action_settings: {e}"))?;

        let created_at_ts: i64 = row.get("created_at_ts");
        let last_modified_ts: i64 = row.get("last_modified_ts");

        Ok(Circuit {
            circuit_id: row.get("circuit_id"),
            name: row.get("name"),
            description: row.get("description"),
            owner_id: row.get("owner_id"),
            default_namespace: String::new(), // Will be loaded from storage if needed
            alias_config,
            created_timestamp: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
            last_modified: DateTime::from_timestamp(last_modified_ts, 0).unwrap_or_else(Utc::now),
            members: Vec::new(), // Load separately if needed
            permissions,
            status,
            pending_requests: Vec::new(), // Load separately if needed
            custom_roles: Vec::new(),     // Load separately if needed
            public_settings,
            adapter_config,
            post_action_settings,
        })
    }

    /// Persist user account to PostgreSQL
    pub async fn persist_user(&self, user: &UserAccount) -> Result<(), String> {
        self.enqueue_persist("persist_user", PersistCommand::User(user.clone()))
            .await
    }

    async fn persist_user_once(&self, user: &UserAccount) -> Result<(), String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let tier_str = user.tier.as_str();
        let status_str = match user.status {
            AccountStatus::Active => "Active",
            AccountStatus::Suspended => "Suspended",
            AccountStatus::Banned => "Banned",
            AccountStatus::PendingVerification => "PendingVerification",
            AccountStatus::TrialExpired => "TrialExpired",
        };

        // Serialize available adapters for PostgreSQL.
        // If no explicit overrides are set, persist the tier defaults so the table
        // reflects the adapters actually granted to the user.
        let adapters_to_persist: Vec<String> = match user.available_adapters.as_ref() {
            Some(custom) => custom.iter().map(|adapter| adapter.to_string()).collect(),
            None => user
                .limits
                .available_adapters
                .iter()
                .map(|adapter| adapter.to_string())
                .collect(),
        };

        let adapters_array: Option<Vec<String>> = if adapters_to_persist.is_empty() {
            None
        } else {
            Some(adapters_to_persist)
        };

        client
            .execute(
                "INSERT INTO user_accounts (
                user_id, username, email, password_hash, tier, status,
                is_admin, workspace_id, created_at_ts, last_login_ts, available_adapters
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (user_id) DO UPDATE SET
                username = EXCLUDED.username,
                email = EXCLUDED.email,
                password_hash = EXCLUDED.password_hash,
                tier = EXCLUDED.tier,
                status = EXCLUDED.status,
                is_admin = EXCLUDED.is_admin,
                workspace_id = EXCLUDED.workspace_id,
                last_login_ts = EXCLUDED.last_login_ts,
                available_adapters = EXCLUDED.available_adapters,
                updated_at = NOW()",
                &[
                    &user.user_id,
                    &user.username,
                    &user.email,
                    &user.password_hash,
                    &tier_str,
                    &status_str,
                    &user.is_admin,
                    &user.workspace_id,
                    &user.created_at.timestamp(),
                    &user.last_login.map(|t| t.timestamp()),
                    &adapters_array,
                ],
            )
            .await
            .map_err(|e| format!("Failed to persist user: {e}"))?;

        // Also persist credit balance
        client
            .execute(
                "INSERT INTO credit_balances (user_id, credits, updated_at_ts)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET
                credits = EXCLUDED.credits,
                updated_at_ts = EXCLUDED.updated_at_ts",
                &[
                    &user.user_id,
                    &{ user.credits },
                    &Utc::now().timestamp(),
                ],
            )
            .await
            .map_err(|e| format!("Failed to persist credit balance: {e}"))?;

        tracing::debug!("✅ Persisted user {} to PostgreSQL", user.username);
        Ok(())
    }

    /// Load all users from PostgreSQL on startup
    pub async fn load_users(&self) -> Result<Vec<UserAccount>, String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let rows = client.query(
            "SELECT u.user_id, u.username, u.email, u.password_hash, u.tier, u.status,
                    u.is_admin, u.workspace_id, u.created_at_ts, u.last_login_ts, u.available_adapters,
                    COALESCE(c.credits, 0) as credits
             FROM user_accounts u
             LEFT JOIN credit_balances c ON u.user_id = c.user_id
             WHERE u.status != 'Banned'
             ORDER BY u.created_at_ts DESC",
            &[],
        ).await
        .map_err(|e| format!("Failed to load users: {e}"))?;

        let mut users = Vec::new();
        for row in rows {
            match self.row_to_user(&row) {
                Ok(user) => users.push(user),
                Err(e) => tracing::warn!("⚠️  Failed to parse user: {}", e),
            }
        }

        tracing::info!("✅ Loaded {} users from PostgreSQL", users.len());
        Ok(users)
    }

    /// Persist item to PostgreSQL (write-through cache)
    pub async fn persist_item(&self, item: &crate::types::Item) -> Result<(), String> {
        self.enqueue_persist("persist_item", PersistCommand::Item(item.clone()))
            .await
    }

    async fn persist_item_once(&self, item: &crate::types::Item) -> Result<(), String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        // Calculate item hash using BLAKE3
        let item_hash = blake3::hash(item.dfid.as_bytes()).to_hex().to_string();

        // Insert/update main item record
        client.execute(
            "INSERT INTO items (dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (dfid) DO UPDATE SET
                status = EXCLUDED.status,
                last_updated_ts = EXCLUDED.last_updated_ts,
                enriched_data = EXCLUDED.enriched_data,
                updated_at = NOW()",
            &[
                &item.dfid,
                &item_hash,
                &format!("{:?}", item.status),
                &item.creation_timestamp.timestamp(),
                &item.last_modified.timestamp(),
                &serde_json::to_value(&item.enriched_data).unwrap_or(serde_json::Value::Null),
            ],
        ).await
        .map_err(|e| format!("Failed to persist item: {e}"))?;

        // Insert identifiers (delete old ones first)
        client
            .execute(
                "DELETE FROM item_identifiers WHERE dfid = $1",
                &[&item.dfid],
            )
            .await
            .map_err(|e| format!("Failed to delete old identifiers: {e}"))?;

        for identifier in &item.identifiers {
            client
                .execute(
                    "INSERT INTO item_identifiers (dfid, key, value) VALUES ($1, $2, $3)",
                    &[&item.dfid, &identifier.key, &identifier.value],
                )
                .await
                .map_err(|e| format!("Failed to insert identifier: {e}"))?;
        }

        // Insert source entries (delete old ones first)
        client
            .execute(
                "DELETE FROM item_source_entries WHERE dfid = $1",
                &[&item.dfid],
            )
            .await
            .map_err(|e| format!("Failed to delete old source entries: {e}"))?;

        for entry_id in &item.source_entries {
            client
                .execute(
                    "INSERT INTO item_source_entries (dfid, entry_id) VALUES ($1, $2)",
                    &[&item.dfid, entry_id],
                )
                .await
                .map_err(|e| format!("Failed to insert source entry: {e}"))?;
        }

        // Insert LID mapping if exists
        if let Some(local_id) = item.local_id {
            client
                .execute(
                    "INSERT INTO lid_dfid_mappings (local_id, dfid) VALUES ($1, $2)
                 ON CONFLICT (local_id) DO UPDATE SET dfid = EXCLUDED.dfid",
                    &[&local_id, &item.dfid],
                )
                .await
                .map_err(|e| format!("Failed to insert LID mapping: {e}"))?;
        }

        Ok(())
    }

    /// Persist event to PostgreSQL (write-through cache)
    pub async fn persist_event(&self, event: &crate::types::Event) -> Result<(), String> {
        self.enqueue_persist("persist_event", PersistCommand::Event(event.clone()))
            .await
    }

    async fn persist_event_once(&self, event: &crate::types::Event) -> Result<(), String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        // Serialize encrypted_data if encrypted
        let encrypted_data: Option<Vec<u8>> = if event.is_encrypted {
            // For now, we'll store the content_hash as encrypted_data
            Some(event.content_hash.as_bytes().to_vec())
        } else {
            None
        };

        client.execute(
            "INSERT INTO events (event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (event_id) DO UPDATE SET
                event_type = EXCLUDED.event_type,
                dfid = EXCLUDED.dfid,
                timestamp = EXCLUDED.timestamp,
                visibility = EXCLUDED.visibility,
                encrypted_data = EXCLUDED.encrypted_data,
                metadata = EXCLUDED.metadata",
            &[
                &event.event_id,
                &format!("{:?}", event.event_type),
                &event.dfid,
                &event.timestamp.timestamp(),
                &format!("{:?}", event.visibility),
                &encrypted_data,
                &serde_json::to_value(&event.metadata).unwrap_or(serde_json::Value::Null),
            ],
        ).await
        .map_err(|e| format!("Failed to persist event: {e}"))?;

        Ok(())
    }

    fn row_to_user(&self, row: &Row) -> Result<UserAccount, String> {
        let tier_str: String = row.get("tier");
        let tier = UserTier::from_str(&tier_str)?;

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Active" => AccountStatus::Active,
            "Suspended" => AccountStatus::Suspended,
            "Banned" => AccountStatus::Banned,
            "PendingVerification" => AccountStatus::PendingVerification,
            "TrialExpired" => AccountStatus::TrialExpired,
            _ => return Err(format!("Invalid account status: {status_str}")),
        };

        let created_at_ts: i64 = row.get("created_at_ts");
        let last_login_ts: Option<i64> = row.get("last_login_ts");
        let credits: i64 = row.get("credits");

        // Calculate limits before moving tier
        let limits = TierLimits::for_tier(&tier);
        let default_adapters = limits.available_adapters.clone();

        // Deserialize available_adapters from TEXT array
        // Parse adapter strings back to AdapterType enum using from_string()
        let adapters_str: Option<Vec<String>> = row.get("available_adapters");
        let available_adapters: Option<Vec<AdapterType>> = adapters_str.and_then(|arr| {
            if arr.is_empty() {
                None // Empty array means use tier defaults
            } else {
                let parsed: Vec<AdapterType> = arr
                    .iter()
                    .filter_map(|s| {
                        AdapterType::from_string(s)
                            .map_err(|e| {
                                tracing::warn!("Failed to parse adapter type '{}': {}", s, e);
                                e
                            })
                            .ok()
                    })
                    .collect();

                if parsed.is_empty() || parsed == default_adapters {
                    None // All parsing failed or matches tier defaults, use tier defaults
                } else {
                    Some(parsed)
                }
            }
        });

        Ok(UserAccount {
            user_id: row.get("user_id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            tier,
            status,
            credits,
            created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
            updated_at: Utc::now(),
            last_login: last_login_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            subscription: None,
            limits,
            is_admin: row.get("is_admin"),
            workspace_id: row.get("workspace_id"),
            available_adapters, // Now properly parsed from PostgreSQL
        })
    }

    // ========================================================================
    // LID-DFID MAPPINGS PERSISTENCE
    // ========================================================================

    /// Persist LID-DFID mapping to PostgreSQL
    pub async fn persist_lid_dfid_mapping(
        &self,
        local_id: &Uuid,
        dfid: &str,
    ) -> Result<(), String> {
        self.enqueue_persist(
            "persist_lid_dfid_mapping",
            PersistCommand::LidMapping {
                local_id: *local_id,
                dfid: dfid.to_string(),
            },
        )
        .await
    }

    async fn persist_lid_dfid_mapping_once(
        &self,
        local_id: &Uuid,
        dfid: &str,
    ) -> Result<(), String> {
        let client = self.get_client().await?;

        client
            .execute(
                "INSERT INTO lid_dfid_mappings (local_id, dfid, created_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (local_id) DO UPDATE
             SET dfid = EXCLUDED.dfid",
                &[&local_id, &dfid],
            )
            .await
            .map_err(|e| format!("Failed to persist LID-DFID mapping: {e}"))?;

        tracing::debug!(
            "✅ Persisted LID-DFID mapping {} -> {} to PostgreSQL",
            local_id,
            dfid
        );
        Ok(())
    }

    /// Load LID-DFID mappings from PostgreSQL
    pub async fn load_lid_dfid_mappings(&self) -> Result<Vec<(Uuid, String)>, String> {
        let client = self.get_client().await?;

        let rows = client
            .query("SELECT local_id, dfid FROM lid_dfid_mappings", &[])
            .await
            .map_err(|e| format!("Failed to load LID-DFID mappings: {e}"))?;

        let mappings = rows
            .iter()
            .map(|row| {
                let local_id: Uuid = row.get("local_id");
                let dfid: String = row.get("dfid");
                (local_id, dfid)
            })
            .collect();

        Ok(mappings)
    }

    // ========================================================================
    // CIRCUIT OPERATIONS PERSISTENCE
    // ========================================================================

    /// Persist circuit operation to PostgreSQL
    pub async fn persist_circuit_operation(
        &self,
        operation: &CircuitOperation,
    ) -> Result<(), String> {
        self.enqueue_persist(
            "persist_circuit_operation",
            PersistCommand::CircuitOperation(operation.clone()),
        )
        .await
    }

    async fn persist_circuit_operation_once(
        &self,
        operation: &CircuitOperation,
    ) -> Result<(), String> {
        let client = self.get_client().await?;

        // Note: approved_at_ts, approver_id, completed_at_ts exist in schema but not in CircuitOperation struct yet
        // They will be NULL for now until the struct is enhanced with approval tracking
        client.execute(
            "INSERT INTO circuit_operations
             (operation_id, circuit_id, operation_type, requester_id, status, created_at_ts, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW())
             ON CONFLICT (operation_id) DO UPDATE
             SET status = EXCLUDED.status",
            &[
                &operation.operation_id,
                &operation.circuit_id,
                &format!("{:?}", operation.operation_type),
                &operation.requester_id,
                &format!("{:?}", operation.status),
                &operation.timestamp.timestamp(),
            ],
        )
        .await
        .map_err(|e| format!("Failed to persist circuit operation: {e}"))?;

        tracing::debug!(
            "✅ Persisted circuit operation {} to PostgreSQL",
            operation.operation_id
        );
        Ok(())
    }

    /// Load circuit operations from PostgreSQL
    pub async fn load_circuit_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, String> {
        let client = self.get_client().await?;

        let rows = client
            .query(
                "SELECT operation_id, circuit_id, operation_type, requester_id, status, created_at_ts
                 FROM circuit_operations
                 WHERE circuit_id = $1
                 ORDER BY created_at_ts DESC",
                &[&circuit_id],
            )
            .await
            .map_err(|e| format!("Failed to load circuit operations: {e}"))?;

        let operations = rows
            .iter()
            .filter_map(|row| {
                let operation_type_str: String = row.get("operation_type");
                let operation_type = match operation_type_str.as_str() {
                    "Push" => OperationType::Push,
                    "Pull" => OperationType::Pull,
                    _ => return None,
                };

                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Pending" => OperationStatus::Pending,
                    "Approved" => OperationStatus::Approved,
                    "Rejected" => OperationStatus::Rejected,
                    "Completed" => OperationStatus::Completed,
                    _ => return None,
                };

                let created_at_ts: i64 = row.get("created_at_ts");

                Some(CircuitOperation {
                    operation_id: row.get("operation_id"),
                    circuit_id: row.get("circuit_id"),
                    dfid: "".to_string(), // Will be populated from circuit_pending_items if needed
                    operation_type,
                    requester_id: row.get("requester_id"),
                    timestamp: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
                    status,
                    metadata: std::collections::HashMap::new(),
                })
            })
            .collect();

        Ok(operations)
    }

    // ========================================================================
    // ACTIVITIES PERSISTENCE
    // ========================================================================

    /// Persist activity to PostgreSQL
    pub async fn persist_activity(&self, activity: &Activity) -> Result<(), String> {
        self.enqueue_persist(
            "persist_activity",
            PersistCommand::Activity(activity.clone()),
        )
        .await
    }

    async fn persist_activity_once(&self, activity: &Activity) -> Result<(), String> {
        let client = self.get_client().await?;

        let details_json = serde_json::to_value(&activity.details)
            .map_err(|e| format!("Failed to serialize activity details: {e}"))?;

        let activity_id_uuid =
            Uuid::parse_str(&activity.activity_id).unwrap_or_else(|_| Uuid::new_v4());

        client
            .execute(
                "INSERT INTO activities
             (activity_id, activity_type, circuit_id, circuit_name, dfids,
              performed_by, status, details, timestamp_ts, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
             ON CONFLICT (activity_id) DO UPDATE
             SET status = EXCLUDED.status",
                &[
                    &activity_id_uuid,
                    &format!("{:?}", activity.activity_type),
                    &activity.circuit_id,
                    &activity.circuit_name,
                    &activity.item_dfids,
                    &activity.user_id,
                    &format!("{:?}", activity.status),
                    &details_json,
                    &activity.timestamp.timestamp(),
                ],
            )
            .await
            .map_err(|e| format!("Failed to persist activity: {e}"))?;

        tracing::debug!(
            "✅ Persisted activity {} to PostgreSQL",
            activity.activity_id
        );
        Ok(())
    }

    /// Load activities from PostgreSQL
    pub async fn load_activities(
        &self,
        circuit_id: Option<&Uuid>,
    ) -> Result<Vec<Activity>, String> {
        let client = self.get_client().await?;

        let rows = if let Some(cid) = circuit_id {
            client
                .query(
                    "SELECT activity_id, activity_type, circuit_id, circuit_name, dfids,
                            performed_by, status, details, timestamp_ts
                     FROM activities
                     WHERE circuit_id = $1
                     ORDER BY timestamp_ts DESC
                     LIMIT 1000",
                    &[&cid],
                )
                .await
        } else {
            client
                .query(
                    "SELECT activity_id, activity_type, circuit_id, circuit_name, dfids,
                            performed_by, status, details, timestamp_ts
                     FROM activities
                     ORDER BY timestamp_ts DESC
                     LIMIT 1000",
                    &[],
                )
                .await
        }
        .map_err(|e| format!("Failed to load activities: {e}"))?;

        let activities = rows
            .iter()
            .filter_map(|row| {
                let activity_id_uuid: Uuid = row.get("activity_id");
                let activity_type_str: String = row.get("activity_type");
                let activity_type = match activity_type_str.as_str() {
                    "Push" => ActivityType::Push,
                    "Pull" => ActivityType::Pull,
                    "Enrich" => ActivityType::Enrich,
                    _ => return None,
                };

                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Success" => ActivityStatus::Success,
                    "Partial" => ActivityStatus::Partial,
                    "Failed" => ActivityStatus::Failed,
                    _ => return None,
                };

                let details_json: serde_json::Value = row.get("details");
                let details: ActivityDetails = serde_json::from_value(details_json).ok()?;

                let timestamp_ts: i64 = row.get("timestamp_ts");

                Some(Activity {
                    activity_id: activity_id_uuid.to_string(),
                    activity_type,
                    circuit_id: row.get("circuit_id"),
                    circuit_name: row.get("circuit_name"),
                    item_dfids: row.get("dfids"),
                    user_id: row.get("performed_by"),
                    timestamp: DateTime::from_timestamp(timestamp_ts, 0).unwrap_or_else(Utc::now),
                    status,
                    details,
                })
            })
            .collect();

        Ok(activities)
    }

    // ========================================================================
    // STORAGE RECORDS PERSISTENCE
    // ========================================================================

    /// Persist storage record to PostgreSQL
    pub async fn persist_storage_record(
        &self,
        dfid: &str,
        record: &StorageRecord,
    ) -> Result<(), String> {
        self.enqueue_persist(
            "persist_storage_record",
            PersistCommand::StorageRecord {
                dfid: dfid.to_string(),
                record: record.clone(),
            },
        )
        .await
    }

    async fn persist_storage_record_once(
        &self,
        dfid: &str,
        record: &StorageRecord,
    ) -> Result<(), String> {
        let client = self.get_client().await?;

        let storage_location_json = serde_json::to_value(&record.storage_location)
            .map_err(|e| format!("Failed to serialize storage location: {e}"))?;

        let metadata_json = serde_json::to_value(&record.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {e}"))?;

        let (events_range_start, events_range_end) = record
            .events_range
            .map(|(start, end)| (Some(start.timestamp()), end.map(|e| e.timestamp())))
            .unwrap_or((None, None));

        client.execute(
            "INSERT INTO storage_history
             (dfid, adapter_type, storage_location, stored_at_ts, triggered_by,
              triggered_by_id, events_range_start, events_range_end, is_active, metadata, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())",
            &[
                &dfid,
                &format!("{:?}", record.adapter_type),
                &storage_location_json,
                &record.stored_at.timestamp(),
                &record.triggered_by,
                &record.triggered_by_id,
                &events_range_start,
                &events_range_end,
                &record.is_active,
                &metadata_json,
            ],
        )
        .await
        .map_err(|e| format!("Failed to persist storage record: {e}"))?;

        tracing::debug!("✅ Persisted storage record for {} to PostgreSQL", dfid);
        Ok(())
    }

    /// Load storage records for a DFID from PostgreSQL
    pub async fn load_storage_records(&self, dfid: &str) -> Result<Vec<StorageRecord>, String> {
        let client = self.get_client().await?;

        let rows = client
            .query(
                "SELECT adapter_type, storage_location, stored_at_ts, triggered_by,
                        triggered_by_id, events_range_start, events_range_end, is_active, metadata
                 FROM storage_history
                 WHERE dfid = $1
                 ORDER BY stored_at_ts DESC",
                &[&dfid],
            )
            .await
            .map_err(|e| format!("Failed to load storage records: {e}"))?;

        let records = rows
            .iter()
            .filter_map(|row| {
                let adapter_type_str: String = row.get("adapter_type");
                let adapter_type = match adapter_type_str.as_str() {
                    "IpfsIpfs" => AdapterType::IpfsIpfs,
                    "StellarTestnetIpfs" => AdapterType::StellarTestnetIpfs,
                    "StellarMainnetIpfs" => AdapterType::StellarMainnetIpfs,
                    "EthereumGoerliIpfs" => AdapterType::EthereumGoerliIpfs,
                    "PolygonArweave" => AdapterType::PolygonArweave,
                    _ => return None,
                };

                let storage_location_json: serde_json::Value = row.get("storage_location");
                let storage_location = serde_json::from_value(storage_location_json).ok()?;

                let stored_at_ts: i64 = row.get("stored_at_ts");
                let events_range_start: Option<i64> = row.get("events_range_start");
                let events_range_end: Option<i64> = row.get("events_range_end");

                let events_range = match (events_range_start, events_range_end) {
                    (Some(start), end) => Some((
                        DateTime::from_timestamp(start, 0)?,
                        end.and_then(|e| DateTime::from_timestamp(e, 0)),
                    )),
                    _ => None,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json).ok()?;

                Some(StorageRecord {
                    adapter_type,
                    storage_location,
                    stored_at: DateTime::from_timestamp(stored_at_ts, 0)?,
                    triggered_by: row.get("triggered_by"),
                    triggered_by_id: row.get("triggered_by_id"),
                    events_range,
                    is_active: row.get("is_active"),
                    metadata,
                })
            })
            .collect();

        Ok(records)
    }

    // ========================================================================
    // ADAPTER CONFIGS PERSISTENCE
    // ========================================================================

    /// Persist adapter config to PostgreSQL
    pub async fn persist_adapter_config(&self, config: &AdapterConfig) -> Result<(), String> {
        self.enqueue_persist(
            "persist_adapter_config",
            PersistCommand::AdapterConfig(config.clone()),
        )
        .await
    }

    async fn persist_adapter_config_once(&self, config: &AdapterConfig) -> Result<(), String> {
        let client = self.get_client().await?;

        let connection_details_json = serde_json::to_value(&config.connection_details)
            .map_err(|e| format!("Failed to serialize connection details: {e}"))?;

        let contract_configs_json = config
            .contract_configs
            .as_ref()
            .and_then(|c| serde_json::to_value(c).ok());

        client
            .execute(
                "INSERT INTO adapter_configs
             (config_id, name, description, adapter_type, connection_details,
              contract_configs, is_active, is_default, created_by,
              created_at_ts, updated_at_ts, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
             ON CONFLICT (config_id) DO UPDATE
             SET name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 connection_details = EXCLUDED.connection_details,
                 contract_configs = EXCLUDED.contract_configs,
                 is_active = EXCLUDED.is_active,
                 is_default = EXCLUDED.is_default,
                 updated_at_ts = EXCLUDED.updated_at_ts,
                 updated_at = NOW()",
                &[
                    &config.config_id,
                    &config.name,
                    &config.description,
                    &format!("{:?}", config.adapter_type),
                    &connection_details_json,
                    &contract_configs_json,
                    &config.is_active,
                    &config.is_default,
                    &config.created_by,
                    &config.created_at.timestamp(),
                    &config.updated_at.timestamp(),
                ],
            )
            .await
            .map_err(|e| format!("Failed to persist adapter config: {e}"))?;

        tracing::debug!(
            "✅ Persisted adapter config {} to PostgreSQL",
            config.config_id
        );
        Ok(())
    }

    /// Load adapter configs from PostgreSQL
    pub async fn load_adapter_configs(&self) -> Result<Vec<AdapterConfig>, String> {
        let client = self.get_client().await?;

        let rows = client
            .query(
                "SELECT config_id, name, description, adapter_type, connection_details,
                        contract_configs, is_active, is_default, created_by,
                        created_at_ts, updated_at_ts
                 FROM adapter_configs
                 ORDER BY created_at_ts DESC",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to load adapter configs: {e}"))?;

        let configs = rows
            .iter()
            .filter_map(|row| {
                let adapter_type_str: String = row.get("adapter_type");
                let adapter_type = match adapter_type_str.as_str() {
                    "IpfsIpfs" => AdapterType::IpfsIpfs,
                    "StellarTestnetIpfs" => AdapterType::StellarTestnetIpfs,
                    "StellarMainnetIpfs" => AdapterType::StellarMainnetIpfs,
                    "EthereumGoerliIpfs" => AdapterType::EthereumGoerliIpfs,
                    "PolygonArweave" => AdapterType::PolygonArweave,
                    _ => return None,
                };

                let connection_details_json: serde_json::Value = row.get("connection_details");
                let connection_details = serde_json::from_value(connection_details_json).ok()?;

                let contract_configs_json: Option<serde_json::Value> = row.get("contract_configs");
                let contract_configs =
                    contract_configs_json.and_then(|j| serde_json::from_value(j).ok());

                let created_at_ts: i64 = row.get("created_at_ts");
                let updated_at_ts: i64 = row.get("updated_at_ts");

                Some(AdapterConfig {
                    config_id: row.get("config_id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    adapter_type,
                    connection_details,
                    contract_configs,
                    is_active: row.get("is_active"),
                    is_default: row.get("is_default"),
                    created_by: row.get("created_by"),
                    created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
                    updated_at: DateTime::from_timestamp(updated_at_ts, 0).unwrap_or_else(Utc::now),
                    last_tested_at: None,
                    test_status: None,
                })
            })
            .collect();

        Ok(configs)
    }

    // ========================================================================
    // WEBHOOK CONFIGS PERSISTENCE
    // ========================================================================

    /// Persist webhook config to PostgreSQL
    pub async fn persist_webhook_config(&self, webhook: &WebhookConfig) -> Result<(), String> {
        self.enqueue_persist(
            "persist_webhook_config",
            PersistCommand::WebhookConfig(webhook.clone()),
        )
        .await
    }

    async fn persist_webhook_config_once(&self, webhook: &WebhookConfig) -> Result<(), String> {
        let client = self.get_client().await?;

        let auth_config_json = serde_json::to_value(&webhook.auth_type)
            .map_err(|e| format!("Failed to serialize auth config: {e}"))?;

        let retry_config_json = serde_json::to_value(&webhook.retry_config)
            .map_err(|e| format!("Failed to serialize retry config: {e}"))?;

        client.execute(
            "INSERT INTO webhook_configs
             (webhook_id, circuit_id, name, url, enabled, trigger_events,
              auth_type, auth_config, retry_config, created_at_ts, updated_at_ts, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
             ON CONFLICT (webhook_id) DO UPDATE
             SET name = EXCLUDED.name,
                 url = EXCLUDED.url,
                 enabled = EXCLUDED.enabled,
                 trigger_events = EXCLUDED.trigger_events,
                 auth_type = EXCLUDED.auth_type,
                 auth_config = EXCLUDED.auth_config,
                 retry_config = EXCLUDED.retry_config,
                 updated_at_ts = EXCLUDED.updated_at_ts,
                 updated_at = NOW()",
            &[
                &webhook.id,
                &Uuid::nil(), // Placeholder - circuit_id should be passed as parameter in real implementation
                &webhook.name,
                &webhook.url,
                &webhook.enabled,
                &vec![format!("{:?}", webhook.method)], // Simplified - trigger_events needs proper handling
                &format!("{:?}", webhook.auth_type),
                &auth_config_json,
                &retry_config_json,
                &webhook.created_at.timestamp(),
                &webhook.updated_at.timestamp(),
            ],
        )
        .await
        .map_err(|e| format!("Failed to persist webhook config: {e}"))?;

        tracing::debug!("✅ Persisted webhook config {} to PostgreSQL", webhook.id);
        Ok(())
    }

    /// Load webhook configs for a circuit from PostgreSQL
    pub async fn load_webhook_configs(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<WebhookConfig>, String> {
        let client = self.get_client().await?;

        let rows = client
            .query(
                "SELECT webhook_id, name, url, enabled, trigger_events,
                        auth_type, auth_config, retry_config, created_at_ts, updated_at_ts
                 FROM webhook_configs
                 WHERE circuit_id = $1
                 ORDER BY created_at_ts DESC",
                &[&circuit_id],
            )
            .await
            .map_err(|e| format!("Failed to load webhook configs: {e}"))?;

        let webhooks = rows
            .iter()
            .filter_map(|row| {
                let auth_type_str: String = row.get("auth_type");
                let auth_type = match auth_type_str.as_str() {
                    "None" => WebhookAuthType::None,
                    "BearerToken" => WebhookAuthType::BearerToken,
                    "ApiKey" => WebhookAuthType::ApiKey,
                    "BasicAuth" => WebhookAuthType::BasicAuth,
                    "CustomHeader" => WebhookAuthType::CustomHeader,
                    _ => return None,
                };

                let retry_config_json: serde_json::Value = row.get("retry_config");
                let retry_config = serde_json::from_value(retry_config_json).ok()?;

                let created_at_ts: i64 = row.get("created_at_ts");
                let updated_at_ts: i64 = row.get("updated_at_ts");

                Some(WebhookConfig {
                    id: row.get("webhook_id"),
                    name: row.get("name"),
                    url: row.get("url"),
                    method: HttpMethod::Post, // Default to POST
                    headers: std::collections::HashMap::new(),
                    auth_type,
                    auth_credentials: None,
                    enabled: row.get("enabled"),
                    retry_config,
                    created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
                    updated_at: DateTime::from_timestamp(updated_at_ts, 0).unwrap_or_else(Utc::now),
                })
            })
            .collect();

        Ok(webhooks)
    }
}
