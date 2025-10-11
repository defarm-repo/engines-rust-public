use tokio_postgres::{NoTls, Error as PgError, Client};
use deadpool_postgres::{Pool, Manager, ManagerConfig, RecyclingMethod, Runtime};
use uuid::Uuid;
use chrono::Utc;
use serde_json;
use blake3;

use crate::types::*;
use crate::identifier_types::*;
use crate::storage::StorageError;

/// Lightweight PostgreSQL persistence layer
/// Persists critical data while keeping in-memory storage for queries
pub struct PostgresPersistence {
    pool: Pool,
}

impl PostgresPersistence {
    /// Create a new PostgreSQL persistence layer
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let config = database_url.parse::<tokio_postgres::Config>()
            .map_err(|e| StorageError::ConfigurationError(format!("Invalid database URL: {}", e)))?;

        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, manager_config);

        let pool = Pool::builder(manager)
            .max_size(16)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| StorageError::ConnectionError(format!("Failed to create pool: {}", e)))?;

        // Test connection
        let client = pool.get().await
            .map_err(|e| StorageError::ConnectionError(format!("Failed to get test connection: {}", e)))?;

        tracing::info!("PostgreSQL persistence layer connected successfully");

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool_postgres::Client, StorageError> {
        self.pool.get().await
            .map_err(|e| StorageError::ConnectionError(format!("Failed to get connection: {}", e)))
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<(), StorageError> {
        // Run migrations using raw SQL for simplicity
        let client = self.get_conn().await?;

        // Check if migrations table exists
        let exists = client.query_one(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'items')",
            &[]
        ).await
        .map_err(|e| StorageError::ConfigurationError(format!("Failed to check migrations: {}", e)))?;

        let table_exists: bool = exists.get(0);

        if !table_exists {
            tracing::info!("Running database migrations from SQL file...");
            let migration_sql = include_str!("../migrations/V1__initial_schema.sql");
            client.batch_execute(migration_sql).await
                .map_err(|e| StorageError::ConfigurationError(format!("Migration failed: {}", e)))?;
            tracing::info!("Database migrations completed successfully");
        } else {
            tracing::info!("Database already migrated");
        }

        Ok(())
    }

    // ========================================================================
    // ITEM PERSISTENCE
    // ========================================================================

    /// Persist an item to PostgreSQL
    pub async fn persist_item(&self, item: &Item) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        // Compute hash from DFID for storage (items no longer have hash field)
        let item_hash = blake3::hash(item.dfid.as_bytes()).to_hex().to_string();

        // Insert or update item
        client.execute(
            "INSERT INTO items (dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
             ON CONFLICT (dfid)
             DO UPDATE SET
                item_hash = EXCLUDED.item_hash,
                status = EXCLUDED.status,
                last_updated_ts = EXCLUDED.last_updated_ts,
                enriched_data = EXCLUDED.enriched_data,
                updated_at = NOW()",
            &[
                &item.dfid,
                &item_hash,
                &format!("{:?}", item.status),
                &(item.creation_timestamp.timestamp_nanos_opt().unwrap_or(0)),
                &(item.last_modified.timestamp_nanos_opt().unwrap_or(0)),
                &serde_json::to_value(&item.enriched_data).unwrap_or(serde_json::json!({})),
            ],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist item: {}", e)))?;

        // Persist enhanced identifiers
        for identifier in &item.enhanced_identifiers {
            self.persist_enhanced_identifier(&item.dfid, identifier).await?;
        }

        Ok(())
    }

    /// Persist enhanced identifier
    async fn persist_enhanced_identifier(&self, dfid: &str, identifier: &EnhancedIdentifier) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        let id_type_str = match &identifier.id_type {
            IdentifierType::Canonical { registry, .. } => format!("Canonical:{}", registry),
            IdentifierType::Contextual { scope } => format!("Contextual:{}", scope),
        };

        client.execute(
            "INSERT INTO enhanced_identifiers (dfid, namespace, key, value, id_type, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())
             ON CONFLICT (dfid, namespace, key, value) DO NOTHING",
            &[
                &dfid,
                &identifier.namespace,
                &identifier.key,
                &identifier.value,
                &id_type_str,
            ],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist identifier: {}", e)))?;

        Ok(())
    }

    /// Load item from PostgreSQL
    pub async fn load_item(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
        let client = self.get_conn().await?;

        let row_opt = client.query_opt(
            "SELECT dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data
             FROM items WHERE dfid = $1",
            &[&dfid],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load item: {}", e)))?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Active" => ItemStatus::Active,
            "Merged" => ItemStatus::Merged,
            "Split" => ItemStatus::Split,
            "Deprecated" => ItemStatus::Deprecated,
            _ => ItemStatus::Active,
        };

        let enriched_data: serde_json::Value = row.get("enriched_data");
        let enriched_data_map = serde_json::from_value(enriched_data)
            .unwrap_or_else(|_| std::collections::HashMap::new());

        // Load enhanced identifiers
        let identifier_rows = client.query(
            "SELECT namespace, key, value, id_type
             FROM enhanced_identifiers WHERE dfid = $1",
            &[&dfid],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load identifiers: {}", e)))?;

        let mut enhanced_identifiers = Vec::new();
        for id_row in identifier_rows {
            let id_type_str: String = id_row.get("id_type");
            let id_type = if id_type_str.starts_with("Canonical:") {
                let registry = id_type_str.strip_prefix("Canonical:").unwrap_or("unknown").to_string();
                IdentifierType::Canonical {
                    registry,
                    verified: false,
                    verification_date: None,
                }
            } else if id_type_str.starts_with("Contextual:") {
                let scope = id_type_str.strip_prefix("Contextual:").unwrap_or("user").to_string();
                IdentifierType::Contextual { scope }
            } else {
                IdentifierType::Contextual { scope: "user".to_string() }
            };

            enhanced_identifiers.push(EnhancedIdentifier {
                namespace: id_row.get("namespace"),
                key: id_row.get("key"),
                value: id_row.get("value"),
                id_type,
            });
        }

        Ok(Some(Item {
            dfid: row.get("dfid"),
            local_id: None,
            legacy_mode: false,
            identifiers: Vec::new(),
            enhanced_identifiers,
            aliases: Vec::new(),
            fingerprint: None,
            enriched_data: enriched_data_map,
            creation_timestamp: chrono::DateTime::from_timestamp_nanos(row.get("created_at_ts")),
            last_modified: chrono::DateTime::from_timestamp_nanos(row.get("last_updated_ts")),
            source_entries: Vec::new(),
            confidence_score: 1.0,
            status,
        }))
    }

    /// Load all items from PostgreSQL
    pub async fn load_all_items(&self) -> Result<Vec<Item>, StorageError> {
        let client = self.get_conn().await?;

        let rows = client.query(
            "SELECT dfid FROM items ORDER BY created_at DESC LIMIT 1000",
            &[],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load items: {}", e)))?;

        let mut items = Vec::new();
        for row in rows {
            let dfid: String = row.get("dfid");
            if let Some(item) = self.load_item(&dfid).await? {
                items.push(item);
            }
        }

        Ok(items)
    }

    // ========================================================================
    // CIRCUIT PERSISTENCE
    // ========================================================================

    /// Persist a circuit to PostgreSQL
    pub async fn persist_circuit(&self, circuit: &Circuit) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        client.execute(
            "INSERT INTO circuits (circuit_id, name, description, owner_id, default_namespace, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
             ON CONFLICT (circuit_id)
             DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                default_namespace = EXCLUDED.default_namespace,
                updated_at = NOW()",
            &[
                &circuit.circuit_id,
                &circuit.name,
                &circuit.description,
                &circuit.owner_id,
                &circuit.default_namespace,
            ],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist circuit: {}", e)))?;

        // Persist circuit members
        for member in &circuit.members {
            client.execute(
                "INSERT INTO circuit_members (circuit_id, user_id, role, added_at)
                 VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (circuit_id, user_id)
                 DO UPDATE SET role = EXCLUDED.role",
                &[
                    &circuit.circuit_id,
                    &member.member_id,
                    &format!("{:?}", member.role),
                ],
            ).await
            .map_err(|e| StorageError::WriteError(format!("Failed to persist circuit member: {}", e)))?;
        }

        Ok(())
    }

    /// Load circuit from PostgreSQL
    pub async fn load_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, StorageError> {
        let client = self.get_conn().await?;

        let row_opt = client.query_opt(
            "SELECT circuit_id, name, description, owner_id, default_namespace
             FROM circuits WHERE circuit_id = $1",
            &[circuit_id],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load circuit: {}", e)))?;

        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        // Load members
        let member_rows = client.query(
            "SELECT user_id, role FROM circuit_members WHERE circuit_id = $1",
            &[circuit_id],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load circuit members: {}", e)))?;

        let mut members = std::collections::HashMap::new();
        for member_row in member_rows {
            let user_id: String = member_row.get("user_id");
            let role_str: String = member_row.get("role");
            let role = match role_str.as_str() {
                "Owner" => MemberRole::Owner,
                "Admin" => MemberRole::Admin,
                "Member" => MemberRole::Member,
                _ => MemberRole::Member,
            };
            members.insert(user_id, role);
        }

        // Convert HashMap members to Vec<CircuitMember>
        let circuit_members: Vec<CircuitMember> = members.into_iter().map(|(user_id, role)| {
            CircuitMember {
                member_id: user_id,
                role,
                custom_role_name: None,
                permissions: vec![Permission::Push, Permission::Pull],
                joined_timestamp: Utc::now(),
            }
        }).collect();

        Ok(Some(Circuit {
            circuit_id: row.get("circuit_id"),
            name: row.get("name"),
            description: row.get("description"),
            owner_id: row.get("owner_id"),
            default_namespace: row.get("default_namespace"),
            alias_config: None,
            created_timestamp: Utc::now(),
            last_modified: Utc::now(),
            members: circuit_members,
            permissions: CircuitPermissions::default(),
            status: CircuitStatus::Active,
            pending_requests: Vec::new(),
            custom_roles: Vec::new(),
            public_settings: None,
            adapter_config: None,
            post_action_settings: None,
        }))
    }

    /// Load all circuits from PostgreSQL
    pub async fn load_all_circuits(&self) -> Result<Vec<Circuit>, StorageError> {
        let client = self.get_conn().await?;

        let rows = client.query(
            "SELECT circuit_id FROM circuits ORDER BY created_at DESC LIMIT 1000",
            &[],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load circuits: {}", e)))?;

        let mut circuits = Vec::new();
        for row in rows {
            let circuit_id: Uuid = row.get("circuit_id");
            if let Some(circuit) = self.load_circuit(&circuit_id).await? {
                circuits.push(circuit);
            }
        }

        Ok(circuits)
    }

    // ========================================================================
    // USER PERSISTENCE
    // ========================================================================

    /// Persist a user to PostgreSQL
    pub async fn persist_user(&self, user: &UserAccount) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        client.execute(
            "INSERT INTO users (user_id, username, email, password_hash, tier, workspace_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
             ON CONFLICT (user_id)
             DO UPDATE SET
                username = EXCLUDED.username,
                email = EXCLUDED.email,
                password_hash = EXCLUDED.password_hash,
                tier = EXCLUDED.tier,
                updated_at = NOW()",
            &[
                &user.user_id,
                &user.username,
                &user.email,
                &user.password_hash,
                &format!("{:?}", user.tier),
                &user.workspace_id,
            ],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist user: {}", e)))?;

        Ok(())
    }

    /// Load all users from PostgreSQL
    pub async fn load_all_users(&self) -> Result<Vec<UserAccount>, StorageError> {
        let client = self.get_conn().await?;

        let rows = client.query(
            "SELECT user_id, username, email, password_hash, tier, workspace_id
             FROM users ORDER BY created_at DESC",
            &[],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load users: {}", e)))?;

        let mut users = Vec::new();
        for row in rows {
            let tier_str: String = row.get("tier");
            let tier = match tier_str.as_str() {
                "Admin" => UserTier::Admin,
                "Enterprise" => UserTier::Enterprise,
                "Professional" => UserTier::Professional,
                "Basic" => UserTier::Basic,
                _ => UserTier::Basic,
            };

            users.push(UserAccount {
                user_id: row.get("user_id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                tier,
                status: AccountStatus::Active,
                credits: 0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_login: None,
                subscription: None,
                limits: TierLimits {
                    max_items_per_month: None,
                    max_circuits: None,
                    max_storage_locations: None,
                    max_api_requests_per_hour: None,
                    max_workspace_members: None,
                    available_adapters: Vec::new(),
                    can_use_premium_adapters: false,
                    max_audit_retention_days: 90,
                    priority_support: false,
                },
                is_admin: false,
                workspace_id: row.get("workspace_id"),
                available_adapters: None,
            });
        }

        Ok(users)
    }

    // ========================================================================
    // STORAGE HISTORY PERSISTENCE
    // ========================================================================

    /// Persist storage history record
    pub async fn persist_storage_history(&self, dfid: &str, record: &StorageRecord) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        let location_json = serde_json::to_value(&record.storage_location)
            .map_err(|e| StorageError::WriteError(format!("Failed to serialize location: {}", e)))?;

        client.execute(
            "INSERT INTO storage_history (dfid, adapter_type, storage_location, stored_at, triggered_by, triggered_by_id, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
            &[
                &dfid,
                &format!("{:?}", record.adapter_type),
                &location_json,
                &record.stored_at,
                &record.triggered_by,
                &record.triggered_by_id,
                &record.is_active,
            ],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist storage history: {}", e)))?;

        Ok(())
    }

    // ========================================================================
    // LID-DFID MAPPING PERSISTENCE
    // ========================================================================

    /// Persist LID-DFID mapping
    pub async fn persist_lid_mapping(&self, local_id: &Uuid, dfid: &str, workspace_id: &str) -> Result<(), StorageError> {
        let client = self.get_conn().await?;

        client.execute(
            "INSERT INTO lid_dfid_mappings (local_id, dfid, workspace_id, created_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (local_id)
             DO UPDATE SET dfid = EXCLUDED.dfid",
            &[local_id, &dfid, &workspace_id],
        ).await
        .map_err(|e| StorageError::WriteError(format!("Failed to persist LID mapping: {}", e)))?;

        Ok(())
    }

    /// Load DFID by LID
    pub async fn load_dfid_by_lid(&self, local_id: &Uuid) -> Result<Option<String>, StorageError> {
        let client = self.get_conn().await?;

        let row_opt = client.query_opt(
            "SELECT dfid FROM lid_dfid_mappings WHERE local_id = $1",
            &[local_id],
        ).await
        .map_err(|e| StorageError::ReadError(format!("Failed to load LID mapping: {}", e)))?;

        Ok(row_opt.map(|row| row.get("dfid")))
    }
}
