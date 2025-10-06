use crate::events_engine::EventsEngine;
use crate::logging::LoggingEngine;
use crate::storage::StorageBackend;
use crate::storage_history_manager::StorageHistoryManager;
use crate::api::adapters::create_adapter_instance;
use crate::types::{
    Activity, ActivityDetails, ActivityStatus, ActivityType, BatchPushResult, BatchPushItemResult,
    Circuit, CircuitAdapterConfig, CircuitItem, CircuitOperation, CircuitStatus, EventVisibility,
    Item, MemberRole, Notification, NotificationType, OperationStatus, OperationType, Permission, CustomRole, AdapterType, UserTier,
};
use chrono::Utc;
use std::sync::Arc;
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
            CircuitsError::StorageError(e) => write!(f, "Storage error: {}", e),
            CircuitsError::PermissionDenied(e) => write!(f, "Permission denied: {}", e),
            CircuitsError::AdapterPermissionDenied(e) => write!(f, "Adapter permission denied: {}", e),
            CircuitsError::ValidationError(e) => write!(f, "Validation error: {}", e),
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
            AdapterType::LocalLocal,
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::StellarMainnetIpfs,
            AdapterType::LocalIpfs,
            AdapterType::StellarMainnetStellarMainnet,
        ],
        UserTier::Professional => vec![
            AdapterType::LocalLocal,
            AdapterType::IpfsIpfs,
            AdapterType::StellarTestnetIpfs,
            AdapterType::LocalIpfs,
        ],
        UserTier::Basic => vec![
            AdapterType::LocalLocal,
            AdapterType::LocalIpfs,
        ],
    }
}

// Helper function to validate if a user tier has access to an adapter
fn validate_adapter_tier_access(user_tier: &UserTier, adapter_type: &AdapterType) -> bool {
    match adapter_type {
        // Basic tier adapters - all tiers have access
        AdapterType::LocalLocal | AdapterType::LocalIpfs => true,

        // Professional tier adapters - Professional, Enterprise, Admin
        AdapterType::IpfsIpfs | AdapterType::StellarTestnetIpfs => {
            matches!(user_tier, UserTier::Professional | UserTier::Enterprise | UserTier::Admin)
        }

        // Enterprise tier adapters - Enterprise, Admin only
        AdapterType::StellarMainnetIpfs | AdapterType::StellarMainnetStellarMainnet => {
            matches!(user_tier, UserTier::Enterprise | UserTier::Admin)
        }

        // Other adapters (Ethereum, Polygon, etc.) - currently not available
        _ => false,
    }
}

pub struct CircuitsEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
    logger: std::cell::RefCell<LoggingEngine>,
    events_engine: EventsEngine<S>,
    storage_history_manager: Option<StorageHistoryManager<S>>,
}

impl<S: StorageBackend> CircuitsEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        let logger = LoggingEngine::new();
        let events_engine = EventsEngine::new(Arc::clone(&storage));
        Self {
            storage,
            logger: std::cell::RefCell::new(logger),
            events_engine,
            storage_history_manager: None,
        }
    }

    pub fn with_storage_history_manager(mut self, storage_history_manager: StorageHistoryManager<S>) -> Self {
        self.storage_history_manager = Some(storage_history_manager);
        self
    }

    fn handle_auto_publish(
        &self,
        circuit: &Circuit,
        dfid: &str,
        circuit_id: &Uuid,
        storage: &mut std::sync::MutexGuard<S>,
    ) -> Result<(), CircuitsError> {
        println!("DEBUG AUTO-PUBLISH: Checking auto-publish for circuit {}", circuit_id);
        if let Some(ref public_settings) = circuit.public_settings {
            println!("DEBUG AUTO-PUBLISH: Circuit has public_settings, auto_publish_pushed_items: {}", public_settings.auto_publish_pushed_items);
            if public_settings.auto_publish_pushed_items {
                println!("DEBUG AUTO-PUBLISH: Auto-publish is enabled, adding {} to published_items", dfid);
                let mut updated_circuit = circuit.clone();
                if let Some(ref mut settings) = updated_circuit.public_settings {
                    if !settings.published_items.contains(&dfid.to_string()) {
                        println!("DEBUG AUTO-PUBLISH: Adding {} to published_items (not already present)", dfid);
                        settings.published_items.push(dfid.to_string());
                        storage
                            .store_circuit(&updated_circuit)
                            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
                        println!("DEBUG AUTO-PUBLISH: Successfully added {} to published_items", dfid);
                    } else {
                        println!("DEBUG AUTO-PUBLISH: {} already in published_items", dfid);
                    }
                } else {
                    println!("DEBUG AUTO-PUBLISH: No public_settings found in updated_circuit");
                }
            } else {
                println!("DEBUG AUTO-PUBLISH: Auto-publish is disabled");
            }
        } else {
            println!("DEBUG AUTO-PUBLISH: Circuit has no public_settings");
        }
        Ok(())
    }

    pub fn create_circuit(
        &mut self,
        name: String,
        description: String,
        owner_id: String,
    ) -> Result<Circuit, CircuitsError> {
        let circuit = Circuit::new(name.clone(), description.clone(), owner_id.clone());

        self.logger.borrow_mut().info("circuits_engine", "circuit_creation_started", &format!("Creating circuit: {}", name))
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("owner_id", owner_id.clone());

        let mut storage = self.storage.lock().unwrap();
        storage
            .store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "circuit_created", "Circuit created successfully")
            .with_context("circuit_id", circuit.circuit_id.to_string())
            .with_context("name", name);

        Ok(circuit)
    }

    pub fn add_member_to_circuit(
        &mut self,
        circuit_id: &Uuid,
        member_id: String,
        role: MemberRole,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();
        let mut circuit = storage
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

        circuit.add_member(member_id.clone(), role.clone());

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "member_added", "Member added to circuit")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("member_id", member_id)
            .with_context("role", format!("{:?}", role))
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub async fn push_item_to_circuit(
        &mut self,
        dfid: &str,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<CircuitOperation, CircuitsError> {
        println!("DEBUG PUSH: Attempting to push item {} to circuit {} by user {}", dfid, circuit_id, requester_id);

        let mut storage = self.storage.lock().unwrap();

        let _item = storage
            .get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        let circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| {
                println!("DEBUG PUSH: Storage error while getting circuit {}: {}", circuit_id, e);
                CircuitsError::StorageError(e.to_string())
            })?
            .ok_or_else(|| {
                println!("DEBUG PUSH: Circuit {} not found in storage", circuit_id);
                CircuitsError::CircuitNotFound
            })?;

        println!("DEBUG PUSH: Circuit {} found with name '{}', checking permissions...", circuit_id, circuit.name);

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
                    let user = storage
                        .get_user_account(requester_id)
                        .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                        .ok_or_else(|| CircuitsError::ValidationError("User not found".to_string()))?;

                    // Get user's available adapters (custom or tier defaults)
                    let user_adapters = if let Some(custom_adapters) = &user.available_adapters {
                        custom_adapters.clone()
                    } else {
                        get_tier_default_adapters(&user.tier)
                    };

                    // Check if user has access to the required adapter
                    if !user_adapters.contains(required_adapter) {
                        let adapter_name = format!("{:?}", required_adapter);
                        let user_adapters_str = user_adapters.iter()
                            .map(|a| format!("{:?}", a))
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(CircuitsError::AdapterPermissionDenied(
                            format!(
                                "This circuit requires '{}' adapter access. You currently have access to: {}. Please contact your administrator to request access to this adapter.",
                                adapter_name,
                                user_adapters_str
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
            storage
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
            storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        }

        // Handle auto-publish for immediate pushes (not requiring approval)
        if !circuit.permissions.require_approval_for_push {
            self.handle_auto_publish(&circuit, dfid, circuit_id, &mut storage)?;
        }

        storage
            .store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        drop(storage);

        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine.create_circuit_operation_event(
            dfid.to_string(),
            circuit_id.to_string(),
            "push".to_string(),
            requester_id.to_string(),
            visibility,
        ).map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Handle storage migration if circuit has a different adapter configuration
        if !circuit.permissions.require_approval_for_push {
            self.handle_storage_migration(dfid, &circuit, requester_id).await
                .map_err(|e| CircuitsError::StorageError(format!("Storage migration failed: {}", e)))?;
        }

        self.logger.borrow_mut().info("circuits_engine", "item_pushed", "Item pushed to circuit")
            .with_context("dfid", dfid.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("operation_id", operation.operation_id.to_string());

        Ok(operation)
    }

    async fn handle_storage_migration(
        &self,
        dfid: &str,
        circuit: &Circuit,
        user_id: &str,
    ) -> Result<(), String> {
        // Check if storage history manager is available
        // TODO: Re-enable adapter configuration when Circuit struct includes adapter_config field
        // Circuit adapter configuration would need to be stored separately or added to Circuit struct
        /*
        if let Some(ref history_manager) = self.storage_history_manager {
            // Check if circuit has adapter configuration
            if let Some(ref adapter_config) = circuit.adapter_config {
                self.logger.borrow_mut().info("circuits_engine", "storage_migration_start", "Starting storage migration for item")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string())
                    .with_context("target_adapter", format!("{:?}", adapter_config.adapter_type));

                // Use the storage history manager to handle migration
                history_manager.migrate_to_circuit_adapter(
                    dfid,
                    &create_adapter_instance(&adapter_config.adapter_type),
                    circuit.circuit_id,
                    user_id,
                ).await.map_err(|e| e.to_string())?;

                self.logger.borrow_mut().info("circuits_engine", "storage_migration_complete", "Storage migration completed successfully")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string());
            } else {
                self.logger.borrow_mut().info("circuits_engine", "storage_migration_skipped", "No adapter configuration found for circuit")
                    .with_context("dfid", dfid.to_string())
                    .with_context("circuit_id", circuit.circuit_id.to_string());
            }
        } else {
            self.logger.borrow_mut().warn("circuits_engine", "storage_migration_unavailable", "Storage history manager not available for migration")
                .with_context("dfid", dfid.to_string())
                .with_context("circuit_id", circuit.circuit_id.to_string());
        }
        */

        Ok(())
    }

    pub fn pull_item_from_circuit(
        &mut self,
        dfid: &str,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<(Item, CircuitOperation), CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Pull) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to pull from this circuit".to_string(),
            ));
        }

        let item = storage
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
            storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
        }

        storage
            .store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        drop(storage);

        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine.create_circuit_operation_event(
            dfid.to_string(),
            circuit_id.to_string(),
            "pull".to_string(),
            requester_id.to_string(),
            visibility,
        ).map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "item_pulled", "Item pulled from circuit")
            .with_context("dfid", dfid.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("operation_id", operation.operation_id.to_string());

        Ok((item, operation))
    }

    pub fn approve_operation(
        &mut self,
        operation_id: &Uuid,
        approver_id: &str,
    ) -> Result<CircuitOperation, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();
        let mut operation = storage
            .get_circuit_operation(operation_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::NotFound)?;

        let circuit = storage
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

        // If this is a push operation, store the circuit item and log activity
        if matches!(operation.operation_type, OperationType::Push) {
            let circuit_item = CircuitItem::new(
                operation.dfid.clone(),
                operation.circuit_id,
                operation.requester_id.clone(),
                vec!["read".to_string(), "verify".to_string()],
            );
            storage
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
            storage
                .store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            // Handle auto-publish if enabled
            if let Some(ref public_settings) = circuit.public_settings {
                if public_settings.auto_publish_pushed_items {
                    let mut updated_circuit = circuit.clone();
                    if let Some(ref mut settings) = updated_circuit.public_settings {
                        if !settings.published_items.contains(&operation.dfid) {
                            settings.published_items.push(operation.dfid.clone());
                            storage
                                .store_circuit(&updated_circuit)
                                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
                        }
                    }
                }
            }
        }

        storage
            .update_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "operation_approved", "Circuit operation approved")
            .with_context("operation_id", operation_id.to_string())
            .with_context("approver_id", approver_id.to_string());

        Ok(operation)
    }

    pub fn reject_operation(
        &mut self,
        operation_id: &Uuid,
        rejecter_id: &str,
        reason: Option<String>,
    ) -> Result<CircuitOperation, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();
        let mut operation = storage
            .get_circuit_operation(operation_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::NotFound)?;

        let circuit = storage
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

        storage
            .update_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "operation_rejected", "Circuit operation rejected")
            .with_context("operation_id", operation_id.to_string())
            .with_context("rejecter_id", rejecter_id.to_string());

        Ok(operation)
    }

    pub fn list_circuits(&self) -> Result<Vec<Circuit>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .list_circuits()
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_circuits_for_member(member_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_circuit_operations(&self, circuit_id: &Uuid) -> Result<Vec<CircuitOperation>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_circuit_operations(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_pending_operations(&self, circuit_id: &Uuid) -> Result<Vec<CircuitOperation>, CircuitsError> {
        let operations = self.get_circuit_operations(circuit_id)?;
        Ok(operations
            .into_iter()
            .filter(|op| matches!(op.status, OperationStatus::Pending))
            .collect())
    }

    pub fn deactivate_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();
        let mut circuit = storage
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

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "circuit_deactivated", "Circuit deactivated")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub fn get_logs(&self) -> Vec<crate::logging::LogEntry> {
        self.logger.borrow().get_logs().to_vec()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<crate::logging::LogEntry> {
        self.logger.borrow().get_logs_by_event_type(event_type).into_iter().cloned().collect()
    }

    pub fn request_to_join_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        message: Option<String>,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        circuit.add_join_request(requester_id.to_string(), message)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "join_request_created", "Join request submitted")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub fn approve_join_request(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        approver_id: &str,
        role: MemberRole,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if approver has permission to manage members
        if !circuit.has_permission(approver_id, &crate::types::Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to approve join requests".to_string(),
            ));
        }

        circuit.approve_join_request(requester_id, role)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "join_request_approved", "Join request approved")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("approver_id", approver_id.to_string());

        Ok(circuit)
    }

    pub fn reject_join_request(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        rejector_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if rejector has permission to manage members
        if !circuit.has_permission(rejector_id, &crate::types::Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to reject join requests".to_string(),
            ));
        }

        circuit.reject_join_request(requester_id)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "join_request_rejected", "Join request rejected")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("rejector_id", rejector_id.to_string());

        Ok(circuit)
    }

    pub fn get_pending_join_requests(&self, circuit_id: &Uuid) -> Result<Vec<crate::types::JoinRequest>, CircuitsError> {
        let storage = self.storage.lock().unwrap();

        let circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        Ok(circuit.get_pending_requests().into_iter().cloned().collect())
    }

    pub fn update_circuit(
        &mut self,
        circuit_id: &Uuid,
        name: Option<String>,
        description: Option<String>,
        permissions: Option<crate::types::CircuitPermissions>,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to update circuit
        if !circuit.has_permission(requester_id, &crate::types::Permission::ManagePermissions) &&
           circuit.owner_id != requester_id {
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

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "circuit_updated", "Circuit updated successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string());

        Ok(circuit)
    }

    pub fn set_circuit_adapter_config(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        adapter_type: Option<AdapterType>,
        auto_migrate_existing: bool,
        requires_approval: bool,
        sponsor_adapter_access: bool,
    ) -> Result<CircuitAdapterConfig, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        // Get the circuit
        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Validate requester is owner or admin
        if circuit.owner_id != requester_id &&
           !circuit.has_permission(requester_id, &Permission::ManagePermissions) {
            return Err(CircuitsError::PermissionDenied(
                "Only circuit owner or admins can configure adapter settings".to_string(),
            ));
        }

        // If an adapter is specified, validate requester's tier has access to it
        if let Some(ref adapter) = adapter_type {
            let user = storage
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

        storage
            .update_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Send notifications to all circuit members
        for member in &circuit.members {
            let notification = Notification::new(
                member.member_id.clone(),
                NotificationType::CircuitAdapterConfigUpdated,
                "Circuit Adapter Configuration Updated".to_string(),
                format!(
                    "The adapter configuration for circuit '{}' has been updated by {}",
                    circuit.name,
                    requester_id
                ),
                serde_json::json!({
                    "circuit_id": circuit_id,
                    "circuit_name": circuit.name,
                    "adapter_type": adapter_config.adapter_type.as_ref().map(|a| format!("{:?}", a)),
                    "sponsor_adapter_access": sponsor_adapter_access,
                    "configured_by": requester_id,
                }),
            );

            storage
                .store_notification(&notification)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
        }

        self.logger.borrow_mut().info("circuits_engine", "adapter_config_updated", "Circuit adapter configuration updated")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("adapter_type", format!("{:?}", adapter_config.adapter_type))
            .with_context("sponsor_adapter_access", sponsor_adapter_access.to_string());

        Ok(adapter_config)
    }

    pub fn create_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: String,
        permissions: Vec<Permission>,
        description: String,
        color: Option<String>,
        requester_id: &str,
    ) -> Result<CustomRole, CircuitsError> {
        let mut circuit = self.get_circuit(circuit_id)?.ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Add the custom role
        circuit.add_custom_role(role_name.clone(), permissions, description, color, requester_id.to_string())
            .map_err(|e| CircuitsError::ValidationError(e))?;

        // Save the updated circuit
        self.storage.lock().unwrap().store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let custom_role = circuit.get_custom_role(&role_name)
            .ok_or(CircuitsError::ValidationError("Failed to create role".to_string()))?
            .clone();

        self.logger.borrow_mut().info("circuits_engine", "custom_role_created", "Custom role created successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name)
            .with_context("created_by", requester_id.to_string());

        Ok(custom_role)
    }

    pub fn get_custom_roles(&self, circuit_id: &Uuid) -> Result<Vec<CustomRole>, CircuitsError> {
        let circuit = self.get_circuit(circuit_id)?.ok_or(CircuitsError::CircuitNotFound)?;
        Ok(circuit.custom_roles.clone())
    }

    pub fn assign_member_custom_role(
        &mut self,
        circuit_id: &Uuid,
        member_id: &str,
        role_name: &str,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut circuit = self.get_circuit(circuit_id)?.ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage members
        if !circuit.has_permission(requester_id, &Permission::ManageMembers) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage members".to_string(),
            ));
        }

        // Assign the custom role
        circuit.assign_custom_role(member_id, role_name)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        // Save the updated circuit
        self.storage.lock().unwrap().store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "member_role_assigned", "Member role assigned successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("member_id", member_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("assigned_by", requester_id.to_string());

        Ok(circuit)
    }

    pub fn remove_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: &str,
        requester_id: &str,
    ) -> Result<(), CircuitsError> {
        let mut circuit = self.get_circuit(circuit_id)?.ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Remove the custom role
        circuit.remove_custom_role(role_name)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        // Save the updated circuit
        self.storage.lock().unwrap().store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "custom_role_removed", "Custom role removed successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("removed_by", requester_id.to_string());

        Ok(())
    }

    pub fn update_custom_role(
        &mut self,
        circuit_id: &Uuid,
        role_name: &str,
        new_permissions: Option<Vec<Permission>>,
        new_description: Option<String>,
        new_color: Option<String>,
        requester_id: &str,
    ) -> Result<CustomRole, CircuitsError> {
        let mut circuit = self.get_circuit(circuit_id)?.ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage roles
        if !circuit.has_permission(requester_id, &Permission::ManageRoles) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to manage roles".to_string(),
            ));
        }

        // Update the custom role
        circuit.update_custom_role(role_name, new_permissions.clone(), new_description.clone(), new_color.clone())
            .map_err(|e| CircuitsError::ValidationError(e))?;

        // Save the updated circuit
        self.storage.lock().unwrap().store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        let updated_role = circuit.get_custom_role(role_name)
            .ok_or(CircuitsError::ValidationError("Failed to retrieve updated role".to_string()))?
            .clone();

        self.logger.borrow_mut().info("circuits_engine", "custom_role_updated", "Custom role updated successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("role_name", role_name.to_string())
            .with_context("updated_by", requester_id.to_string());

        Ok(updated_role)
    }

    pub fn update_public_settings(
        &mut self,
        circuit_id: &Uuid,
        settings: crate::types::PublicSettings,
        requester_id: &str,
    ) -> Result<Circuit, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        // Check if requester has permission to manage circuit settings
        if !circuit.has_permission(requester_id, &crate::types::Permission::ManagePermissions) &&
           circuit.owner_id != requester_id {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to update public settings".to_string(),
            ));
        }

        // Update public settings
        circuit.update_public_settings(settings)
            .map_err(|e| CircuitsError::ValidationError(e))?;

        // Store updated circuit
        storage.store_circuit(&circuit)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "public_settings_updated", "Public settings updated successfully")
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("updated_by", requester_id.to_string());

        Ok(circuit)
    }

    pub fn get_public_circuit_info(&self, circuit_id: &Uuid) -> Result<Option<crate::types::PublicCircuitInfo>, CircuitsError> {
        let (mut public_info, show_encrypted_events) = {
            let storage = self.storage.lock().unwrap();

            let circuit = storage
                .get_circuit(circuit_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?
                .ok_or(CircuitsError::CircuitNotFound)?;

            let public_info = match circuit.get_public_info() {
                Some(info) => info,
                None => return Ok(None),
            };

            let show_encrypted_events = circuit.public_settings
                .as_ref()
                .map(|s| s.show_encrypted_events)
                .unwrap_or(false);

            (public_info, show_encrypted_events)
        }; // Storage lock is released here

        // Get events for each published item (storage lock is now free)
        let mut published_items_with_events = Vec::new();

        for dfid in &public_info.published_items {
            // Get events for this item
            let all_events = self.events_engine.get_events_for_item(dfid)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            // Filter events: must be Public AND (not encrypted OR show_encrypted_events is true)
            let filtered_events: Vec<crate::types::Event> = all_events
                .into_iter()
                .filter(|event| {
                    matches!(event.visibility, crate::types::EventVisibility::Public) &&
                    (!event.is_encrypted || show_encrypted_events)
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

    pub fn join_public_circuit(
        &mut self,
        circuit_id: &Uuid,
        requester_id: &str,
        access_password: Option<String>,
        message: Option<String>,
    ) -> Result<(bool, String), CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let mut circuit = storage
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
        let auto_approve = circuit.public_settings
            .as_ref()
            .map(|s| s.auto_approve_members)
            .unwrap_or(false);

        if auto_approve {
            // Add member directly
            circuit.add_member(requester_id.to_string(), crate::types::MemberRole::Member);

            storage.store_circuit(&circuit)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            self.logger.borrow_mut().info("circuits_engine", "public_join_auto_approved", "User automatically joined public circuit")
                .with_context("circuit_id", circuit_id.to_string())
                .with_context("new_member", requester_id.to_string());

            Ok((false, "Automatically approved".to_string()))
        } else {
            // Create join request
            circuit.add_join_request(requester_id.to_string(), message)
                .map_err(|e| CircuitsError::ValidationError(e))?;

            storage.store_circuit(&circuit)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

            self.logger.borrow_mut().info("circuits_engine", "public_join_request_created", "Join request created for public circuit")
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
        permissions: Option<Vec<String>>,
    ) -> Result<BatchPushResult, CircuitsError> {
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failed_count = 0;

        for dfid in dfids {
            match self.push_item_to_circuit(dfid, circuit_id, requester_id).await {
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
        let storage = self.storage.lock().unwrap();
        storage
            .get_circuit_items(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_activities_for_user(user_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_activities_for_circuit(&self, circuit_id: &Uuid) -> Result<Vec<Activity>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_activities_for_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
    }

    pub fn get_all_activities(&self) -> Result<Vec<Activity>, CircuitsError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get_all_activities()
            .map_err(|e| CircuitsError::StorageError(e.to_string()))
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
        storage.lock().unwrap().store_item(&item).unwrap();
    }

    #[test]
    fn test_create_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut circuits_engine = CircuitsEngine::new(storage);

        let result = circuits_engine.create_circuit(
            "Test Circuit".to_string(),
            "A test circuit".to_string(),
            "owner123".to_string(),
        );

        assert!(result.is_ok());
        let circuit = result.unwrap();
        assert_eq!(circuit.name, "Test Circuit");
        assert_eq!(circuit.owner_id, "owner123");
        assert_eq!(circuit.members.len(), 1);
        assert_eq!(circuit.members[0].member_id, "owner123");
    }

    #[test]
    fn test_add_member_to_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine.create_circuit(
            "Test Circuit".to_string(),
            "A test circuit".to_string(),
            "owner123".to_string(),
        ).unwrap();

        let result = circuits_engine.add_member_to_circuit(
            &circuit.circuit_id,
            "member456".to_string(),
            MemberRole::Member,
            "owner123",
        );

        assert!(result.is_ok());
        let updated_circuit = result.unwrap();
        assert_eq!(updated_circuit.members.len(), 2);
    }

    #[test]
    fn test_push_item_to_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine.create_circuit(
            "Test Circuit".to_string(),
            "A test circuit".to_string(),
            "owner123".to_string(),
        ).unwrap();

        let result = circuits_engine.push_item_to_circuit("DFID-123", &circuit.circuit_id, "owner123");
        assert!(result.is_ok());

        let operation = result.unwrap();
        assert_eq!(operation.dfid, "DFID-123");
        assert_eq!(operation.circuit_id, circuit.circuit_id);
        assert_eq!(operation.requester_id, "owner123");
    }

    #[test]
    fn test_pull_item_from_circuit() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine.create_circuit(
            "Test Circuit".to_string(),
            "A test circuit".to_string(),
            "owner123".to_string(),
        ).unwrap();

        let result = circuits_engine.pull_item_from_circuit("DFID-123", &circuit.circuit_id, "owner123");
        assert!(result.is_ok());

        let (item, operation) = result.unwrap();
        assert_eq!(item.dfid, "DFID-123");
        assert_eq!(operation.dfid, "DFID-123");
        assert_eq!(operation.circuit_id, circuit.circuit_id);
    }

    #[test]
    fn test_permission_denied() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        create_test_item(&storage, "DFID-123");
        let mut circuits_engine = CircuitsEngine::new(storage);

        let circuit = circuits_engine.create_circuit(
            "Test Circuit".to_string(),
            "A test circuit".to_string(),
            "owner123".to_string(),
        ).unwrap();

        let result = circuits_engine.push_item_to_circuit("DFID-123", &circuit.circuit_id, "unauthorized_user");
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), CircuitsError::PermissionDenied(_)));
    }
}