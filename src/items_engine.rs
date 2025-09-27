use crate::dfid_engine::DfidEngine;
use crate::logging::{LoggingEngine, LogEntry};
use crate::storage::{StorageError, StorageBackend};
use crate::types::{Item, Identifier, ItemStatus};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub enum ItemsError {
    StorageError(StorageError),
    ItemNotFound(String),
    InvalidOperation(String),
}

impl std::fmt::Display for ItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemsError::StorageError(e) => write!(f, "Storage error: {}", e),
            ItemsError::ItemNotFound(dfid) => write!(f, "Item not found: {}", dfid),
            ItemsError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for ItemsError {}

impl From<StorageError> for ItemsError {
    fn from(err: StorageError) -> Self {
        ItemsError::StorageError(err)
    }
}

// Implement ItemsStorage for any StorageBackend
impl<T: StorageBackend> ItemsStorage for T {
    fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.store_item(item)
    }

    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
        self.get_item_by_dfid(dfid)
    }

    fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.update_item(item)
    }

    fn list_items(&self) -> Result<Vec<Item>, StorageError> {
        self.list_items()
    }

    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
        self.find_items_by_identifier(identifier)
    }

    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError> {
        self.find_items_by_status(status)
    }

    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError> {
        self.delete_item(dfid)
    }
}

pub trait ItemsStorage {
    fn store_item(&mut self, item: &Item) -> Result<(), StorageError>;
    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError>;
    fn update_item(&mut self, item: &Item) -> Result<(), StorageError>;
    fn list_items(&self) -> Result<Vec<Item>, StorageError>;
    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError>;
    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError>;
    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError>;
}

pub struct ItemsEngine<S: ItemsStorage> {
    storage: S,
    logger: LoggingEngine,
    dfid_engine: DfidEngine,
}

impl<S: ItemsStorage> ItemsEngine<S> {
    pub fn new(storage: S) -> Self {
        let mut logger = LoggingEngine::new();
        logger.info("ItemsEngine", "initialization", "Items engine initialized");

        Self {
            storage,
            logger,
            dfid_engine: DfidEngine::new(),
        }
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
            .with_context("dfid", dfid);

        Ok(item)
    }

    pub fn create_item_with_generated_dfid(&mut self, identifiers: Vec<Identifier>, source_entry: Uuid, enriched_data: Option<HashMap<String, serde_json::Value>>) -> Result<Item, ItemsError> {
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

    pub fn get_item(&self, dfid: &str) -> Result<Option<Item>, ItemsError> {
        self.storage.get_item_by_dfid(dfid).map_err(ItemsError::from)
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

    impl ItemsStorage for MockItemsStorage {
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