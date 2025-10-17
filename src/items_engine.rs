use crate::dfid_engine::DfidEngine;
use crate::logging::{LogEntry, LoggingEngine};
use crate::storage::{StorageBackend, StorageError};
use crate::types::{
    Identifier, Item, ItemShare, ItemStatus, MergeStrategy, PendingItem, PendingReason,
    SharedItemResponse,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

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
            ItemsError::StorageError(e) => write!(f, "Storage error: {e}"),
            ItemsError::ItemNotFound(dfid) => write!(f, "Item not found: {dfid}"),
            ItemsError::InvalidOperation(msg) => write!(f, "Invalid operation: {msg}"),
            ItemsError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
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
}

impl<S: StorageBackend + 'static> ItemsEngine<S> {
    pub fn new(storage: S) -> Self {
        let mut logger = LoggingEngine::new();
        logger.info("ItemsEngine", "initialization", "Items engine initialized");

        Self {
            storage,
            logger,
            dfid_engine: DfidEngine::new(),
        }
    }

    pub fn create_item(
        &mut self,
        dfid: String,
        identifiers: Vec<Identifier>,
        source_entry: Uuid,
    ) -> Result<Item, ItemsError> {
        // Check if item already exists
        if self.storage.get_item_by_dfid(&dfid)?.is_some() {
            return Err(ItemsError::InvalidOperation(format!(
                "Item with DFID {dfid} already exists"
            )));
        }

        let item = Item::new(dfid.clone(), identifiers, source_entry);

        self.logger
            .info("ItemsEngine", "item_creation", "Creating new item")
            .with_context("dfid", dfid.clone())
            .with_context("source_entry", source_entry.to_string());

        self.storage.store_item(&item)?;

        self.logger
            .info("ItemsEngine", "item_created", "Item created successfully")
            .with_context("dfid", dfid.clone());

        Ok(item)
    }

    pub fn create_item_with_generated_dfid(
        &mut self,
        identifiers: Vec<Identifier>,
        source_entry: Uuid,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Item, ItemsError> {
        // Step 0: Check for conflicts and handle them
        if let Some(pending_reason) =
            self.detect_conflicts(&identifiers, &enriched_data, source_entry)?
        {
            // Store as pending item
            let pending_item = PendingItem::new(
                identifiers,
                enriched_data,
                source_entry,
                pending_reason,
                None,
                None,
            );
            self.storage.store_pending_item(&pending_item)?;

            self.logger
                .info(
                    "ItemsEngine",
                    "pending_item_created",
                    "Item stored as pending due to conflicts",
                )
                .with_context("pending_id", pending_item.pending_id.to_string())
                .with_context("reason", format!("{:?}", pending_item.reason))
                .with_context("source_entry", source_entry.to_string());

            // For now, we'll return an error to maintain API compatibility
            // Later we can modify the API to return a "pending" result
            return Err(ItemsError::ValidationError(format!(
                "Item stored as pending: {:?}",
                pending_item.reason
            )));
        }

        // Step 1: Check if any identifier matches existing items (entity resolution)
        for identifier in &identifiers {
            if let Ok(existing_items) = self.find_items_by_identifier(identifier) {
                if let Some(existing_item) = existing_items.first() {
                    let dfid = existing_item.dfid.clone();

                    self.logger
                        .info(
                            "ItemsEngine",
                            "duplicate_detected",
                            "Found existing item with matching identifier",
                        )
                        .with_context("existing_dfid", dfid.clone())
                        .with_context(
                            "matching_identifier",
                            format!("{}:{}", identifier.key, identifier.value),
                        )
                        .with_context("source_entry", source_entry.to_string());

                    // Add any new identifiers to existing item
                    let new_identifiers: Vec<Identifier> = identifiers
                        .into_iter()
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
                    return self.get_item(&dfid)?.ok_or(ItemsError::ItemNotFound(dfid));
                }
            }
        }

        // Step 2: No duplicate found - generate DFID and create new item
        let dfid = self.dfid_engine.generate_dfid();

        self.logger
            .info(
                "ItemsEngine",
                "new_item_creation",
                "Creating new item - no duplicates found",
            )
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

        self.logger
            .info(
                "ItemsEngine",
                "local_item_creation",
                "Creating local item without DFID",
            )
            .with_context("local_id", local_id.to_string())
            .with_context("identifiers_count", identifiers.len().to_string())
            .with_context(
                "enhanced_identifiers_count",
                enhanced_identifiers.len().to_string(),
            )
            .with_context("source_entry", source_entry.to_string());

        // Create item with local_id and temporary DFID format
        let item = Item {
            dfid: format!("LID-{local_id}"), // Temporary DFID format
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

        self.logger
            .info(
                "ItemsEngine",
                "local_item_created",
                "Local item created successfully",
            )
            .with_context("local_id", local_id.to_string());

        Ok(item)
    }

    pub fn get_item_by_lid(&self, local_id: &Uuid) -> Result<Option<Item>, ItemsError> {
        // Try to get DFID from mapping first
        if let Ok(Some(dfid)) = self.storage.get_dfid_by_lid(local_id) {
            return self
                .storage
                .get_item_by_dfid(&dfid)
                .map_err(ItemsError::from);
        }

        // If no mapping exists, look for item with LID-based temporary DFID
        let temp_dfid = format!("LID-{local_id}");
        self.storage
            .get_item_by_dfid(&temp_dfid)
            .map_err(ItemsError::from)
    }

    /// Merge enriched_data from multiple items based on strategy
    fn merge_enriched_data(
        master: &HashMap<String, serde_json::Value>,
        others: Vec<&HashMap<String, serde_json::Value>>,
        strategy: &MergeStrategy,
    ) -> HashMap<String, serde_json::Value> {
        use serde_json::Value;

        let mut result = master.clone();

        match strategy {
            MergeStrategy::Append => {
                // Merge all data intelligently
                for other in others {
                    for (key, value) in other {
                        match (result.get(key), value) {
                            // Both are arrays: append unique values
                            (Some(Value::Array(existing)), Value::Array(new)) => {
                                let mut merged = existing.clone();
                                for item in new {
                                    if !merged.contains(item) {
                                        merged.push(item.clone());
                                    }
                                }
                                result.insert(key.clone(), Value::Array(merged));
                            }
                            // Both are objects: deep merge
                            (Some(Value::Object(existing)), Value::Object(new)) => {
                                let mut merged = existing.clone();
                                for (k, v) in new {
                                    merged.insert(k.clone(), v.clone());
                                }
                                result.insert(key.clone(), Value::Object(merged));
                            }
                            // For scalars: keep the new value
                            _ => {
                                result.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }
            }
            MergeStrategy::KeepFirst => {
                // Keep master data, ignore others
                // result is already master.clone(), nothing to do
            }
            MergeStrategy::Overwrite => {
                // Last item wins for all fields
                for other in others {
                    for (key, value) in other {
                        result.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        result
    }

    /// Merge multiple local items into a master item
    pub fn merge_local_items(
        &mut self,
        master_lid: &Uuid,
        merge_lids: Vec<Uuid>,
        strategy: MergeStrategy,
    ) -> Result<Item, ItemsError> {
        self.logger
            .info(
                "ItemsEngine",
                "merge_local_items_start",
                "Starting local items merge",
            )
            .with_context("master_lid", master_lid.to_string())
            .with_context("merge_count", merge_lids.len().to_string())
            .with_context("strategy", format!("{strategy:?}"));

        // Validate master item exists and is local-only
        let mut master_item = self.get_item_by_lid(master_lid)?.ok_or_else(|| {
            ItemsError::ItemNotFound(format!("Master LID not found: {master_lid}"))
        })?;

        // Check if master is already pushed to circuit
        if !master_item.dfid.starts_with("LID-") {
            return Err(ItemsError::InvalidOperation(
                "Cannot merge: master item has already been pushed to a circuit".to_string(),
            ));
        }

        // Validate all merge items exist and are local-only
        let mut merge_items = Vec::new();
        for lid in &merge_lids {
            let item = self
                .get_item_by_lid(lid)?
                .ok_or_else(|| ItemsError::ItemNotFound(format!("Merge LID not found: {lid}")))?;

            // Check if item is already pushed to circuit
            if !item.dfid.starts_with("LID-") {
                return Err(ItemsError::InvalidOperation(format!(
                    "Cannot merge: item {lid} has already been pushed to a circuit"
                )));
            }

            // Check if item is already merged
            if matches!(item.status, ItemStatus::MergedInto(_)) {
                return Err(ItemsError::InvalidOperation(format!(
                    "Cannot merge: item {lid} has already been merged into another item"
                )));
            }

            merge_items.push(item);
        }

        // Merge enriched_data
        let merge_data_refs: Vec<&HashMap<String, serde_json::Value>> =
            merge_items.iter().map(|item| &item.enriched_data).collect();
        let merged_data =
            Self::merge_enriched_data(&master_item.enriched_data, merge_data_refs, &strategy);

        // Update master item
        master_item.enriched_data = merged_data;
        master_item.last_modified = Utc::now();

        // Collect all source entries
        for item in &merge_items {
            for source in &item.source_entries {
                if !master_item.source_entries.contains(source) {
                    master_item.source_entries.push(*source);
                }
            }
        }

        // Store updated master item
        self.storage.store_item(&master_item)?;

        // Mark merge items as MergedInto master
        for (lid, mut item) in merge_lids.iter().zip(merge_items.into_iter()) {
            item.status = ItemStatus::MergedInto(master_lid.to_string());
            item.last_modified = Utc::now();
            self.storage.store_item(&item)?;

            self.logger
                .info("ItemsEngine", "item_merged", "Item marked as merged")
                .with_context("merged_lid", lid.to_string())
                .with_context("into_lid", master_lid.to_string());
        }

        self.logger
            .info(
                "ItemsEngine",
                "merge_complete",
                "Local items merge completed successfully",
            )
            .with_context("master_lid", master_lid.to_string())
            .with_context("merged_count", merge_lids.len().to_string());

        Ok(master_item)
    }

    /// Find duplicate local items by identifier
    pub fn find_duplicate_local_items(
        &mut self,
    ) -> Result<Vec<(String, String, Vec<Item>)>, ItemsError> {
        use std::collections::HashMap as StdHashMap;

        self.logger.info(
            "ItemsEngine",
            "find_duplicates_start",
            "Finding duplicate local items",
        );

        // Get all items
        let all_items = self.storage.list_items().map_err(ItemsError::from)?;

        // Filter local-only items (not yet pushed)
        let local_items: Vec<Item> = all_items
            .into_iter()
            .filter(|item| item.dfid.starts_with("LID-"))
            .filter(|item| !matches!(item.status, ItemStatus::MergedInto(_)))
            .collect();

        // Group by enhanced identifiers (canonical ones)
        let mut groups: StdHashMap<(String, String), Vec<Item>> = StdHashMap::new();

        for item in local_items {
            // Look for canonical identifiers
            for enh_id in &item.enhanced_identifiers {
                if matches!(
                    enh_id.id_type,
                    crate::identifier_types::IdentifierType::Canonical { .. }
                ) {
                    let key_str = format!("{}:{}", enh_id.namespace, enh_id.key);
                    let value_str = enh_id.value.clone();

                    groups
                        .entry((key_str, value_str))
                        .or_default()
                        .push(item.clone());
                    break; // Only use first canonical identifier
                }
            }
        }

        // Filter groups with more than one item
        let duplicates: Vec<(String, String, Vec<Item>)> = groups
            .into_iter()
            .filter(|(_, items)| items.len() > 1)
            .map(|((key, value), items)| (key, value, items))
            .collect();

        self.logger
            .info(
                "ItemsEngine",
                "find_duplicates_complete",
                "Duplicate detection complete",
            )
            .with_context("duplicate_groups", duplicates.len().to_string());

        Ok(duplicates)
    }

    /// Undo a merge operation by restoring a merged item to Active status
    pub fn unmerge_local_item(&mut self, merged_lid: &Uuid) -> Result<Item, ItemsError> {
        self.logger
            .info("ItemsEngine", "unmerge_start", "Starting unmerge operation")
            .with_context("merged_lid", merged_lid.to_string());

        // Get the merged item
        let mut merged_item = self.get_item_by_lid(merged_lid)?.ok_or_else(|| {
            ItemsError::ItemNotFound(format!("Merged LID not found: {merged_lid}"))
        })?;

        // Check if item is actually merged
        let master_lid = match &merged_item.status {
            ItemStatus::MergedInto(lid) => lid.clone(),
            _ => {
                return Err(ItemsError::InvalidOperation(format!(
                    "Item {merged_lid} is not in MergedInto status, cannot unmerge"
                )))
            }
        };

        // Check if item is still local-only
        if !merged_item.dfid.starts_with("LID-") {
            return Err(ItemsError::InvalidOperation(
                "Cannot unmerge: item has been pushed to a circuit".to_string(),
            ));
        }

        // Restore item to Active status
        merged_item.status = ItemStatus::Active;
        merged_item.last_modified = Utc::now();

        // Store updated item
        self.storage.store_item(&merged_item)?;

        self.logger
            .info(
                "ItemsEngine",
                "unmerge_complete",
                "Unmerge operation completed",
            )
            .with_context("merged_lid", merged_lid.to_string())
            .with_context("previous_master", master_lid);

        Ok(merged_item)
    }

    pub fn get_item(&self, dfid: &str) -> Result<Option<Item>, ItemsError> {
        self.storage
            .get_item_by_dfid(dfid)
            .map_err(ItemsError::from)
    }

    pub async fn get_item_from_storage_locations(
        &self,
        dfid: &str,
    ) -> Result<Option<Item>, ItemsError> {
        // First try local storage
        if let Some(item) = self.get_item(dfid)? {
            return Ok(Some(item));
        }

        // Multi-storage retrieval is not yet implemented
        // This would require integrating with StorageHistoryReader and implementing
        // cross-adapter retrieval logic to fetch items from IPFS, Stellar, etc.
        // For now, items are only retrieved from local storage

        Ok(None)
    }

    pub fn enrich_item(
        &mut self,
        dfid: &str,
        data: HashMap<String, serde_json::Value>,
        source_entry: Uuid,
    ) -> Result<Item, ItemsError> {
        let mut item = self
            .storage
            .get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger
            .info("ItemsEngine", "item_enrichment", "Enriching item")
            .with_context("dfid", dfid.to_string())
            .with_context("source_entry", source_entry.to_string())
            .with_context(
                "data_keys",
                data.keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );

        item.enrich(data, source_entry);
        self.storage.update_item(&item)?;

        self.logger
            .info("ItemsEngine", "item_enriched", "Item enriched successfully")
            .with_context("dfid", dfid.to_string());

        Ok(item)
    }

    pub fn add_identifiers(
        &mut self,
        dfid: &str,
        identifiers: Vec<Identifier>,
    ) -> Result<Item, ItemsError> {
        let mut item = self
            .storage
            .get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger
            .info(
                "ItemsEngine",
                "identifier_addition",
                "Adding identifiers to item",
            )
            .with_context("dfid", dfid.to_string())
            .with_context("new_identifiers_count", identifiers.len().to_string());

        let original_count = item.identifiers.len();
        item.add_identifiers(identifiers);
        self.storage.update_item(&item)?;

        let new_count = item.identifiers.len();
        let added_count = new_count - original_count;

        self.logger
            .info(
                "ItemsEngine",
                "identifiers_added",
                "Identifiers added successfully",
            )
            .with_context("dfid", dfid.to_string())
            .with_context("added_count", added_count.to_string());

        Ok(item)
    }

    pub fn merge_items(
        &mut self,
        primary_dfid: &str,
        secondary_dfid: &str,
    ) -> Result<Item, ItemsError> {
        let mut primary_item = self
            .storage
            .get_item_by_dfid(primary_dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(primary_dfid.to_string()))?;

        let secondary_item = self
            .storage
            .get_item_by_dfid(secondary_dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(secondary_dfid.to_string()))?;

        self.logger
            .info("ItemsEngine", "item_merge", "Merging items")
            .with_context("primary_dfid", primary_dfid.to_string())
            .with_context("secondary_dfid", secondary_dfid.to_string());

        // Merge identifiers
        primary_item.add_identifiers(secondary_item.identifiers.clone());

        // Merge enriched data
        primary_item
            .enriched_data
            .extend(secondary_item.enriched_data.clone());

        // Merge source entries
        primary_item
            .source_entries
            .extend(secondary_item.source_entries.clone());

        // Update confidence score (simple average)
        primary_item.confidence_score =
            (primary_item.confidence_score + secondary_item.confidence_score) / 2.0;

        // Update the primary item
        self.storage.update_item(&primary_item)?;

        // Mark secondary item as merged and deprecate it
        let mut deprecated_secondary = secondary_item;
        deprecated_secondary.status = ItemStatus::Merged;
        self.storage.update_item(&deprecated_secondary)?;

        self.logger
            .info("ItemsEngine", "items_merged", "Items merged successfully")
            .with_context("primary_dfid", primary_dfid.to_string())
            .with_context("secondary_dfid", secondary_dfid.to_string());

        Ok(primary_item)
    }

    pub fn split_item(
        &mut self,
        dfid: &str,
        identifiers_for_new_item: Vec<Identifier>,
        new_dfid: String,
    ) -> Result<(Item, Item), ItemsError> {
        let mut original_item = self
            .storage
            .get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger
            .info("ItemsEngine", "item_split", "Splitting item")
            .with_context("original_dfid", dfid.to_string())
            .with_context("new_dfid", new_dfid.clone());

        // Create new item with specified identifiers
        let new_item = Item::new(
            new_dfid.clone(),
            identifiers_for_new_item.clone(),
            original_item.source_entries[0], // Use first source entry
        );

        // Remove the split identifiers from the original item
        original_item
            .identifiers
            .retain(|id| !identifiers_for_new_item.contains(id));

        // Mark original item as split
        original_item.status = ItemStatus::Split;

        // Store both items
        self.storage.update_item(&original_item)?;
        self.storage.store_item(&new_item)?;

        self.logger
            .info(
                "ItemsEngine",
                "item_split_completed",
                "Item split completed",
            )
            .with_context("original_dfid", dfid.to_string())
            .with_context("new_dfid", new_dfid);

        Ok((original_item, new_item))
    }

    pub fn split_item_with_generated_dfid(
        &mut self,
        dfid: &str,
        identifiers_for_new_item: Vec<Identifier>,
    ) -> Result<(Item, Item), ItemsError> {
        // Generate a unique DFID for the new item
        let new_dfid = self.dfid_engine.generate_dfid();

        // Use the existing split_item method
        self.split_item(dfid, identifiers_for_new_item, new_dfid)
    }

    pub fn deprecate_item(&mut self, dfid: &str) -> Result<Item, ItemsError> {
        let mut item = self
            .storage
            .get_item_by_dfid(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        self.logger
            .info("ItemsEngine", "item_deprecation", "Deprecating item")
            .with_context("dfid", dfid.to_string());

        item.status = ItemStatus::Deprecated;
        self.storage.update_item(&item)?;

        self.logger
            .info(
                "ItemsEngine",
                "item_deprecated",
                "Item deprecated successfully",
            )
            .with_context("dfid", dfid.to_string());

        Ok(item)
    }

    pub fn list_items(&self) -> Result<Vec<Item>, ItemsError> {
        self.storage.list_items().map_err(ItemsError::from)
    }

    pub fn find_items_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<Item>, ItemsError> {
        self.storage
            .find_items_by_identifier(identifier)
            .map_err(ItemsError::from)
    }

    pub fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, ItemsError> {
        self.storage
            .find_items_by_status(status)
            .map_err(ItemsError::from)
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
                ItemStatus::Merged | ItemStatus::MergedInto(_) => stats.merged_items += 1,
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
    pub fn share_item(
        &mut self,
        dfid: &str,
        shared_by: String,
        recipient_user_id: String,
        permissions: Option<Vec<String>>,
    ) -> Result<ItemShare, ItemsError> {
        // Verify the item exists
        let _item = self
            .get_item(dfid)?
            .ok_or_else(|| ItemsError::ItemNotFound(dfid.to_string()))?;

        // Create the share
        let share = ItemShare::new(dfid.to_string(), shared_by, recipient_user_id, permissions);

        // Store the share
        self.storage.store_item_share(&share)?;

        self.logger
            .info("ItemsEngine", "item_shared", "Item shared with user")
            .with_context("dfid", dfid.to_string())
            .with_context("share_id", share.share_id.clone())
            .with_context("shared_by", share.shared_by.clone())
            .with_context("recipient", share.recipient_user_id.clone());

        Ok(share)
    }

    pub fn get_shares_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<SharedItemResponse>, ItemsError> {
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
        self.storage
            .get_shares_for_item(dfid)
            .map_err(ItemsError::from)
    }

    pub fn is_item_shared_with_user(&self, dfid: &str, user_id: &str) -> Result<bool, ItemsError> {
        self.storage
            .is_item_shared_with_user(dfid, user_id)
            .map_err(ItemsError::from)
    }

    pub fn revoke_share(&mut self, share_id: &str) -> Result<(), ItemsError> {
        // Get share info for logging before deletion
        if let Ok(Some(share)) = self.storage.get_item_share(share_id) {
            self.logger
                .info("ItemsEngine", "share_revoked", "Item share revoked")
                .with_context("share_id", share_id.to_string())
                .with_context("dfid", share.dfid)
                .with_context("recipient", share.recipient_user_id);
        }

        self.storage
            .delete_item_share(share_id)
            .map_err(ItemsError::from)
    }

    // Conflict Detection
    fn detect_conflicts(
        &self,
        identifiers: &[Identifier],
        enriched_data: &Option<HashMap<String, serde_json::Value>>,
        _source_entry: Uuid,
    ) -> Result<Option<PendingReason>, ItemsError> {
        // Check for empty identifiers
        if identifiers.is_empty() {
            return Ok(Some(PendingReason::NoIdentifiers));
        }

        // Check for invalid identifiers (basic validation)
        for identifier in identifiers {
            if identifier.key.trim().is_empty() || identifier.value.trim().is_empty() {
                return Ok(Some(PendingReason::InvalidIdentifiers(format!(
                    "Invalid identifier: {}:{}",
                    identifier.key, identifier.value
                ))));
            }
        }

        // Check for conflicting DFIDs (when same identifier maps to multiple different DFIDs)
        for identifier in identifiers {
            if let Ok(existing_items) = self.find_items_by_identifier(identifier) {
                if existing_items.len() > 1 {
                    let mut conflicting_dfids: Vec<String> = existing_items
                        .iter()
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
                        details: format!("Null value for key: {key}"),
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
        self.storage
            .get_pending_item(pending_id)
            .map_err(ItemsError::from)
    }

    pub fn resolve_pending_item(
        &mut self,
        pending_id: &Uuid,
        resolution_action: ResolutionAction,
    ) -> Result<Option<Item>, ItemsError> {
        let pending_item = self.storage.get_pending_item(pending_id)?.ok_or_else(|| {
            ItemsError::ItemNotFound(format!("Pending item not found: {pending_id}"))
        })?;

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
                    }
                    Err(_) => {
                        // Still has conflicts, update priority and keep pending
                        let mut updated_pending = pending_item;
                        updated_pending.priority += 1; // Increase priority
                        self.storage.update_pending_item(&updated_pending)?;
                        Ok(None)
                    }
                }
            }
            ResolutionAction::Reject => {
                // Simply remove from pending
                self.storage.delete_pending_item(pending_id)?;
                Ok(None)
            }
            ResolutionAction::Modify(new_identifiers, new_data) => {
                // Update the pending item with new data
                let mut updated_pending = pending_item;
                updated_pending.identifiers = new_identifiers;
                updated_pending.enriched_data = new_data;
                self.storage.update_pending_item(&updated_pending)?;
                Ok(None)
            }
        }
    }

    pub fn get_pending_items_by_reason(
        &self,
        reason: &str,
    ) -> Result<Vec<PendingItem>, ItemsError> {
        let all_pending = self.get_pending_items()?;
        let filtered: Vec<PendingItem> = all_pending
            .into_iter()
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
