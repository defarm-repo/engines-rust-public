use crate::identifier_types::EnhancedIdentifier;
use crate::logging::LogEntry;
use crate::types::{
    Activity, AdapterConfig, AdapterTestResult, AdapterType, AdminAction, AuditDashboardMetrics,
    AuditEvent, AuditEventType, AuditQuery, AuditSeverity, Circuit, CircuitAdapterConfig,
    CircuitItem, CircuitOperation, CircuitType, ComplianceReport, ComplianceStatus,
    ConflictResolution, CreditTransaction, DataLakeEntry, Event, EventCidMapping, EventType,
    EventVisibility, Identifier, IdentifierMapping, IndexingProgress, Item, ItemShare, ItemStatus,
    ItemStorageHistory, Notification, PendingItem, PendingPriority, PendingReason,
    ProcessingStatus, Receipt, SecurityIncident, SecurityIncidentSummary, StorageRecord,
    SystemStatistics, TimelineEntry, UserAccount, WebhookDelivery,
};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub enum StorageError {
    IoError(String),
    SerializationError(serde_json::Error),
    EncryptionError(String),
    NotFound,
    AlreadyExists(String),
    NotImplemented(String),
    ConnectionError(String),
    ConfigurationError(String),
    WriteError(String),
    ReadError(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err)
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(e) => write!(f, "IO error: {e}"),
            StorageError::SerializationError(e) => write!(f, "Serialization error: {e}"),
            StorageError::EncryptionError(e) => write!(f, "Encryption error: {e}"),
            StorageError::NotFound => write!(f, "Record not found"),
            StorageError::AlreadyExists(e) => write!(f, "Already exists: {e}"),
            StorageError::NotImplemented(e) => write!(f, "Not implemented: {e}"),
            StorageError::ConnectionError(e) => write!(f, "Connection error: {e}"),
            StorageError::ConfigurationError(e) => write!(f, "Configuration error: {e}"),
            StorageError::WriteError(e) => write!(f, "Write error: {e}"),
            StorageError::ReadError(e) => write!(f, "Read error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub data: Vec<u8>,
    pub nonce: [u8; 12],
}

#[derive(Debug, Clone)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn as_aes_key(&self) -> &Key<Aes256Gcm> {
        Key::<Aes256Gcm>::from_slice(&self.0)
    }
}

pub trait StorageBackend {
    fn store_receipt(&mut self, receipt: &Receipt) -> Result<(), StorageError>;
    fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError>;
    fn find_receipts_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<Receipt>, StorageError>;
    fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError>;

    fn store_log(&mut self, log: &LogEntry) -> Result<(), StorageError>;
    fn get_logs(&self) -> Result<Vec<LogEntry>, StorageError>;

    // Data Lake operations
    fn store_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError>;
    fn get_data_lake_entry(&self, entry_id: &Uuid) -> Result<Option<DataLakeEntry>, StorageError>;
    fn update_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError>;
    fn get_data_lake_entries_by_status(
        &self,
        status: ProcessingStatus,
    ) -> Result<Vec<DataLakeEntry>, StorageError>;
    fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError>;

    // Items operations
    fn store_item(&mut self, item: &Item) -> Result<(), StorageError>;
    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError>;
    fn update_item(&mut self, item: &Item) -> Result<(), StorageError>;
    fn list_items(&self) -> Result<Vec<Item>, StorageError>;
    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError>;
    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError>;
    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError>;

    // Identifier Mapping operations
    fn store_identifier_mapping(&mut self, mapping: &IdentifierMapping)
        -> Result<(), StorageError>;
    fn get_identifier_mappings(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<IdentifierMapping>, StorageError>;
    fn update_identifier_mapping(
        &mut self,
        mapping: &IdentifierMapping,
    ) -> Result<(), StorageError>;
    fn list_identifier_mappings(&self) -> Result<Vec<IdentifierMapping>, StorageError>;

    // Conflict Resolution operations
    fn store_conflict_resolution(
        &mut self,
        conflict: &ConflictResolution,
    ) -> Result<(), StorageError>;
    fn get_conflict_resolution(
        &self,
        conflict_id: &Uuid,
    ) -> Result<Option<ConflictResolution>, StorageError>;
    fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError>;

    // Event operations
    fn store_event(&mut self, event: &Event) -> Result<(), StorageError>;
    fn get_event(&self, event_id: &Uuid) -> Result<Option<Event>, StorageError>;
    fn update_event(&mut self, event: &Event) -> Result<(), StorageError>;
    fn list_events(&self) -> Result<Vec<Event>, StorageError>;
    fn get_events_by_dfid(&self, dfid: &str) -> Result<Vec<Event>, StorageError>;
    fn get_events_by_type(&self, event_type: EventType) -> Result<Vec<Event>, StorageError>;
    fn get_events_by_visibility(
        &self,
        visibility: EventVisibility,
    ) -> Result<Vec<Event>, StorageError>;
    fn get_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>, StorageError>;

    // Circuit operations
    fn store_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError>;
    fn get_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, StorageError>;
    fn update_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError>;
    fn list_circuits(&self) -> Result<Vec<Circuit>, StorageError>;
    fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, StorageError>;

    // Circuit Operation operations
    fn store_circuit_operation(&mut self, operation: &CircuitOperation)
        -> Result<(), StorageError>;
    fn get_circuit_operation(
        &self,
        operation_id: &Uuid,
    ) -> Result<Option<CircuitOperation>, StorageError>;
    fn update_circuit_operation(
        &mut self,
        operation: &CircuitOperation,
    ) -> Result<(), StorageError>;
    fn get_circuit_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, StorageError>;

    // Item Share operations
    fn store_item_share(&mut self, share: &ItemShare) -> Result<(), StorageError>;
    fn get_item_share(&self, share_id: &str) -> Result<Option<ItemShare>, StorageError>;
    fn get_shares_for_user(&self, user_id: &str) -> Result<Vec<ItemShare>, StorageError>;
    fn get_shares_for_item(&self, dfid: &str) -> Result<Vec<ItemShare>, StorageError>;
    fn is_item_shared_with_user(&self, dfid: &str, user_id: &str) -> Result<bool, StorageError>;
    fn delete_item_share(&mut self, share_id: &str) -> Result<(), StorageError>;

    // Activity operations
    fn store_activity(&mut self, activity: &Activity) -> Result<(), StorageError>;
    fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, StorageError>;
    fn get_activities_for_circuit(&self, circuit_id: &Uuid) -> Result<Vec<Activity>, StorageError>;
    fn get_all_activities(&self) -> Result<Vec<Activity>, StorageError>;

    // Circuit Items operations
    fn store_circuit_item(&mut self, circuit_item: &CircuitItem) -> Result<(), StorageError>;
    fn get_circuit_items(&self, circuit_id: &Uuid) -> Result<Vec<CircuitItem>, StorageError>;
    fn remove_circuit_item(&mut self, circuit_id: &Uuid, dfid: &str) -> Result<(), StorageError>;

    // Audit Event operations
    fn store_audit_event(&mut self, event: &AuditEvent) -> Result<(), StorageError>;
    fn get_audit_event(&self, event_id: &Uuid) -> Result<Option<AuditEvent>, StorageError>;
    fn query_audit_events(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>, StorageError>;
    fn list_audit_events(&self) -> Result<Vec<AuditEvent>, StorageError>;
    fn get_audit_events_by_user(&self, user_id: &str) -> Result<Vec<AuditEvent>, StorageError>;
    fn get_audit_events_by_type(
        &self,
        event_type: AuditEventType,
    ) -> Result<Vec<AuditEvent>, StorageError>;
    fn get_audit_events_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<AuditEvent>, StorageError>;
    fn get_audit_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AuditEvent>, StorageError>;
    fn sync_audit_events(&mut self, events: Vec<AuditEvent>) -> Result<(), StorageError>;

    // Security Incident operations
    fn store_security_incident(&mut self, incident: &SecurityIncident) -> Result<(), StorageError>;
    fn get_security_incident(
        &self,
        incident_id: &Uuid,
    ) -> Result<Option<SecurityIncident>, StorageError>;
    fn update_security_incident(&mut self, incident: &SecurityIncident)
        -> Result<(), StorageError>;
    fn list_security_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError>;
    fn get_incidents_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<SecurityIncident>, StorageError>;
    fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError>;
    fn get_incidents_by_assignee(
        &self,
        assignee: &str,
    ) -> Result<Vec<SecurityIncident>, StorageError>;

    // Compliance Report operations
    fn store_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError>;
    fn get_compliance_report(
        &self,
        report_id: &Uuid,
    ) -> Result<Option<ComplianceReport>, StorageError>;
    fn update_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError>;
    fn list_compliance_reports(&self) -> Result<Vec<ComplianceReport>, StorageError>;
    fn get_reports_by_type(&self, report_type: &str)
        -> Result<Vec<ComplianceReport>, StorageError>;
    fn get_pending_reports(&self) -> Result<Vec<ComplianceReport>, StorageError>;

    // Audit Dashboard operations
    fn get_audit_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, StorageError>;
    fn get_event_count_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, StorageError>;

    // Pending Items operations
    fn store_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError>;
    fn get_pending_item(&self, pending_id: &Uuid) -> Result<Option<PendingItem>, StorageError>;
    fn list_pending_items(&self) -> Result<Vec<PendingItem>, StorageError>;
    fn get_pending_items_by_reason(
        &self,
        reason_type: &str,
    ) -> Result<Vec<PendingItem>, StorageError>;
    fn get_pending_items_by_user(&self, user_id: &str) -> Result<Vec<PendingItem>, StorageError>;
    fn get_pending_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PendingItem>, StorageError>;
    fn get_pending_items_by_priority(
        &self,
        priority: PendingPriority,
    ) -> Result<Vec<PendingItem>, StorageError>;
    fn update_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError>;
    fn delete_pending_item(&mut self, pending_id: &Uuid) -> Result<(), StorageError>;
    fn get_pending_items_requiring_manual_review(&self) -> Result<Vec<PendingItem>, StorageError>;

    // ZK Proof operations
    fn store_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError>;
    fn get_zk_proof(
        &self,
        proof_id: &Uuid,
    ) -> Result<Option<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn update_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError>;
    fn query_zk_proofs(
        &self,
        query: &crate::api::zk_proofs::ZkProofQuery,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn list_zk_proofs(&self) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn get_zk_proofs_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn get_zk_proofs_by_circuit_type(
        &self,
        circuit_type: CircuitType,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn get_zk_proofs_by_status(
        &self,
        status: crate::zk_proof_engine::ProofStatus,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError>;
    fn get_zk_proof_statistics(
        &self,
    ) -> Result<crate::api::zk_proofs::ZkProofStatistics, StorageError>;
    fn delete_zk_proof(&mut self, proof_id: &Uuid) -> Result<(), StorageError>;

    // Storage History operations
    fn store_storage_history(&mut self, history: &ItemStorageHistory) -> Result<(), StorageError>;
    fn get_storage_history(&self, dfid: &str) -> Result<Option<ItemStorageHistory>, StorageError>;
    fn add_storage_record(&mut self, dfid: &str, record: StorageRecord)
        -> Result<(), StorageError>;

    // CID Timeline operations (populated by blockchain event listener)
    fn add_cid_to_timeline(
        &mut self,
        dfid: &str,
        cid: &str,
        ipcm_tx: &str,
        timestamp: i64,
        network: &str,
    ) -> Result<(), StorageError>;
    fn get_item_timeline(&self, dfid: &str) -> Result<Vec<TimelineEntry>, StorageError>;
    fn get_timeline_by_sequence(
        &self,
        dfid: &str,
        sequence: i32,
    ) -> Result<Option<TimelineEntry>, StorageError>;
    fn map_event_to_cid(
        &mut self,
        event_id: &Uuid,
        dfid: &str,
        cid: &str,
        sequence: i32,
    ) -> Result<(), StorageError>;
    fn get_event_first_cid(&self, event_id: &Uuid)
        -> Result<Option<EventCidMapping>, StorageError>;
    fn get_events_in_cid(&self, cid: &str) -> Result<Vec<EventCidMapping>, StorageError>;

    // Blockchain indexing progress operations
    fn update_indexing_progress(
        &mut self,
        network: &str,
        last_ledger: i64,
        confirmed_ledger: i64,
    ) -> Result<(), StorageError>;
    fn get_indexing_progress(
        &self,
        network: &str,
    ) -> Result<Option<IndexingProgress>, StorageError>;
    fn increment_events_indexed(&mut self, network: &str, count: i64) -> Result<(), StorageError>;

    // Circuit Adapter Configuration operations
    fn store_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError>;
    fn get_circuit_adapter_config(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Option<CircuitAdapterConfig>, StorageError>;
    fn update_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError>;
    fn list_circuit_adapter_configs(&self) -> Result<Vec<CircuitAdapterConfig>, StorageError>;

    // User Account operations
    fn store_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError>;
    fn get_user_account(&self, user_id: &str) -> Result<Option<UserAccount>, StorageError>;
    fn get_user_by_username(&self, username: &str) -> Result<Option<UserAccount>, StorageError>;
    fn get_user_by_email(&self, email: &str) -> Result<Option<UserAccount>, StorageError>;
    fn update_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError>;
    fn list_user_accounts(&self) -> Result<Vec<UserAccount>, StorageError>;
    fn delete_user_account(&mut self, user_id: &str) -> Result<(), StorageError>;

    // Credit Transaction operations
    fn record_credit_transaction(
        &mut self,
        transaction: &CreditTransaction,
    ) -> Result<(), StorageError>;
    fn get_credit_transaction(
        &self,
        transaction_id: &str,
    ) -> Result<Option<CreditTransaction>, StorageError>;
    fn get_credit_transactions(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CreditTransaction>, StorageError>;
    fn get_credit_transactions_by_operation(
        &self,
        operation_type: &str,
    ) -> Result<Vec<CreditTransaction>, StorageError>;

    // Admin Action operations
    fn record_admin_action(&mut self, action: &AdminAction) -> Result<(), StorageError>;
    fn get_admin_actions(
        &self,
        admin_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<AdminAction>, StorageError>;
    fn get_admin_actions_by_type(
        &self,
        action_type: &str,
    ) -> Result<Vec<AdminAction>, StorageError>;

    // System Statistics operations
    fn get_system_statistics(&self) -> Result<SystemStatistics, StorageError>;
    fn update_system_statistics(&mut self, stats: &SystemStatistics) -> Result<(), StorageError>;

    // Notification operations
    fn store_notification(&mut self, notification: &Notification) -> Result<(), StorageError>;
    fn get_notification(&self, notification_id: &str)
        -> Result<Option<Notification>, StorageError>;
    fn get_user_notifications(
        &self,
        user_id: &str,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, StorageError>;
    fn update_notification(&mut self, notification: &Notification) -> Result<(), StorageError>;
    fn delete_notification(&mut self, notification_id: &str) -> Result<(), StorageError>;
    fn mark_all_notifications_read(&mut self, user_id: &str) -> Result<usize, StorageError>;
    fn get_unread_notification_count(&self, user_id: &str) -> Result<usize, StorageError>;

    // Adapter Configuration Management operations
    fn store_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError>;
    fn get_adapter_config(&self, config_id: &Uuid) -> Result<Option<AdapterConfig>, StorageError>;
    fn update_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError>;
    fn delete_adapter_config(&mut self, config_id: &Uuid) -> Result<(), StorageError>;
    fn list_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError>;
    fn list_active_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError>;
    fn get_adapter_configs_by_type(
        &self,
        adapter_type: &AdapterType,
    ) -> Result<Vec<AdapterConfig>, StorageError>;
    fn get_default_adapter_config(&self) -> Result<Option<AdapterConfig>, StorageError>;
    fn set_default_adapter(&mut self, config_id: &Uuid) -> Result<(), StorageError>;
    fn store_adapter_test_result(&mut self, result: &AdapterTestResult)
        -> Result<(), StorageError>;
    fn get_adapter_test_result(
        &self,
        config_id: &Uuid,
    ) -> Result<Option<AdapterTestResult>, StorageError>;

    // LID ↔ DFID mapping operations
    fn store_lid_dfid_mapping(&mut self, lid: &Uuid, dfid: &str) -> Result<(), StorageError>;
    fn get_dfid_by_lid(&self, lid: &Uuid) -> Result<Option<String>, StorageError>;

    // Canonical identifier lookups (optimized)
    fn get_dfid_by_canonical(
        &self,
        namespace: &str,
        registry: &str,
        value: &str,
    ) -> Result<Option<String>, StorageError>;

    // Fingerprint mappings
    fn store_fingerprint_mapping(
        &mut self,
        fingerprint: &str,
        dfid: &str,
        circuit_id: &Uuid,
    ) -> Result<(), StorageError>;
    fn get_dfid_by_fingerprint(
        &self,
        fingerprint: &str,
        circuit_id: &Uuid,
    ) -> Result<Option<String>, StorageError>;

    // Enhanced identifier mappings
    fn store_enhanced_identifier_mapping(
        &mut self,
        identifier: &EnhancedIdentifier,
        dfid: &str,
    ) -> Result<(), StorageError>;

    // Webhook delivery operations
    fn store_webhook_delivery(&mut self, delivery: &WebhookDelivery) -> Result<(), StorageError>;
    fn get_webhook_delivery(
        &self,
        delivery_id: &Uuid,
    ) -> Result<Option<WebhookDelivery>, StorageError>;
    fn get_webhook_deliveries_by_circuit(
        &self,
        circuit_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError>;
    fn get_webhook_deliveries_by_webhook(
        &self,
        webhook_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError>;
}

pub struct InMemoryStorage {
    receipts: HashMap<Uuid, Receipt>,
    identifier_index: HashMap<Identifier, Vec<Uuid>>,
    logs: Vec<LogEntry>,
    data_lake_entries: HashMap<Uuid, DataLakeEntry>,
    items: HashMap<String, Item>,
    identifier_mappings: HashMap<Identifier, Vec<IdentifierMapping>>,
    conflicts: HashMap<Uuid, ConflictResolution>,
    events: HashMap<Uuid, Event>,
    circuits: HashMap<Uuid, Circuit>,
    circuit_operations: HashMap<Uuid, CircuitOperation>,
    item_shares: HashMap<String, ItemShare>,
    // New fields for tokenization
    lid_dfid_map: HashMap<Uuid, String>,
    canonical_index: HashMap<String, String>, // "namespace:registry:value" -> dfid
    fingerprint_map: HashMap<(String, Uuid), String>, // (fingerprint, circuit_id) -> dfid
    activities: HashMap<String, Activity>,
    circuit_items: HashMap<(Uuid, String), CircuitItem>,
    pending_items: HashMap<Uuid, PendingItem>,
    audit_events: HashMap<Uuid, AuditEvent>,
    security_incidents: HashMap<Uuid, SecurityIncident>,
    compliance_reports: HashMap<Uuid, ComplianceReport>,
    zk_proofs: HashMap<Uuid, crate::zk_proof_engine::ZkProof>,
    storage_histories: HashMap<String, ItemStorageHistory>,
    circuit_adapter_configs: HashMap<Uuid, CircuitAdapterConfig>,
    user_accounts: HashMap<String, UserAccount>,
    user_accounts_by_username: HashMap<String, String>, // username -> user_id
    user_accounts_by_email: HashMap<String, String>,    // email -> user_id
    credit_transactions: HashMap<String, CreditTransaction>,
    credit_transactions_by_user: HashMap<String, Vec<String>>, // user_id -> transaction_ids
    admin_actions: HashMap<String, AdminAction>,
    #[allow(dead_code)]
    system_statistics: Option<SystemStatistics>,
    notifications: HashMap<String, Notification>,
    notifications_by_user: HashMap<String, Vec<String>>, // user_id -> notification_ids
    adapter_configs: HashMap<Uuid, AdapterConfig>,
    adapter_test_results: HashMap<Uuid, AdapterTestResult>,
    webhook_deliveries: HashMap<Uuid, WebhookDelivery>,
    webhook_deliveries_by_circuit: HashMap<Uuid, Vec<Uuid>>, // circuit_id -> delivery_ids
    webhook_deliveries_by_webhook: HashMap<Uuid, Vec<Uuid>>, // webhook_id -> delivery_ids
    // CID Timeline tracking
    cid_timeline: HashMap<String, Vec<TimelineEntry>>, // dfid -> timeline entries
    event_cid_mappings: HashMap<Uuid, EventCidMapping>, // event_id -> mapping
    indexing_progress: HashMap<String, IndexingProgress>, // network -> progress
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            receipts: HashMap::new(),
            identifier_index: HashMap::new(),
            logs: Vec::new(),
            data_lake_entries: HashMap::new(),
            items: HashMap::new(),
            identifier_mappings: HashMap::new(),
            conflicts: HashMap::new(),
            events: HashMap::new(),
            circuits: HashMap::new(),
            circuit_operations: HashMap::new(),
            item_shares: HashMap::new(),
            lid_dfid_map: HashMap::new(),
            canonical_index: HashMap::new(),
            fingerprint_map: HashMap::new(),
            activities: HashMap::new(),
            circuit_items: HashMap::new(),
            pending_items: HashMap::new(),
            audit_events: HashMap::new(),
            security_incidents: HashMap::new(),
            compliance_reports: HashMap::new(),
            zk_proofs: HashMap::new(),
            storage_histories: HashMap::new(),
            circuit_adapter_configs: HashMap::new(),
            user_accounts: HashMap::new(),
            user_accounts_by_username: HashMap::new(),
            user_accounts_by_email: HashMap::new(),
            credit_transactions: HashMap::new(),
            credit_transactions_by_user: HashMap::new(),
            admin_actions: HashMap::new(),
            system_statistics: None,
            notifications: HashMap::new(),
            notifications_by_user: HashMap::new(),
            adapter_configs: HashMap::new(),
            adapter_test_results: HashMap::new(),
            webhook_deliveries: HashMap::new(),
            webhook_deliveries_by_circuit: HashMap::new(),
            webhook_deliveries_by_webhook: HashMap::new(),
            cid_timeline: HashMap::new(),
            event_cid_mappings: HashMap::new(),
            indexing_progress: HashMap::new(),
        }
    }
}

impl StorageBackend for InMemoryStorage {
    fn store_receipt(&mut self, receipt: &Receipt) -> Result<(), StorageError> {
        for identifier in &receipt.identifiers {
            self.identifier_index
                .entry(identifier.clone())
                .or_default()
                .push(receipt.id);
        }
        self.receipts.insert(receipt.id, receipt.clone());
        Ok(())
    }

    fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError> {
        Ok(self.receipts.get(id).cloned())
    }

    fn find_receipts_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<Receipt>, StorageError> {
        if let Some(receipt_ids) = self.identifier_index.get(identifier) {
            Ok(receipt_ids
                .iter()
                .filter_map(|id| self.receipts.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError> {
        Ok(self.receipts.values().cloned().collect())
    }

    fn store_log(&mut self, log: &LogEntry) -> Result<(), StorageError> {
        self.logs.push(log.clone());
        Ok(())
    }

    fn get_logs(&self) -> Result<Vec<LogEntry>, StorageError> {
        Ok(self.logs.clone())
    }

    // Data Lake operations
    fn store_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        self.data_lake_entries.insert(entry.entry_id, entry.clone());
        Ok(())
    }

    fn get_data_lake_entry(&self, entry_id: &Uuid) -> Result<Option<DataLakeEntry>, StorageError> {
        Ok(self.data_lake_entries.get(entry_id).cloned())
    }

    fn update_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        self.data_lake_entries.insert(entry.entry_id, entry.clone());
        Ok(())
    }

    fn get_data_lake_entries_by_status(
        &self,
        status: ProcessingStatus,
    ) -> Result<Vec<DataLakeEntry>, StorageError> {
        Ok(self
            .data_lake_entries
            .values()
            .filter(|entry| {
                std::mem::discriminant(&entry.status) == std::mem::discriminant(&status)
            })
            .cloned()
            .collect())
    }

    fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError> {
        Ok(self.data_lake_entries.values().cloned().collect())
    }

    // Items operations
    fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.items.insert(item.dfid.clone(), item.clone());
        Ok(())
    }

    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
        Ok(self.items.get(dfid).cloned())
    }

    fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.items.insert(item.dfid.clone(), item.clone());
        Ok(())
    }

    fn list_items(&self) -> Result<Vec<Item>, StorageError> {
        Ok(self.items.values().cloned().collect())
    }

    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
        Ok(self
            .items
            .values()
            .filter(|item| item.identifiers.contains(identifier))
            .cloned()
            .collect())
    }

    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError> {
        Ok(self
            .items
            .values()
            .filter(|item| std::mem::discriminant(&item.status) == std::mem::discriminant(&status))
            .cloned()
            .collect())
    }

    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError> {
        self.items.remove(dfid);
        Ok(())
    }

    // Identifier Mapping operations
    fn store_identifier_mapping(
        &mut self,
        mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        self.identifier_mappings
            .entry(mapping.identifier.clone())
            .or_default()
            .push(mapping.clone());
        Ok(())
    }

    fn get_identifier_mappings(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<IdentifierMapping>, StorageError> {
        Ok(self
            .identifier_mappings
            .get(identifier)
            .cloned()
            .unwrap_or_default())
    }

    fn update_identifier_mapping(
        &mut self,
        mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        if let Some(mappings) = self.identifier_mappings.get_mut(&mapping.identifier) {
            for existing_mapping in mappings.iter_mut() {
                if existing_mapping.dfid == mapping.dfid {
                    *existing_mapping = mapping.clone();
                    return Ok(());
                }
            }
            mappings.push(mapping.clone());
        } else {
            self.identifier_mappings
                .insert(mapping.identifier.clone(), vec![mapping.clone()]);
        }
        Ok(())
    }

    fn list_identifier_mappings(&self) -> Result<Vec<IdentifierMapping>, StorageError> {
        Ok(self
            .identifier_mappings
            .values()
            .flat_map(|mappings| mappings.iter())
            .cloned()
            .collect())
    }

    // Conflict Resolution operations
    fn store_conflict_resolution(
        &mut self,
        conflict: &ConflictResolution,
    ) -> Result<(), StorageError> {
        self.conflicts
            .insert(conflict.conflict_id, conflict.clone());
        Ok(())
    }

    fn get_conflict_resolution(
        &self,
        conflict_id: &Uuid,
    ) -> Result<Option<ConflictResolution>, StorageError> {
        Ok(self.conflicts.get(conflict_id).cloned())
    }

    fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError> {
        Ok(self
            .conflicts
            .values()
            .filter(|conflict| conflict.requires_manual_review)
            .cloned()
            .collect())
    }

    // Event operations
    fn store_event(&mut self, event: &Event) -> Result<(), StorageError> {
        self.events.insert(event.event_id, event.clone());
        Ok(())
    }

    fn get_event(&self, event_id: &Uuid) -> Result<Option<Event>, StorageError> {
        Ok(self.events.get(event_id).cloned())
    }

    fn update_event(&mut self, event: &Event) -> Result<(), StorageError> {
        self.events.insert(event.event_id, event.clone());
        Ok(())
    }

    fn list_events(&self) -> Result<Vec<Event>, StorageError> {
        Ok(self.events.values().cloned().collect())
    }

    fn get_events_by_dfid(&self, dfid: &str) -> Result<Vec<Event>, StorageError> {
        Ok(self
            .events
            .values()
            .filter(|event| event.dfid == dfid)
            .cloned()
            .collect())
    }

    fn get_events_by_type(&self, event_type: EventType) -> Result<Vec<Event>, StorageError> {
        Ok(self
            .events
            .values()
            .filter(|event| {
                std::mem::discriminant(&event.event_type) == std::mem::discriminant(&event_type)
            })
            .cloned()
            .collect())
    }

    fn get_events_by_visibility(
        &self,
        visibility: EventVisibility,
    ) -> Result<Vec<Event>, StorageError> {
        Ok(self
            .events
            .values()
            .filter(|event| {
                std::mem::discriminant(&event.visibility) == std::mem::discriminant(&visibility)
            })
            .cloned()
            .collect())
    }

    fn get_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>, StorageError> {
        Ok(self
            .events
            .values()
            .filter(|event| event.timestamp >= start && event.timestamp <= end)
            .cloned()
            .collect())
    }

    // Circuit operations
    fn store_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        self.circuits.insert(circuit.circuit_id, circuit.clone());

        // Also store the adapter_config if present
        if let Some(ref adapter_config) = circuit.adapter_config {
            self.circuit_adapter_configs
                .insert(circuit.circuit_id, adapter_config.clone());
        }

        Ok(())
    }

    fn get_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, StorageError> {
        Ok(self.circuits.get(circuit_id).cloned())
    }

    fn update_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        self.circuits.insert(circuit.circuit_id, circuit.clone());

        // Also update the adapter_config if present
        if let Some(ref adapter_config) = circuit.adapter_config {
            self.circuit_adapter_configs
                .insert(circuit.circuit_id, adapter_config.clone());
        }

        Ok(())
    }

    fn list_circuits(&self) -> Result<Vec<Circuit>, StorageError> {
        Ok(self.circuits.values().cloned().collect())
    }

    fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, StorageError> {
        Ok(self
            .circuits
            .values()
            .filter(|circuit| circuit.get_member(member_id).is_some())
            .cloned()
            .collect())
    }

    // Circuit Operation operations
    fn store_circuit_operation(
        &mut self,
        operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        self.circuit_operations
            .insert(operation.operation_id, operation.clone());
        Ok(())
    }

    fn get_circuit_operation(
        &self,
        operation_id: &Uuid,
    ) -> Result<Option<CircuitOperation>, StorageError> {
        Ok(self.circuit_operations.get(operation_id).cloned())
    }

    fn update_circuit_operation(
        &mut self,
        operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        self.circuit_operations
            .insert(operation.operation_id, operation.clone());
        Ok(())
    }

    fn get_circuit_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, StorageError> {
        Ok(self
            .circuit_operations
            .values()
            .filter(|operation| operation.circuit_id == *circuit_id)
            .cloned()
            .collect())
    }

    // Item Share operations
    fn store_item_share(&mut self, share: &ItemShare) -> Result<(), StorageError> {
        self.item_shares
            .insert(share.share_id.clone(), share.clone());
        Ok(())
    }

    fn get_item_share(&self, share_id: &str) -> Result<Option<ItemShare>, StorageError> {
        Ok(self.item_shares.get(share_id).cloned())
    }

    fn get_shares_for_user(&self, user_id: &str) -> Result<Vec<ItemShare>, StorageError> {
        Ok(self
            .item_shares
            .values()
            .filter(|share| share.recipient_user_id == user_id)
            .cloned()
            .collect())
    }

    fn get_shares_for_item(&self, dfid: &str) -> Result<Vec<ItemShare>, StorageError> {
        Ok(self
            .item_shares
            .values()
            .filter(|share| share.dfid == dfid)
            .cloned()
            .collect())
    }

    fn is_item_shared_with_user(&self, dfid: &str, user_id: &str) -> Result<bool, StorageError> {
        Ok(self
            .item_shares
            .values()
            .any(|share| share.dfid == dfid && share.recipient_user_id == user_id))
    }

    fn delete_item_share(&mut self, share_id: &str) -> Result<(), StorageError> {
        self.item_shares.remove(share_id);
        Ok(())
    }

    fn store_activity(&mut self, activity: &Activity) -> Result<(), StorageError> {
        self.activities
            .insert(activity.activity_id.clone(), activity.clone());
        Ok(())
    }

    fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, StorageError> {
        Ok(self
            .activities
            .values()
            .filter(|activity| activity.user_id == user_id)
            .cloned()
            .collect())
    }

    fn get_activities_for_circuit(&self, circuit_id: &Uuid) -> Result<Vec<Activity>, StorageError> {
        Ok(self
            .activities
            .values()
            .filter(|activity| activity.circuit_id == *circuit_id)
            .cloned()
            .collect())
    }

    fn get_all_activities(&self) -> Result<Vec<Activity>, StorageError> {
        Ok(self.activities.values().cloned().collect())
    }

    fn store_circuit_item(&mut self, circuit_item: &CircuitItem) -> Result<(), StorageError> {
        let key = (circuit_item.circuit_id, circuit_item.dfid.clone());
        self.circuit_items.insert(key, circuit_item.clone());
        Ok(())
    }

    fn get_circuit_items(&self, circuit_id: &Uuid) -> Result<Vec<CircuitItem>, StorageError> {
        Ok(self
            .circuit_items
            .values()
            .filter(|item| item.circuit_id == *circuit_id)
            .cloned()
            .collect())
    }

    fn remove_circuit_item(&mut self, circuit_id: &Uuid, dfid: &str) -> Result<(), StorageError> {
        let key = (*circuit_id, dfid.to_string());
        self.circuit_items.remove(&key);
        Ok(())
    }

    // Pending Items operations
    fn store_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError> {
        self.pending_items.insert(item.pending_id, item.clone());
        Ok(())
    }

    fn get_pending_item(&self, pending_id: &Uuid) -> Result<Option<PendingItem>, StorageError> {
        Ok(self.pending_items.get(pending_id).cloned())
    }

    fn list_pending_items(&self) -> Result<Vec<PendingItem>, StorageError> {
        Ok(self.pending_items.values().cloned().collect())
    }

    fn get_pending_items_by_reason(
        &self,
        reason_type: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        let items = self
            .pending_items
            .values()
            .filter(|item| match &item.reason {
                PendingReason::NoIdentifiers => reason_type == "NoIdentifiers",
                PendingReason::InvalidIdentifiers(_) => reason_type == "InvalidIdentifiers",
                PendingReason::ConflictingDFIDs { .. } => reason_type == "ConflictingDFIDs",
                PendingReason::IdentifierMappingConflict { .. } => {
                    reason_type == "IdentifierMappingConflict"
                }
                PendingReason::DataQualityIssue { .. } => reason_type == "DataQualityIssue",
                PendingReason::ProcessingError(_) => reason_type == "ProcessingError",
                PendingReason::ValidationError(_) => reason_type == "ValidationError",
                PendingReason::DuplicateDetectionAmbiguous { .. } => {
                    reason_type == "DuplicateDetectionAmbiguous"
                }
                PendingReason::CrossSystemConflict { .. } => reason_type == "CrossSystemConflict",
            })
            .cloned()
            .collect();
        Ok(items)
    }

    fn get_pending_items_by_user(&self, user_id: &str) -> Result<Vec<PendingItem>, StorageError> {
        let items = self
            .pending_items
            .values()
            .filter(|item| item.user_id.as_ref().is_some_and(|id| id == user_id))
            .cloned()
            .collect();
        Ok(items)
    }

    fn get_pending_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        let items = self
            .pending_items
            .values()
            .filter(|item| {
                item.workspace_id
                    .as_ref()
                    .is_some_and(|id| id == workspace_id)
            })
            .cloned()
            .collect();
        Ok(items)
    }

    fn get_pending_items_by_priority(
        &self,
        priority: PendingPriority,
    ) -> Result<Vec<PendingItem>, StorageError> {
        let items = self
            .pending_items
            .values()
            .filter(|item| {
                std::mem::discriminant(&item.priority) == std::mem::discriminant(&priority)
            })
            .cloned()
            .collect();
        Ok(items)
    }

    fn update_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError> {
        self.pending_items.insert(item.pending_id, item.clone());
        Ok(())
    }

    fn delete_pending_item(&mut self, pending_id: &Uuid) -> Result<(), StorageError> {
        self.pending_items.remove(pending_id);
        Ok(())
    }

    fn get_pending_items_requiring_manual_review(&self) -> Result<Vec<PendingItem>, StorageError> {
        let items = self
            .pending_items
            .values()
            .filter(|item| item.manual_review_required)
            .cloned()
            .collect();
        Ok(items)
    }

    // Audit Event operations
    fn store_audit_event(&mut self, event: &AuditEvent) -> Result<(), StorageError> {
        self.audit_events.insert(event.event_id, event.clone());
        Ok(())
    }

    fn get_audit_event(&self, event_id: &Uuid) -> Result<Option<AuditEvent>, StorageError> {
        Ok(self.audit_events.get(event_id).cloned())
    }

    fn query_audit_events(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>, StorageError> {
        let mut events: Vec<AuditEvent> = self.audit_events.values().cloned().collect();

        // Apply filters
        if let Some(user_id) = &query.user_id {
            events.retain(|e| e.user_id == *user_id);
        }

        if let Some(event_types) = &query.event_types {
            events.retain(|e| {
                event_types
                    .iter()
                    .any(|t| std::mem::discriminant(t) == std::mem::discriminant(&e.event_type))
            });
        }

        if let Some(actions) = &query.actions {
            events.retain(|e| actions.contains(&e.action));
        }

        if let Some(resources) = &query.resources {
            events.retain(|e| resources.contains(&e.resource));
        }

        if let Some(outcomes) = &query.outcomes {
            events.retain(|e| {
                outcomes
                    .iter()
                    .any(|o| std::mem::discriminant(o) == std::mem::discriminant(&e.outcome))
            });
        }

        if let Some(severities) = &query.severities {
            events.retain(|e| {
                severities
                    .iter()
                    .any(|s| std::mem::discriminant(s) == std::mem::discriminant(&e.severity))
            });
        }

        if let Some(start_date) = query.start_date {
            events.retain(|e| e.timestamp >= start_date);
        }

        if let Some(end_date) = query.end_date {
            events.retain(|e| e.timestamp <= end_date);
        }

        // Apply sorting
        if let Some(sort_by) = &query.sort_by {
            match sort_by {
                crate::types::AuditSortBy::Timestamp => {
                    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                }
                crate::types::AuditSortBy::Severity => {
                    events.sort_by(|a, b| {
                        let severity_order = |s: &AuditSeverity| match s {
                            AuditSeverity::Low => 0,
                            AuditSeverity::Medium => 1,
                            AuditSeverity::High => 2,
                            AuditSeverity::Critical => 3,
                        };
                        severity_order(&a.severity).cmp(&severity_order(&b.severity))
                    });
                }
                crate::types::AuditSortBy::EventType => {
                    events.sort_by(|a, b| {
                        let type_order = |t: &AuditEventType| match t {
                            AuditEventType::System => 0,
                            AuditEventType::User => 1,
                            AuditEventType::Data => 2,
                            AuditEventType::Access => 3,
                            AuditEventType::Compliance => 4,
                            AuditEventType::Security => 5,
                        };
                        type_order(&a.event_type).cmp(&type_order(&b.event_type))
                    });
                }
            }

            if let Some(sort_order) = &query.sort_order {
                if matches!(sort_order, crate::types::SortOrder::Desc) {
                    events.reverse();
                }
            }
        }

        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(100) as usize;

        events = events.into_iter().skip(offset).take(limit).collect();

        Ok(events)
    }

    fn list_audit_events(&self) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(self.audit_events.values().cloned().collect())
    }

    fn get_audit_events_by_user(&self, user_id: &str) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(self
            .audit_events
            .values()
            .filter(|event| event.user_id == user_id)
            .cloned()
            .collect())
    }

    fn get_audit_events_by_type(
        &self,
        event_type: AuditEventType,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(self
            .audit_events
            .values()
            .filter(|event| {
                std::mem::discriminant(&event.event_type) == std::mem::discriminant(&event_type)
            })
            .cloned()
            .collect())
    }

    fn get_audit_events_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(self
            .audit_events
            .values()
            .filter(|event| {
                std::mem::discriminant(&event.severity) == std::mem::discriminant(&severity)
            })
            .cloned()
            .collect())
    }

    fn get_audit_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(self
            .audit_events
            .values()
            .filter(|event| event.timestamp >= start && event.timestamp <= end)
            .cloned()
            .collect())
    }

    fn sync_audit_events(&mut self, events: Vec<AuditEvent>) -> Result<(), StorageError> {
        for event in events {
            self.audit_events.insert(event.event_id, event);
        }
        Ok(())
    }

    // Security Incident operations
    fn store_security_incident(&mut self, incident: &SecurityIncident) -> Result<(), StorageError> {
        self.security_incidents
            .insert(incident.incident_id, incident.clone());
        Ok(())
    }

    fn get_security_incident(
        &self,
        incident_id: &Uuid,
    ) -> Result<Option<SecurityIncident>, StorageError> {
        Ok(self.security_incidents.get(incident_id).cloned())
    }

    fn update_security_incident(
        &mut self,
        incident: &SecurityIncident,
    ) -> Result<(), StorageError> {
        self.security_incidents
            .insert(incident.incident_id, incident.clone());
        Ok(())
    }

    fn list_security_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(self.security_incidents.values().cloned().collect())
    }

    fn get_incidents_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(self
            .security_incidents
            .values()
            .filter(|incident| {
                std::mem::discriminant(&incident.severity) == std::mem::discriminant(&severity)
            })
            .cloned()
            .collect())
    }

    fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(self
            .security_incidents
            .values()
            .filter(|incident| {
                matches!(
                    incident.status,
                    crate::types::IncidentStatus::Open | crate::types::IncidentStatus::InProgress
                )
            })
            .cloned()
            .collect())
    }

    fn get_incidents_by_assignee(
        &self,
        assignee: &str,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(self
            .security_incidents
            .values()
            .filter(|incident| incident.assigned_to.as_ref().is_some_and(|a| a == assignee))
            .cloned()
            .collect())
    }

    // Compliance Report operations
    fn store_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError> {
        self.compliance_reports
            .insert(report.report_id, report.clone());
        Ok(())
    }

    fn get_compliance_report(
        &self,
        report_id: &Uuid,
    ) -> Result<Option<ComplianceReport>, StorageError> {
        Ok(self.compliance_reports.get(report_id).cloned())
    }

    fn update_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError> {
        self.compliance_reports
            .insert(report.report_id, report.clone());
        Ok(())
    }

    fn list_compliance_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(self.compliance_reports.values().cloned().collect())
    }

    fn get_reports_by_type(
        &self,
        report_type: &str,
    ) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(self
            .compliance_reports
            .values()
            .filter(|report| match &report.report_type {
                crate::types::ComplianceReportType::GdprDataSubject => {
                    report_type == "gdpr-data-subject"
                }
                crate::types::ComplianceReportType::CcpaConsumer => report_type == "ccpa-consumer",
                crate::types::ComplianceReportType::SoxFinancial => report_type == "sox-financial",
                crate::types::ComplianceReportType::AuditTrail => report_type == "audit-trail",
                crate::types::ComplianceReportType::SecurityIncident => {
                    report_type == "security-incident"
                }
                crate::types::ComplianceReportType::FoodSafety => report_type == "food-safety",
                crate::types::ComplianceReportType::GDPR => report_type == "gdpr",
            })
            .cloned()
            .collect())
    }

    fn get_pending_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(self
            .compliance_reports
            .values()
            .filter(|report| {
                matches!(
                    report.status,
                    crate::types::ReportStatus::Pending | crate::types::ReportStatus::Generating
                )
            })
            .cloned()
            .collect())
    }

    // Audit Dashboard operations
    fn get_audit_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, StorageError> {
        use crate::types::*;
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);
        let seven_days_ago = now - chrono::Duration::days(7);

        let total_events = self.audit_events.len() as u64;
        let events_last_24h = self
            .audit_events
            .values()
            .filter(|e| e.timestamp >= twenty_four_hours_ago)
            .count() as u64;
        let events_last_7d = self
            .audit_events
            .values()
            .filter(|e| e.timestamp >= seven_days_ago)
            .count() as u64;

        let open_incidents = self
            .security_incidents
            .values()
            .filter(|i| matches!(i.status, IncidentStatus::Open | IncidentStatus::InProgress))
            .count() as u64;
        let critical_incidents = self
            .security_incidents
            .values()
            .filter(|i| matches!(i.severity, AuditSeverity::Critical))
            .count() as u64;
        let resolved_incidents = self
            .security_incidents
            .values()
            .filter(|i| matches!(i.status, IncidentStatus::Resolved | IncidentStatus::Closed))
            .count() as u64;

        let gdpr_events = self
            .audit_events
            .values()
            .filter(|e| e.compliance.gdpr.unwrap_or(false))
            .count() as u64;
        let ccpa_events = self
            .audit_events
            .values()
            .filter(|e| e.compliance.ccpa.unwrap_or(false))
            .count() as u64;
        let hipaa_events = self
            .audit_events
            .values()
            .filter(|e| e.compliance.hipaa.unwrap_or(false))
            .count() as u64;
        let sox_events = self
            .audit_events
            .values()
            .filter(|e| e.compliance.sox.unwrap_or(false))
            .count() as u64;

        // Calculate user risk profiles
        let mut user_event_counts = HashMap::new();
        for event in self.audit_events.values() {
            *user_event_counts
                .entry(event.user_id.clone())
                .or_insert(0u64) += 1;
        }

        let mut top_users: Vec<UserRiskProfile> = user_event_counts
            .into_iter()
            .map(|(user_id, event_count)| {
                // Simple risk score calculation based on event count and severity
                let risk_score = event_count as f64 * 0.1; // Basic calculation
                UserRiskProfile {
                    user_id,
                    event_count,
                    risk_score,
                }
            })
            .collect();

        top_users.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_users.truncate(10); // Top 10 users

        // Generate some basic anomalies (placeholder implementation)
        let anomalies = vec![SecurityAnomaly {
            anomaly_type: "unusual_access_pattern".to_string(),
            description: "Detected unusual access patterns in last 24 hours".to_string(),
            severity: AuditSeverity::Medium,
            detected_at: now,
        }];

        Ok(AuditDashboardMetrics {
            total_events,
            events_last_24h,
            events_last_7d,
            security_incidents: SecurityIncidentSummary {
                open: open_incidents,
                critical: critical_incidents,
                resolved: resolved_incidents,
            },
            compliance_status: ComplianceStatus {
                gdpr_events,
                ccpa_events,
                hipaa_events,
                sox_events,
            },
            top_users,
            anomalies,
        })
    }

    fn get_event_count_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, StorageError> {
        let count = self
            .audit_events
            .values()
            .filter(|event| event.timestamp >= start && event.timestamp <= end)
            .count() as u64;
        Ok(count)
    }

    // ZK Proof operations
    fn store_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        self.zk_proofs.insert(proof.proof_id, proof.clone());
        Ok(())
    }

    fn get_zk_proof(
        &self,
        proof_id: &Uuid,
    ) -> Result<Option<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(self.zk_proofs.get(proof_id).cloned())
    }

    fn update_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        self.zk_proofs.insert(proof.proof_id, proof.clone());
        Ok(())
    }

    fn query_zk_proofs(
        &self,
        query: &crate::api::zk_proofs::ZkProofQuery,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        let mut proofs: Vec<crate::zk_proof_engine::ZkProof> =
            self.zk_proofs.values().cloned().collect();

        // Apply filters
        if let Some(prover_id) = &query.prover_id {
            proofs.retain(|p| p.prover_id == *prover_id);
        }

        if let Some(circuit_types) = &query.circuit_types {
            proofs.retain(|p| {
                circuit_types
                    .iter()
                    .any(|t| std::mem::discriminant(t) == std::mem::discriminant(&p.circuit_type))
            });
        }

        if let Some(statuses) = &query.statuses {
            proofs.retain(|p| {
                statuses
                    .iter()
                    .any(|s| std::mem::discriminant(s) == std::mem::discriminant(&p.status))
            });
        }

        if let Some(start_date) = query.start_date {
            proofs.retain(|p| p.created_at >= start_date);
        }

        if let Some(end_date) = query.end_date {
            proofs.retain(|p| p.created_at <= end_date);
        }

        // Apply sorting by creation date (default)
        proofs.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(100) as usize;

        proofs = proofs.into_iter().skip(offset).take(limit).collect();

        Ok(proofs)
    }

    fn list_zk_proofs(&self) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(self.zk_proofs.values().cloned().collect())
    }

    fn get_zk_proofs_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(self
            .zk_proofs
            .values()
            .filter(|proof| proof.prover_id == user_id)
            .cloned()
            .collect())
    }

    fn get_zk_proofs_by_circuit_type(
        &self,
        circuit_type: CircuitType,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(self
            .zk_proofs
            .values()
            .filter(|proof| {
                std::mem::discriminant(&proof.circuit_type) == std::mem::discriminant(&circuit_type)
            })
            .cloned()
            .collect())
    }

    fn get_zk_proofs_by_status(
        &self,
        status: crate::zk_proof_engine::ProofStatus,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(self
            .zk_proofs
            .values()
            .filter(|proof| {
                std::mem::discriminant(&proof.status) == std::mem::discriminant(&status)
            })
            .cloned()
            .collect())
    }

    fn get_zk_proof_statistics(
        &self,
    ) -> Result<crate::api::zk_proofs::ZkProofStatistics, StorageError> {
        let total_proofs = self.zk_proofs.len() as u64;
        let verified_proofs = self
            .zk_proofs
            .values()
            .filter(|p| matches!(p.status, crate::zk_proof_engine::ProofStatus::Verified))
            .count() as u64;
        let failed_proofs = self
            .zk_proofs
            .values()
            .filter(|p| matches!(p.status, crate::zk_proof_engine::ProofStatus::Failed))
            .count() as u64;
        let pending_proofs = self
            .zk_proofs
            .values()
            .filter(|p| matches!(p.status, crate::zk_proof_engine::ProofStatus::Pending))
            .count() as u64;

        // Count by circuit type
        let mut proof_types = std::collections::HashMap::new();
        for proof in self.zk_proofs.values() {
            let type_name = match &proof.circuit_type {
                CircuitType::OrganicCertification => "organic_certification",
                CircuitType::PesticideThreshold => "pesticide_threshold",
                CircuitType::QualityGrade => "quality_grade",
                CircuitType::OwnershipProof => "ownership_proof",
                CircuitType::TimestampFreshness => "timestamp_freshness",
                CircuitType::Custom(name) => name,
            };
            *proof_types.entry(type_name.to_string()).or_insert(0u64) += 1;
        }

        Ok(crate::api::zk_proofs::ZkProofStatistics {
            total_proofs,
            verified_proofs,
            failed_proofs,
            pending_proofs,
            proof_types,
        })
    }

    fn delete_zk_proof(&mut self, proof_id: &Uuid) -> Result<(), StorageError> {
        self.zk_proofs.remove(proof_id);
        Ok(())
    }

    fn store_storage_history(&mut self, history: &ItemStorageHistory) -> Result<(), StorageError> {
        self.storage_histories
            .insert(history.dfid.clone(), history.clone());
        Ok(())
    }

    fn get_storage_history(&self, dfid: &str) -> Result<Option<ItemStorageHistory>, StorageError> {
        Ok(self.storage_histories.get(dfid).cloned())
    }

    fn add_storage_record(
        &mut self,
        dfid: &str,
        record: StorageRecord,
    ) -> Result<(), StorageError> {
        if let Some(history) = self.storage_histories.get_mut(dfid) {
            history.storage_records.push(record);
            history.updated_at = chrono::Utc::now();
        } else {
            let history = ItemStorageHistory {
                dfid: dfid.to_string(),
                storage_records: vec![record],
                current_primary: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.storage_histories.insert(dfid.to_string(), history);
        }
        Ok(())
    }

    // CID Timeline operations - real implementations using HashMaps
    fn add_cid_to_timeline(
        &mut self,
        dfid: &str,
        cid: &str,
        ipcm_tx: &str,
        timestamp: i64,
        network: &str,
    ) -> Result<(), StorageError> {
        let timeline = self
            .cid_timeline
            .entry(dfid.to_string())
            .or_insert_with(Vec::new);

        // Auto-increment sequence
        let event_sequence = timeline.len() as i32 + 1;

        let entry = TimelineEntry {
            id: Uuid::new_v4(),
            dfid: dfid.to_string(),
            cid: cid.to_string(),
            event_sequence,
            blockchain_timestamp: timestamp,
            ipcm_transaction_hash: ipcm_tx.to_string(),
            network: network.to_string(),
            created_at: Utc::now(),
        };

        timeline.push(entry);
        Ok(())
    }

    fn get_item_timeline(&self, dfid: &str) -> Result<Vec<TimelineEntry>, StorageError> {
        Ok(self.cid_timeline.get(dfid).cloned().unwrap_or_default())
    }

    fn get_timeline_by_sequence(
        &self,
        dfid: &str,
        sequence: i32,
    ) -> Result<Option<TimelineEntry>, StorageError> {
        if let Some(timeline) = self.cid_timeline.get(dfid) {
            Ok(timeline
                .iter()
                .find(|entry| entry.event_sequence == sequence)
                .cloned())
        } else {
            Ok(None)
        }
    }

    fn map_event_to_cid(
        &mut self,
        event_id: &Uuid,
        dfid: &str,
        cid: &str,
        sequence: i32,
    ) -> Result<(), StorageError> {
        let mapping = EventCidMapping {
            id: Uuid::new_v4(),
            event_id: *event_id,
            dfid: dfid.to_string(),
            first_cid: cid.to_string(),
            appeared_in_sequence: sequence,
            created_at: Utc::now(),
        };

        self.event_cid_mappings.insert(*event_id, mapping);
        Ok(())
    }

    fn get_event_first_cid(
        &self,
        event_id: &Uuid,
    ) -> Result<Option<EventCidMapping>, StorageError> {
        Ok(self.event_cid_mappings.get(event_id).cloned())
    }

    fn get_events_in_cid(&self, cid: &str) -> Result<Vec<EventCidMapping>, StorageError> {
        Ok(self
            .event_cid_mappings
            .values()
            .filter(|mapping| mapping.first_cid == cid)
            .cloned()
            .collect())
    }

    fn update_indexing_progress(
        &mut self,
        network: &str,
        last_ledger: i64,
        confirmed_ledger: i64,
    ) -> Result<(), StorageError> {
        let progress = self
            .indexing_progress
            .entry(network.to_string())
            .or_insert_with(|| IndexingProgress {
                network: network.to_string(),
                last_indexed_ledger: 0,
                last_confirmed_ledger: 0,
                last_indexed_at: Utc::now(),
                status: "active".to_string(),
                error_message: None,
                total_events_indexed: 0,
                last_error_at: None,
            });

        progress.last_indexed_ledger = last_ledger;
        progress.last_confirmed_ledger = confirmed_ledger;
        progress.last_indexed_at = Utc::now();
        Ok(())
    }

    fn get_indexing_progress(
        &self,
        network: &str,
    ) -> Result<Option<IndexingProgress>, StorageError> {
        Ok(self.indexing_progress.get(network).cloned())
    }

    fn increment_events_indexed(&mut self, network: &str, count: i64) -> Result<(), StorageError> {
        if let Some(progress) = self.indexing_progress.get_mut(network) {
            progress.total_events_indexed += count;
        }
        Ok(())
    }

    fn store_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        self.circuit_adapter_configs
            .insert(config.circuit_id, config.clone());
        Ok(())
    }

    fn get_circuit_adapter_config(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Option<CircuitAdapterConfig>, StorageError> {
        Ok(self.circuit_adapter_configs.get(circuit_id).cloned())
    }

    fn update_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        self.circuit_adapter_configs
            .insert(config.circuit_id, config.clone());
        Ok(())
    }

    fn list_circuit_adapter_configs(&self) -> Result<Vec<CircuitAdapterConfig>, StorageError> {
        Ok(self.circuit_adapter_configs.values().cloned().collect())
    }

    // User Account operations
    fn store_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError> {
        // Check for duplicate username
        if self.user_accounts_by_username.contains_key(&user.username) {
            return Err(StorageError::AlreadyExists(format!(
                "Username '{}' already exists",
                user.username
            )));
        }

        // Check for duplicate email
        if self.user_accounts_by_email.contains_key(&user.email) {
            return Err(StorageError::AlreadyExists(format!(
                "Email '{}' already exists",
                user.email
            )));
        }

        // Store user account
        self.user_accounts
            .insert(user.user_id.clone(), user.clone());

        // Update indices
        self.user_accounts_by_username
            .insert(user.username.clone(), user.user_id.clone());
        self.user_accounts_by_email
            .insert(user.email.clone(), user.user_id.clone());

        Ok(())
    }

    fn get_user_account(&self, user_id: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(self.user_accounts.get(user_id).cloned())
    }

    fn get_user_by_username(&self, username: &str) -> Result<Option<UserAccount>, StorageError> {
        if let Some(user_id) = self.user_accounts_by_username.get(username) {
            Ok(self.user_accounts.get(user_id).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_user_by_email(&self, email: &str) -> Result<Option<UserAccount>, StorageError> {
        if let Some(user_id) = self.user_accounts_by_email.get(email) {
            Ok(self.user_accounts.get(user_id).cloned())
        } else {
            Ok(None)
        }
    }

    fn update_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError> {
        // Check if user exists
        if !self.user_accounts.contains_key(&user.user_id) {
            return Err(StorageError::NotFound);
        }

        // Get old user to update indices if username/email changed
        if let Some(old_user) = self.user_accounts.get(&user.user_id) {
            // Update username index if changed
            if old_user.username != user.username {
                self.user_accounts_by_username.remove(&old_user.username);

                // Check new username isn't taken
                if self.user_accounts_by_username.contains_key(&user.username) {
                    return Err(StorageError::AlreadyExists(format!(
                        "Username '{}' already exists",
                        user.username
                    )));
                }

                self.user_accounts_by_username
                    .insert(user.username.clone(), user.user_id.clone());
            }

            // Update email index if changed
            if old_user.email != user.email {
                self.user_accounts_by_email.remove(&old_user.email);

                // Check new email isn't taken
                if self.user_accounts_by_email.contains_key(&user.email) {
                    return Err(StorageError::AlreadyExists(format!(
                        "Email '{}' already exists",
                        user.email
                    )));
                }

                self.user_accounts_by_email
                    .insert(user.email.clone(), user.user_id.clone());
            }
        }

        // Update user account
        self.user_accounts
            .insert(user.user_id.clone(), user.clone());
        Ok(())
    }

    fn list_user_accounts(&self) -> Result<Vec<UserAccount>, StorageError> {
        Ok(self.user_accounts.values().cloned().collect())
    }

    fn delete_user_account(&mut self, user_id: &str) -> Result<(), StorageError> {
        if let Some(user) = self.user_accounts.remove(user_id) {
            // Remove from indices
            self.user_accounts_by_username.remove(&user.username);
            self.user_accounts_by_email.remove(&user.email);
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    // Credit Transaction operations
    fn record_credit_transaction(
        &mut self,
        transaction: &CreditTransaction,
    ) -> Result<(), StorageError> {
        // Store the transaction
        self.credit_transactions
            .insert(transaction.transaction_id.clone(), transaction.clone());

        // Add to user's transaction list
        self.credit_transactions_by_user
            .entry(transaction.user_id.clone())
            .or_default()
            .push(transaction.transaction_id.clone());

        Ok(())
    }

    fn get_credit_transaction(
        &self,
        transaction_id: &str,
    ) -> Result<Option<CreditTransaction>, StorageError> {
        Ok(self.credit_transactions.get(transaction_id).cloned())
    }

    fn get_credit_transactions(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        let transaction_ids = self
            .credit_transactions_by_user
            .get(user_id)
            .cloned()
            .unwrap_or_default();

        let mut transactions: Vec<CreditTransaction> = transaction_ids
            .iter()
            .filter_map(|id| self.credit_transactions.get(id))
            .cloned()
            .collect();

        // Sort by timestamp (newest first)
        transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if specified
        if let Some(limit) = limit {
            transactions.truncate(limit);
        }

        Ok(transactions)
    }

    fn get_credit_transactions_by_operation(
        &self,
        operation_type: &str,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        Ok(self
            .credit_transactions
            .values()
            .filter(|t| t.operation_type.as_deref() == Some(operation_type))
            .cloned()
            .collect())
    }

    // Admin Action operations
    fn record_admin_action(&mut self, action: &AdminAction) -> Result<(), StorageError> {
        self.admin_actions
            .insert(action.action_id.clone(), action.clone());
        Ok(())
    }

    fn get_admin_actions(
        &self,
        admin_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<AdminAction>, StorageError> {
        let mut actions: Vec<AdminAction> = if let Some(admin_id) = admin_id {
            self.admin_actions
                .values()
                .filter(|a| a.admin_user_id == admin_id)
                .cloned()
                .collect()
        } else {
            self.admin_actions.values().cloned().collect()
        };

        // Sort by timestamp (newest first)
        actions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if specified
        if let Some(limit) = limit {
            actions.truncate(limit);
        }

        Ok(actions)
    }

    fn get_admin_actions_by_type(
        &self,
        action_type: &str,
    ) -> Result<Vec<AdminAction>, StorageError> {
        Ok(self
            .admin_actions
            .values()
            .filter(|a| format!("{:?}", a.action_type) == action_type)
            .cloned()
            .collect())
    }

    // System Statistics operations (stub implementations for now)
    fn get_system_statistics(&self) -> Result<SystemStatistics, StorageError> {
        Ok(SystemStatistics {
            total_users: 0,
            active_users_24h: 0,
            active_users_30d: 0,
            total_items: 0,
            total_circuits: 0,
            total_storage_operations: 0,
            credits_consumed_24h: 0,
            tier_distribution: HashMap::new(),
            adapter_usage_stats: HashMap::new(),
            generated_at: Utc::now(),
        })
    }

    fn update_system_statistics(&mut self, _stats: &SystemStatistics) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "System statistics operations not yet implemented".to_string(),
        ))
    }

    // Notification operations
    fn store_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        self.notifications
            .insert(notification.id.clone(), notification.clone());
        self.notifications_by_user
            .entry(notification.user_id.clone())
            .or_default()
            .push(notification.id.clone());
        Ok(())
    }

    fn get_notification(
        &self,
        notification_id: &str,
    ) -> Result<Option<Notification>, StorageError> {
        Ok(self.notifications.get(notification_id).cloned())
    }

    fn get_user_notifications(
        &self,
        user_id: &str,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, StorageError> {
        let notification_ids = self.notifications_by_user.get(user_id);

        let empty_vec = Vec::new();
        let mut notifications: Vec<Notification> = notification_ids
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|id| self.notifications.get(id).cloned())
            .filter(|n| {
                // Filter by timestamp if provided
                if let Some(since_time) = since {
                    if n.timestamp <= since_time {
                        return false;
                    }
                }

                // Filter by read status if unread_only
                if unread_only && n.read {
                    return false;
                }

                true
            })
            .collect();

        // Sort by timestamp descending (newest first)
        notifications.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if provided
        if let Some(limit_count) = limit {
            notifications.truncate(limit_count);
        }

        Ok(notifications)
    }

    fn update_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        if self.notifications.contains_key(&notification.id) {
            self.notifications
                .insert(notification.id.clone(), notification.clone());
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    fn delete_notification(&mut self, notification_id: &str) -> Result<(), StorageError> {
        if let Some(notification) = self.notifications.remove(notification_id) {
            // Also remove from user's notification list
            if let Some(user_notifications) =
                self.notifications_by_user.get_mut(&notification.user_id)
            {
                user_notifications.retain(|id| id != notification_id);
            }
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    fn mark_all_notifications_read(&mut self, user_id: &str) -> Result<usize, StorageError> {
        let mut count = 0;

        if let Some(notification_ids) = self.notifications_by_user.get(user_id) {
            for id in notification_ids {
                if let Some(notification) = self.notifications.get_mut(id) {
                    if !notification.read {
                        notification.read = true;
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    fn get_unread_notification_count(&self, user_id: &str) -> Result<usize, StorageError> {
        let count = self
            .notifications_by_user
            .get(user_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.notifications.get(id))
                    .filter(|n| !n.read)
                    .count()
            })
            .unwrap_or(0);

        Ok(count)
    }

    // Adapter Configuration Management operations
    fn store_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        self.adapter_configs
            .insert(config.config_id, config.clone());
        Ok(())
    }

    fn get_adapter_config(&self, config_id: &Uuid) -> Result<Option<AdapterConfig>, StorageError> {
        Ok(self.adapter_configs.get(config_id).cloned())
    }

    fn update_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        let mut updated_config = config.clone();
        updated_config.updated_at = Utc::now();
        self.adapter_configs
            .insert(updated_config.config_id, updated_config);
        Ok(())
    }

    fn delete_adapter_config(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        self.adapter_configs.remove(config_id);
        self.adapter_test_results.remove(config_id);
        Ok(())
    }

    fn list_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        Ok(self.adapter_configs.values().cloned().collect())
    }

    fn list_active_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        Ok(self
            .adapter_configs
            .values()
            .filter(|c| c.is_active)
            .cloned()
            .collect())
    }

    fn get_adapter_configs_by_type(
        &self,
        adapter_type: &AdapterType,
    ) -> Result<Vec<AdapterConfig>, StorageError> {
        Ok(self
            .adapter_configs
            .values()
            .filter(|c| {
                std::mem::discriminant(&c.adapter_type) == std::mem::discriminant(adapter_type)
            })
            .cloned()
            .collect())
    }

    fn get_default_adapter_config(&self) -> Result<Option<AdapterConfig>, StorageError> {
        Ok(self
            .adapter_configs
            .values()
            .find(|c| c.is_default)
            .cloned())
    }

    fn set_default_adapter(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        // Unset all defaults first
        for config in self.adapter_configs.values_mut() {
            config.is_default = false;
        }

        // Set the new default
        if let Some(config) = self.adapter_configs.get_mut(config_id) {
            config.is_default = true;
            config.updated_at = Utc::now();
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    fn store_adapter_test_result(
        &mut self,
        result: &AdapterTestResult,
    ) -> Result<(), StorageError> {
        // Update the adapter config's last_tested_at and test_status
        if let Some(config) = self.adapter_configs.get_mut(&result.config_id) {
            config.last_tested_at = Some(result.tested_at);
            config.test_status = Some(result.status.clone());
            config.updated_at = Utc::now();
        }

        self.adapter_test_results
            .insert(result.config_id, result.clone());
        Ok(())
    }

    fn get_adapter_test_result(
        &self,
        config_id: &Uuid,
    ) -> Result<Option<AdapterTestResult>, StorageError> {
        Ok(self.adapter_test_results.get(config_id).cloned())
    }

    // LID ↔ DFID mapping operations
    fn store_lid_dfid_mapping(&mut self, lid: &Uuid, dfid: &str) -> Result<(), StorageError> {
        self.lid_dfid_map.insert(*lid, dfid.to_string());
        Ok(())
    }

    fn get_dfid_by_lid(&self, lid: &Uuid) -> Result<Option<String>, StorageError> {
        Ok(self.lid_dfid_map.get(lid).cloned())
    }

    // Canonical identifier lookups (optimized)
    fn get_dfid_by_canonical(
        &self,
        namespace: &str,
        registry: &str,
        value: &str,
    ) -> Result<Option<String>, StorageError> {
        let key = format!("{namespace}:{registry}:{value}");
        Ok(self.canonical_index.get(&key).cloned())
    }

    // Fingerprint mappings
    fn store_fingerprint_mapping(
        &mut self,
        fingerprint: &str,
        dfid: &str,
        circuit_id: &Uuid,
    ) -> Result<(), StorageError> {
        self.fingerprint_map
            .insert((fingerprint.to_string(), *circuit_id), dfid.to_string());
        Ok(())
    }

    fn get_dfid_by_fingerprint(
        &self,
        fingerprint: &str,
        circuit_id: &Uuid,
    ) -> Result<Option<String>, StorageError> {
        Ok(self
            .fingerprint_map
            .get(&(fingerprint.to_string(), *circuit_id))
            .cloned())
    }

    // Enhanced identifier mappings
    fn store_enhanced_identifier_mapping(
        &mut self,
        identifier: &EnhancedIdentifier,
        dfid: &str,
    ) -> Result<(), StorageError> {
        if identifier.is_canonical() {
            let key = identifier.unique_key();
            self.canonical_index.insert(key, dfid.to_string());
        }
        Ok(())
    }

    fn store_webhook_delivery(&mut self, delivery: &WebhookDelivery) -> Result<(), StorageError> {
        // Store delivery
        self.webhook_deliveries
            .insert(delivery.id, delivery.clone());

        // Index by circuit
        self.webhook_deliveries_by_circuit
            .entry(delivery.circuit_id)
            .or_default()
            .push(delivery.id);

        // Index by webhook
        self.webhook_deliveries_by_webhook
            .entry(delivery.webhook_id)
            .or_default()
            .push(delivery.id);

        Ok(())
    }

    fn get_webhook_delivery(
        &self,
        delivery_id: &Uuid,
    ) -> Result<Option<WebhookDelivery>, StorageError> {
        Ok(self.webhook_deliveries.get(delivery_id).cloned())
    }

    fn get_webhook_deliveries_by_circuit(
        &self,
        circuit_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        let delivery_ids = self.webhook_deliveries_by_circuit.get(circuit_id);

        if let Some(ids) = delivery_ids {
            let mut deliveries: Vec<WebhookDelivery> = ids
                .iter()
                .filter_map(|id| self.webhook_deliveries.get(id).cloned())
                .collect();

            // Sort by created_at descending (most recent first)
            deliveries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            if let Some(limit) = limit {
                deliveries.truncate(limit);
            }

            Ok(deliveries)
        } else {
            Ok(vec![])
        }
    }

    fn get_webhook_deliveries_by_webhook(
        &self,
        webhook_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        let delivery_ids = self.webhook_deliveries_by_webhook.get(webhook_id);

        if let Some(ids) = delivery_ids {
            let mut deliveries: Vec<WebhookDelivery> = ids
                .iter()
                .filter_map(|id| self.webhook_deliveries.get(id).cloned())
                .collect();

            // Sort by created_at descending (most recent first)
            deliveries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            if let Some(limit) = limit {
                deliveries.truncate(limit);
            }

            Ok(deliveries)
        } else {
            Ok(vec![])
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EncryptedFileStorage {
    base_path: String,
    encryption_key: Option<EncryptionKey>,
}

impl EncryptedFileStorage {
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            encryption_key: None,
        }
    }

    pub fn with_encryption(mut self, key: EncryptionKey) -> Self {
        self.encryption_key = Some(key);
        self
    }

    fn encrypt_data(&self, data: &[u8]) -> Result<EncryptedData, StorageError> {
        if let Some(key) = &self.encryption_key {
            let cipher = Aes256Gcm::new(key.as_aes_key());
            let mut nonce_bytes = [0u8; 12];
            OsRng.fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher
                .encrypt(nonce, data)
                .map_err(|e| StorageError::EncryptionError(format!("Encryption failed: {e}")))?;

            Ok(EncryptedData {
                data: ciphertext,
                nonce: nonce_bytes,
            })
        } else {
            Ok(EncryptedData {
                data: data.to_vec(),
                nonce: [0u8; 12],
            })
        }
    }

    fn decrypt_data(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, StorageError> {
        if let Some(key) = &self.encryption_key {
            let cipher = Aes256Gcm::new(key.as_aes_key());
            let nonce = Nonce::from_slice(&encrypted.nonce);

            cipher
                .decrypt(nonce, encrypted.data.as_ref())
                .map_err(|e| StorageError::EncryptionError(format!("Decryption failed: {e}")))
        } else {
            Ok(encrypted.data.clone())
        }
    }

    fn ensure_directory(&self, path: &str) -> Result<(), StorageError> {
        let dir_path = Path::new(&self.base_path).join(path);
        fs::create_dir_all(dir_path)?;
        Ok(())
    }
}

impl StorageBackend for EncryptedFileStorage {
    fn store_receipt(&mut self, receipt: &Receipt) -> Result<(), StorageError> {
        self.ensure_directory("receipts")?;

        let serialized = serde_json::to_vec(receipt)?;
        let encrypted = self.encrypt_data(&serialized)?;
        let encrypted_json = serde_json::to_vec(&encrypted)?;

        let file_path = Path::new(&self.base_path)
            .join("receipts")
            .join(format!("{}.json", receipt.id));

        fs::write(file_path, encrypted_json)?;
        Ok(())
    }

    fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError> {
        let file_path = Path::new(&self.base_path)
            .join("receipts")
            .join(format!("{id}.json"));

        if !file_path.exists() {
            return Ok(None);
        }

        let encrypted_json = fs::read(file_path)?;
        let encrypted: EncryptedData = serde_json::from_slice(&encrypted_json)?;
        let decrypted = self.decrypt_data(&encrypted)?;
        let receipt: Receipt = serde_json::from_slice(&decrypted)?;

        Ok(Some(receipt))
    }

    fn find_receipts_by_identifier(
        &self,
        _identifier: &Identifier,
    ) -> Result<Vec<Receipt>, StorageError> {
        let receipts = self.list_receipts()?;
        Ok(receipts
            .into_iter()
            .filter(|receipt| receipt.identifiers.contains(_identifier))
            .collect())
    }

    fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError> {
        let receipts_dir = Path::new(&self.base_path).join("receipts");
        if !receipts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut receipts = Vec::new();
        for entry in fs::read_dir(receipts_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let encrypted_json = fs::read(entry.path())?;
                let encrypted: EncryptedData = serde_json::from_slice(&encrypted_json)?;
                let decrypted = self.decrypt_data(&encrypted)?;
                let receipt: Receipt = serde_json::from_slice(&decrypted)?;
                receipts.push(receipt);
            }
        }

        Ok(receipts)
    }

    fn store_log(&mut self, log: &LogEntry) -> Result<(), StorageError> {
        self.ensure_directory("logs")?;

        let serialized = serde_json::to_vec(log)?;
        let encrypted = self.encrypt_data(&serialized)?;
        let encrypted_json = serde_json::to_vec(&encrypted)?;

        let file_path = Path::new(&self.base_path)
            .join("logs")
            .join(format!("{}.json", log.id));

        fs::write(file_path, encrypted_json)?;
        Ok(())
    }

    fn get_logs(&self) -> Result<Vec<LogEntry>, StorageError> {
        let logs_dir = Path::new(&self.base_path).join("logs");
        if !logs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut logs = Vec::new();
        for entry in fs::read_dir(logs_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                let encrypted_json = fs::read(entry.path())?;
                let encrypted: EncryptedData = serde_json::from_slice(&encrypted_json)?;
                let decrypted = self.decrypt_data(&encrypted)?;
                let log: LogEntry = serde_json::from_slice(&decrypted)?;
                logs.push(log);
            }
        }

        logs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(logs)
    }

    // For simplicity, EncryptedFileStorage will delegate to an internal InMemoryStorage for new data types
    // In a real implementation, these would be properly encrypted and stored to files

    fn store_data_lake_entry(&mut self, _entry: &DataLakeEntry) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Data lake operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_data_lake_entry(&self, _entry_id: &Uuid) -> Result<Option<DataLakeEntry>, StorageError> {
        Ok(None)
    }

    fn update_data_lake_entry(&mut self, _entry: &DataLakeEntry) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Data lake operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_data_lake_entries_by_status(
        &self,
        _status: ProcessingStatus,
    ) -> Result<Vec<DataLakeEntry>, StorageError> {
        Ok(Vec::new())
    }

    fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError> {
        Ok(Vec::new())
    }

    fn store_item(&mut self, _item: &Item) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Item operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_item_by_dfid(&self, _dfid: &str) -> Result<Option<Item>, StorageError> {
        Ok(None)
    }

    fn update_item(&mut self, _item: &Item) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Item operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn list_items(&self) -> Result<Vec<Item>, StorageError> {
        Ok(Vec::new())
    }

    fn find_items_by_identifier(
        &self,
        _identifier: &Identifier,
    ) -> Result<Vec<Item>, StorageError> {
        Ok(Vec::new())
    }

    fn find_items_by_status(&self, _status: ItemStatus) -> Result<Vec<Item>, StorageError> {
        Ok(Vec::new())
    }

    fn delete_item(&mut self, _dfid: &str) -> Result<(), StorageError> {
        Ok(())
    }

    fn store_identifier_mapping(
        &mut self,
        _mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Identifier mapping operations not yet implemented for EncryptedFileStorage"
                .to_string(),
        ))
    }

    fn get_identifier_mappings(
        &self,
        _identifier: &Identifier,
    ) -> Result<Vec<IdentifierMapping>, StorageError> {
        Ok(Vec::new())
    }

    fn update_identifier_mapping(
        &mut self,
        _mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Identifier mapping operations not yet implemented for EncryptedFileStorage"
                .to_string(),
        ))
    }

    fn list_identifier_mappings(&self) -> Result<Vec<IdentifierMapping>, StorageError> {
        Ok(Vec::new())
    }

    fn store_conflict_resolution(
        &mut self,
        _conflict: &ConflictResolution,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Conflict resolution operations not yet implemented for EncryptedFileStorage"
                .to_string(),
        ))
    }

    fn get_conflict_resolution(
        &self,
        _conflict_id: &Uuid,
    ) -> Result<Option<ConflictResolution>, StorageError> {
        Ok(None)
    }

    fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError> {
        Ok(Vec::new())
    }

    // Event operations - placeholder implementations
    fn store_event(&mut self, _event: &Event) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Event operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_event(&self, _event_id: &Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }

    fn update_event(&mut self, _event: &Event) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Event operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn list_events(&self) -> Result<Vec<Event>, StorageError> {
        Ok(Vec::new())
    }

    fn get_events_by_dfid(&self, _dfid: &str) -> Result<Vec<Event>, StorageError> {
        Ok(Vec::new())
    }

    fn get_events_by_type(&self, _event_type: EventType) -> Result<Vec<Event>, StorageError> {
        Ok(Vec::new())
    }

    fn get_events_by_visibility(
        &self,
        _visibility: EventVisibility,
    ) -> Result<Vec<Event>, StorageError> {
        Ok(Vec::new())
    }

    fn get_events_in_time_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Event>, StorageError> {
        Ok(Vec::new())
    }

    // Circuit operations - placeholder implementations
    fn store_circuit(&mut self, _circuit: &Circuit) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Circuit operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_circuit(&self, _circuit_id: &Uuid) -> Result<Option<Circuit>, StorageError> {
        Ok(None)
    }

    fn update_circuit(&mut self, _circuit: &Circuit) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Circuit operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn list_circuits(&self) -> Result<Vec<Circuit>, StorageError> {
        Ok(Vec::new())
    }

    fn get_circuits_for_member(&self, _member_id: &str) -> Result<Vec<Circuit>, StorageError> {
        Ok(Vec::new())
    }

    // Circuit Operation operations - placeholder implementations
    fn store_circuit_operation(
        &mut self,
        _operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Circuit operation operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_circuit_operation(
        &self,
        _operation_id: &Uuid,
    ) -> Result<Option<CircuitOperation>, StorageError> {
        Ok(None)
    }

    fn update_circuit_operation(
        &mut self,
        _operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Circuit operation operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_circuit_operations(
        &self,
        _circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, StorageError> {
        Ok(Vec::new())
    }

    // Item Share operations - Not implemented for EncryptedFileStorage yet
    fn store_item_share(&mut self, _share: &ItemShare) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Item share operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
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

    fn store_activity(&mut self, _activity: &Activity) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_activities_for_user(&self, _user_id: &str) -> Result<Vec<Activity>, StorageError> {
        Ok(vec![])
    }

    fn get_activities_for_circuit(
        &self,
        _circuit_id: &Uuid,
    ) -> Result<Vec<Activity>, StorageError> {
        Ok(vec![])
    }

    fn get_all_activities(&self) -> Result<Vec<Activity>, StorageError> {
        Ok(vec![])
    }

    fn store_circuit_item(&mut self, _circuit_item: &CircuitItem) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_circuit_items(&self, _circuit_id: &Uuid) -> Result<Vec<CircuitItem>, StorageError> {
        Ok(vec![])
    }

    fn remove_circuit_item(&mut self, _circuit_id: &Uuid, _dfid: &str) -> Result<(), StorageError> {
        Ok(())
    }

    // Pending Items operations - placeholder implementations
    fn store_pending_item(&mut self, _item: &PendingItem) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Pending item operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_pending_item(&self, _pending_id: &Uuid) -> Result<Option<PendingItem>, StorageError> {
        Ok(None)
    }

    fn list_pending_items(&self) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    fn get_pending_items_by_reason(
        &self,
        _reason_type: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    fn get_pending_items_by_user(&self, _user_id: &str) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    fn get_pending_items_by_workspace(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    fn get_pending_items_by_priority(
        &self,
        _priority: PendingPriority,
    ) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    fn update_pending_item(&mut self, _item: &PendingItem) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Pending item operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn delete_pending_item(&mut self, _pending_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Pending item operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_pending_items_requiring_manual_review(&self) -> Result<Vec<PendingItem>, StorageError> {
        Ok(Vec::new())
    }

    // Audit methods - not yet implemented for EncryptedFileStorage
    fn store_audit_event(&mut self, _event: &AuditEvent) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Audit operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_audit_event(&self, _event_id: &Uuid) -> Result<Option<AuditEvent>, StorageError> {
        Ok(None)
    }

    fn query_audit_events(&self, _query: &AuditQuery) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn list_audit_events(&self) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn get_audit_events_by_user(&self, _user_id: &str) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn get_audit_events_by_type(
        &self,
        _event_type: AuditEventType,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn get_audit_events_by_severity(
        &self,
        _severity: AuditSeverity,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn get_audit_events_in_time_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        Ok(Vec::new())
    }

    fn sync_audit_events(&mut self, _events: Vec<AuditEvent>) -> Result<(), StorageError> {
        Ok(())
    }

    fn store_security_incident(
        &mut self,
        _incident: &SecurityIncident,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Security incident operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_security_incident(
        &self,
        _incident_id: &Uuid,
    ) -> Result<Option<SecurityIncident>, StorageError> {
        Ok(None)
    }

    fn update_security_incident(
        &mut self,
        _incident: &SecurityIncident,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Security incident operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn list_security_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(Vec::new())
    }

    fn get_incidents_by_severity(
        &self,
        _severity: AuditSeverity,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(Vec::new())
    }

    fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(Vec::new())
    }

    fn get_incidents_by_assignee(
        &self,
        _assignee: &str,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        Ok(Vec::new())
    }

    fn store_compliance_report(&mut self, _report: &ComplianceReport) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Compliance report operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_compliance_report(
        &self,
        _report_id: &Uuid,
    ) -> Result<Option<ComplianceReport>, StorageError> {
        Ok(None)
    }

    fn update_compliance_report(&mut self, _report: &ComplianceReport) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "Compliance report operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn list_compliance_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(Vec::new())
    }

    fn get_reports_by_type(
        &self,
        _report_type: &str,
    ) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(Vec::new())
    }

    fn get_pending_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Ok(Vec::new())
    }

    fn get_audit_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, StorageError> {
        Ok(AuditDashboardMetrics {
            total_events: 0,
            events_last_24h: 0,
            events_last_7d: 0,
            security_incidents: SecurityIncidentSummary {
                open: 0,
                critical: 0,
                resolved: 0,
            },
            compliance_status: ComplianceStatus {
                gdpr_events: 0,
                ccpa_events: 0,
                hipaa_events: 0,
                sox_events: 0,
            },
            top_users: Vec::new(),
            anomalies: Vec::new(),
        })
    }

    fn get_event_count_by_time_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<u64, StorageError> {
        Ok(0)
    }

    // ZK Proof operations - placeholder implementations
    fn store_zk_proof(
        &mut self,
        _proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "ZK proof operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_zk_proof(
        &self,
        _proof_id: &Uuid,
    ) -> Result<Option<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(None)
    }

    fn update_zk_proof(
        &mut self,
        _proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        Err(StorageError::IoError(
            "ZK proof operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn query_zk_proofs(
        &self,
        _query: &crate::api::zk_proofs::ZkProofQuery,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(Vec::new())
    }

    fn list_zk_proofs(&self) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(Vec::new())
    }

    fn get_zk_proofs_by_user(
        &self,
        _user_id: &str,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(Vec::new())
    }

    fn get_zk_proofs_by_circuit_type(
        &self,
        _circuit_type: CircuitType,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(Vec::new())
    }

    fn get_zk_proofs_by_status(
        &self,
        _status: crate::zk_proof_engine::ProofStatus,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Ok(Vec::new())
    }

    fn get_zk_proof_statistics(
        &self,
    ) -> Result<crate::api::zk_proofs::ZkProofStatistics, StorageError> {
        Ok(crate::api::zk_proofs::ZkProofStatistics {
            total_proofs: 0,
            verified_proofs: 0,
            failed_proofs: 0,
            pending_proofs: 0,
            proof_types: std::collections::HashMap::new(),
        })
    }

    fn delete_zk_proof(&mut self, _proof_id: &Uuid) -> Result<(), StorageError> {
        Ok(())
    }

    fn store_storage_history(&mut self, _history: &ItemStorageHistory) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Storage history operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_storage_history(&self, _dfid: &str) -> Result<Option<ItemStorageHistory>, StorageError> {
        Ok(None)
    }

    fn add_storage_record(
        &mut self,
        _dfid: &str,
        _record: StorageRecord,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Storage history operations not yet implemented for EncryptedFileStorage".to_string(),
        ))
    }

    // CID Timeline operations - not implemented for EncryptedFileStorage
    fn add_cid_to_timeline(
        &mut self,
        _dfid: &str,
        _cid: &str,
        _ipcm_tx: &str,
        _timestamp: i64,
        _network: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_item_timeline(&self, _dfid: &str) -> Result<Vec<TimelineEntry>, StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_timeline_by_sequence(
        &self,
        _dfid: &str,
        _sequence: i32,
    ) -> Result<Option<TimelineEntry>, StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn map_event_to_cid(
        &mut self,
        _event_id: &Uuid,
        _dfid: &str,
        _cid: &str,
        _sequence: i32,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_event_first_cid(
        &self,
        _event_id: &Uuid,
    ) -> Result<Option<EventCidMapping>, StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_events_in_cid(&self, _cid: &str) -> Result<Vec<EventCidMapping>, StorageError> {
        Err(StorageError::NotImplemented(
            "CID timeline operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn update_indexing_progress(
        &mut self,
        _network: &str,
        _last_ledger: i64,
        _confirmed_ledger: i64,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Indexing progress operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn get_indexing_progress(
        &self,
        _network: &str,
    ) -> Result<Option<IndexingProgress>, StorageError> {
        Err(StorageError::NotImplemented(
            "Indexing progress operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn increment_events_indexed(
        &mut self,
        _network: &str,
        _count: i64,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Indexing progress operations not implemented for EncryptedFileStorage".to_string(),
        ))
    }

    fn store_circuit_adapter_config(
        &mut self,
        _config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Circuit adapter config operations not yet implemented for EncryptedFileStorage"
                .to_string(),
        ))
    }

    fn get_circuit_adapter_config(
        &self,
        _circuit_id: &Uuid,
    ) -> Result<Option<CircuitAdapterConfig>, StorageError> {
        Ok(None)
    }

    fn update_circuit_adapter_config(
        &mut self,
        _config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Circuit adapter config operations not yet implemented for EncryptedFileStorage"
                .to_string(),
        ))
    }

    fn list_circuit_adapter_configs(&self) -> Result<Vec<CircuitAdapterConfig>, StorageError> {
        Ok(Vec::new())
    }

    // User Account operations (stub implementations for now)
    fn store_user_account(&mut self, _user: &UserAccount) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    fn get_user_account(&self, _user_id: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn get_user_by_username(&self, _username: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn get_user_by_email(&self, _email: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn update_user_account(&mut self, _user: &UserAccount) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    fn list_user_accounts(&self) -> Result<Vec<UserAccount>, StorageError> {
        Ok(Vec::new())
    }

    fn delete_user_account(&mut self, _user_id: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    // Credit Transaction operations (stub implementations for now)
    fn record_credit_transaction(
        &mut self,
        _transaction: &CreditTransaction,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Credit transaction operations not yet implemented".to_string(),
        ))
    }

    fn get_credit_transaction(
        &self,
        _transaction_id: &str,
    ) -> Result<Option<CreditTransaction>, StorageError> {
        Ok(None)
    }

    fn get_credit_transactions(
        &self,
        _user_id: &str,
        _limit: Option<usize>,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        Ok(Vec::new())
    }

    fn get_credit_transactions_by_operation(
        &self,
        _operation_type: &str,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        Ok(Vec::new())
    }

    // Admin Action operations (stub implementations for now)
    fn record_admin_action(&mut self, _action: &AdminAction) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Admin action operations not yet implemented".to_string(),
        ))
    }

    fn get_admin_actions(
        &self,
        _admin_id: Option<&str>,
        _limit: Option<usize>,
    ) -> Result<Vec<AdminAction>, StorageError> {
        Ok(Vec::new())
    }

    fn get_admin_actions_by_type(
        &self,
        _action_type: &str,
    ) -> Result<Vec<AdminAction>, StorageError> {
        Ok(Vec::new())
    }

    // System Statistics operations (stub implementations for now)
    fn get_system_statistics(&self) -> Result<SystemStatistics, StorageError> {
        Ok(SystemStatistics {
            total_users: 0,
            active_users_24h: 0,
            active_users_30d: 0,
            total_items: 0,
            total_circuits: 0,
            total_storage_operations: 0,
            credits_consumed_24h: 0,
            tier_distribution: HashMap::new(),
            adapter_usage_stats: HashMap::new(),
            generated_at: Utc::now(),
        })
    }

    fn update_system_statistics(&mut self, _stats: &SystemStatistics) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "System statistics operations not yet implemented".to_string(),
        ))
    }

    // Notification operations - not implemented for file storage yet
    fn store_notification(&mut self, _notification: &Notification) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_notification(
        &self,
        _notification_id: &str,
    ) -> Result<Option<Notification>, StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_user_notifications(
        &self,
        _user_id: &str,
        _since: Option<DateTime<Utc>>,
        _limit: Option<usize>,
        _unread_only: bool,
    ) -> Result<Vec<Notification>, StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn update_notification(&mut self, _notification: &Notification) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn delete_notification(&mut self, _notification_id: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn mark_all_notifications_read(&mut self, _user_id: &str) -> Result<usize, StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_unread_notification_count(&self, _user_id: &str) -> Result<usize, StorageError> {
        Err(StorageError::NotImplemented(
            "Notification operations not yet implemented for file storage".to_string(),
        ))
    }

    // Adapter Configuration Management operations - not implemented for file storage yet
    fn store_adapter_config(&mut self, _config: &AdapterConfig) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_adapter_config(&self, _config_id: &Uuid) -> Result<Option<AdapterConfig>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn update_adapter_config(&mut self, _config: &AdapterConfig) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn delete_adapter_config(&mut self, _config_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn list_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn list_active_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_adapter_configs_by_type(
        &self,
        _adapter_type: &AdapterType,
    ) -> Result<Vec<AdapterConfig>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_default_adapter_config(&self) -> Result<Option<AdapterConfig>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn set_default_adapter(&mut self, _config_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn store_adapter_test_result(
        &mut self,
        _result: &AdapterTestResult,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_adapter_test_result(
        &self,
        _config_id: &Uuid,
    ) -> Result<Option<AdapterTestResult>, StorageError> {
        Err(StorageError::NotImplemented(
            "Adapter config operations not yet implemented for file storage".to_string(),
        ))
    }

    // LID ↔ DFID mapping operations - not implemented for file storage yet
    fn store_lid_dfid_mapping(&mut self, _lid: &Uuid, _dfid: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "LID-DFID mapping not yet implemented for file storage".to_string(),
        ))
    }

    fn get_dfid_by_lid(&self, _lid: &Uuid) -> Result<Option<String>, StorageError> {
        Err(StorageError::NotImplemented(
            "LID-DFID mapping not yet implemented for file storage".to_string(),
        ))
    }

    // Canonical identifier lookups - not implemented for file storage yet
    fn get_dfid_by_canonical(
        &self,
        _namespace: &str,
        _registry: &str,
        _value: &str,
    ) -> Result<Option<String>, StorageError> {
        Err(StorageError::NotImplemented(
            "Canonical lookup not yet implemented for file storage".to_string(),
        ))
    }

    // Fingerprint mappings - not implemented for file storage yet
    fn store_fingerprint_mapping(
        &mut self,
        _fingerprint: &str,
        _dfid: &str,
        _circuit_id: &Uuid,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Fingerprint mapping not yet implemented for file storage".to_string(),
        ))
    }

    fn get_dfid_by_fingerprint(
        &self,
        _fingerprint: &str,
        _circuit_id: &Uuid,
    ) -> Result<Option<String>, StorageError> {
        Err(StorageError::NotImplemented(
            "Fingerprint lookup not yet implemented for file storage".to_string(),
        ))
    }

    // Enhanced identifier mappings - not implemented for file storage yet
    fn store_enhanced_identifier_mapping(
        &mut self,
        _identifier: &EnhancedIdentifier,
        _dfid: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Enhanced identifier mapping not yet implemented for file storage".to_string(),
        ))
    }

    // Webhook delivery operations - not implemented for file storage yet
    fn store_webhook_delivery(&mut self, _delivery: &WebhookDelivery) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Webhook delivery operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_webhook_delivery(
        &self,
        _delivery_id: &Uuid,
    ) -> Result<Option<WebhookDelivery>, StorageError> {
        Err(StorageError::NotImplemented(
            "Webhook delivery operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_webhook_deliveries_by_circuit(
        &self,
        _circuit_id: &Uuid,
        _limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        Err(StorageError::NotImplemented(
            "Webhook delivery operations not yet implemented for file storage".to_string(),
        ))
    }

    fn get_webhook_deliveries_by_webhook(
        &self,
        _webhook_id: &Uuid,
        _limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        Err(StorageError::NotImplemented(
            "Webhook delivery operations not yet implemented for file storage".to_string(),
        ))
    }
}

// Implementation of StorageBackend for Arc<Mutex<InMemoryStorage>>
// This enables shared storage across multiple engines
impl StorageBackend for Arc<std::sync::Mutex<InMemoryStorage>> {
    fn store_receipt(&mut self, receipt: &Receipt) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_receipt(receipt)
    }

    fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_receipt(id)
    }

    fn find_receipts_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<Receipt>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .find_receipts_by_identifier(identifier)
    }

    fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_receipts()
    }

    fn store_log(&mut self, log: &LogEntry) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_log(log)
    }

    fn get_logs(&self) -> Result<Vec<LogEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_logs()
    }

    fn store_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_data_lake_entry(entry)
    }

    fn get_data_lake_entry(&self, entry_id: &Uuid) -> Result<Option<DataLakeEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_data_lake_entry(entry_id)
    }

    fn update_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_data_lake_entry(entry)
    }

    fn get_data_lake_entries_by_status(
        &self,
        status: ProcessingStatus,
    ) -> Result<Vec<DataLakeEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_data_lake_entries_by_status(status)
    }

    fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_data_lake_entries()
    }

    fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_item(item)
    }

    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_item_by_dfid(dfid)
    }

    fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_item(item)
    }

    fn list_items(&self) -> Result<Vec<Item>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_items()
    }

    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .find_items_by_identifier(identifier)
    }

    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .find_items_by_status(status)
    }

    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_item(dfid)
    }

    fn store_identifier_mapping(
        &mut self,
        mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_identifier_mapping(mapping)
    }

    fn get_identifier_mappings(
        &self,
        from_id: &Identifier,
    ) -> Result<Vec<IdentifierMapping>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_identifier_mappings(from_id)
    }

    fn update_identifier_mapping(
        &mut self,
        mapping: &IdentifierMapping,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_identifier_mapping(mapping)
    }

    fn list_identifier_mappings(&self) -> Result<Vec<IdentifierMapping>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_identifier_mappings()
    }

    fn store_conflict_resolution(
        &mut self,
        resolution: &ConflictResolution,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_conflict_resolution(resolution)
    }

    fn get_conflict_resolution(
        &self,
        conflict_id: &Uuid,
    ) -> Result<Option<ConflictResolution>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_conflict_resolution(conflict_id)
    }

    fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_conflicts()
    }

    fn store_event(&mut self, event: &Event) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_event(event)
    }

    fn get_event(&self, id: &Uuid) -> Result<Option<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_event(id)
    }

    fn list_events(&self) -> Result<Vec<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_events()
    }

    fn update_event(&mut self, event: &Event) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_event(event)
    }

    fn get_events_by_dfid(&self, dfid: &str) -> Result<Vec<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_events_by_dfid(dfid)
    }

    fn get_events_by_type(&self, event_type: EventType) -> Result<Vec<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_events_by_type(event_type)
    }

    fn get_events_by_visibility(
        &self,
        visibility: EventVisibility,
    ) -> Result<Vec<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_events_by_visibility(visibility)
    }

    fn get_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_events_in_time_range(start, end)
    }

    fn store_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_circuit(circuit)
    }

    fn get_circuit(&self, id: &Uuid) -> Result<Option<Circuit>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuit(id)
    }

    fn update_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_circuit(circuit)
    }

    fn list_circuits(&self) -> Result<Vec<Circuit>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_circuits()
    }

    fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuits_for_member(member_id)
    }

    fn store_circuit_operation(
        &mut self,
        operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_circuit_operation(operation)
    }

    fn get_circuit_operation(&self, id: &Uuid) -> Result<Option<CircuitOperation>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuit_operation(id)
    }

    fn update_circuit_operation(
        &mut self,
        operation: &CircuitOperation,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_circuit_operation(operation)
    }

    fn get_circuit_operations(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Vec<CircuitOperation>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuit_operations(circuit_id)
    }

    fn store_activity(&mut self, activity: &Activity) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_activity(activity)
    }

    fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_activities_for_user(user_id)
    }

    fn get_activities_for_circuit(&self, circuit_id: &Uuid) -> Result<Vec<Activity>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_activities_for_circuit(circuit_id)
    }

    fn get_all_activities(&self) -> Result<Vec<Activity>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_all_activities()
    }

    fn store_item_share(&mut self, share: &ItemShare) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_item_share(share)
    }

    fn get_shares_for_item(&self, dfid: &str) -> Result<Vec<ItemShare>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_shares_for_item(dfid)
    }

    fn get_shares_for_user(&self, user_id: &str) -> Result<Vec<ItemShare>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_shares_for_user(user_id)
    }

    fn is_item_shared_with_user(&self, dfid: &str, user_id: &str) -> Result<bool, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .is_item_shared_with_user(dfid, user_id)
    }

    fn store_circuit_item(&mut self, circuit_item: &CircuitItem) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_circuit_item(circuit_item)
    }

    fn get_circuit_items(&self, circuit_id: &Uuid) -> Result<Vec<CircuitItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuit_items(circuit_id)
    }

    fn get_item_share(&self, share_id: &str) -> Result<Option<ItemShare>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_item_share(share_id)
    }

    fn delete_item_share(&mut self, share_id: &str) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_item_share(share_id)
    }

    fn remove_circuit_item(&mut self, circuit_id: &Uuid, dfid: &str) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .remove_circuit_item(circuit_id, dfid)
    }

    // Pending Items operations - delegate to underlying InMemoryStorage
    fn store_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_pending_item(item)
    }

    fn get_pending_item(&self, pending_id: &Uuid) -> Result<Option<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_item(pending_id)
    }

    fn list_pending_items(&self) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_pending_items()
    }

    fn get_pending_items_by_reason(
        &self,
        reason_type: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_items_by_reason(reason_type)
    }

    fn get_pending_items_by_user(&self, user_id: &str) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_items_by_user(user_id)
    }

    fn get_pending_items_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_items_by_workspace(workspace_id)
    }

    fn get_pending_items_by_priority(
        &self,
        priority: PendingPriority,
    ) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_items_by_priority(priority)
    }

    fn update_pending_item(&mut self, item: &PendingItem) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_pending_item(item)
    }

    fn delete_pending_item(&mut self, pending_id: &Uuid) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_pending_item(pending_id)
    }

    fn get_pending_items_requiring_manual_review(&self) -> Result<Vec<PendingItem>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_items_requiring_manual_review()
    }

    // Audit Event operations - delegate to underlying InMemoryStorage
    fn store_audit_event(&mut self, event: &AuditEvent) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_audit_event(event)
    }

    fn get_audit_event(&self, event_id: &Uuid) -> Result<Option<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_event(event_id)
    }

    fn query_audit_events(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .query_audit_events(query)
    }

    fn list_audit_events(&self) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_audit_events()
    }

    fn get_audit_events_by_user(&self, user_id: &str) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_events_by_user(user_id)
    }

    fn get_audit_events_by_type(
        &self,
        event_type: AuditEventType,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_events_by_type(event_type)
    }

    fn get_audit_events_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_events_by_severity(severity)
    }

    fn get_audit_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AuditEvent>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_events_in_time_range(start, end)
    }

    fn sync_audit_events(&mut self, events: Vec<AuditEvent>) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .sync_audit_events(events)
    }

    // Security Incident operations - delegate to underlying InMemoryStorage
    fn store_security_incident(&mut self, incident: &SecurityIncident) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_security_incident(incident)
    }

    fn get_security_incident(
        &self,
        incident_id: &Uuid,
    ) -> Result<Option<SecurityIncident>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_security_incident(incident_id)
    }

    fn update_security_incident(
        &mut self,
        incident: &SecurityIncident,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_security_incident(incident)
    }

    fn list_security_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_security_incidents()
    }

    fn get_incidents_by_severity(
        &self,
        severity: AuditSeverity,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_incidents_by_severity(severity)
    }

    fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_open_incidents()
    }

    fn get_incidents_by_assignee(
        &self,
        assignee: &str,
    ) -> Result<Vec<SecurityIncident>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_incidents_by_assignee(assignee)
    }

    // Compliance Report operations - delegate to underlying InMemoryStorage
    fn store_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_compliance_report(report)
    }

    fn get_compliance_report(
        &self,
        report_id: &Uuid,
    ) -> Result<Option<ComplianceReport>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_compliance_report(report_id)
    }

    fn update_compliance_report(&mut self, report: &ComplianceReport) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_compliance_report(report)
    }

    fn list_compliance_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_compliance_reports()
    }

    fn get_reports_by_type(
        &self,
        report_type: &str,
    ) -> Result<Vec<ComplianceReport>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_reports_by_type(report_type)
    }

    fn get_pending_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_pending_reports()
    }

    // Audit Dashboard operations - delegate to underlying InMemoryStorage
    fn get_audit_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_audit_dashboard_metrics()
    }

    fn get_event_count_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_event_count_by_time_range(start, end)
    }

    // ZK Proof operations - delegate to underlying InMemoryStorage
    fn store_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_zk_proof(proof)
    }

    fn get_zk_proof(
        &self,
        proof_id: &Uuid,
    ) -> Result<Option<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_zk_proof(proof_id)
    }

    fn update_zk_proof(
        &mut self,
        proof: &crate::zk_proof_engine::ZkProof,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_zk_proof(proof)
    }

    fn query_zk_proofs(
        &self,
        query: &crate::api::zk_proofs::ZkProofQuery,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .query_zk_proofs(query)
    }

    fn list_zk_proofs(&self) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_zk_proofs()
    }

    fn get_zk_proofs_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_zk_proofs_by_user(user_id)
    }

    fn get_zk_proofs_by_circuit_type(
        &self,
        circuit_type: CircuitType,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_zk_proofs_by_circuit_type(circuit_type)
    }

    fn get_zk_proofs_by_status(
        &self,
        status: crate::zk_proof_engine::ProofStatus,
    ) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_zk_proofs_by_status(status)
    }

    fn get_zk_proof_statistics(
        &self,
    ) -> Result<crate::api::zk_proofs::ZkProofStatistics, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_zk_proof_statistics()
    }

    fn delete_zk_proof(&mut self, proof_id: &Uuid) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_zk_proof(proof_id)
    }

    fn store_storage_history(&mut self, history: &ItemStorageHistory) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_storage_history(history)
    }

    fn get_storage_history(&self, dfid: &str) -> Result<Option<ItemStorageHistory>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_storage_history(dfid)
    }

    fn add_storage_record(
        &mut self,
        dfid: &str,
        record: StorageRecord,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .add_storage_record(dfid, record)
    }

    // CID Timeline operations - delegate to inner InMemoryStorage
    fn add_cid_to_timeline(
        &mut self,
        dfid: &str,
        cid: &str,
        ipcm_tx: &str,
        timestamp: i64,
        network: &str,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .add_cid_to_timeline(dfid, cid, ipcm_tx, timestamp, network)
    }

    fn get_item_timeline(&self, dfid: &str) -> Result<Vec<TimelineEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_item_timeline(dfid)
    }

    fn get_timeline_by_sequence(
        &self,
        dfid: &str,
        sequence: i32,
    ) -> Result<Option<TimelineEntry>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_timeline_by_sequence(dfid, sequence)
    }

    fn map_event_to_cid(
        &mut self,
        event_id: &Uuid,
        dfid: &str,
        cid: &str,
        sequence: i32,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .map_event_to_cid(event_id, dfid, cid, sequence)
    }

    fn get_event_first_cid(
        &self,
        event_id: &Uuid,
    ) -> Result<Option<EventCidMapping>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_event_first_cid(event_id)
    }

    fn get_events_in_cid(&self, cid: &str) -> Result<Vec<EventCidMapping>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_events_in_cid(cid)
    }

    fn update_indexing_progress(
        &mut self,
        network: &str,
        last_ledger: i64,
        confirmed_ledger: i64,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_indexing_progress(network, last_ledger, confirmed_ledger)
    }

    fn get_indexing_progress(
        &self,
        network: &str,
    ) -> Result<Option<IndexingProgress>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_indexing_progress(network)
    }

    fn increment_events_indexed(&mut self, network: &str, count: i64) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .increment_events_indexed(network, count)
    }

    fn store_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_circuit_adapter_config(config)
    }

    fn get_circuit_adapter_config(
        &self,
        circuit_id: &Uuid,
    ) -> Result<Option<CircuitAdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_circuit_adapter_config(circuit_id)
    }

    fn update_circuit_adapter_config(
        &mut self,
        config: &CircuitAdapterConfig,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_circuit_adapter_config(config)
    }

    fn list_circuit_adapter_configs(&self) -> Result<Vec<CircuitAdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_circuit_adapter_configs()
    }

    // User Account operations (stub implementations for now)
    fn store_user_account(&mut self, _user: &UserAccount) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    fn get_user_account(&self, _user_id: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn get_user_by_username(&self, _username: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn get_user_by_email(&self, _email: &str) -> Result<Option<UserAccount>, StorageError> {
        Ok(None)
    }

    fn update_user_account(&mut self, _user: &UserAccount) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    fn list_user_accounts(&self) -> Result<Vec<UserAccount>, StorageError> {
        Ok(Vec::new())
    }

    fn delete_user_account(&mut self, _user_id: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "User account operations not yet implemented".to_string(),
        ))
    }

    // Credit Transaction operations (stub implementations for now)
    fn record_credit_transaction(
        &mut self,
        _transaction: &CreditTransaction,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Credit transaction operations not yet implemented".to_string(),
        ))
    }

    fn get_credit_transaction(
        &self,
        _transaction_id: &str,
    ) -> Result<Option<CreditTransaction>, StorageError> {
        Ok(None)
    }

    fn get_credit_transactions(
        &self,
        _user_id: &str,
        _limit: Option<usize>,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        Ok(Vec::new())
    }

    fn get_credit_transactions_by_operation(
        &self,
        _operation_type: &str,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        Ok(Vec::new())
    }

    // Admin Action operations (stub implementations for now)
    fn record_admin_action(&mut self, _action: &AdminAction) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "Admin action operations not yet implemented".to_string(),
        ))
    }

    fn get_admin_actions(
        &self,
        _admin_id: Option<&str>,
        _limit: Option<usize>,
    ) -> Result<Vec<AdminAction>, StorageError> {
        Ok(Vec::new())
    }

    fn get_admin_actions_by_type(
        &self,
        _action_type: &str,
    ) -> Result<Vec<AdminAction>, StorageError> {
        Ok(Vec::new())
    }

    // System Statistics operations (stub implementations for now)
    fn get_system_statistics(&self) -> Result<SystemStatistics, StorageError> {
        Ok(SystemStatistics {
            total_users: 0,
            active_users_24h: 0,
            active_users_30d: 0,
            total_items: 0,
            total_circuits: 0,
            total_storage_operations: 0,
            credits_consumed_24h: 0,
            tier_distribution: HashMap::new(),
            adapter_usage_stats: HashMap::new(),
            generated_at: Utc::now(),
        })
    }

    fn update_system_statistics(&mut self, _stats: &SystemStatistics) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(
            "System statistics operations not yet implemented".to_string(),
        ))
    }

    // Notification operations - delegate to inner storage
    fn store_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_notification(notification)
    }

    fn get_notification(
        &self,
        notification_id: &str,
    ) -> Result<Option<Notification>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_notification(notification_id)
    }

    fn get_user_notifications(
        &self,
        user_id: &str,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_user_notifications(user_id, since, limit, unread_only)
    }

    fn update_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_notification(notification)
    }

    fn delete_notification(&mut self, notification_id: &str) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_notification(notification_id)
    }

    fn mark_all_notifications_read(&mut self, user_id: &str) -> Result<usize, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .mark_all_notifications_read(user_id)
    }

    fn get_unread_notification_count(&self, user_id: &str) -> Result<usize, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_unread_notification_count(user_id)
    }

    // Adapter Configuration Management operations
    fn store_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_adapter_config(config)
    }

    fn get_adapter_config(&self, config_id: &Uuid) -> Result<Option<AdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_adapter_config(config_id)
    }

    fn update_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .update_adapter_config(config)
    }

    fn delete_adapter_config(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .delete_adapter_config(config_id)
    }

    fn list_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_adapter_configs()
    }

    fn list_active_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .list_active_adapter_configs()
    }

    fn get_adapter_configs_by_type(
        &self,
        adapter_type: &AdapterType,
    ) -> Result<Vec<AdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_adapter_configs_by_type(adapter_type)
    }

    fn get_default_adapter_config(&self) -> Result<Option<AdapterConfig>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_default_adapter_config()
    }

    fn set_default_adapter(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .set_default_adapter(config_id)
    }

    fn store_adapter_test_result(
        &mut self,
        result: &AdapterTestResult,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_adapter_test_result(result)
    }

    fn get_adapter_test_result(
        &self,
        config_id: &Uuid,
    ) -> Result<Option<AdapterTestResult>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_adapter_test_result(config_id)
    }

    // LID ↔ DFID mapping operations
    fn store_lid_dfid_mapping(&mut self, lid: &Uuid, dfid: &str) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_lid_dfid_mapping(lid, dfid)
    }

    fn get_dfid_by_lid(&self, lid: &Uuid) -> Result<Option<String>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_dfid_by_lid(lid)
    }

    // Canonical identifier lookups
    fn get_dfid_by_canonical(
        &self,
        namespace: &str,
        registry: &str,
        value: &str,
    ) -> Result<Option<String>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_dfid_by_canonical(namespace, registry, value)
    }

    // Fingerprint mappings
    fn store_fingerprint_mapping(
        &mut self,
        fingerprint: &str,
        dfid: &str,
        circuit_id: &Uuid,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_fingerprint_mapping(fingerprint, dfid, circuit_id)
    }

    fn get_dfid_by_fingerprint(
        &self,
        fingerprint: &str,
        circuit_id: &Uuid,
    ) -> Result<Option<String>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_dfid_by_fingerprint(fingerprint, circuit_id)
    }

    // Enhanced identifier mappings
    fn store_enhanced_identifier_mapping(
        &mut self,
        identifier: &EnhancedIdentifier,
        dfid: &str,
    ) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_enhanced_identifier_mapping(identifier, dfid)
    }

    // Webhook delivery operations
    fn store_webhook_delivery(&mut self, delivery: &WebhookDelivery) -> Result<(), StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .store_webhook_delivery(delivery)
    }

    fn get_webhook_delivery(
        &self,
        delivery_id: &Uuid,
    ) -> Result<Option<WebhookDelivery>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_webhook_delivery(delivery_id)
    }

    fn get_webhook_deliveries_by_circuit(
        &self,
        circuit_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_webhook_deliveries_by_circuit(circuit_id, limit)
    }

    fn get_webhook_deliveries_by_webhook(
        &self,
        webhook_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, StorageError> {
        self.lock()
            .map_err(|_| StorageError::IoError("Storage mutex poisoned".to_string()))?
            .get_webhook_deliveries_by_webhook(webhook_id, limit)
    }
}
