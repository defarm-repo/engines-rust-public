use crate::dfid_engine::DfidEngine;
use crate::logging::{LoggingEngine, LogEntry};
use crate::storage::{StorageError, StorageBackend};
use crate::types::{
    DataLakeEntry, Identifier, IdentifierMapping, Item, ConflictResolution,
    ProcessingStatus, MappingStatus
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
            VerificationError::StorageError(e) => write!(f, "Storage error: {}", e),
            VerificationError::ConflictDetected(c) => write!(f, "Conflict detected: {:?}", c.conflict_id),
            VerificationError::ProcessingError(e) => write!(f, "Processing error: {}", e),
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
        logger.info("VerificationEngine", "initialization", "Verification engine initialized");

        Self {
            storage,
            dfid_engine,
            logger,
        }
    }

    pub fn process_pending_entries(&mut self) -> Result<Vec<VerificationResult>, VerificationError> {
        let pending_entries = self.storage.get_data_lake_entries_by_status(ProcessingStatus::Pending)?;
        let mut results = Vec::new();

        self.logger.info("VerificationEngine", "batch_processing", "Processing pending data lake entries")
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

                    self.logger.error("VerificationEngine", "entry_processing_failed", "Failed to process entry")
                        .with_context("entry_id", entry.entry_id.to_string())
                        .with_context("error", e.to_string());
                }
            }
        }

        Ok(results)
    }

    pub fn process_entry(&mut self, entry: &mut DataLakeEntry) -> Result<VerificationResult, VerificationError> {
        self.logger.info("VerificationEngine", "entry_processing", "Processing data lake entry")
            .with_context("entry_id", entry.entry_id.to_string())
            .with_context("identifiers_count", entry.identifiers.len().to_string());

        // Check for existing mappings for each identifier
        let identifier_analysis = self.analyze_identifiers(&entry.identifiers)?;

        match identifier_analysis {
            IdentifierAnalysis::AllNew => {
                self.create_new_item(entry)
            }
            IdentifierAnalysis::ExistingSingle(dfid) => {
                self.enrich_existing_item(entry, &dfid)
            }
            IdentifierAnalysis::Conflict(conflict_info) => {
                self.handle_conflict(entry, conflict_info)
            }
        }
    }

    fn analyze_identifiers(&self, identifiers: &[Identifier]) -> Result<IdentifierAnalysis, VerificationError> {
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
                    .or_insert_with(Vec::new)
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
                let conflict_identifiers: Vec<Identifier> = dfid_map
                    .values()
                    .flatten()
                    .cloned()
                    .collect();

                Ok(IdentifierAnalysis::Conflict(ConflictInfo {
                    conflicting_dfids: dfids,
                    conflicting_identifiers: conflict_identifiers,
                }))
            }
        }
    }

    fn create_new_item(&mut self, entry: &mut DataLakeEntry) -> Result<VerificationResult, VerificationError> {
        let dfid = self.dfid_engine.generate_dfid();

        self.logger.info("VerificationEngine", "item_creation", "Creating new item")
            .with_context("dfid", dfid.clone())
            .with_context("entry_id", entry.entry_id.to_string());

        // Create new item
        let item = Item::new(dfid.clone(), entry.identifiers.clone(), entry.entry_id);
        self.storage.store_item(&item)?;

        // Create identifier mappings
        for identifier in &entry.identifiers {
            let mapping = IdentifierMapping::new(
                identifier.clone(),
                dfid.clone(),
                "primary".to_string(),
            );
            self.storage.store_identifier_mapping(&mapping)?;
        }

        entry.mark_completed(dfid.clone());

        self.logger.info("VerificationEngine", "item_created", "New item created successfully")
            .with_context("dfid", dfid.clone());

        Ok(VerificationResult::NewItemCreated { dfid })
    }

    fn enrich_existing_item(&mut self, entry: &mut DataLakeEntry, dfid: &str) -> Result<VerificationResult, VerificationError> {
        self.logger.info("VerificationEngine", "item_enrichment", "Enriching existing item")
            .with_context("dfid", dfid.to_string())
            .with_context("entry_id", entry.entry_id.to_string());

        let mut item = self.storage.get_item_by_dfid(dfid)?
            .ok_or_else(|| VerificationError::ProcessingError(format!("Item with DFID {} not found", dfid)))?;

        // Add new identifiers if any
        item.add_identifiers(entry.identifiers.clone());

        // Create enriched data (simplified - in practice, this would extract meaningful data)
        let mut enriched_data = HashMap::new();
        enriched_data.insert("data_hash".to_string(), serde_json::Value::String(entry.data_hash.clone()));
        enriched_data.insert("data_size".to_string(), serde_json::Value::Number(entry.data_size.into()));

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

        self.logger.info("VerificationEngine", "item_enriched", "Item enriched successfully")
            .with_context("dfid", dfid.to_string());

        Ok(VerificationResult::ItemEnriched { dfid: dfid.to_string() })
    }

    fn handle_conflict(&mut self, entry: &mut DataLakeEntry, conflict_info: ConflictInfo) -> Result<VerificationResult, VerificationError> {
        self.logger.warn("VerificationEngine", "conflict_detected", "Identifier conflict detected")
            .with_context("entry_id", entry.entry_id.to_string())
            .with_context("conflicting_dfids", conflict_info.conflicting_dfids.join(","));

        let conflict_resolution = ConflictResolution::new(
            conflict_info.conflicting_identifiers.clone(),
            conflict_info.conflicting_dfids.clone(),
        );

        self.storage.store_conflict_resolution(&conflict_resolution)?;
        entry.mark_conflicted();

        // Attempt automatic resolution based on confidence
        if let Some(resolved_dfid) = self.attempt_auto_resolution(&conflict_info)? {
            self.logger.info("VerificationEngine", "conflict_auto_resolved", "Conflict automatically resolved")
                .with_context("resolved_dfid", resolved_dfid.clone());

            return self.enrich_existing_item(entry, &resolved_dfid);
        }

        self.logger.warn("VerificationEngine", "conflict_requires_manual_review", "Conflict requires manual review")
            .with_context("conflict_id", conflict_resolution.conflict_id.to_string());

        Ok(VerificationResult::ConflictDetected {
            conflict_id: conflict_resolution.conflict_id,
            conflicting_dfids: conflict_info.conflicting_dfids,
        })
    }

    fn attempt_auto_resolution(&mut self, conflict_info: &ConflictInfo) -> Result<Option<String>, VerificationError> {
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
    NewItemCreated { dfid: String },
    ItemEnriched { dfid: String },
    ConflictDetected { conflict_id: Uuid, conflicting_dfids: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProcessingStatus;
    use std::collections::HashMap;
    use uuid::Uuid;

    // Mock storage implementation for testing
    struct MockVerificationStorage {
        data_lake_entries: HashMap<Uuid, DataLakeEntry>,
        identifier_mappings: HashMap<Identifier, Vec<IdentifierMapping>>,
        items: HashMap<String, Item>,
        conflicts: Vec<ConflictResolution>,
    }

    impl MockVerificationStorage {
        fn new() -> Self {
            Self {
                data_lake_entries: HashMap::new(),
                identifier_mappings: HashMap::new(),
                items: HashMap::new(),
                conflicts: Vec::new(),
            }
        }
    }

    impl StorageBackend for MockVerificationStorage {

        fn update_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
            self.data_lake_entries.insert(entry.entry_id, entry.clone());
            Ok(())
        }

        fn get_identifier_mappings(&self, identifier: &Identifier) -> Result<Vec<IdentifierMapping>, StorageError> {
            Ok(self.identifier_mappings
                .get(identifier)
                .cloned()
                .unwrap_or_default())
        }

        fn store_identifier_mapping(&mut self, mapping: &IdentifierMapping) -> Result<(), StorageError> {
            self.identifier_mappings
                .entry(mapping.identifier.clone())
                .or_insert_with(Vec::new)
                .push(mapping.clone());
            Ok(())
        }

        fn update_identifier_mapping(&mut self, mapping: &IdentifierMapping) -> Result<(), StorageError> {
            self.store_identifier_mapping(mapping)
        }

        fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
            Ok(self.items.get(dfid).cloned())
        }

        fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
            self.items.insert(item.dfid.clone(), item.clone());
            Ok(())
        }

        fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
            self.store_item(item)
        }

        fn store_conflict_resolution(&mut self, conflict: &ConflictResolution) -> Result<(), StorageError> {
            self.conflicts.push(conflict.clone());
            Ok(())
        }

        fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError> {
            Ok(self.conflicts
                .iter()
                .filter(|c| c.requires_manual_review)
                .cloned()
                .collect())
        }

        // Add all missing StorageBackend methods for testing
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
        fn list_items(&self) -> Result<Vec<Item>, StorageError> {
            Ok(Vec::new())
        }
        fn find_items_by_identifier(&self, _identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
            Ok(Vec::new())
        }
        fn find_items_by_status(&self, _status: crate::types::ItemStatus) -> Result<Vec<Item>, StorageError> {
            Ok(Vec::new())
        }
        fn delete_item(&mut self, _dfid: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn store_item_share(&mut self, _share: &crate::types::ItemShare) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_item_share(&self, _share_id: &str) -> Result<Option<crate::types::ItemShare>, StorageError> {
            Ok(None)
        }
        fn get_shares_for_user(&self, _user_id: &str) -> Result<Vec<crate::types::ItemShare>, StorageError> {
            Ok(Vec::new())
        }
        fn get_shares_for_item(&self, _dfid: &str) -> Result<Vec<crate::types::ItemShare>, StorageError> {
            Ok(Vec::new())
        }
        fn is_item_shared_with_user(&self, _dfid: &str, _user_id: &str) -> Result<bool, StorageError> {
            Ok(false)
        }
        fn delete_item_share(&mut self, _share_id: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_conflict_resolution(&self, _conflict_id: &uuid::Uuid) -> Result<Option<ConflictResolution>, StorageError> {
            Ok(None)
        }
        fn store_data_lake_entry(&mut self, _entry: &DataLakeEntry) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_data_lake_entry(&self, _entry_id: &uuid::Uuid) -> Result<Option<DataLakeEntry>, StorageError> {
            Ok(None)
        }
        fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError> {
            Ok(Vec::new())
        }
        fn get_data_lake_entries_by_status(&self, status: ProcessingStatus) -> Result<Vec<DataLakeEntry>, StorageError> {
            Ok(self.data_lake_entries
                .values()
                .filter(|entry| matches!(entry.status, ProcessingStatus::Pending) == matches!(status, ProcessingStatus::Pending))
                .cloned()
                .collect())
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
    fn test_create_new_item() {
        let storage = MockVerificationStorage::new();
        let dfid_engine = DfidEngine::new();
        let mut engine = VerificationEngine::new(storage, dfid_engine);

        let identifiers = vec![Identifier::new("user_id", "12345")];
        let mut entry = DataLakeEntry::new(
            Uuid::new_v4(),
            identifiers,
            "test_hash".to_string(),
            100,
        );

        let result = engine.process_entry(&mut entry).unwrap();

        match result {
            VerificationResult::NewItemCreated { dfid } => {
                assert!(dfid.starts_with("DFID-"));
                assert!(matches!(entry.status, ProcessingStatus::Completed));
                assert_eq!(entry.linked_dfid, Some(dfid));
            }
            _ => panic!("Expected NewItemCreated result"),
        }
    }

    #[test]
    fn test_enrich_existing_item() {
        let mut storage = MockVerificationStorage::new();
        let dfid_engine = DfidEngine::new();

        // Setup existing item and mapping
        let dfid = "DFID-20240926-000001-TEST".to_string();
        let identifier = Identifier::new("user_id", "12345");
        let existing_item = Item::new(dfid.clone(), vec![identifier.clone()], Uuid::new_v4());
        let mapping = IdentifierMapping::new(identifier.clone(), dfid.clone(), "primary".to_string());

        storage.store_item(&existing_item).unwrap();
        storage.store_identifier_mapping(&mapping).unwrap();

        let mut engine = VerificationEngine::new(storage, dfid_engine);

        let identifiers = vec![identifier, Identifier::new("email", "test@example.com")];
        let mut entry = DataLakeEntry::new(
            Uuid::new_v4(),
            identifiers,
            "test_hash2".to_string(),
            200,
        );

        let result = engine.process_entry(&mut entry).unwrap();

        match result {
            VerificationResult::ItemEnriched { dfid: result_dfid } => {
                assert_eq!(result_dfid, dfid);
                assert!(matches!(entry.status, ProcessingStatus::Completed));
            }
            _ => panic!("Expected ItemEnriched result"),
        }
    }
}