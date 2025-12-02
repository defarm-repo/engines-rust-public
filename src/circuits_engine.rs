use crate::adapters::{
    base::StorageLocation, IpfsIpfsAdapter, StellarMainnetIpfsAdapter, StellarTestnetIpfsAdapter,
    StorageAdapter,
};
use crate::dfid_engine::DfidEngine;
use crate::events_engine::EventsEngine;
use crate::identifier_types::{
    CircuitAliasConfig, EnhancedIdentifier, ExternalAlias, IdentifierType,
};
use crate::logging::LoggingEngine;
use crate::postgres_persistence::PostgresPersistence;
use crate::storage::StorageBackend;
use crate::types::{
    Activity, ActivityDetails, ActivityStatus, ActivityType, AdapterType, BatchPushItemResult,
    BatchPushResult, Circuit, CircuitAdapterConfig, CircuitItem, CircuitOperation,
    CircuitPermissions, CircuitStatus, CustomRole, EventVisibility, Identifier, Item, ItemStatus,
    MemberRole, Notification, NotificationType, OperationStatus, OperationType, Permission,
    PostActionTrigger, PublicSettings, UserTier, WebhookItemData, WebhookPayload,
    WebhookStorageData,
};
use crate::webhook_engine::WebhookEngine;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub enum CircuitsError {
    StorageError(String),
    PermissionDenied(String),
    AdapterPermissionDenied(String),
    ValidationError(String),
    NotFound,
    ItemNotFound,
    CircuitNotFound,
    MemberNotFound,
}

impl std::fmt::Display for CircuitsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitsError::StorageError(e) => write!(f, "Storage error: {e}"),
            CircuitsError::PermissionDenied(e) => write!(f, "Permission denied: {e}"),
            CircuitsError::AdapterPermissionDenied(e) => {
                write!(f, "Adapter permission denied: {e}")
            }
            CircuitsError::ValidationError(e) => write!(f, "Validation error: {e}"),
            CircuitsError::NotFound => write!(f, "Circuit not found"),
            CircuitsError::ItemNotFound => write!(f, "Item not found"),
            CircuitsError::CircuitNotFound => write!(f, "Circuit not found"),
            CircuitsError::MemberNotFound => write!(f, "Member not found"),
        }
    }
}

impl std::error::Error for CircuitsError {}

// Helper function to get tier default adapters
fn get_tier_default_adapters(tier: &UserTier) -> Vec<AdapterType> {
    match tier {
        UserTier::Admin | UserTier::Enterprise => vec![
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::StellarMainnetIpfs,
        ],
        UserTier::Professional => vec![AdapterType::IpfsIpfs, AdapterType::StellarTestnetIpfs],
        UserTier::Basic => vec![AdapterType::IpfsIpfs],
    }
}

// Helper function to validate if a user tier has access to an adapter
fn validate_adapter_tier_access(user_tier: &UserTier, adapter_type: &AdapterType) -> bool {
    match adapter_type {
        // None adapter - all tiers have access (no storage)
        AdapterType::None => true,

        // Basic tier adapter - all tiers have access
        AdapterType::IpfsIpfs => true,

        // Professional tier adapter - Professional, Enterprise, Admin
        AdapterType::StellarTestnetIpfs => {
            matches!(
                user_tier,
                UserTier::Professional | UserTier::Enterprise | UserTier::Admin
            )
        }

        // Enterprise tier adapter - Enterprise, Admin only
        AdapterType::StellarMainnetIpfs => {
            matches!(user_tier, UserTier::Enterprise | UserTier::Admin)
        }

        // Other adapters - currently not available
        _ => false,
    }
}

pub struct CircuitsEngine<S: StorageBackend> {
    storage: S,
    logger: Arc<std::sync::Mutex<LoggingEngine>>,
    events_engine: EventsEngine<S>,
    dfid_engine: DfidEngine,
    webhook_engine: Arc<tokio::sync::RwLock<WebhookEngine<S>>>,
    postgres: Option<Arc<RwLock<Option<PostgresPersistence>>>>,
}

impl<S: StorageBackend + 'static> CircuitsEngine<S> {
    pub fn new(storage: S) -> Self
    where
        S: Clone,
    {
        let logger = LoggingEngine::new();
        let events_engine = EventsEngine::new(storage.clone());
        let webhook_engine = WebhookEngine::new(storage.clone());
        Self {
            storage,
            logger: Arc::new(std::sync::Mutex::new(logger)),
            events_engine,
            dfid_engine: DfidEngine::new(),
            webhook_engine: Arc::new(tokio::sync::RwLock::new(webhook_engine)),
            postgres: None,
        }
    }

    pub fn set_postgres(&mut self, postgres: Arc<RwLock<Option<PostgresPersistence>>>) {
        self.postgres = Some(postgres);
    }

    fn spawn_persist_activity(&self, activity: Activity) {
        if let Some(pg_ref) = &self.postgres {
            let pg = Arc::clone(pg_ref);
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_persistence) = &*pg_lock {
                    if let Err(e) = pg_persistence.persist_activity(&activity).await {
                        tracing::warn!(
                            "Failed to persist circuit activity {}: {}",
                            activity.activity_id,
                            e
                        );
                    }
                }
            });
        }
    }

    async fn handle_auto_publish(
        &self,
        circuit: &Circuit,
        dfid: &str,
        circuit_id: &Uuid,
    ) -> Result<(), CircuitsError> {
        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "auto_publish_check",
                format!("Checking auto-publish for circuit {circuit_id}"),
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("dfid", dfid.to_string());

        if let Some(ref public_settings) = circuit.public_settings {
            if public_settings.auto_publish_pushed_items {
                let mut updated_circuit = circuit.clone();
                if let Some(ref mut settings) = updated_circuit.public_settings {
                    if !settings.published_items.contains(&dfid.to_string()) {
                        settings.published_items.push(dfid.to_string());
                        self.storage
                            .store_circuit(&updated_circuit)
                            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

                        self.logger
                            .lock()
                            .unwrap()
                            .info(
                                "circuits_engine",
                                "auto_publish_success",
                                format!("Auto-published item {dfid} to circuit {circuit_id}"),
                            )
                            .with_context("circuit_id", circuit_id.to_string())
                            .with_context("dfid", dfid.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn create_circuit(
        &mut self,
        name: String,
        description: String,
        owner_id: String,
        adapter_config: Option<CircuitAdapterConfig>,
        alias_config: Option<CircuitAliasConfig>,
    ) -> Result<Circuit, CircuitsError> {
        self.create_circuit_with_namespace(
            name,
            description,
            owner_id,
            "generic".to_string(),
            adapter_config,
            alias_config,
        )
        .await
    }

    pub async fn create_circuit_with_namespace(
        &mut self,
        name: String,
        description: String,
        owner_id: String,
        default_namespace: String,
        adapter_config: Option<CircuitAdapterConfig>,
        alias_config: Option<CircuitAliasConfig>,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = Circuit::new(name.clone(), description.clone(), owner_id.clone());
        circuit.default_namespace = default_namespace;
        circuit.adapter_config = adapter_config;
        circuit.alias_config = alias_config;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_creation_started",
                format!("Creating circuit: {name}"),
            )
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("owner_id", owner_id.clone());

        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_created",
                "Circuit created successfully",
            )
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("name", name);

        Ok(circuit)
    }

    pub async fn create_circuit_with_settings(
        &mut self,
        name: String,
        description: String,
        owner_id: String,
        adapter_config: Option<CircuitAdapterConfig>,
        alias_config: Option<CircuitAliasConfig>,
        initial_permissions: Option<CircuitPermissions>,
        public_settings: Option<PublicSettings>,
    ) -> Result<Circuit, CircuitsError> {
        // Create circuit with provided settings
        let mut circuit = Circuit::new(name.clone(), description.clone(), owner_id.clone());
        circuit.adapter_config = adapter_config;
        circuit.alias_config = alias_config;

        // Apply initial permissions if provided
        if let Some(permissions) = initial_permissions {
            circuit.permissions = permissions;
        }

        // Apply public settings if provided
        if let Some(settings) = public_settings {
            circuit.public_settings = Some(settings);
        }

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_creation_started",
                format!("Creating circuit with settings: {name}"),
            )
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("owner_id", owner_id.clone())
            .with_context(
                "public_visibility",
                circuit.permissions.allow_public_visibility.to_string(),
            );

        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_created_with_settings",
                "Circuit created successfully with initial settings",
            )
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("name", name)
            .with_context(
                "public_visibility",
                circuit.permissions.allow_public_visibility.to_string(),
            );

        Ok(circuit)
    }

    pub async fn add_member_to_circuit(
        &mut self,
        circuit_id: &Uuid,
        member_id: String,
        role: MemberRole,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Invite) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to invite members".to_string(),
            ));
        }

        if circuit.get_member(&member_id).is_some() {
            return Err(CircuitsError::ValidationError(
                "Member is already in the circuit".to_string(),
            ));
        }

        circuit.add_member(member_id.clone(), role);

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info("circuits_engine", "member_added", "Member added to circuit")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("member_id", member_id)
            .with_context("role", format!("{role:?}"))
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn push_item_to_circuit(
        &mut self,
        dfid: &str,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<CircuitOperation, CircuitsError> {
        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "push_item_attempt",
                format!(
                    "Attempting to push item {dfid} to circuit {circuit_id} by user {requester_id}"
                ),
            )
            .with_context("dfid", dfid)
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id);

        let _item = self
            .storage
            .get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        // Get circuit with error handling that works with async logging
        let circuit_result = self.storage.get_circuit(circuit_id);
        let circuit = match circuit_result {
            Err(e) => {
                self.logger
                    .lock()
                    .unwrap()
                    .error(
                        "circuits_engine",
                        "push_item_storage_error",
                        format!("Storage error while getting circuit {circuit_id}: {e}"),
                    )
                    .with_context("circuit_id", circuit_id.to_string())
                    .with_context("error", e.to_string());
                return Err(CircuitsError::StorageError(e.to_string()));
            }
            Ok(None) => {
                self.logger
                    .lock()
                    .unwrap()
                    .warn(
                        "circuits_engine",
                        "push_item_circuit_not_found",
                        format!("Circuit {circuit_id} not found in storage"),
                    )
                    .with_context("circuit_id", circuit_id.to_string());
                return Err(CircuitsError::CircuitNotFound);
            }
            Ok(Some(circuit)) => circuit,
        };

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "push_item_permission_check",
                format!("Circuit {circuit_id} found, checking permissions"),
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("circuit_name", &circuit.name)
            .with_context("requester_id", requester_id);

        if !circuit.has_permission(requester_id, &Permission::Push) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to push to this circuit".to_string(),
            ));
        }

        // Check adapter permissions if circuit has a configured adapter
        if let Some(adapter_config) = &circuit.adapter_config {
            if let Some(required_adapter) = &adapter_config.adapter_type {
                // Skip validation if circuit sponsors adapter access
                if !adapter_config.sponsor_adapter_access {
                    // Get user account to check their available adapters
                    let user = self
                        .storage
                        .get_user_account(requester_id)
                        .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                        .ok_or_else(|| {
                            CircuitsError::ValidationError("User not found".to_string())
                        })?;

                    // Get user's available adapters (custom or tier defaults)
                    let user_adapters = if let Some(custom_adapters) = &user.available_adapters {
                        custom_adapters.clone()
                    } else {
                        get_tier_default_adapters(&user.tier)
                    };

                    // Check if user has access to the required adapter
                    if !user_adapters.contains(required_adapter) {
                        let adapter_name = format!("{required_adapter:?}");
                        let user_adapters_str = user_adapters
                            .iter()
                            .map(|a| format!("{a:?}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(CircuitsError::AdapterPermissionDenied(
                            format!(
                                "This circuit requires '{adapter_name}' adapter access. You currently have access to: {user_adapters_str}. Please contact your administrator to request access to this adapter."
                            )
                        ));
                    }
                }
            }
        }

        let mut operation = CircuitOperation::new(
            *circuit_id,
            dfid.to_string(),
            OperationType::Push,
            requester_id.to_string(),
        );

        if circuit.permissions.require_approval_for_push {
            operation.status = crate::types::OperationStatus::Pending;
        } else {
            operation.approve();
            operation.complete();

            // Store circuit item and log activity
            let circuit_item = CircuitItem::new(
                dfid.to_string(),
                *circuit_id,
                requester_id.to_string(),
                vec!["read".to_string(), "verify".to_string()],
            );
            self.storage
                .store_circuit_item(&circuit_item)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            let activity = Activity::new(
                ActivityType::Push,
                *circuit_id,
                circuit.name.clone(),
                vec![dfid.to_string()],
                requester_id.to_string(),
                ActivityStatus::Success,
                ActivityDetails {
                    items_affected: 1,
                    enrichments_made: None,
                    error_message: None,
                },
            );
            self.storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            self.spawn_persist_activity(activity.clone());
        }

        // Handle auto-publish for immediate pushes (not requiring approval)
        if !circuit.permissions.require_approval_for_push {
            self.handle_auto_publish(&circuit, dfid, circuit_id).await?;
        }

        self.storage
            .store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine
            .create_circuit_operation_event(
                dfid.to_string(),
                circuit_id.to_string(),
                "push".to_string(),
                requester_id.to_string(),
                visibility,
            )
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Handle storage migration if circuit has a different adapter configuration
        if !circuit.permissions.require_approval_for_push {
            self.handle_storage_migration(dfid, &circuit, requester_id)
                .await
                .map_err(|e| {
                    CircuitsError::StorageError(format!("Storage migration failed: {e}"))
                })?;
        }

        self.logger
            .lock()
            .unwrap()
            .info("circuits_engine", "item_pushed", "Item pushed to circuit")
            .with_context("dfid", dfid.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("operation_id", operation.operation_id.to_string());

        // Trigger webhooks if configured (optional)
        self.trigger_post_action_webhooks(
            &circuit,
            dfid,
            None, // no local_id for legacy push
            requester_id,
            &operation.operation_id,
            PostActionTrigger::ItemPushed,
            None, // TODO: extract storage details from adapter if needed
        )
        .await;

        Ok(operation)
    }

    // NEW: Push with LID (tokenization in circuit)
    #[allow(clippy::await_holding_lock)]
    pub async fn push_local_item_to_circuit(
        &mut self,
        local_id: &Uuid,
        mut identifiers: Vec<EnhancedIdentifier>,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<PushResult, CircuitsError> {
        // 1. Get circuit and validate permissions
        let circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Push) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to push to this circuit".to_string(),
            ));
        }

        // 2. Auto-apply namespace if configured
        if circuit
            .alias_config
            .as_ref()
            .map(|c| c.auto_apply_namespace)
            .unwrap_or(true)
        {
            for identifier in &mut identifiers {
                if identifier.namespace.is_empty() {
                    identifier.namespace = circuit.default_namespace.clone();
                }
            }
        }

        // 3. Validate circuit requirements
        self.validate_circuit_requirements(&circuit, &identifiers)?;

        // 4. Resolve or create DFID (core of tokenization)
        let (dfid, status) = self
            .resolve_or_create_dfid(
                &identifiers,
                &circuit,
                requester_id,
                local_id,
                enriched_data.clone(),
            )
            .await?;

        // 5. Save LID -> DFID mapping
        self.storage
            .store_lid_dfid_mapping(local_id, &dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // 6. Create circuit item and operation
        let circuit_item = CircuitItem::new(
            dfid.clone(),
            *circuit_id,
            requester_id.to_string(),
            vec!["read".to_string(), "verify".to_string()],
        );
        self.storage
            .store_circuit_item(&circuit_item)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // 6.5. CALL ADAPTER TO ACTUALLY UPLOAD TO BLOCKCHAIN/IPFS
        // This is where the REAL blockchain integration happens!
        let storage_details = if let Some(circuit_adapter_config) = self
            .storage
            .get_circuit_adapter_config(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
        {
            if let Some(adapter_type) = circuit_adapter_config.adapter_type {
                // Get the full adapter configuration by type
                let adapter_configs = self
                    .storage
                    .get_adapter_configs_by_type(&adapter_type)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
                let full_adapter_config = adapter_configs.into_iter().find(|c| c.is_active);
                // Get the item from storage to upload it
                let item = self
                    .storage
                    .get_item_by_dfid(&dfid)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                    .ok_or(CircuitsError::ItemNotFound)?;

                // Determine if this is a new DFID (for NFT minting)
                let is_new_dfid = matches!(status, PushStatus::NewItemCreated);

                // Create adapter instance based on type and call store_new_item with mint flag
                let upload_result = match adapter_type {
                    AdapterType::None => {
                        // No storage adapter - skip upload entirely
                        return Err(CircuitsError::StorageError(
                            "Circuit has no storage adapter configured (adapter type: None)"
                                .to_string(),
                        ));
                    }
                    AdapterType::IpfsIpfs => {
                        let adapter = IpfsIpfsAdapter::new().map_err(|e| {
                            CircuitsError::StorageError(format!(
                                "Failed to create IPFS adapter: {e}"
                            ))
                        })?;
                        adapter
                            .store_new_item(&item, is_new_dfid, requester_id)
                            .await
                            .map_err(|e| {
                                CircuitsError::StorageError(format!(
                                    "Failed to upload to IPFS: {e}"
                                ))
                            })?
                    }
                    AdapterType::StellarTestnetIpfs => {
                        let adapter = StellarTestnetIpfsAdapter::new_with_config(
                            full_adapter_config.as_ref(),
                        )
                        .map_err(|e| {
                            CircuitsError::StorageError(format!(
                                "Failed to create Stellar Testnet adapter: {e}"
                            ))
                        })?;
                        adapter
                            .store_new_item(&item, is_new_dfid, requester_id)
                            .await
                            .map_err(|e| {
                                CircuitsError::StorageError(format!(
                                    "Failed to upload to Stellar Testnet: {e}"
                                ))
                            })?
                    }
                    AdapterType::StellarMainnetIpfs => {
                        let adapter = StellarMainnetIpfsAdapter::new_with_config(
                            full_adapter_config.as_ref(),
                        )
                        .map_err(|e| {
                            CircuitsError::StorageError(format!(
                                "Failed to create Stellar Mainnet adapter: {e}"
                            ))
                        })?;
                        adapter
                            .store_new_item(&item, is_new_dfid, requester_id)
                            .await
                            .map_err(|e| {
                                CircuitsError::StorageError(format!(
                                    "Failed to upload to Stellar Mainnet: {e}"
                                ))
                            })?
                    }
                    _ => {
                        return Err(CircuitsError::StorageError(format!(
                            "Unsupported adapter type: {adapter_type:?}"
                        )));
                    }
                };

                // Extract storage location from adapter result
                let storage_location = upload_result.metadata.item_location.clone();
                let storage_hash = match &storage_location {
                    StorageLocation::IPFS { cid, .. } => cid.clone(),
                    StorageLocation::Stellar { transaction_id, .. } => transaction_id.clone(),
                    StorageLocation::Local { id } => id.clone(),
                    StorageLocation::Arweave { transaction_id } => transaction_id.clone(),
                    StorageLocation::Ethereum {
                        transaction_hash, ..
                    } => transaction_hash.clone(),
                };

                // Extract transaction hashes and CIDs from both item_location and event_locations
                let mut transaction_metadata = HashMap::new();
                transaction_metadata.insert(
                    "network".to_string(),
                    serde_json::json!(match adapter_type {
                        crate::types::AdapterType::StellarTestnetIpfs => "stellar-testnet",
                        crate::types::AdapterType::StellarMainnetIpfs => "stellar-mainnet",
                        _ => "unknown",
                    }),
                );

                // IMPORTANT: For StellarTestnetIpfs adapter, the IPCM update transaction
                // is stored in item_location (primary location for data retrieval)
                if let StorageLocation::Stellar {
                    transaction_id,
                    asset_id,
                    ..
                } = &storage_location
                {
                    // This is the IPCM update transaction (used for data retrieval)
                    transaction_metadata.insert(
                        "ipcm_update_tx".to_string(),
                        serde_json::json!(transaction_id),
                    );

                    // The asset_id contains the IPFS CID
                    if let Some(cid) = asset_id {
                        transaction_metadata.insert("ipfs_cid".to_string(), serde_json::json!(cid));
                    }
                }

                // Process event_locations for NFT mint and additional metadata
                for (idx, location) in upload_result.metadata.event_locations.iter().enumerate() {
                    match location {
                        StorageLocation::Stellar {
                            transaction_id,
                            contract_address,
                            ..
                        } => {
                            // First Stellar transaction in event_locations is NFT mint
                            if idx == 0 {
                                transaction_metadata.insert(
                                    "nft_mint_tx".to_string(),
                                    serde_json::json!(transaction_id),
                                );
                                transaction_metadata.insert(
                                    "nft_contract".to_string(),
                                    serde_json::json!(contract_address),
                                );
                            }
                            // Note: IPCM update is now handled from item_location above
                        }
                        StorageLocation::IPFS { cid, pinned } => {
                            // Also capture IPFS CID from event_locations
                            transaction_metadata
                                .insert("ipfs_cid".to_string(), serde_json::json!(cid));
                            transaction_metadata
                                .insert("ipfs_pinned".to_string(), serde_json::json!(pinned));
                        }
                        _ => {}
                    }
                }

                // ============================================================
                // IMPORTANT: Storage History Recording
                // ============================================================
                // This is where storage history is ACTUALLY recorded.
                // The flow:
                // 1. Adapter (e.g., StellarTestnetIpfsAdapter) performs:
                //    - NFT minting (if new DFID)
                //    - IPFS upload (generates CID)
                //    - IPCM contract update (registers CID on-chain)
                // 2. Adapter returns AdapterResult with StorageMetadata containing:
                //    - item_location: Primary storage (IPCM transaction)
                //    - event_locations: Secondary storage (NFT mint tx, IPFS CID)
                // 3. We extract all blockchain/IPFS details into transaction_metadata
                // 4. We create StorageRecord with all transaction hashes and CIDs
                // 5. We call storage.add_storage_record() directly (NOT StorageHistoryManager)
                //
                // NOTE: StorageHistoryManager.record_item_storage() is NEVER called.
                // That's deprecated code with placeholder values. The real recording
                // happens right here using actual blockchain transaction data.
                // ============================================================
                let storage_record = crate::types::StorageRecord {
                    adapter_type: adapter_type.clone(),
                    storage_location: storage_location.clone(), // Primary: IPCM tx or IPFS CID
                    stored_at: Utc::now(),
                    triggered_by: "circuit_push".to_string(),
                    triggered_by_id: Some(circuit_id.to_string()),
                    events_range: None,
                    is_active: true,
                    metadata: transaction_metadata.clone(), // Contains: nft_mint_tx, ipcm_update_tx, ipfs_cid, etc.
                };

                self.storage
                    .add_storage_record(&dfid, storage_record) // ← THIS is where history is recorded
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

                // ============================================================
                // DUAL-WRITE STRATEGY: CID Timeline Entry
                // ============================================================
                // Write CID timeline entry immediately to storage (InMemory + PostgreSQL queue)
                // This provides instant timeline availability without waiting for blockchain polling.
                // The event listener will write the same data later for verification/redundancy.
                // Benefits:
                // - Instant timeline queries
                // - Blockchain verification via event listener
                // - Can compare both sources to detect issues
                // - Fallback if event listener has problems
                // ============================================================
                if let (Some(cid), Some(ipcm_tx)) = (
                    transaction_metadata
                        .get("ipfs_cid")
                        .and_then(|v| v.as_str()),
                    transaction_metadata
                        .get("ipcm_update_tx")
                        .and_then(|v| v.as_str()),
                ) {
                    let network = transaction_metadata
                        .get("network")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let blockchain_timestamp = Utc::now().timestamp();

                    // Write to storage backend (InMemoryStorage + PostgreSQL queue)
                    if let Err(e) = self.storage.add_cid_to_timeline(
                        &dfid,
                        cid,
                        ipcm_tx,
                        blockchain_timestamp,
                        network,
                    ) {
                        tracing::warn!(
                            "⚠️  Failed to add CID to timeline (non-fatal): {} -> {} ({})",
                            dfid,
                            cid,
                            e
                        );
                        // Don't fail the push operation if timeline write fails
                    } else {
                        tracing::info!(
                            "✅ Added CID to timeline: {} -> {} (TX: {}, source: push_direct)",
                            dfid,
                            cid,
                            ipcm_tx
                        );
                    }
                }

                self.logger
                    .lock()
                    .unwrap()
                    .info(
                        "circuits_engine",
                        "adapter_upload_success",
                        "Item uploaded to adapter",
                    )
                    .with_context("dfid", dfid.clone())
                    .with_context("adapter_type", format!("{adapter_type:?}"))
                    .with_context("storage_hash", storage_hash.clone());

                Some(WebhookStorageData {
                    adapter_type: format!("{adapter_type:?}"),
                    location: format!("{:?}", upload_result.metadata.item_location),
                    hash: storage_hash.clone(),
                    cid: if matches!(storage_location, StorageLocation::IPFS { .. }) {
                        Some(storage_hash.clone())
                    } else {
                        None
                    },
                    metadata: {
                        let mut map = HashMap::new();
                        map.insert(
                            "stored_at".to_string(),
                            serde_json::json!(upload_result.metadata.created_at.to_rfc3339()),
                        );
                        map
                    },
                })
            } else {
                None
            }
        } else {
            None
        };

        // 7. Create and store operation
        let mut operation = CircuitOperation::new(
            *circuit_id,
            dfid.clone(),
            OperationType::Push,
            requester_id.to_string(),
        );

        if circuit.permissions.require_approval_for_push {
            operation.status = crate::types::OperationStatus::Pending;
        } else {
            operation.approve();
            operation.complete();

            // Log activity
            let activity = Activity::new(
                ActivityType::Push,
                *circuit_id,
                circuit.name.clone(),
                vec![dfid.clone()],
                requester_id.to_string(),
                ActivityStatus::Success,
                ActivityDetails {
                    items_affected: 1,
                    enrichments_made: None,
                    error_message: None,
                },
            );
            self.storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            self.spawn_persist_activity(activity.clone());
        }

        self.storage
            .store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Create event
        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine
            .create_circuit_operation_event(
                dfid.clone(),
                circuit_id.to_string(),
                "push".to_string(),
                requester_id.to_string(),
                visibility,
            )
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "item_tokenized",
                "Item tokenized and pushed to circuit",
            )
            .with_context("local_id", local_id.to_string())
            .with_context("dfid", dfid.clone())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("status", format!("{status:?}"));

        // Trigger webhooks if configured (optional)
        let trigger_event = match status {
            PushStatus::NewItemCreated => PostActionTrigger::ItemTokenized,
            PushStatus::ExistingItemEnriched => PostActionTrigger::ItemPushed,
            PushStatus::ConflictDetected { .. } => PostActionTrigger::ItemPushed,
        };

        self.trigger_post_action_webhooks(
            &circuit,
            &dfid,
            Some(local_id),
            requester_id,
            &operation.operation_id,
            trigger_event,
            storage_details, // NOW INCLUDES REAL STORAGE DETAILS!
        )
        .await;

        Ok(PushResult {
            dfid,
            status,
            operation_id: operation.operation_id,
            local_id: *local_id,
        })
    }

    fn validate_circuit_requirements(
        &self,
        circuit: &Circuit,
        identifiers: &[EnhancedIdentifier],
    ) -> Result<(), CircuitsError> {
        let Some(ref config) = circuit.alias_config else {
            return Ok(());
        };

        // Validate allowed namespaces
        if let Some(ref allowed) = config.allowed_namespaces {
            for id in identifiers {
                if !allowed.contains(&id.namespace) {
                    return Err(CircuitsError::ValidationError(format!(
                        "Namespace '{}' not allowed in this circuit",
                        id.namespace
                    )));
                }
            }
        }

        // Validate required canonical identifiers
        for required in &config.required_canonical {
            let found = identifiers.iter().any(|id| {
                if let IdentifierType::Canonical { ref registry, .. } = id.id_type {
                    registry == required
                } else {
                    false
                }
            });

            if !found {
                return Err(CircuitsError::ValidationError(format!(
                    "Required canonical identifier '{required}' not provided"
                )));
            }
        }

        // Validate required contextual identifiers
        for required in &config.required_contextual {
            let found = identifiers.iter().any(|id| {
                matches!(id.id_type, IdentifierType::Contextual { .. }) && id.key == *required
            });

            if !found {
                return Err(CircuitsError::ValidationError(format!(
                    "Required contextual identifier '{required}' not provided"
                )));
            }
        }

        // Validate identifier formats
        for id in identifiers {
            if !id.validate() {
                return Err(CircuitsError::ValidationError(format!(
                    "Invalid identifier format: {}",
                    id.unique_key()
                )));
            }
        }

        Ok(())
    }

    async fn resolve_or_create_dfid(
        &self,
        identifiers: &[EnhancedIdentifier],
        circuit: &Circuit,
        requester_id: &str,
        local_id: &Uuid,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<(String, PushStatus), CircuitsError> {
        // STEP 1: Look for canonical identifiers
        for identifier in identifiers {
            if let IdentifierType::Canonical { ref registry, .. } = identifier.id_type {
                if let Some(dfid) = self
                    .storage
                    .get_dfid_by_canonical(&identifier.namespace, registry, &identifier.value)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                {
                    // Found! Enrich existing item
                    self.enrich_existing_item_internal(
                        &dfid,
                        identifiers,
                        enriched_data,
                        requester_id,
                    )?;
                    return Ok((dfid, PushStatus::ExistingItemEnriched));
                }
            }
        }

        // STEP 2: Look for fingerprint (if configured)
        if circuit
            .alias_config
            .as_ref()
            .map(|c| c.use_fingerprint)
            .unwrap_or(false)
        {
            let fingerprint = self.generate_fingerprint(identifiers, requester_id, local_id);

            if let Some(dfid) = self
                .storage
                .get_dfid_by_fingerprint(&fingerprint, &circuit.circuit_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            {
                self.enrich_existing_item_internal(
                    &dfid,
                    identifiers,
                    enriched_data,
                    requester_id,
                )?;
                return Ok((dfid, PushStatus::ExistingItemEnriched));
            }

            // Save fingerprint for future lookups
            let dfid = self.create_new_tokenized_item(
                identifiers,
                enriched_data,
                requester_id,
                local_id,
                Some(fingerprint.clone()),
            )?;

            self.storage
                .store_fingerprint_mapping(&fingerprint, &dfid, &circuit.circuit_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            return Ok((dfid, PushStatus::NewItemCreated));
        }

        // STEP 3: Create new tokenized item
        let dfid = self.create_new_tokenized_item(
            identifiers,
            enriched_data,
            requester_id,
            local_id,
            None,
        )?;

        Ok((dfid, PushStatus::NewItemCreated))
    }

    fn generate_fingerprint(
        &self,
        identifiers: &[Identifier],
        requester_id: &str,
        local_id: &Uuid,
    ) -> String {
        let mut sorted_keys: Vec<String> = identifiers.iter().map(|id| id.unique_key()).collect();
        sorted_keys.sort();

        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let combined = format!(
            "user:{}|lid:{}|time:{}|ids:{}",
            requester_id,
            local_id,
            timestamp,
            sorted_keys.join("|")
        );

        blake3::hash(combined.as_bytes()).to_hex().to_string()
    }

    fn create_new_tokenized_item(
        &self,
        identifiers: &[Identifier],
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        requester_id: &str,
        local_id: &Uuid,
        fingerprint: Option<String>,
    ) -> Result<String, CircuitsError> {
        let dfid = self.dfid_engine.generate_dfid();

        let mut item = Item {
            dfid: dfid.clone(),
            local_id: Some(*local_id),
            legacy_mode: false,
            identifiers: identifiers.to_vec(),
            aliases: vec![],
            fingerprint,
            enriched_data: enriched_data.unwrap_or_default(),
            creation_timestamp: Utc::now(),
            last_modified: Utc::now(),
            source_entries: vec![Uuid::new_v4()],
            confidence_score: 1.0,
            status: ItemStatus::Active,
        };

        // Add alias from requester
        item.aliases.push(ExternalAlias::new(
            &format!("user:{requester_id}"),
            &local_id.to_string(),
            requester_id,
            blake3::hash(local_id.as_bytes()).to_hex().as_ref(),
        ));

        self.storage
            .store_item(&item)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Save canonical identifier mappings
        for identifier in identifiers {
            if identifier.is_canonical() {
                self.storage
                    .store_enhanced_identifier_mapping(identifier, &dfid)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            }
        }

        Ok(dfid)
    }

    fn enrich_existing_item_internal(
        &self,
        dfid: &str,
        new_identifiers: &[Identifier],
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        requester_id: &str,
    ) -> Result<(), CircuitsError> {
        let mut item = self
            .storage
            .get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        // Add new identifiers
        for id in new_identifiers {
            if !item.identifiers.contains(id) {
                item.identifiers.push(id.clone());
            }
        }

        // Add alias from this push
        item.aliases.push(ExternalAlias::new(
            &format!("user:{requester_id}"),
            &Uuid::new_v4().to_string(),
            requester_id,
            blake3::hash(dfid.as_bytes()).to_hex().as_ref(),
        ));

        // Add enriched data
        if let Some(data) = enriched_data {
            item.enriched_data.extend(data);
        }

        item.last_modified = Utc::now();

        self.storage
            .update_item(&item)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn handle_storage_migration(
        &self,
        _dfid: &str,
        _circuit: &Circuit,
        _user_id: &str,
    ) -> Result<(), String> {
        // Check if storage history manager is available
        // TODO: Re-enable adapter configuration when Circuit struct includes adapter_config field
        // Circuit adapter configuration would need to be stored separately or added to Circuit struct
        /*
        if let Some(ref history_manager) = self.storage_history_manager {
            // Check if circuit has adapter configuration
            if let Some(ref adapter_config) = circuit.adapter_config {
                self.logger.lock().unwrap().info("circuits_engine", "storage_migration_start", "Starting storage migration for item")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string())
                    .with_context("target_adapter", format!("{:?}", adapter_config.adapter_type));

                // Use the storage history manager to handle migration
                let adapter_instance = create_adapter_instance(&adapter_config.adapter_type)
                    .map_err(|e| format!("Failed to create adapter instance: {}", e))?;
                history_manager.migrate_to_circuit_adapter(
                    dfid,
                    &adapter_instance,
                    circuit.circuit_id,
                    user_id,
                ).await.map_err(|e| e.to_string())?;

                self.logger.lock().unwrap().info("circuits_engine", "storage_migration_complete", "Storage migration completed successfully")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string());
            } else {
                self.logger.lock().unwrap().info("circuits_engine", "storage_migration_skipped", "No adapter configuration found for circuit")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string());
            }
        } else {
            self.logger.lock().unwrap().warn("circuits_engine", "storage_migration_unavailable", "Storage history manager not available for migration")
                .with_context("dfid", dfid.to_string())
                .with_context("circuit_id", circuit.circuit_id.to_string());
        }
        */

        Ok(())
    }

    pub async fn pull_item_from_circuit(
        &mut self,
        dfid: &str,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<(Item, CircuitOperation), CircuitsError> {
        let circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Pull) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to pull from this circuit".to_string(),
            ));
        }

        let item = self
            .storage
            .get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        let mut operation = CircuitOperation::new(
            *circuit_id,
            dfid.to_string(),
            OperationType::Pull,
            requester_id.to_string(),
        );

        if circuit.permissions.require_approval_for_pull {
            operation.status = crate::types::OperationStatus::Pending;
        } else {
            operation.approve();
            operation.complete();

            // Log pull activity
            let activity = Activity::new(
                ActivityType::Pull,
                *circuit_id,
                circuit.name.clone(),
                vec![dfid.to_string()],
                requester_id.to_string(),
                ActivityStatus::Success,
                ActivityDetails {
                    items_affected: 1,
                    enrichments_made: None,
                    error_message: None,
                },
            );
            self.storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            self.spawn_persist_activity(activity.clone());
        }

        self.storage
            .store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine
            .create_circuit_operation_event(
                dfid.to_string(),
                circuit_id.to_string(),
                "pull".to_string(),
                requester_id.to_string(),
                visibility,
            )
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        if !circuit.permissions.require_approval_for_pull {
            self.storage
                .remove_circuit_item(circuit_id, dfid)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
        }

        self.logger
            .lock()
            .unwrap()
            .info("circuits_engine", "item_pulled", "Item pulled from circuit")
            .with_context("dfid", dfid.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("operation_id", operation.operation_id.to_string());

        Ok((item, operation))
    }

    pub async fn approve_operation(
        &mut self,
        operation_id: &Uuid,
        approver_id: &str,
    ) -> Result<CircuitOperation, CircuitsError> {
        let mut operation = self
            .storage
            .get_circuit_operation(operation_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::NotFound)?;

        let circuit = self
            .storage
            .get_circuit(&operation.circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(approver_id, &Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to approve operations".to_string(),
            ));
        }

        operation.approve();
        operation.complete();

        let op_type = operation.operation_type.clone();
        match op_type {
            OperationType::Push => {
                let circuit_item = CircuitItem::new(
                    operation.dfid.clone(),
                    operation.circuit_id,
                    operation.requester_id.clone(),
                    vec!["read".to_string(), "verify".to_string()],
                );
                self.storage
                    .store_circuit_item(&circuit_item)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

                let activity = Activity::new(
                    ActivityType::Push,
                    operation.circuit_id,
                    circuit.name.clone(),
                    vec![operation.dfid.clone()],
                    operation.requester_id.clone(),
                    ActivityStatus::Success,
                    ActivityDetails {
                        items_affected: 1,
                        enrichments_made: None,
                        error_message: None,
                    },
                );
                self.storage
                    .store_activity(&activity)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
                self.spawn_persist_activity(activity.clone());

                // Handle auto-publish if enabled
                if let Some(ref public_settings) = circuit.public_settings {
                    if public_settings.auto_publish_pushed_items {
                        let mut updated_circuit = circuit.clone();
                        if let Some(ref mut settings) = updated_circuit.public_settings {
                            if !settings.published_items.contains(&operation.dfid) {
                                settings.published_items.push(operation.dfid.clone());
                                self.storage
                                    .store_circuit(&updated_circuit)
                                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
                            }
                        }
                    }
                }
            }
            OperationType::Pull => {
                self.storage
                    .remove_circuit_item(&operation.circuit_id, &operation.dfid)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            }
        }

        self.storage
            .update_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "operation_approved",
                "Circuit operation approved",
            )
            .with_context("operation_id", operation_id.to_string())
            .with_context("approver_id", approver_id.to_string());

        Ok(operation)
    }

    pub async fn reject_operation(
        &mut self,
        operation_id: &Uuid,
        rejecter_id: &str,
        reason: Option<String>,
    ) -> Result<CircuitOperation, CircuitsError> {
        let mut operation = self
            .storage
            .get_circuit_operation(operation_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::NotFound)?;

        let circuit = self
            .storage
            .get_circuit(&operation.circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(rejecter_id, &Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to reject operations".to_string(),
            ));
        }

        operation.status = OperationStatus::Rejected;

        // Store rejection reason in metadata
        if let Some(reason_text) = reason {
            operation.metadata.insert(
                "rejection_reason".to_string(),
                serde_json::Value::String(reason_text),
            );
        }
        operation.metadata.insert(
            "rejected_by".to_string(),
            serde_json::Value::String(rejecter_id.to_string()),
        );
        operation.metadata.insert(
            "rejected_at".to_string(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );

        self.storage
            .update_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "operation_rejected",
                "Circuit operation rejected",
            )
            .with_context("operation_id", operation_id.to_string())
            .with_context("rejecter_id", rejecter_id.to_string());

        Ok(operation)
    }

    pub fn list_circuits(&self) -> Result<Vec<Circuit>, CircuitsError> {
        self.storage
            .list_circuits()
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, CircuitsError> {
        self.storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, CircuitsError> {
        self.storage
            .get_circuits_for_member(member_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuit_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, CircuitsError> {
        self.storage
            .get_circuit_operations(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_pending_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, CircuitsError> {
        let operations = self.get_circuit_operations(circuit_id)?;
        Ok(operations
            .into_iter()
            .filter(|op| matches!(op.status, OperationStatus::Pending))
            .collect())
    }

    pub async fn deactivate_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::ManagePermissions) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to deactivate circuit".to_string(),
            ));
        }

        circuit.status = CircuitStatus::Inactive;
        circuit.last_modified = chrono::Utc::now();

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_deactivated",
                "Circuit deactivated",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub async fn get_logs(&self) -> Vec<crate::logging::LogEntry> {
        self.logger.lock().unwrap().get_logs().to_vec()
    }

    pub async fn get_logs_by_event_type(&self, event_type: &str) -> Vec<crate::logging::LogEntry> {
        self.logger
            .lock()
            .unwrap()
            .get_logs_by_event_type(event_type)
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn request_to_join_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        message: Option<String>,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        circuit
            .add_join_request(requester_id.to_string(), message)
            .map_err(CircuitsError::ValidationError)?;

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "join_request_created",
                "Join request submitted",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub async fn approve_join_request(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        approver_id: &str,
        role: MemberRole,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if approver has permission to manage members
        if !circuit.has_permission(approver_id, &crate::types::Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to approve join requests".to_string(),
            ));
        }

        circuit
            .approve_join_request(requester_id, role)
            .map_err(CircuitsError::ValidationError)?;

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "join_request_approved",
                "Join request approved",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("approver_id", approver_id.to_string());

        Ok(circuit)
    }

    pub async fn reject_join_request(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        rejector_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if rejector has permission to manage members
        if !circuit.has_permission(rejector_id, &crate::types::Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to reject join requests".to_string(),
            ));
        }

        circuit
            .reject_join_request(requester_id)
            .map_err(CircuitsError::ValidationError)?;

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "join_request_rejected",
                "Join request rejected",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("rejector_id", rejector_id.to_string());

        Ok(circuit)
    }

    pub fn get_pending_join_requests(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<crate::types::JoinRequest>, CircuitsError> {
        let circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        Ok(circuit
            .get_pending_requests()
            .into_iter()
            .cloned()
            .collect())
    }

    pub async fn update_circuit(
        &mut self,
        circuit_id: &Uuid,
        name: Option<String>,
        description: Option<String>,
        permissions: Option<crate::types::CircuitPermissions>,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to update circuit
        if !circuit.has_permission(requester_id, &crate::types::Permission::ManagePermissions)
            && circuit.owner_id != requester_id
        {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to update circuit".to_string(),
            ));
        }

        // Apply updates
        if let Some(new_name) = name {
            circuit.update_name(new_name);
        }

        if let Some(new_description) = description {
            circuit.update_description(new_description);
        }

        if let Some(new_permissions) = permissions {
            circuit.update_permissions(new_permissions);
        }

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "circuit_updated",
                "Circuit updated successfully",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub async fn set_circuit_adapter_config(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        adapter_type: Option<AdapterType>,
        auto_migrate_existing: bool,
        requires_approval: bool,
        sponsor_adapter_access: bool,
    ) -> Result<CircuitAdapterConfig, CircuitsError> {
        // Get the circuit
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Validate requester is owner or admin
        if circuit.owner_id != requester_id
            && !circuit.has_permission(requester_id, &Permission::ManagePermissions)
        {
            return Err(CircuitsError::PermissionDenied(
                "Only circuit owner or admins can configure adapter settings".to_string(),
            ));
        }

        // If an adapter is specified, validate requester's tier has access to it
        if let Some(ref adapter) = adapter_type {
            let user = self
                .storage
                .get_user_account(requester_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                .ok_or_else(|| CircuitsError::ValidationError("User not found".to_string()))?;

            if !validate_adapter_tier_access(&user.tier, adapter) {
                return Err(CircuitsError::PermissionDenied(
                    format!(
                        "Your tier ({}) does not have access to the {:?} adapter. Please upgrade your tier or contact an administrator.",
                        user.tier.as_str(),
                        adapter
                    )
                ));
            }
        }

        // Create the adapter config
        let adapter_config = CircuitAdapterConfig {
            circuit_id: *circuit_id,
            adapter_type,
            configured_by: requester_id.to_string(),
            configured_at: chrono::Utc::now(),
            requires_approval,
            auto_migrate_existing,
            sponsor_adapter_access,
        };

        // Update the circuit
        circuit.adapter_config = Some(adapter_config.clone());
        circuit.last_modified = chrono::Utc::now();

        tracing::info!(
            "🔧 Setting circuit {} adapter_config: {:?}",
            circuit_id,
            circuit.adapter_config
        );

        self.storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        tracing::info!(
            "✅ Circuit {} adapter config persisted via storage.update_circuit()",
            circuit_id
        );

        // Send notifications to all circuit members
        for member in &circuit.members {
            let notification = Notification::new(
                member.member_id.clone(),
                NotificationType::CircuitAdapterConfigUpdated,
                "Circuit Adapter Configuration Updated".to_string(),
                format!(
                    "The adapter configuration for circuit '{}' has been updated by {}",
                    circuit.name, requester_id
                ),
                serde_json::json!({
                    "circuit_id": circuit_id,
                    "circuit_name": circuit.name,
                    "adapter_type": adapter_config.adapter_type.as_ref().map(|a| format!("{a:?}")),
                    "sponsor_adapter_access": sponsor_adapter_access,
                    "configured_by": requester_id,
                }),
            );

            self.storage
                .store_notification(&notification)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
        }

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "adapter_config_updated",
                "Circuit adapter configuration updated",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("adapter_type", format!("{:?}", adapter_config.adapter_type))
            .with_context("sponsor_adapter_access", sponsor_adapter_access.to_string());

        Ok(adapter_config)
    }

    pub async fn create_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: String,
        permissions: Vec<Permission>,
        description: String,
        color: Option<String>,
        requester_id: &str,
    ) -> Result<CustomRole, CircuitsError> {
        let mut circuit = self
            .get_circuit(circuit_id)?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Add the custom role
        circuit
            .add_custom_role(
                role_name.clone(),
                permissions,
                description,
                color,
                requester_id.to_string(),
            )
            .map_err(CircuitsError::ValidationError)?;

        // Save the updated circuit
        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let custom_role = circuit
            .get_custom_role(&role_name)
            .ok_or(CircuitsError::ValidationError(
                "Failed to create role".to_string(),
            ))?
            .clone();

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "custom_role_created",
                "Custom role created successfully",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name)
            .with_context("created_by", requester_id.to_string());

        Ok(custom_role)
    }

    pub fn get_custom_roles(&self, circuit_id: &Uuid) -> Result<Vec<CustomRole>, CircuitsError> {
        let circuit = self
            .get_circuit(circuit_id)?
            .ok_or(CircuitsError::CircuitNotFound)?;
        Ok(circuit.custom_roles.clone())
    }

    pub async fn assign_member_custom_role(
        &mut self,
        circuit_id: &Uuid,
        member_id: &str,
        role_name: &str,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .get_circuit(circuit_id)?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage members
        if !circuit.has_permission(requester_id, &Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage members".to_string(),
            ));
        }

        // Assign the custom role
        circuit
            .assign_custom_role(member_id, role_name)
            .map_err(CircuitsError::ValidationError)?;

        // Save the updated circuit
        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "member_role_assigned",
                "Member role assigned successfully",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("member_id", member_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("assigned_by", requester_id.to_string());

        Ok(circuit)
    }

    pub async fn remove_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: &str,
        requester_id: &str,
    ) -> Result<(), CircuitsError> {
        let mut circuit = self
            .get_circuit(circuit_id)?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Remove the custom role
        circuit
            .remove_custom_role(role_name)
            .map_err(CircuitsError::ValidationError)?;

        // Save the updated circuit
        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "custom_role_removed",
                "Custom role removed successfully",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("removed_by", requester_id.to_string());

        Ok(())
    }

    pub async fn update_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: &str,
        new_permissions: Option<Vec<Permission>>,
        new_description: Option<String>,
        new_color: Option<String>,
        requester_id: &str,
    ) -> Result<CustomRole, CircuitsError> {
        let mut circuit = self
            .get_circuit(circuit_id)?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Update the custom role
        circuit
            .update_custom_role(
                role_name,
                new_permissions.clone(),
                new_description.clone(),
                new_color.clone(),
            )
            .map_err(CircuitsError::ValidationError)?;

        // Save the updated circuit
        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let updated_role = circuit
            .get_custom_role(role_name)
            .ok_or(CircuitsError::ValidationError(
                "Failed to retrieve updated role".to_string(),
            ))?
            .clone();

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "custom_role_updated",
                "Custom role updated successfully",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("updated_by", requester_id.to_string());

        Ok(updated_role)
    }

    pub async fn update_public_settings(
        &mut self,
        circuit_id: &Uuid,
        settings: crate::types::PublicSettings,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage circuit settings
        if !circuit.has_permission(requester_id, &crate::types::Permission::ManagePermissions)
            && circuit.owner_id != requester_id
        {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to update public settings".to_string(),
            ));
        }

        // Update public settings
        circuit
            .update_public_settings(settings)
            .map_err(CircuitsError::ValidationError)?;

        // Automatically enable public visibility when public settings are configured
        // This ensures the circuit becomes accessible after configuring public settings
        circuit.permissions.allow_public_visibility = true;

        // Store updated circuit
        self.storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "circuits_engine",
                "public_settings_updated",
                "Public settings updated and public visibility enabled",
            )
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("updated_by", requester_id.to_string())
            .with_context("public_visibility", "true");

        Ok(circuit)
    }

    pub fn get_public_circuit_info(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Option<crate::types::PublicCircuitInfo>, CircuitsError> {
        let (mut public_info, show_encrypted_events) = {
            let circuit = self
                .storage
                .get_circuit(circuit_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                .ok_or(CircuitsError::CircuitNotFound)?;

            let public_info = match circuit.get_public_info() {
                Some(info) => info,
                None => return Ok(None),
            };

            let show_encrypted_events = circuit
                .public_settings
                .as_ref()
                .map(|s| s.show_encrypted_events)
                .unwrap_or(false);

            (public_info, show_encrypted_events)
        }; // Storage lock is released here

        // Get events for each published item (storage lock is now free)
        let mut published_items_with_events = Vec::new();

        for dfid in &public_info.published_items {
            // Get events for this item
            let all_events = self
                .events_engine
                .get_events_for_item(dfid)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            // Filter events: must be Public AND (not encrypted OR show_encrypted_events is true)
            let filtered_events: Vec<crate::types::Event> = all_events
                .into_iter()
                .filter(|event| {
                    matches!(event.visibility, crate::types::EventVisibility::Public)
                        && (!event.is_encrypted || show_encrypted_events)
                })
                .collect();

            published_items_with_events.push(crate::types::PublicItemWithEvents {
                dfid: dfid.clone(),
                events: filtered_events,
            });
        }

        public_info.published_items_with_events = published_items_with_events;

        Ok(Some(public_info))
    }

    pub async fn join_public_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        access_password: Option<String>,
        message: Option<String>,
    ) -> Result<(bool, String), CircuitsError> {
        let mut circuit = self
            .storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if circuit is publicly accessible
        if !circuit.is_publicly_accessible() {
            return Err(CircuitsError::PermissionDenied(
                "Circuit is not publicly accessible".to_string(),
            ));
        }

        // Check if requester is already a member
        if circuit.is_member(requester_id) {
            return Err(CircuitsError::ValidationError(
                "User is already a member of this circuit".to_string(),
            ));
        }

        // Check password for protected circuits
        if let Some(ref settings) = circuit.public_settings {
            if let crate::types::PublicAccessMode::Protected = settings.access_mode {
                if let Some(ref expected_password) = settings.access_password {
                    if access_password.as_ref() != Some(expected_password) {
                        return Err(CircuitsError::PermissionDenied(
                            "Invalid access password".to_string(),
                        ));
                    }
                } else {
                    return Err(CircuitsError::ValidationError(
                        "Circuit requires password but none is configured".to_string(),
                    ));
                }
            }
        }

        // Check if auto-approval is enabled
        let auto_approve = circuit
            .public_settings
            .as_ref()
            .map(|s| s.auto_approve_members)
            .unwrap_or(false);

        if auto_approve {
            // Add member directly
            circuit.add_member(requester_id.to_string(), crate::types::MemberRole::Member);

            self.storage
                .store_circuit(&circuit)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            self.logger
                .lock()
                .unwrap()
                .info(
                    "circuits_engine",
                    "public_join_auto_approved",
                    "User automatically joined public circuit",
                )
                .with_context("circuit_id", circuit_id.to_string())
                .with_context("new_member", requester_id.to_string());

            Ok((false, "Automatically approved".to_string()))
        } else {
            // Create join request
            circuit
                .add_join_request(requester_id.to_string(), message)
                .map_err(CircuitsError::ValidationError)?;

            self.storage
                .store_circuit(&circuit)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            self.logger
                .lock()
                .unwrap()
                .info(
                    "circuits_engine",
                    "public_join_request_created",
                    "Join request created for public circuit",
                )
                .with_context("circuit_id", circuit_id.to_string())
                .with_context("requester", requester_id.to_string());

            Ok((true, "Join request submitted".to_string()))
        }
    }

    pub async fn batch_push_items(
        &mut self,
        dfids: &[String],
        circuit_id: &Uuid,
        requester_id: &str,
        _permissions: Option<Vec<String>>,
    ) -> Result<BatchPushResult, CircuitsError> {
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;

        for dfid in dfids {
            match self
                .push_item_to_circuit(dfid, circuit_id, requester_id)
                .await
            {
                Ok(_) => {
                    success_count += 1;
                    results.push(BatchPushItemResult {
                        dfid: dfid.clone(),
                        success: true,
                        error_message: None,
                    });
                }
                Err(e) => {
                    failed_count += 1;
                    results.push(BatchPushItemResult {
                        dfid: dfid.clone(),
                        success: false,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(BatchPushResult {
            success_count,
            failed_count,
            results,
        })
    }

    pub fn get_circuit_items(&self, circuit_id: &Uuid) -> Result<Vec<CircuitItem>, CircuitsError> {
        self.storage
            .get_circuit_items(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, CircuitsError> {
        self.storage
            .get_activities_for_user(user_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_activities_for_circuit(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<Activity>, CircuitsError> {
        self.storage
            .get_activities_for_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_all_activities(&self) -> Result<Vec<Activity>, CircuitsError> {
        self.storage
            .get_all_activities()
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_events_for_item(
        &self,
        dfid: &str,
    ) -> Result<Vec<crate::types::Event>, CircuitsError> {
        self.events_engine
            .get_events_for_item(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    /// Trigger webhooks for post-action events (completely optional for circuit owner)
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::await_holding_refcell_ref)]
    async fn trigger_post_action_webhooks(
        &mut self,
        circuit: &Circuit,
        dfid: &str,
        local_id: Option<&Uuid>,
        requester_id: &str,
        operation_id: &Uuid,
        trigger_event: PostActionTrigger,
        storage_details: Option<WebhookStorageData>,
    ) {
        // Check if post-action settings are enabled (optional)
        let post_settings = match &circuit.post_action_settings {
            Some(settings) if settings.enabled => settings,
            _ => return, // Not enabled, skip webhook trigger
        };

        // Check if this trigger event is configured
        if !post_settings.trigger_events.contains(&trigger_event) {
            return; // Event not configured, skip
        }

        // Get item for identifiers
        let item = self.storage.get_item_by_dfid(dfid).ok().flatten();

        let identifiers = if let Some(item) = &item {
            // Convert identifiers to simple HashMap format
            item.identifiers
                .iter()
                .map(|id| {
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), id.key.clone());
                    map.insert("value".to_string(), id.value.clone());
                    map
                })
                .collect()
        } else {
            vec![]
        };

        // Build webhook payload
        let payload = WebhookPayload {
            event_type: trigger_event.as_str().to_string(),
            circuit_id: circuit.circuit_id.to_string(),
            circuit_name: circuit.name.clone(),
            timestamp: Utc::now(),
            item: WebhookItemData {
                dfid: dfid.to_string(),
                local_id: local_id.map(|lid| lid.to_string()),
                identifiers,
                pushed_by: requester_id.to_string(),
            },
            storage: if post_settings.include_storage_details {
                storage_details
            } else {
                None
            },
            operation_id: operation_id.to_string(),
            status: "completed".to_string(),
        };

        // Trigger webhooks asynchronously
        let webhook_result = {
            let mut webhook_guard = self.webhook_engine.write().await;
            webhook_guard
                .trigger_webhooks(&circuit.circuit_id, trigger_event, payload)
                .await
        };

        match webhook_result {
            Ok(delivery_ids) => {
                if !delivery_ids.is_empty() {
                    self.logger
                        .lock()
                        .unwrap()
                        .info(
                            "circuits_engine",
                            "webhooks_triggered",
                            format!(
                                "Triggered {} webhooks for event {:?}",
                                delivery_ids.len(),
                                trigger_event
                            ),
                        )
                        .with_context("circuit_id", circuit.circuit_id.to_string())
                        .with_context("dfid", dfid.to_string())
                        .with_context("delivery_count", delivery_ids.len().to_string());
                }
            }
            Err(e) => {
                self.logger
                    .lock()
                    .unwrap()
                    .warn(
                        "circuits_engine",
                        "webhook_trigger_failed",
                        format!("Failed to trigger webhooks: {e}"),
                    )
                    .with_context("circuit_id", circuit.circuit_id.to_string())
                    .with_context("dfid", dfid.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::types::Identifier;
    use std::sync::Arc;

    fn create_test_item(storage: &Arc<std::sync::Mutex<InMemoryStorage>>, dfid: &str) {
        let identifiers = vec![Identifier::new("test_key", "test_value")];
        let item = crate::types::Item::new(dfid.to_string(), identifiers, uuid::Uuid::new_v4());
        let s = storage.lock().unwrap();
        let _ = s.store_item(&item);
    }

    #[tokio::test]
    async fn test_create_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut circuits_engine = CircuitsEngine::new(storage);

        let result = circuits_engine
            .create_circuit(
                "Test Circuit".to_string(),
                "A test circuit".to_string(),
                "owner123".to_string(),
                None,
                None,
            )
            .await;

        assert!(result.is_ok());
        let circuit = result.unwrap();
        assert_eq!(circuit.name, "Test Circuit");
        assert_eq!(circuit.owner_id, "owner123");
        assert_eq!(circuit.members.len(), 1);
        assert_eq!(circuit.members[0].member_id, "owner123");
    }

    #[tokio::test]
    async fn test_add_member_to_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine
            .create_circuit(
                "Test Circuit".to_string(),
                "A test circuit".to_string(),
                "owner123".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let result = circuits_engine
            .add_member_to_circuit(
                &circuit.circuit_id,
                "member456".to_string(),
                MemberRole::Member,
                "owner123",
            )
            .await;

        assert!(result.is_ok());
        let updated_circuit = result.unwrap();
        assert_eq!(updated_circuit.members.len(), 2);
    }

    #[tokio::test]
    async fn test_push_item_to_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine
            .create_circuit(
                "Test Circuit".to_string(),
                "A test circuit".to_string(),
                "owner123".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let result = circuits_engine
            .push_item_to_circuit("DFID-123", &circuit.circuit_id, "owner123")
            .await;
        assert!(result.is_ok());

        let operation = result.unwrap();
        assert_eq!(operation.dfid, "DFID-123");
        assert_eq!(operation.circuit_id, circuit.circuit_id);
        assert_eq!(operation.requester_id, "owner123");
    }

    #[tokio::test]
    async fn test_pull_item_from_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine
            .create_circuit(
                "Test Circuit".to_string(),
                "A test circuit".to_string(),
                "owner123".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let result = circuits_engine
            .pull_item_from_circuit("DFID-123", &circuit.circuit_id, "owner123")
            .await;
        assert!(result.is_ok());

        let (item, operation) = result.unwrap();
        assert_eq!(item.dfid, "DFID-123");
        assert_eq!(operation.dfid, "DFID-123");
        assert_eq!(operation.circuit_id, circuit.circuit_id);
    }

    #[tokio::test]
    async fn test_permission_denied() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine
            .create_circuit(
                "Test Circuit".to_string(),
                "A test circuit".to_string(),
                "owner123".to_string(),
                None,
                None,
            )
            .await
            .unwrap();

        let result = circuits_engine
            .push_item_to_circuit("DFID-123", &circuit.circuit_id, "unauthorized_user")
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            CircuitsError::PermissionDenied(_)
        ));
    }
}

// New structures for push_local_item_to_circuit
#[derive(Debug, Clone)]
pub struct PushResult {
    pub dfid: String,
    pub status: PushStatus,
    pub operation_id: Uuid,
    pub local_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum PushStatus {
    NewItemCreated,
    ExistingItemEnriched,
    ConflictDetected { conflicting_dfids: Vec<String> },
}
