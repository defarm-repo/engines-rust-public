// This file contains the new push_local_item_to_circuit method
// It will be added to circuits_engine.rs

use crate::identifier_types::{EnhancedIdentifier, ExternalAlias, IdentifierType};
use crate::types::{Item, ItemStatus, Identifier, CircuitItem, Activity, ActivityType, ActivityStatus, ActivityDetails, EventVisibility};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

// Add these methods to CircuitsEngine impl block:

    // NEW: Push with LID (tokenization in circuit)
    pub async fn push_local_item_to_circuit(
        &mut self,
        local_id: &Uuid,
        mut identifiers: Vec<EnhancedIdentifier>,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        circuit_id: &Uuid,
        requester_id: &str,
    ) -> Result<PushResult, CircuitsError> {
        let mut storage = self.storage.lock().unwrap();

        // 1. Get circuit and validate permissions
        let circuit = storage.get_circuit(circuit_id)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::CircuitNotFound)?;

        if !circuit.has_permission(requester_id, &Permission::Push) {
            return Err(CircuitsError::PermissionDenied(
                "User does not have permission to push to this circuit".to_string()
            ));
        }

        // 2. Auto-apply namespace if configured
        if circuit.alias_config.as_ref()
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
        let (dfid, status) = self.resolve_or_create_dfid(
            &identifiers,
            &circuit,
            requester_id,
            local_id,
            enriched_data.clone(),
            &mut storage,
        ).await?;

        // 5. Save LID -> DFID mapping
        storage.store_lid_dfid_mapping(local_id, &dfid)?;

        // 6. Create circuit item and operation
        let circuit_item = CircuitItem::new(
            dfid.clone(),
            *circuit_id,
            requester_id.to_string(),
            vec!["read".to_string(), "verify".to_string()],
        );
        storage.store_circuit_item(&circuit_item)?;

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
            storage.store_activity(&activity)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
        }

        storage.store_circuit_operation(&operation)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        drop(storage);

        // Create event
        let visibility = if circuit.permissions.allow_public_visibility {
            EventVisibility::Public
        } else {
            EventVisibility::CircuitOnly
        };

        self.events_engine.create_circuit_operation_event(
            dfid.clone(),
            circuit_id.to_string(),
            "push".to_string(),
            requester_id.to_string(),
            visibility,
        ).map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        self.logger.borrow_mut().info("circuits_engine", "item_tokenized", "Item tokenized and pushed to circuit")
            .with_context("local_id", local_id.to_string())
            .with_context("dfid", dfid.clone())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("status", format!("{:?}", status));

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
                    return Err(CircuitsError::ValidationError(
                        format!("Namespace '{}' not allowed in this circuit", id.namespace)
                    ));
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
                return Err(CircuitsError::ValidationError(
                    format!("Required canonical identifier '{}' not provided", required)
                ));
            }
        }

        // Validate required contextual identifiers
        for required in &config.required_contextual {
            let found = identifiers.iter().any(|id| {
                matches!(id.id_type, IdentifierType::Contextual { .. }) && id.key == *required
            });

            if !found {
                return Err(CircuitsError::ValidationError(
                    format!("Required contextual identifier '{}' not provided", required)
                ));
            }
        }

        // Validate identifier formats
        for id in identifiers {
            if !id.validate() {
                return Err(CircuitsError::ValidationError(
                    format!("Invalid identifier format: {}", id.unique_key())
                ));
            }
        }

        Ok(())
    }

    async fn resolve_or_create_dfid(
        &mut self,
        identifiers: &[EnhancedIdentifier],
        circuit: &Circuit,
        requester_id: &str,
        local_id: &Uuid,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        storage: &mut std::sync::MutexGuard<S>,
    ) -> Result<(String, PushStatus), CircuitsError> {
        // STEP 1: Look for canonical identifiers
        for identifier in identifiers {
            if let IdentifierType::Canonical { ref registry, .. } = identifier.id_type {
                if let Some(dfid) = storage.get_dfid_by_canonical(
                    &identifier.namespace,
                    registry,
                    &identifier.value
                ).map_err(|e| CircuitsError::StorageError(e.to_string()))? {
                    // Found! Enrich existing item
                    self.enrich_existing_item_internal(
                        &dfid,
                        identifiers,
                        enriched_data,
                        requester_id,
                        storage,
                    )?;
                    return Ok((dfid, PushStatus::ExistingItemEnriched));
                }
            }
        }

        // STEP 2: Look for fingerprint (if configured)
        if circuit.alias_config.as_ref()
            .map(|c| c.use_fingerprint)
            .unwrap_or(false)
        {
            let fingerprint = self.generate_fingerprint(identifiers, requester_id, local_id);

            if let Some(dfid) = storage.get_dfid_by_fingerprint(&fingerprint, &circuit.circuit_id)
                .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            {
                self.enrich_existing_item_internal(
                    &dfid,
                    identifiers,
                    enriched_data,
                    requester_id,
                    storage,
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
                storage,
            )?;

            storage.store_fingerprint_mapping(&fingerprint, &dfid, &circuit.circuit_id)
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
            storage,
        )?;

        Ok((dfid, PushStatus::NewItemCreated))
    }

    fn generate_fingerprint(
        &self,
        identifiers: &[EnhancedIdentifier],
        requester_id: &str,
        local_id: &Uuid,
    ) -> String {
        let mut sorted_keys: Vec<String> = identifiers.iter()
            .map(|id| id.unique_key())
            .collect();
        sorted_keys.sort();

        let timestamp = chrono::Utc::now().timestamp_nanos();
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
        &mut self,
        identifiers: &[EnhancedIdentifier],
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        requester_id: &str,
        local_id: &Uuid,
        fingerprint: Option<String>,
        storage: &mut std::sync::MutexGuard<S>,
    ) -> Result<String, CircuitsError> {
        let dfid = self.dfid_engine.generate_dfid();

        // Convert enhanced identifiers to legacy identifiers (for compatibility)
        let legacy_identifiers: Vec<Identifier> = identifiers.iter()
            .map(|id| Identifier::new(&id.key, &id.value))
            .collect();

        let mut item = Item {
            dfid: dfid.clone(),
            local_id: Some(*local_id),
            legacy_mode: false,
            identifiers: legacy_identifiers,
            enhanced_identifiers: identifiers.to_vec(),
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
            &format!("user:{}", requester_id),
            &local_id.to_string(),
            requester_id,
            &blake3::hash(local_id.as_bytes()).to_hex().to_string(),
        ));

        storage.store_item(&item)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        // Save canonical identifier mappings
        for identifier in identifiers {
            if identifier.is_canonical() {
                storage.store_enhanced_identifier_mapping(identifier, &dfid)
                    .map_err(|e| CircuitsError::StorageError(e.to_string()))?;
            }
        }

        Ok(dfid)
    }

    fn enrich_existing_item_internal(
        &mut self,
        dfid: &str,
        new_identifiers: &[EnhancedIdentifier],
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        requester_id: &str,
        storage: &mut std::sync::MutexGuard<S>,
    ) -> Result<(), CircuitsError> {
        let mut item = storage.get_item_by_dfid(dfid)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?
            .ok_or(CircuitsError::ItemNotFound)?;

        // Add new enhanced identifiers
        for id in new_identifiers {
            if !item.enhanced_identifiers.contains(id) {
                item.enhanced_identifiers.push(id.clone());
            }
        }

        // Add alias from this push
        item.aliases.push(ExternalAlias::new(
            &format!("user:{}", requester_id),
            &Uuid::new_v4().to_string(),
            requester_id,
            &blake3::hash(dfid.as_bytes()).to_hex().to_string(),
        ));

        // Add enriched data
        if let Some(data) = enriched_data {
            item.enriched_data.extend(data);
        }

        item.last_modified = Utc::now();

        storage.update_item(&item)
            .map_err(|e| CircuitsError::StorageError(e.to_string()))?;

        Ok(())
    }

// Add these types/structs:

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    pub dfid: String,
    pub status: PushStatus,
    pub operation_id: Uuid,
    pub local_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushStatus {
    NewItemCreated,
    ExistingItemEnriched,
    ConflictDetected { conflicting_dfids: Vec<String> },
}