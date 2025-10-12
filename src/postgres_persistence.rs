/// PostgreSQL Persistence Layer with Retry Logic and Lazy Initialization
///
/// This module provides:
/// - Connection pooling with automatic retry
/// - Lazy initialization (won't block server startup)
/// - Migration execution with timeout handling
/// - Circuit breaker pattern for failed connections
/// - Background sync from in-memory to PostgreSQL

use tokio_postgres::{NoTls, Error as PgError, Row};
use deadpool_postgres::{Pool, Manager, ManagerConfig, RecyclingMethod, Runtime, CreatePoolError};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{timeout, sleep};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::types::*;
use crate::storage::{StorageBackend, StorageError, InMemoryStorage};

/// PostgreSQL persistence manager with circuit breaker
pub struct PostgresPersistence {
    pool: Option<Pool>,
    database_url: String,
    connection_state: Arc<tokio::sync::RwLock<ConnectionState>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionState {
    Connected,
    Connecting,
    Failed,
    CircuitOpen,
}

impl PostgresPersistence {
    /// Create a new PostgreSQL persistence layer
    /// This does NOT connect immediately - connection is lazy
    pub fn new(database_url: String) -> Self {
        Self {
            pool: None,
            database_url,
            connection_state: Arc::new(tokio::sync::RwLock::new(ConnectionState::Connecting)),
        }
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
                    tracing::info!("✅ PostgreSQL connected successfully on attempt {}", attempt);

                    // Run migrations
                    if let Err(e) = self.run_migrations().await {
                        tracing::error!("❌ Migration failed: {}", e);
                        return Err(format!("Migration failed: {}", e));
                    }

                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("⚠️  PostgreSQL connection attempt {} failed: {}", attempt, e);

                    if attempt < max_retries {
                        tracing::info!("🔄 Retrying in {:?}...", retry_delay);
                        sleep(retry_delay).await;
                        retry_delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        *self.connection_state.write().await = ConnectionState::Failed;
        Err(format!("Failed to connect to PostgreSQL after {} attempts", max_retries))
    }

    /// Try to establish a connection pool
    async fn try_connect(&self) -> Result<Pool, String> {
        let config = self.database_url.parse::<tokio_postgres::Config>()
            .map_err(|e| format!("Invalid database URL: {}", e))?;

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
            .map_err(|e| format!("Failed to create pool: {}", e))?;

        // Test the connection
        let client = timeout(Duration::from_secs(5), pool.get())
            .await
            .map_err(|_| "Connection timeout".to_string())?
            .map_err(|e| format!("Failed to get test connection: {}", e))?;

        // Verify connection works
        client.execute("SELECT 1", &[])
            .await
            .map_err(|e| format!("Connection test failed: {}", e))?;

        Ok(pool)
    }

    /// Run database migrations with timeout
    pub async fn run_migrations(&self) -> Result<(), String> {
        tracing::info!("🔄 Running database migrations...");

        let pool = self.pool.as_ref()
            .ok_or_else(|| "PostgreSQL not connected".to_string())?;

        let client = timeout(Duration::from_secs(10), pool.get())
            .await
            .map_err(|_| "Migration connection timeout".to_string())?
            .map_err(|e| format!("Failed to get connection for migration: {}", e))?;

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
                    Err(format!("Migration failed: {}", e))
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
        let pool = self.pool.as_ref()
            .ok_or_else(|| "PostgreSQL not connected".to_string())?;

        timeout(Duration::from_secs(5), pool.get())
            .await
            .map_err(|_| "Connection pool timeout".to_string())?
            .map_err(|e| format!("Failed to get connection: {}", e))
    }

    /// Persist a circuit to PostgreSQL
    pub async fn persist_circuit(&self, circuit: &Circuit) -> Result<(), String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let permissions_json = serde_json::to_value(&circuit.permissions)
            .map_err(|e| format!("Failed to serialize permissions: {}", e))?;

        let alias_config_json = circuit.alias_config.as_ref()
            .map(|c| serde_json::to_value(c))
            .transpose()
            .map_err(|e| format!("Failed to serialize alias_config: {}", e))?;

        let adapter_config_json = circuit.adapter_config.as_ref()
            .map(|c| serde_json::to_value(c))
            .transpose()
            .map_err(|e| format!("Failed to serialize adapter_config: {}", e))?;

        let public_settings_json = circuit.public_settings.as_ref()
            .map(|s| serde_json::to_value(s))
            .transpose()
            .map_err(|e| format!("Failed to serialize public_settings: {}", e))?;

        let post_action_json = circuit.post_action_settings.as_ref()
            .map(|s| serde_json::to_value(s))
            .transpose()
            .map_err(|e| format!("Failed to serialize post_action_settings: {}", e))?;

        let status_str = match circuit.status {
            CircuitStatus::Active => "Active",
            CircuitStatus::Inactive => "Inactive",
            CircuitStatus::Archived => "Archived",
        };

        client.execute(
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
        ).await
        .map_err(|e| format!("Failed to persist circuit: {}", e))?;

        tracing::debug!("✅ Persisted circuit {} to PostgreSQL", circuit.circuit_id);
        Ok(())
    }

    /// Load all circuits from PostgreSQL on startup
    pub async fn load_circuits(&self) -> Result<Vec<Circuit>, String> {
        if !self.is_connected().await {
            return Err("PostgreSQL not connected".to_string());
        }

        let client = self.get_client().await?;

        let rows = client.query(
            "SELECT circuit_id, name, description, owner_id, status,
                    created_at_ts, last_modified_ts, permissions,
                    alias_config, adapter_config, public_settings, post_action_settings
             FROM circuits
             WHERE status != 'Archived'
             ORDER BY created_at_ts DESC",
            &[],
        ).await
        .map_err(|e| format!("Failed to load circuits: {}", e))?;

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
            _ => return Err(format!("Invalid circuit status: {}", status_str)),
        };

        let permissions_json: serde_json::Value = row.get("permissions");
        let permissions = serde_json::from_value(permissions_json)
            .map_err(|e| format!("Failed to parse permissions: {}", e))?;

        let alias_config: Option<serde_json::Value> = row.get("alias_config");
        let alias_config = alias_config
            .map(|v| serde_json::from_value(v))
            .transpose()
            .map_err(|e| format!("Failed to parse alias_config: {}", e))?;

        let adapter_config: Option<serde_json::Value> = row.get("adapter_config");
        let adapter_config = adapter_config
            .map(|v| serde_json::from_value(v))
            .transpose()
            .map_err(|e| format!("Failed to parse adapter_config: {}", e))?;

        let public_settings: Option<serde_json::Value> = row.get("public_settings");
        let public_settings = public_settings
            .map(|v| serde_json::from_value(v))
            .transpose()
            .map_err(|e| format!("Failed to parse public_settings: {}", e))?;

        let post_action_settings: Option<serde_json::Value> = row.get("post_action_settings");
        let post_action_settings = post_action_settings
            .map(|v| serde_json::from_value(v))
            .transpose()
            .map_err(|e| format!("Failed to parse post_action_settings: {}", e))?;

        let created_at_ts: i64 = row.get("created_at_ts");
        let last_modified_ts: i64 = row.get("last_modified_ts");

        Ok(Circuit {
            circuit_id: row.get("circuit_id"),
            name: row.get("name"),
            description: row.get("description"),
            owner_id: row.get("owner_id"),
            default_namespace: String::new(), // Will be loaded from storage if needed
            alias_config,
            created_timestamp: DateTime::from_timestamp(created_at_ts, 0)
                .unwrap_or_else(Utc::now),
            last_modified: DateTime::from_timestamp(last_modified_ts, 0)
                .unwrap_or_else(Utc::now),
            members: Vec::new(), // Load separately if needed
            permissions,
            status,
            pending_requests: Vec::new(), // Load separately if needed
            custom_roles: Vec::new(), // Load separately if needed
            public_settings,
            adapter_config,
            post_action_settings,
        })
    }

    /// Persist user account to PostgreSQL
    pub async fn persist_user(&self, user: &UserAccount) -> Result<(), String> {
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

        // Serialize available_adapters as TEXT array for PostgreSQL
        let adapters_array: Option<Vec<String>> = user.available_adapters.as_ref()
            .map(|adapters| adapters.iter().map(|a| format!("{:?}", a)).collect());

        client.execute(
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
                available_adapters = EXCLUDED.available_adapters",
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
        ).await
        .map_err(|e| format!("Failed to persist user: {}", e))?;

        // Also persist credit balance
        client.execute(
            "INSERT INTO credit_balances (user_id, credits, updated_at_ts)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET
                credits = EXCLUDED.credits,
                updated_at_ts = EXCLUDED.updated_at_ts",
            &[&user.user_id, &(user.credits as i64), &Utc::now().timestamp()],
        ).await
        .map_err(|e| format!("Failed to persist credit balance: {}", e))?;

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
        .map_err(|e| format!("Failed to load users: {}", e))?;

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
        .map_err(|e| format!("Failed to persist item: {}", e))?;

        // Insert identifiers (delete old ones first)
        client.execute(
            "DELETE FROM item_identifiers WHERE dfid = $1",
            &[&item.dfid],
        ).await
        .map_err(|e| format!("Failed to delete old identifiers: {}", e))?;

        for identifier in &item.identifiers {
            client.execute(
                "INSERT INTO item_identifiers (dfid, key, value) VALUES ($1, $2, $3)",
                &[&item.dfid, &identifier.key, &identifier.value],
            ).await
            .map_err(|e| format!("Failed to insert identifier: {}", e))?;
        }

        // Insert source entries (delete old ones first)
        client.execute(
            "DELETE FROM item_source_entries WHERE dfid = $1",
            &[&item.dfid],
        ).await
        .map_err(|e| format!("Failed to delete old source entries: {}", e))?;

        for entry_id in &item.source_entries {
            client.execute(
                "INSERT INTO item_source_entries (dfid, entry_id) VALUES ($1, $2)",
                &[&item.dfid, entry_id],
            ).await
            .map_err(|e| format!("Failed to insert source entry: {}", e))?;
        }

        // Insert LID mapping if exists
        if let Some(local_id) = item.local_id {
            client.execute(
                "INSERT INTO lid_dfid_mappings (local_id, dfid) VALUES ($1, $2)
                 ON CONFLICT (local_id) DO UPDATE SET dfid = EXCLUDED.dfid",
                &[&local_id, &item.dfid],
            ).await
            .map_err(|e| format!("Failed to insert LID mapping: {}", e))?;
        }

        Ok(())
    }

    /// Persist event to PostgreSQL (write-through cache)
    pub async fn persist_event(&self, event: &crate::types::Event) -> Result<(), String> {
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
        .map_err(|e| format!("Failed to persist event: {}", e))?;

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
            _ => return Err(format!("Invalid account status: {}", status_str)),
        };

        let created_at_ts: i64 = row.get("created_at_ts");
        let last_login_ts: Option<i64> = row.get("last_login_ts");
        let credits: i64 = row.get("credits");

        // Deserialize available_adapters from TEXT array - for now just return None
        // TODO: Parse adapter strings back to AdapterType enum when needed
        let _available_adapters: Option<Vec<String>> = row.get("available_adapters");

        // Calculate limits before moving tier
        let limits = TierLimits::for_tier(&tier);

        Ok(UserAccount {
            user_id: row.get("user_id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            tier,
            status,
            credits: credits as i64,
            created_at: DateTime::from_timestamp(created_at_ts, 0)
                .unwrap_or_else(Utc::now),
            updated_at: Utc::now(),
            last_login: last_login_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            subscription: None,
            limits,
            is_admin: row.get("is_admin"),
            workspace_id: row.get("workspace_id"),
            available_adapters: None, // TODO: Parse from available_adapters string array
        })
    }
}
