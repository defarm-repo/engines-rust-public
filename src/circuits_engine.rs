use crate::events_engine::EventsEngine;
use crate::logging::LoggingEngine;
use crate::storage::StorageBackend;
use crate::types::{
    Circuit, CircuitOperation, CircuitStatus, EventVisibility,
    Item, MemberRole, OperationStatus, OperationType, Permission, CustomRole,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub enum CircuitsError {
    StorageError(String),
    PermissionDenied(String),
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
            CircuitsError::ValidationError(e) => write!(f, "Validation error: {}", e),
            CircuitsError::NotFound => write!(f, "Circuit not found"),
            CircuitsError::ItemNotFound => write!(f, "Item not found"),
            CircuitsError::CircuitNotFound => write!(f, "Circuit not found"),
            CircuitsError::MemberNotFound => write!(f, "Member not found"),
        }
    }
}

impl std::error::Error for CircuitsError {}

pub struct CircuitsEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
    logger: std::cell::RefCell<LoggingEngine>,
    events_engine: EventsEngine<S>,
}

impl<S: StorageBackend> CircuitsEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        let logger = LoggingEngine::new();
        let events_engine = EventsEngine::new(Arc::clone(&storage));
        Self { storage, logger: std::cell::RefCell::new(logger), events_engine }
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

    pub fn push_item_to_circuit(
        &mut self,
        dfid: &str,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<CircuitOperation, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        let _item = storage
            .get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        let circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Push) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to push to this circuit".to_string(),
            ));
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

        self.logger.borrow_mut().info("circuits_engine", "item_pushed", "Item pushed to circuit")
            .with_context("dfid", dfid.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("requester_id", requester_id.to_string())
            .with_context("operation_id", operation.operation_id.to_string());

        Ok(operation)
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

        storage
            .update_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "operation_approved", "Circuit operation approved")
            .with_context("operation_id", operation_id.to_string())
            .with_context("approver_id", approver_id.to_string());

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
        let storage = self.storage.lock().unwrap();

        let circuit = storage
            .get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        Ok(circuit.get_public_info())
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