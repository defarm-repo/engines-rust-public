use crate::dfid_engine::DfidEngine;
use crate::logging::{LogEntry, LoggingEngine};
use crate::storage::{StorageBackend, StorageError};
use crate::types::{
    ConflictResolution, DataLakeEntry, Identifier, IdentifierMapping, Item, MappingStatus,
    ProcessingStatus,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub enum VerificationError {
    StorageError(StorageError),
    ConflictDetected(ConflictResolution),
    ProcessingError(String),
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::StorageError(e) => write!(f, "Storage error: {e}"),
            VerificationError::ConflictDetected(c) => {
                write!(f, "Conflict detected: {:?}", c.conflict_id)
            }
            VerificationError::ProcessingError(e) => write!(f, "Processing error: {e}"),
        }
    }
}

impl std::error::Error for VerificationError {}

impl From<StorageError> for VerificationError {
    fn from(err: StorageError) -> Self {
        VerificationError::StorageError(err)
    }
}

pub struct VerificationEngine<S: StorageBackend> {
    storage: S,
    dfid_engine: DfidEngine,
    logger: LoggingEngine,
}

impl<S: StorageBackend> VerificationEngine<S> {
    pub fn new(storage: S, dfid_engine: DfidEngine) -> Self {
        let mut logger = LoggingEngine::new();
        logger.info(
            "VerificationEngine",
            "initialization",
            "Verification engine initialized",
        );

        Self {
            storage,
            dfid_engine,
            logger,
        }
    }

    pub fn process_pending_entries(
        &mut self,
    ) -> Result<Vec<VerificationResult>, VerificationError> {
        let pending_entries = self
            .storage
            .get_data_lake_entries_by_status(ProcessingStatus::Pending)?;
        let mut results = Vec::new();

        self.logger
            .info(
                "VerificationEngine",
                "batch_processing",
                "Processing pending data lake entries",
            )
            .with_context("entries_count", pending_entries.len().to_string());

        for mut entry in pending_entries {
            entry.mark_processing();
            self.storage.update_data_lake_entry(&entry)?;

            match self.process_entry(&mut entry) {
                Ok(result) => {
                    results.push(result);
                    self.storage.update_data_lake_entry(&entry)?;
                }
                Err(e) => {
                    entry.mark_failed(e.to_string());
                    self.storage.update_data_lake_entry(&entry)?;

                    self.logger
                        .error(
                            "VerificationEngine",
                            "entry_processing_failed",
                            "Failed to process entry",
                        )
                        .with_context("entry_id", entry.entry_id.to_string())
                        .with_context("error", e.to_string());
                }
            }
        }

        Ok(results)
    }

    pub fn process_entry(
        &mut self,
        entry: &mut DataLakeEntry,
    ) -> Result<VerificationResult, VerificationError> {
        self.logger
            .info(
                "VerificationEngine",
                "entry_processing",
                "Processing data lake entry",
            )
            .with_context("entry_id", entry.entry_id.to_string())
            .with_context("identifiers_count", entry.identifiers.len().to_string());

        // Check for existing mappings for each identifier
        let identifier_analysis = self.analyze_identifiers(&entry.identifiers)?;

        match identifier_analysis {
            IdentifierAnalysis::AllNew => self.create_new_item(entry),
            IdentifierAnalysis::ExistingSingle(dfid) => self.enrich_existing_item(entry, &dfid),
            IdentifierAnalysis::Conflict(conflict_info) => {
                self.handle_conflict(entry, conflict_info)
            }
        }
    }

    fn analyze_identifiers(
        &self,
        identifiers: &[Identifier],
    ) -> Result<IdentifierAnalysis, VerificationError> {
        let mut dfid_map: HashMap<String, Vec<Identifier>> = HashMap::new();

        for identifier in identifiers {
            let mappings = self.storage.get_identifier_mappings(identifier)?;
            let active_mappings: Vec<_> = mappings
                .into_iter()
                .filter(|m| matches!(m.status, MappingStatus::Active))
                .collect();

            if active_mappings.is_empty() {
                continue; // This identifier is new
            }

            for mapping in active_mappings {
                dfid_map
                    .entry(mapping.dfid.clone())
                    .or_default()
                    .push(identifier.clone());
            }
        }

        match dfid_map.len() {
            0 => Ok(IdentifierAnalysis::AllNew),
            1 => {
                let (dfid, _) = dfid_map.into_iter().next().unwrap();
                Ok(IdentifierAnalysis::ExistingSingle(dfid))
            }
            _ => {
                let dfids: Vec<String> = dfid_map.keys().cloned().collect();
                let conflict_identifiers: Vec<Identifier> =
                    dfid_map.values().flatten().cloned().collect();

                Ok(IdentifierAnalysis::Conflict(ConflictInfo {
                    conflicting_dfids: dfids,
                    conflicting_identifiers: conflict_identifiers,
                }))
            }
        }
    }

    fn create_new_item(
        &mut self,
        entry: &mut DataLakeEntry,
    ) -> Result<VerificationResult, VerificationError> {
        let dfid = self.dfid_engine.generate_dfid();

        self.logger
            .info("VerificationEngine", "item_creation", "Creating new item")
            .with_context("dfid", dfid.clone())
            .with_context("entry_id", entry.entry_id.to_string());

        // Create new item
        let item = Item::new(dfid.clone(), entry.identifiers.clone(), entry.entry_id);
        self.storage.store_item(&item)?;

        // Create identifier mappings
        for identifier in &entry.identifiers {
            let mapping =
                IdentifierMapping::new(identifier.clone(), dfid.clone(), "primary".to_string());
            self.storage.store_identifier_mapping(&mapping)?;
        }

        entry.mark_completed(dfid.clone());

        self.logger
            .info(
                "VerificationEngine",
                "item_created",
                "New item created successfully",
            )
            .with_context("dfid", dfid.clone());

        Ok(VerificationResult::NewItemCreated { dfid })
    }

    fn enrich_existing_item(
        &mut self,
        entry: &mut DataLakeEntry,
        dfid: &str,
    ) -> Result<VerificationResult, VerificationError> {
        self.logger
            .info(
                "VerificationEngine",
                "item_enrichment",
                "Enriching existing item",
            )
            .with_context("dfid", dfid.to_string())
            .with_context("entry_id", entry.entry_id.to_string());

        let mut item = self.storage.get_item_by_dfid(dfid)?.ok_or_else(|| {
            VerificationError::ProcessingError(format!("Item with DFID {dfid} not found"))
        })?;

        // Add new identifiers if any
        item.add_identifiers(entry.identifiers.clone());

        // Create enriched data (simplified - in practice, this would extract meaningful data)
        let mut enriched_data = HashMap::new();
        enriched_data.insert(
            "data_hash".to_string(),
            serde_json::Value::String(entry.data_hash.clone()),
        );
        enriched_data.insert(
            "data_size".to_string(),
            serde_json::Value::Number(entry.data_size.into()),
        );

        item.enrich(enriched_data, entry.entry_id);
        self.storage.update_item(&item)?;

        // Create mappings for any new identifiers
        for identifier in &entry.identifiers {
            let existing_mappings = self.storage.get_identifier_mappings(identifier)?;
            if existing_mappings.is_empty() {
                let mapping = IdentifierMapping::new(
                    identifier.clone(),
                    dfid.to_string(),
                    "enrichment".to_string(),
                );
                self.storage.store_identifier_mapping(&mapping)?;
            }
        }

        entry.mark_completed(dfid.to_string());

        self.logger
            .info(
                "VerificationEngine",
                "item_enriched",
                "Item enriched successfully",
            )
            .with_context("dfid", dfid.to_string());

        Ok(VerificationResult::ItemEnriched {
            dfid: dfid.to_string(),
        })
    }

    fn handle_conflict(
        &mut self,
        entry: &mut DataLakeEntry,
        conflict_info: ConflictInfo,
    ) -> Result<VerificationResult, VerificationError> {
        self.logger
            .warn(
                "VerificationEngine",
                "conflict_detected",
                "Identifier conflict detected",
            )
            .with_context("entry_id", entry.entry_id.to_string())
            .with_context(
                "conflicting_dfids",
                conflict_info.conflicting_dfids.join(","),
            );

        let conflict_resolution = ConflictResolution::new(
            conflict_info.conflicting_identifiers.clone(),
            conflict_info.conflicting_dfids.clone(),
        );

        self.storage
            .store_conflict_resolution(&conflict_resolution)?;
        entry.mark_conflicted();

        // Attempt automatic resolution based on confidence
        if let Some(resolved_dfid) = self.attempt_auto_resolution(&conflict_info)? {
            self.logger
                .info(
                    "VerificationEngine",
                    "conflict_auto_resolved",
                    "Conflict automatically resolved",
                )
                .with_context("resolved_dfid", resolved_dfid.clone());

            return self.enrich_existing_item(entry, &resolved_dfid);
        }

        self.logger
            .warn(
                "VerificationEngine",
                "conflict_requires_manual_review",
                "Conflict requires manual review",
            )
            .with_context("conflict_id", conflict_resolution.conflict_id.to_string());

        Ok(VerificationResult::ConflictDetected {
            conflict_id: conflict_resolution.conflict_id,
            conflicting_dfids: conflict_info.conflicting_dfids,
        })
    }

    fn attempt_auto_resolution(
        &mut self,
        conflict_info: &ConflictInfo,
    ) -> Result<Option<String>, VerificationError> {
        // Simple confidence-based resolution: prefer the item with more source entries
        let mut best_dfid = None;
        let mut max_sources = 0;

        for dfid in &conflict_info.conflicting_dfids {
            if let Some(item) = self.storage.get_item_by_dfid(dfid)? {
                if item.source_entries.len() > max_sources {
                    max_sources = item.source_entries.len();
                    best_dfid = Some(dfid.clone());
                }
            }
        }

        Ok(best_dfid)
    }

    pub fn get_logs(&self) -> &[LogEntry] {
        self.logger.get_logs()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<&LogEntry> {
        self.logger.get_logs_by_event_type(event_type)
    }
}

#[derive(Debug)]
enum IdentifierAnalysis {
    AllNew,
    ExistingSingle(String), // DFID
    Conflict(ConflictInfo),
}

#[derive(Debug)]
struct ConflictInfo {
    conflicting_dfids: Vec<String>,
    conflicting_identifiers: Vec<Identifier>,
}

#[derive(Debug, Clone)]
pub enum VerificationResult {
    NewItemCreated {
        dfid: String,
    },
    ItemEnriched {
        dfid: String,
    },
    ConflictDetected {
        conflict_id: Uuid,
        conflicting_dfids: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use crate::types::ProcessingStatus;
    use std::sync::Arc;
    use uuid::Uuid;

    fn new_engine() -> (
        Arc<std::sync::Mutex<InMemoryStorage>>,
        VerificationEngine<Arc<std::sync::Mutex<InMemoryStorage>>>,
    ) {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let engine = VerificationEngine::new(Arc::clone(&storage), DfidEngine::new());
        (storage, engine)
    }

    #[test]
    fn test_process_entry_creates_new_item() {
        let (storage, mut engine) = new_engine();
        let identifiers = vec![Identifier::new("batch_id", "001")];
        let mut entry = DataLakeEntry::new(
            Uuid::new_v4(),
            identifiers.clone(),
            "hash1".to_string(),
            128,
        );

        let result = engine.process_entry(&mut entry).unwrap();

        match result {
            VerificationResult::NewItemCreated { dfid } => {
                assert!(dfid.starts_with("DFID-"));
                assert_eq!(entry.status, ProcessingStatus::Completed);
                let guard = storage.lock().unwrap();
                let item = guard.get_item_by_dfid(&dfid).unwrap().unwrap();
                assert_eq!(item.identifiers, identifiers);
            }
            other => panic!("expected VerificationResult::NewItemCreated, got {other:?}"),
        }
    }

    #[test]
    fn test_process_entry_enriches_existing_item() {
        let (storage, mut engine) = new_engine();
        let dfid = "DFID-EXISTING-001".to_string();
        let base_identifier = Identifier::new("user_id", "12345");

        {
            let guard = storage.lock().unwrap();
            let existing_item =
                Item::new(dfid.clone(), vec![base_identifier.clone()], Uuid::new_v4());
            guard.store_item(&existing_item).unwrap();

            let mapping =
                IdentifierMapping::new(base_identifier.clone(), dfid.clone(), "primary".into());
            guard.store_identifier_mapping(&mapping).unwrap();
        }

        let mut entry = DataLakeEntry::new(
            Uuid::new_v4(),
            vec![
                base_identifier.clone(),
                Identifier::new("email", "user@example.com"),
            ],
            "hash2".to_string(),
            256,
        );

        let result = engine.process_entry(&mut entry).unwrap();

        match result {
            VerificationResult::ItemEnriched {
                dfid: enriched_dfid,
            } => {
                assert_eq!(enriched_dfid, dfid);
                assert_eq!(entry.status, ProcessingStatus::Completed);
            }
            other => panic!("expected VerificationResult::ItemEnriched, got {other:?}"),
        }
    }
}
