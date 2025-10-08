use crate::dfid_engine::DfidEngine;
use crate::logging::{LoggingEngine, LogEntry};
use crate::storage::{StorageError, StorageBackend};
use crate::conflict_detection::ConflictDetectionEngine;
use crate::types::{Item, Identifier, ItemStatus, ItemShare, SharedItemResponse, PendingItem, PendingReason};
use crate::storage_history_manager::StorageHistoryManager;
use crate::adapters::{AdapterInstance, StorageLocation, StorageAdapter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub enum ItemsError {
    StorageError(StorageError),
    ItemNotFound(String),
    InvalidOperation(String),
    ValidationError(String),
}

impl std::fmt::Display for ItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemsError::StorageError(e) => write!(f, "Storage error: {}", e),
            ItemsError::ItemNotFound(dfid) => write!(f, "Item not found: {}", dfid),
            ItemsError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            ItemsError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ItemsError {}

impl From<StorageError> for ItemsError {
    fn from(err: StorageError) -> Self {
        ItemsError::StorageError(err)
    }
}


pub struct ItemsEngine<S: StorageBackend> {
    storage: S,
    logger: LoggingEngine,
    dfid_engine: DfidEngine,
    storage_history_manager: Option<StorageHistoryManager<S>>,
}

impl<S: StorageBackend + 'static> ItemsEngine<S> {
    pub fn new(storage: S) -> Self {
        let mut logger = LoggingEngine::new();
        logger.info("ItemsEngine", "initialization", "Items engine initialized");

        Self {
            storage,
            logger,
            dfid_engine: DfidEngine::new(),
            storage_history_manager: None,
        }
    }

    pub fn with_storage_history_manager(mut self, storage_history_manager: StorageHistoryManager<S>) -> Self {
        self.storage_history_manager = Some(storage_history_manager);
        self
    }

    pub fn create_item(&mut self, dfid: String, identifiers: Vec<Identifier>, source_entry: Uuid) -> Result<Item, ItemsError> {
        // Check if item already exists
        if self.storage.get_item_by_dfid(&dfid)?.is_some() {
            return Err(ItemsError::InvalidOperation(format!("Item with DFID {} already exists", dfid)));
        }

        let item = Item::new(dfid.clone(), identifiers, source_entry);

        self.logger.info("ItemsEngine", "item_creation", "Creating new item")
            .with_context("dfid", dfid.clone())
            .with_context("source_entry", source_entry.to_string());

        self.storage.store_item(&item)?;

        self.logger.info("ItemsEngine", "item_created", "Item created successfully")
            .with_context("dfid", dfid.clone());


        Ok(item)
    }

    pub fn create_item_with_generated_dfid(&mut self, identifiers: Vec<Identifier>, source_entry: Uuid, enriched_data: Option<HashMap<String, serde_json::Value>>) -> Result<Item, ItemsError> {
        // Step 0: Check for conflicts and handle them
        if let Some(pending_reason) = self.detect_conflicts(&identifiers, &enriched_data, source_entry)? {
            // Store as pending item
            let pending_item = PendingItem::new(identifiers, enriched_data, source_entry, pending_reason, None, None);
            self.storage.store_pending_item(&pending_item)?;

            self.logger.info("ItemsEngine", "pending_item_created", "Item stored as pending due to conflicts")
                .with_context("pending_id", pending_item.pending_id.to_string())
                .with_context("reason", format!("{:?}", pending_item.reason))
                .with_context("source_entry", source_entry.to_string());

            // For now, we'll return an error to maintain API compatibility
            // Later we can modify the API to return a "pending" result
            return Err(ItemsError::ValidationError(format!("Item stored as pending: {:?}", pending_item.reason)));
        }

        // Step 1: Check if any identifier matches existing items (entity resolution)
        for identifier in &identifiers {
            if let Ok(existing_items) = self.find_items_by_identifier(identifier) {
                if let Some(existing_item) = existing_items.first() {
                    let dfid = existing_item.dfid.clone();

                    self.logger.info("ItemsEngine", "duplicate_detected", "Found existing item with matching identifier")
                        .with_context("existing_dfid", dfid.clone())
                        .with_context("matching_identifier", format!("{}:{}", identifier.key, identifier.value))
                        .with_context("source_entry", source_entry.to_string());

                    // Add any new identifiers to existing item
                    let new_identifiers: Vec<Identifier> = identifiers.into_iter()
                        .filter(|id| !existing_item.identifiers.contains(id))
                        .collect();

                    if !new_identifiers.is_empty() {
                        self.add_identifiers(&dfid, new_identifiers)?;
                    }

                    // Enrich existing item with new data
                    if let Some(data) = enriched_data {
                        return self.enrich_item(&dfid, data, source_entry);
                    }

                    // Return the existing item (potentially with new identifiers)
                    return self.get_item(&dfid)?.ok_or_else(|| ItemsError::ItemNotFound(dfid));
                }
            }
        }

        // Step 2: No duplicate found - generate DFID and create new item
        let dfid = self.dfid_engine.generate_dfid();

        self.logger.info("ItemsEngine", "new_item_creation", "Creating new item - no duplicates found")
            .with_context("new_dfid", dfid.clone())
            .with_context("identifiers_count", identifiers.len().to_string())
            .with_context("source_entry", source_entry.to_string());

        let mut item = self.create_item(dfid, identifiers, source_entry)?;

        // Add enriched data if provided
        if let Some(data) = enriched_data {
            item.enrich(data, source_entry);
            self.storage.store_item(&item)?;
        }

        Ok(item)
    }

    pub fn create_local_item(
        &mut self,
        identifiers: Vec<Identifier>,
        enhanced_identifiers: Vec<crate::identifier_types::EnhancedIdentifier>,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        source_entry: Uuid,
    ) -> Result<Item, ItemsError> {
        // Generate a UUID for the local ID
        let local_id = Uuid::new_v4();

        self.logger.info("ItemsEngine", "local_item_creation", "Creating local item without DFID")
            .with_context("local_id", local_id.to_string())
            .with_context("identifiers_count", identifiers.len().to_string())
            .with_context("enhanced_identifiers_count", enhanced_identifiers.len().to_string())
            .with_context("source_entry", source_entry.to_string());

        // Create item with local_id and temporary DFID format
        let item = Item {
            dfid: format!("LID-{}", local_id), // Temporary DFID format
            local_id: Some(local_id),
            legacy_mode: false,
            identifiers,
            enhanced_identifiers,
            aliases: vec![],
            fingerprint: None,
            enriched_data: enriched_data.unwrap_or_default(),
            creation_timestamp: Utc::now(),
            last_modified: Utc::now(),
            source_entries: vec![source_entry],
            confidence_score: 1.0,
            status: ItemStatus::Active, // Status will indicate "LocalOnly" through dfid format
        };

        self.storage.store_item(&item)?;

        self.logger.info("ItemsEngine", "local_item_created", "Local item created successfully")
            .with_context("local_id", local_id.to_string());

        Ok(item)
    }

    pub fn get_item_by_lid(&self, local_id: &Uuid) -> Result<Option<Item>, ItemsError> {
        // Try to get DFID from mapping first
        if let Ok(Some(dfid)) = self.storage.get_dfid_by_lid(local_id) {
            return self.storage.get_item_by_dfid(&dfid).map_err(ItemsError::from);
        }

        // If no mapping exists, look for item with LID-based temporary DFID
        let temp_dfid = format!("LID-{}", local_id);
        self.storage.get_item_by_dfid(&temp_dfid).map_err(ItemsError::from)
    }

    pub fn get_item(&self, dfid: &str) -> Result<Option<Item>, ItemsError> {
        self.storage.get_item_by_dfid(dfid).map_err(ItemsError::from)
    }

    pub async fn get_item_from_storage_locations(&self, dfid: &str) -> Result<Option<Item>, ItemsError> {
        // First try local storage
        if let Some(item) = self.get_item(dfid)? {
            return Ok(Some(item));
        }

        // If storage history manager is available, try to retrieve from other locations
        if let Some(ref history_manager) = self.storage_history_manager {
            let storage_locations = history_manager.get_all_storage_locations(dfid).await
                .map_err(|e| ItemsError::StorageError(e))?;

            // TODO: Re-enable logging when logger is made thread-safe (use Arc<Mutex<LoggingEngine>>)
            // self.logger.info("ItemsEngine", "multi_storage_retrieval", "Attempting retrieval from multiple storage locations")
            //     .with_context("dfid", dfid.to_string())
            //     .with_context("locations_count", storage_locations.len().to_string());

            // Try each storage location until we find the item
            for location in storage_locations {
                if let Ok(Some(item)) = self.retrieve_item_from_location(dfid, &location).await {
                    // TODO: Re-enable logging when logger is made thread-safe
                    // self.logger.info("ItemsEngine", "item_found_remote", "Item retrieved from remote storage location")
                    //     .with_context("dfid", dfid.to_string())
                    //     .with_context("location_type", format!("{:?}", location));

                    // Optionally cache the item locally for future access
                    // Note: This would require mutable access to storage
                    return Ok(Some(item));
                }
            }
        }

        Ok(None)
    }

    async fn retrieve_item_from_location(&self, dfid: &str, location: &StorageLocation) -> Result<Option<Item>, ItemsError> {
        // This method would use the appropriate adapter to retrieve the item
        // For now, we'll return None since we need to implement adapter integration
        // TODO: Create adapter instances and retrieve data from them

        // TODO: Re-enable logging when logger is made thread-safe
        // self.logger.info("ItemsEngine", "location_retrieval_attempt", "Attempting to retrieve item from storage location")
        //     .with_context("dfid", dfid.to_string())
        //     .with_context("location", format!("{:?}", location));

        // Placeholder - in a full implementation, this would:
        // 1. Create appropriate adapter instance based on location type
        // 2. Use adapter to retrieve item data
        // 3. Deserialize and return the item
        Ok(None)
    }

    pub fn enrich_item(&mut self, dfid: &str, data: HashMap<String, serde_json::Value>, source_entry: Uuid) -> Result<Item, ItemsError> {
        let mut item = self.storage.get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger.info("ItemsEngine", "item_enrichment", "Enriching item")
            .with_context("dfid", dfid.to_string())
            .with_context("source_entry", source_entry.to_string())
            .with_context("data_keys", data.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(","));

        item.enrich(data, source_entry);
        self.storage.update_item(&item)?;

        self.logger.info("ItemsEngine", "item_enriched", "Item enriched successfully")
            .with_context("dfid", dfid.to_string());

        Ok(item)
    }

    pub fn add_identifiers(&mut self, dfid: &str, identifiers: Vec<Identifier>) -> Result<Item, ItemsError> {
        let mut item = self.storage.get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger.info("ItemsEngine", "identifier_addition", "Adding identifiers to item")
            .with_context("dfid", dfid.to_string())
            .with_context("new_identifiers_count", identifiers.len().to_string());

        let original_count = item.identifiers.len();
        item.add_identifiers(identifiers);
        self.storage.update_item(&item)?;

        let new_count = item.identifiers.len();
        let added_count = new_count - original_count;

        self.logger.info("ItemsEngine", "identifiers_added", "Identifiers added successfully")
            .with_context("dfid", dfid.to_string())
            .with_context("added_count", added_count.to_string());

        Ok(item)
    }

    pub fn merge_items(&mut self, primary_dfid: &str, secondary_dfid: &str) -> Result<Item, ItemsError> {
        let mut primary_item = self.storage.get_item_by_dfid(primary_dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(primary_dfid.to_string()))?;

        let secondary_item = self.storage.get_item_by_dfid(secondary_dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(secondary_dfid.to_string()))?;

        self.logger.info("ItemsEngine", "item_merge", "Merging items")
            .with_context("primary_dfid", primary_dfid.to_string())
            .with_context("secondary_dfid", secondary_dfid.to_string());

        // Merge identifiers
        primary_item.add_identifiers(secondary_item.identifiers.clone());

        // Merge enriched data
        primary_item.enriched_data.extend(secondary_item.enriched_data.clone());

        // Merge source entries
        primary_item.source_entries.extend(secondary_item.source_entries.clone());

        // Update confidence score (simple average)
        primary_item.confidence_score = (primary_item.confidence_score + secondary_item.confidence_score) / 2.0;

        // Update the primary item
        self.storage.update_item(&primary_item)?;

        // Mark secondary item as merged and deprecate it
        let mut deprecated_secondary = secondary_item;
        deprecated_secondary.status = ItemStatus::Merged;
        self.storage.update_item(&deprecated_secondary)?;

        self.logger.info("ItemsEngine", "items_merged", "Items merged successfully")
            .with_context("primary_dfid", primary_dfid.to_string())
            .with_context("secondary_dfid", secondary_dfid.to_string());

        Ok(primary_item)
    }

    pub fn split_item(&mut self, dfid: &str, identifiers_for_new_item: Vec<Identifier>, new_dfid: String) -> Result<(Item, Item), ItemsError> {
        let mut original_item = self.storage.get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger.info("ItemsEngine", "item_split", "Splitting item")
            .with_context("original_dfid", dfid.to_string())
            .with_context("new_dfid", new_dfid.clone());

        // Create new item with specified identifiers
        let new_item = Item::new(
            new_dfid.clone(),
            identifiers_for_new_item.clone(),
            original_item.source_entries[0], // Use first source entry
        );

        // Remove the split identifiers from the original item
        original_item.identifiers.retain(|id| !identifiers_for_new_item.contains(id));

        // Mark original item as split
        original_item.status = ItemStatus::Split;

        // Store both items
        self.storage.update_item(&original_item)?;
        self.storage.store_item(&new_item)?;

        self.logger.info("ItemsEngine", "item_split_completed", "Item split completed")
            .with_context("original_dfid", dfid.to_string())
            .with_context("new_dfid", new_dfid);

        Ok((original_item, new_item))
    }

    pub fn split_item_with_generated_dfid(&mut self, dfid: &str, identifiers_for_new_item: Vec<Identifier>) -> Result<(Item, Item), ItemsError> {
        // Generate a unique DFID for the new item
        let new_dfid = self.dfid_engine.generate_dfid();

        // Use the existing split_item method
        self.split_item(dfid, identifiers_for_new_item, new_dfid)
    }

    pub fn deprecate_item(&mut self, dfid: &str) -> Result<Item, ItemsError> {
        let mut item = self.storage.get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger.info("ItemsEngine", "item_deprecation", "Deprecating item")
            .with_context("dfid", dfid.to_string());

        item.status = ItemStatus::Deprecated;
        self.storage.update_item(&item)?;

        self.logger.info("ItemsEngine", "item_deprecated", "Item deprecated successfully")
            .with_context("dfid", dfid.to_string());

        Ok(item)
    }

    pub fn list_items(&self) -> Result<Vec<Item>, ItemsError> {
        self.storage.list_items().map_err(ItemsError::from)
    }

    pub fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, ItemsError> {
        self.storage.find_items_by_identifier(identifier).map_err(ItemsError::from)
    }

    pub fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, ItemsError> {
        self.storage.find_items_by_status(status).map_err(ItemsError::from)
    }

    pub fn get_item_statistics(&self) -> Result<ItemStatistics, ItemsError> {
        let all_items = self.storage.list_items()?;

        let mut stats = ItemStatistics {
            total_items: all_items.len(),
            active_items: 0,
            deprecated_items: 0,
            merged_items: 0,
            split_items: 0,
            total_identifiers: 0,
            average_confidence: 0.0,
        };

        let mut total_confidence = 0.0;

        for item in &all_items {
            match item.status {
                ItemStatus::Active => stats.active_items += 1,
                ItemStatus::Deprecated => stats.deprecated_items += 1,
                ItemStatus::Merged => stats.merged_items += 1,
                ItemStatus::Split => stats.split_items += 1,
            }

            stats.total_identifiers += item.identifiers.len();
            total_confidence += item.confidence_score;
        }

        if !all_items.is_empty() {
            stats.average_confidence = total_confidence / all_items.len() as f64;
        }

        Ok(stats)
    }

    pub fn get_logs(&self) -> &[LogEntry] {
        self.logger.get_logs()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<&LogEntry> {
        self.logger.get_logs_by_event_type(event_type)
    }

    // Item Sharing methods
    pub fn share_item(&mut self, dfid: &str, shared_by: String, recipient_user_id: String, permissions: Option<Vec<String>>) -> Result<ItemShare, ItemsError> {
        // Verify the item exists
        let _item = self.get_item(dfid)?.ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        // Create the share
        let share = ItemShare::new(dfid.to_string(), shared_by, recipient_user_id, permissions);

        // Store the share
        self.storage.store_item_share(&share)?;

        self.logger.info("ItemsEngine", "item_shared", "Item shared with user")
            .with_context("dfid", dfid.to_string())
            .with_context("share_id", share.share_id.clone())
            .with_context("shared_by", share.shared_by.clone())
            .with_context("recipient", share.recipient_user_id.clone());

        Ok(share)
    }

    pub fn get_shares_for_user(&self, user_id: &str) -> Result<Vec<SharedItemResponse>, ItemsError> {
        let shares = self.storage.get_shares_for_user(user_id)?;
        let mut shared_items = Vec::new();

        for share in shares {
            if let Some(item) = self.get_item(&share.dfid)? {
                shared_items.push(SharedItemResponse {
                    share_id: share.share_id,
                    item,
                    shared_by: share.shared_by,
                    shared_at: share.shared_at,
                    permissions: share.permissions,
                });
            }
        }

        Ok(shared_items)
    }

    pub fn get_shares_for_item(&self, dfid: &str) -> Result<Vec<ItemShare>, ItemsError> {
        self.storage.get_shares_for_item(dfid).map_err(ItemsError::from)
    }

    pub fn is_item_shared_with_user(&self, dfid: &str, user_id: &str) -> Result<bool, ItemsError> {
        self.storage.is_item_shared_with_user(dfid, user_id).map_err(ItemsError::from)
    }

    pub fn revoke_share(&mut self, share_id: &str) -> Result<(), ItemsError> {
        // Get share info for logging before deletion
        if let Ok(Some(share)) = self.storage.get_item_share(share_id) {
            self.logger.info("ItemsEngine", "share_revoked", "Item share revoked")
                .with_context("share_id", share_id.to_string())
                .with_context("dfid", share.dfid)
                .with_context("recipient", share.recipient_user_id);
        }

        self.storage.delete_item_share(share_id).map_err(ItemsError::from)
    }

    // Conflict Detection
    fn detect_conflicts(
        &self,
        identifiers: &[Identifier],
        enriched_data: &Option<HashMap<String, serde_json::Value>>,
        source_entry: Uuid,
    ) -> Result<Option<PendingReason>, ItemsError> {
        // Check for empty identifiers
        if identifiers.is_empty() {
            return Ok(Some(PendingReason::NoIdentifiers));
        }

        // Check for invalid identifiers (basic validation)
        for identifier in identifiers {
            if identifier.key.trim().is_empty() || identifier.value.trim().is_empty() {
                return Ok(Some(PendingReason::InvalidIdentifiers(
                    format!("Invalid identifier: {}:{}", identifier.key, identifier.value)
                )));
            }
        }

        // Check for conflicting DFIDs (when same identifier maps to multiple different DFIDs)
        for identifier in identifiers {
            if let Ok(existing_items) = self.find_items_by_identifier(identifier) {
                if existing_items.len() > 1 {
                    let mut conflicting_dfids: Vec<String> = existing_items.iter()
                        .map(|item| item.dfid.clone())
                        .collect();
                    conflicting_dfids.sort();
                    conflicting_dfids.dedup();

                    if conflicting_dfids.len() > 1 {
                        return Ok(Some(PendingReason::ConflictingDFIDs {
                            identifier: identifier.clone(),
                            conflicting_dfids,
                            confidence_scores: None,
                        }));
                    }
                }
            }
        }

        // Check for data quality issues
        if let Some(data) = enriched_data {
            for (key, value) in data {
                // Basic data quality checks
                if key.trim().is_empty() {
                    return Ok(Some(PendingReason::DataQualityIssue {
                        issue_type: "empty_key".to_string(),
                        severity: crate::types::QualitySeverity::Medium,
                        details: "Empty data key detected".to_string(),
                    }));
                }

                if value.is_null() {
                    return Ok(Some(PendingReason::DataQualityIssue {
                        issue_type: "null_value".to_string(),
                        severity: crate::types::QualitySeverity::Low,
                        details: format!("Null value for key: {}", key),
                    }));
                }
            }
        }

        // No conflicts detected
        Ok(None)
    }

    // Pending items management
    pub fn get_pending_items(&self) -> Result<Vec<PendingItem>, ItemsError> {
        self.storage.list_pending_items().map_err(ItemsError::from)
    }

    pub fn get_pending_item(&self, pending_id: &Uuid) -> Result<Option<PendingItem>, ItemsError> {
        self.storage.get_pending_item(pending_id).map_err(ItemsError::from)
    }

    pub fn resolve_pending_item(&mut self, pending_id: &Uuid, resolution_action: ResolutionAction) -> Result<Option<Item>, ItemsError> {
        let pending_item = self.storage.get_pending_item(pending_id)?
            .ok_or_else(|| ItemsError::ItemNotFound(format!("Pending item not found: {}", pending_id)))?;

        match resolution_action {
            ResolutionAction::Approve => {
                // Try to create the item with the pending data
                let result = self.create_item_with_generated_dfid(
                    pending_item.identifiers.clone(),
                    pending_item.source_entry,
                    pending_item.enriched_data.clone(),
                );

                match result {
                    Ok(item) => {
                        // Successfully created, remove from pending
                        self.storage.delete_pending_item(pending_id)?;
                        Ok(Some(item))
                    },
                    Err(_) => {
                        // Still has conflicts, update priority and keep pending
                        let mut updated_pending = pending_item;
                        updated_pending.priority += 1; // Increase priority
                        self.storage.update_pending_item(&updated_pending)?;
                        Ok(None)
                    }
                }
            },
            ResolutionAction::Reject => {
                // Simply remove from pending
                self.storage.delete_pending_item(pending_id)?;
                Ok(None)
            },
            ResolutionAction::Modify(new_identifiers, new_data) => {
                // Update the pending item with new data
                let mut updated_pending = pending_item;
                updated_pending.identifiers = new_identifiers;
                updated_pending.enriched_data = new_data;
                self.storage.update_pending_item(&updated_pending)?;
                Ok(None)
            },
        }
    }

    pub fn get_pending_items_by_reason(&self, reason: &str) -> Result<Vec<PendingItem>, ItemsError> {
        let all_pending = self.get_pending_items()?;
        let filtered: Vec<PendingItem> = all_pending.into_iter()
            .filter(|item| format!("{:?}", item.reason).contains(reason))
            .collect();
        Ok(filtered)
    }

}

#[derive(Debug, Clone)]
pub enum ResolutionAction {
    Approve,
    Reject,
    Modify(Vec<Identifier>, Option<HashMap<String, serde_json::Value>>),
}

#[derive(Debug, Clone)]
pub struct ItemStatistics {
    pub total_items: usize,
    pub active_items: usize,
    pub deprecated_items: usize,
    pub merged_items: usize,
    pub split_items: usize,
    pub total_identifiers: usize,
    pub average_confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ItemStatus;
    use std::collections::HashMap;

    // Mock storage implementation for testing
    struct MockItemsStorage {
        items: HashMap<String, Item>,
    }

    impl MockItemsStorage {
        fn new() -> Self {
            Self {
                items: HashMap::new(),
            }
        }
    }

    impl StorageBackend for MockItemsStorage {
        fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
            self.items.insert(item.dfid.clone(), item.clone());
            Ok(())
        }

        fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
            Ok(self.items.get(dfid).cloned())
        }

        fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
            self.store_item(item)
        }

        fn list_items(&self) -> Result<Vec<Item>, StorageError> {
            Ok(self.items.values().cloned().collect())
        }

        fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
            Ok(self.items
                .values()
                .filter(|item| item.identifiers.contains(identifier))
                .cloned()
                .collect())
        }

        fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError> {
            Ok(self.items
                .values()
                .filter(|item| std::mem::discriminant(&item.status) == std::mem::discriminant(&status))
                .cloned()
                .collect())
        }

        fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError> {
            self.items.remove(dfid);
            Ok(())
        }

        // Item Share operations - simplified implementations for testing
        fn store_item_share(&mut self, _share: &ItemShare) -> Result<(), StorageError> {
            Ok(())
        }

        fn get_item_share(&self, _share_id: &str) -> Result<Option<ItemShare>, StorageError> {
            Ok(None)
        }

        fn get_shares_for_user(&self, _user_id: &str) -> Result<Vec<ItemShare>, StorageError> {
            Ok(Vec::new())
        }

        fn get_shares_for_item(&self, _dfid: &str) -> Result<Vec<ItemShare>, StorageError> {
            Ok(Vec::new())
        }

        fn is_item_shared_with_user(&self, _dfid: &str, _user_id: &str) -> Result<bool, StorageError> {
            Ok(false)
        }

        fn delete_item_share(&mut self, _share_id: &str) -> Result<(), StorageError> {
            Ok(())
        }

        // Implement all remaining StorageBackend methods with minimal functionality for testing
        fn store_circuit(&mut self, _circuit: &crate::types::Circuit) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_circuit(&self, _circuit_id: &uuid::Uuid) -> Result<Option<crate::types::Circuit>, StorageError> {
            Ok(None)
        }
        fn update_circuit(&mut self, _circuit: &crate::types::Circuit) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_circuits(&self) -> Result<Vec<crate::types::Circuit>, StorageError> {
            Ok(Vec::new())
        }
        fn get_circuits_for_member(&self, _member_id: &str) -> Result<Vec<crate::types::Circuit>, StorageError> {
            Ok(Vec::new())
        }
        fn store_circuit_item(&mut self, _circuit_item: &crate::types::CircuitItem) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_circuit_items(&self, _circuit_id: &uuid::Uuid) -> Result<Vec<crate::types::CircuitItem>, StorageError> {
            Ok(Vec::new())
        }
        fn remove_circuit_item(&mut self, _circuit_id: &uuid::Uuid, _dfid: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn store_activity(&mut self, _activity: &crate::types::Activity) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_all_activities(&self) -> Result<Vec<crate::types::Activity>, StorageError> {
            Ok(Vec::new())
        }
        fn get_activities_for_user(&self, _user_id: &str) -> Result<Vec<crate::types::Activity>, StorageError> {
            Ok(Vec::new())
        }
        fn get_activities_for_circuit(&self, _circuit_id: &uuid::Uuid) -> Result<Vec<crate::types::Activity>, StorageError> {
            Ok(Vec::new())
        }
        fn store_data_lake_entry(&mut self, _entry: &crate::types::DataLakeEntry) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_data_lake_entry(&self, _entry_id: &uuid::Uuid) -> Result<Option<crate::types::DataLakeEntry>, StorageError> {
            Ok(None)
        }
        fn list_data_lake_entries(&self) -> Result<Vec<crate::types::DataLakeEntry>, StorageError> {
            Ok(Vec::new())
        }
        fn get_data_lake_entries_by_status(&self, _status: crate::types::ProcessingStatus) -> Result<Vec<crate::types::DataLakeEntry>, StorageError> {
            Ok(Vec::new())
        }
        fn update_data_lake_entry(&mut self, _entry: &crate::types::DataLakeEntry) -> Result<(), StorageError> {
            Ok(())
        }
        fn store_identifier_mapping(&mut self, _mapping: &crate::types::IdentifierMapping) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_identifier_mappings(&self, _identifier: &Identifier) -> Result<Vec<crate::types::IdentifierMapping>, StorageError> {
            Ok(Vec::new())
        }
        fn list_identifier_mappings(&self) -> Result<Vec<crate::types::IdentifierMapping>, StorageError> {
            Ok(Vec::new())
        }
        fn update_identifier_mapping(&mut self, _mapping: &crate::types::IdentifierMapping) -> Result<(), StorageError> {
            Ok(())
        }
        fn store_conflict_resolution(&mut self, _conflict: &crate::types::ConflictResolution) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_conflict_resolution(&self, _conflict_id: &uuid::Uuid) -> Result<Option<crate::types::ConflictResolution>, StorageError> {
            Ok(None)
        }
        fn get_pending_conflicts(&self) -> Result<Vec<crate::types::ConflictResolution>, StorageError> {
            Ok(Vec::new())
        }
        fn store_receipt(&mut self, _receipt: &crate::types::Receipt) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_receipt(&self, _id: &uuid::Uuid) -> Result<Option<crate::types::Receipt>, StorageError> {
            Ok(None)
        }
        fn find_receipts_by_identifier(&self, _identifier: &Identifier) -> Result<Vec<crate::types::Receipt>, StorageError> {
            Ok(Vec::new())
        }
        fn list_receipts(&self) -> Result<Vec<crate::types::Receipt>, StorageError> {
            Ok(Vec::new())
        }
        fn store_log(&mut self, _log: &crate::logging::LogEntry) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_logs(&self) -> Result<Vec<crate::logging::LogEntry>, StorageError> {
            Ok(Vec::new())
        }
        fn store_event(&mut self, _event: &crate::types::Event) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_event(&self, _event_id: &uuid::Uuid) -> Result<Option<crate::types::Event>, StorageError> {
            Ok(None)
        }
        fn update_event(&mut self, _event: &crate::types::Event) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_events(&self) -> Result<Vec<crate::types::Event>, StorageError> {
            Ok(Vec::new())
        }
        fn get_events_by_dfid(&self, _dfid: &str) -> Result<Vec<crate::types::Event>, StorageError> {
            Ok(Vec::new())
        }
        fn get_events_by_type(&self, _event_type: crate::types::EventType) -> Result<Vec<crate::types::Event>, StorageError> {
            Ok(Vec::new())
        }
        fn get_events_by_visibility(&self, _visibility: crate::types::EventVisibility) -> Result<Vec<crate::types::Event>, StorageError> {
            Ok(Vec::new())
        }
        fn get_events_in_time_range(&self, _start: chrono::DateTime<chrono::Utc>, _end: chrono::DateTime<chrono::Utc>) -> Result<Vec<crate::types::Event>, StorageError> {
            Ok(Vec::new())
        }
        fn store_circuit_operation(&mut self, _operation: &crate::types::CircuitOperation) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_circuit_operation(&self, _operation_id: &uuid::Uuid) -> Result<Option<crate::types::CircuitOperation>, StorageError> {
            Ok(None)
        }
        fn update_circuit_operation(&mut self, _operation: &crate::types::CircuitOperation) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_circuit_operations(&self, _circuit_id: &uuid::Uuid) -> Result<Vec<crate::types::CircuitOperation>, StorageError> {
            Ok(Vec::new())
        }
        fn store_audit_event(&mut self, _event: &crate::types::AuditEvent) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_audit_event(&self, _event_id: &uuid::Uuid) -> Result<Option<crate::types::AuditEvent>, StorageError> {
            Ok(None)
        }
        fn query_audit_events(&self, _query: &crate::types::AuditQuery) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn list_audit_events(&self) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn get_audit_events_by_user(&self, _user_id: &str) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn get_audit_events_by_type(&self, _event_type: crate::types::AuditEventType) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn get_audit_events_by_severity(&self, _severity: crate::types::AuditSeverity) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn get_audit_events_in_time_range(&self, _start: chrono::DateTime<chrono::Utc>, _end: chrono::DateTime<chrono::Utc>) -> Result<Vec<crate::types::AuditEvent>, StorageError> {
            Ok(Vec::new())
        }
        fn sync_audit_events(&mut self, _events: Vec<crate::types::AuditEvent>) -> Result<(), StorageError> {
            Ok(())
        }
        fn store_security_incident(&mut self, _incident: &crate::types::SecurityIncident) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_security_incident(&self, _incident_id: &uuid::Uuid) -> Result<Option<crate::types::SecurityIncident>, StorageError> {
            Ok(None)
        }
        fn update_security_incident(&mut self, _incident: &crate::types::SecurityIncident) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_security_incidents(&self) -> Result<Vec<crate::types::SecurityIncident>, StorageError> {
            Ok(Vec::new())
        }
        fn get_incidents_by_severity(&self, _severity: crate::types::AuditSeverity) -> Result<Vec<crate::types::SecurityIncident>, StorageError> {
            Ok(Vec::new())
        }
        fn get_open_incidents(&self) -> Result<Vec<crate::types::SecurityIncident>, StorageError> {
            Ok(Vec::new())
        }
        fn get_incidents_by_assignee(&self, _assignee: &str) -> Result<Vec<crate::types::SecurityIncident>, StorageError> {
            Ok(Vec::new())
        }
        fn store_compliance_report(&mut self, _report: &crate::types::ComplianceReport) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_compliance_report(&self, _report_id: &uuid::Uuid) -> Result<Option<crate::types::ComplianceReport>, StorageError> {
            Ok(None)
        }
        fn list_compliance_reports(&self) -> Result<Vec<crate::types::ComplianceReport>, StorageError> {
            Ok(Vec::new())
        }
        fn get_reports_by_type(&self, _report_type: &str) -> Result<Vec<crate::types::ComplianceReport>, StorageError> {
            Ok(Vec::new())
        }
        fn get_event_count_by_time_range(&self, _start: chrono::DateTime<chrono::Utc>, _end: chrono::DateTime<chrono::Utc>) -> Result<u64, StorageError> {
            Ok(0)
        }
        fn store_enhanced_identifier_mapping(&mut self, _identifier: &crate::identifier_types::EnhancedIdentifier, _dfid: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_dfid_by_canonical(&self, _namespace: &str, _registry: &str, _value: &str) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
        fn store_lid_dfid_mapping(&mut self, _lid: &uuid::Uuid, _dfid: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_dfid_by_lid(&self, _lid: &uuid::Uuid) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
        fn store_fingerprint_mapping(&mut self, _fingerprint: &str, _dfid: &str, _circuit_id: &uuid::Uuid) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_dfid_by_fingerprint(&self, _fingerprint: &str, _circuit_id: &uuid::Uuid) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
    }

    #[test]
    fn test_create_item() {
        let storage = MockItemsStorage::new();
        let mut engine = ItemsEngine::new(storage);

        let dfid = "DFID-20240926-000001-TEST".to_string();
        let identifiers = vec![Identifier::new("user_id", "12345")];
        let source_entry = Uuid::new_v4();

        let item = engine.create_item(dfid.clone(), identifiers.clone(), source_entry).unwrap();

        assert_eq!(item.dfid, dfid);
        assert_eq!(item.identifiers, identifiers);
        assert_eq!(item.source_entries, vec![source_entry]);
        assert!(matches!(item.status, ItemStatus::Active));
    }

    #[test]
    fn test_enrich_item() {
        let storage = MockItemsStorage::new();
        let mut engine = ItemsEngine::new(storage);

        let dfid = "DFID-20240926-000001-TEST".to_string();
        let identifiers = vec![Identifier::new("user_id", "12345")];
        let source_entry = Uuid::new_v4();

        engine.create_item(dfid.clone(), identifiers, source_entry).unwrap();

        let mut enrichment_data = HashMap::new();
        enrichment_data.insert("name".to_string(), serde_json::Value::String("John Doe".to_string()));
        enrichment_data.insert("age".to_string(), serde_json::Value::Number(30.into()));

        let new_source = Uuid::new_v4();
        let enriched_item = engine.enrich_item(&dfid, enrichment_data.clone(), new_source).unwrap();

        assert_eq!(enriched_item.enriched_data.len(), 2);
        assert_eq!(enriched_item.source_entries.len(), 2);
        assert!(enriched_item.source_entries.contains(&new_source));
    }

    #[test]
    fn test_merge_items() {
        let storage = MockItemsStorage::new();
        let mut engine = ItemsEngine::new(storage);

        let dfid1 = "DFID-20240926-000001-TEST".to_string();
        let dfid2 = "DFID-20240926-000002-TEST".to_string();

        engine.create_item(dfid1.clone(), vec![Identifier::new("user_id", "12345")], Uuid::new_v4()).unwrap();
        engine.create_item(dfid2.clone(), vec![Identifier::new("email", "test@example.com")], Uuid::new_v4()).unwrap();

        let merged_item = engine.merge_items(&dfid1, &dfid2).unwrap();

        assert_eq!(merged_item.dfid, dfid1);
        assert_eq!(merged_item.identifiers.len(), 2);

        // Check that secondary item is marked as merged
        let secondary_item = engine.get_item(&dfid2).unwrap().unwrap();
        assert!(matches!(secondary_item.status, ItemStatus::Merged));
    }

    #[test]
    fn test_item_statistics() {
        let storage = MockItemsStorage::new();
        let mut engine = ItemsEngine::new(storage);

        engine.create_item("DFID-1".to_string(), vec![Identifier::new("id", "1")], Uuid::new_v4()).unwrap();
        engine.create_item("DFID-2".to_string(), vec![Identifier::new("id", "2")], Uuid::new_v4()).unwrap();
        engine.deprecate_item("DFID-2").unwrap();

        let stats = engine.get_item_statistics().unwrap();

        assert_eq!(stats.total_items, 2);
        assert_eq!(stats.active_items, 1);
        assert_eq!(stats.deprecated_items, 1);
        assert_eq!(stats.total_identifiers, 2);
        assert_eq!(stats.average_confidence, 1.0);
    }
}