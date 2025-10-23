use crate::adapters::base::StorageLocation;
pub use crate::identifier_types::Identifier;
use crate::identifier_types::{CircuitAliasConfig, ExternalAlias};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub hash: String,
    pub timestamp: DateTime<Utc>,
    pub data_size: usize,
    pub identifiers: Vec<Identifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLakeEntry {
    pub entry_id: Uuid,
    pub receipt_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub identifiers: Vec<Identifier>,
    pub data_hash: String,
    pub data_size: usize,
    pub status: ProcessingStatus,
    pub linked_dfid: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub dfid: String,
    pub local_id: Option<Uuid>,       // NOVO: LID do workspace
    pub legacy_mode: bool,            // NOVO: true se DFID gerado no workspace
    pub identifiers: Vec<Identifier>, // Identificadores unificados
    pub aliases: Vec<ExternalAlias>,  // NOVO
    pub fingerprint: Option<String>,  // NOVO: para dedup contextual
    pub enriched_data: HashMap<String, serde_json::Value>,
    pub creation_timestamp: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub source_entries: Vec<Uuid>,
    pub confidence_score: f64,
    pub status: ItemStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemStatus {
    Active,
    Deprecated,
    Merged,
    Split,
    MergedInto(String), // Points to master LID that this item was merged into
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MergeStrategy {
    Append,    // Merge all data, append unique values to arrays
    KeepFirst, // Keep master data, ignore merge items' data
    Overwrite, // Last merged item's data wins
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifierMapping {
    pub identifier: Identifier,
    pub dfid: String,
    pub identifier_type: String,
    pub confidence_level: f64,
    pub creation_timestamp: DateTime<Utc>,
    pub status: MappingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingStatus {
    Active,
    Deprecated,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_id: Uuid,
    pub conflicting_identifiers: Vec<Identifier>,
    pub conflicting_dfids: Vec<String>,
    pub resolution_strategy: ResolutionStrategy,
    pub resolved_dfid: Option<String>,
    pub resolution_timestamp: Option<DateTime<Utc>>,
    pub requires_manual_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    ConfidenceBased,
    Temporal,
    ManualReview,
    Split,
    Merge,
    AutoMerge,
    SkipProcessing,
    CreateSeparate,
    MatchBest,
}

impl DataLakeEntry {
    pub fn new(
        receipt_id: Uuid,
        identifiers: Vec<Identifier>,
        data_hash: String,
        data_size: usize,
    ) -> Self {
        Self {
            entry_id: Uuid::new_v4(),
            receipt_id,
            timestamp: Utc::now(),
            identifiers,
            data_hash,
            data_size,
            status: ProcessingStatus::Pending,
            linked_dfid: None,
            error_message: None,
        }
    }

    pub fn mark_processing(&mut self) {
        self.status = ProcessingStatus::Processing;
    }

    pub fn mark_completed(&mut self, dfid: String) {
        self.status = ProcessingStatus::Completed;
        self.linked_dfid = Some(dfid);
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = ProcessingStatus::Failed;
        self.error_message = Some(error);
    }

    pub fn mark_conflicted(&mut self) {
        self.status = ProcessingStatus::Conflicted;
    }
}

impl Item {
    pub fn new(dfid: String, identifiers: Vec<Identifier>, source_entry: Uuid) -> Self {
        Self {
            dfid,
            local_id: None,
            legacy_mode: true, // By default, assume legacy mode for existing code
            identifiers,
            aliases: vec![],
            fingerprint: None,
            enriched_data: HashMap::new(),
            creation_timestamp: Utc::now(),
            last_modified: Utc::now(),
            source_entries: vec![source_entry],
            confidence_score: 1.0,
            status: ItemStatus::Active,
        }
    }

    pub fn new_with_lid(local_id: Uuid, identifiers: Vec<Identifier>, source_entry: Uuid) -> Self {
        Self {
            dfid: format!("LID-{local_id}"), // Temporary DFID
            local_id: Some(local_id),
            legacy_mode: false,
            identifiers,
            aliases: vec![],
            fingerprint: None,
            enriched_data: HashMap::new(),
            creation_timestamp: Utc::now(),
            last_modified: Utc::now(),
            source_entries: vec![source_entry],
            confidence_score: 1.0,
            status: ItemStatus::Active,
        }
    }

    pub fn enrich(&mut self, data: HashMap<String, serde_json::Value>, source_entry: Uuid) {
        self.enriched_data.extend(data);
        self.source_entries.push(source_entry);
        self.last_modified = Utc::now();
    }

    pub fn add_identifiers(&mut self, identifiers: Vec<Identifier>) {
        for identifier in identifiers {
            if !self.identifiers.contains(&identifier) {
                self.identifiers.push(identifier);
            }
        }
        self.last_modified = Utc::now();
    }
}

impl IdentifierMapping {
    pub fn new(identifier: Identifier, dfid: String, identifier_type: String) -> Self {
        Self {
            identifier,
            dfid,
            identifier_type,
            confidence_level: 1.0,
            creation_timestamp: Utc::now(),
            status: MappingStatus::Active,
        }
    }

    pub fn deprecate(&mut self) {
        self.status = MappingStatus::Deprecated;
    }

    pub fn mark_conflicted(&mut self) {
        self.status = MappingStatus::Conflicted;
    }
}

impl ConflictResolution {
    pub fn new(identifiers: Vec<Identifier>, dfids: Vec<String>) -> Self {
        Self {
            conflict_id: Uuid::new_v4(),
            conflicting_identifiers: identifiers,
            conflicting_dfids: dfids,
            resolution_strategy: ResolutionStrategy::ManualReview,
            resolved_dfid: None,
            resolution_timestamp: None,
            requires_manual_review: true,
        }
    }

    pub fn resolve(&mut self, strategy: ResolutionStrategy, dfid: String) {
        self.resolution_strategy = strategy;
        self.resolved_dfid = Some(dfid);
        self.resolution_timestamp = Some(Utc::now());
        self.requires_manual_review = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: Uuid,
    pub dfid: String,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub is_encrypted: bool,
    pub visibility: EventVisibility,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Created,
    Enriched,
    Merged,
    Split,
    PushedToCircuit,
    PulledFromCircuit,
    Updated,
    StatusChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventVisibility {
    Public,
    Private,
    CircuitOnly,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PendingPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl std::ops::AddAssign<u32> for PendingPriority {
    fn add_assign(&mut self, rhs: u32) {
        *self = match (*self as u32) + rhs {
            1 => PendingPriority::Low,
            2 => PendingPriority::Normal,
            3 => PendingPriority::High,
            _ => PendingPriority::Critical,
        };
    }
}

impl PartialOrd<u32> for PendingPriority {
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        (*self as u32).partial_cmp(other)
    }
}

impl PartialEq<u32> for PendingPriority {
    fn eq(&self, other: &u32) -> bool {
        (*self as u32).eq(other)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: String,
    pub description: String,
    pub confidence: f64,
    pub automated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItem {
    pub pending_id: Uuid,
    pub identifiers: Vec<Identifier>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub source_entry: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub reason: PendingReason,
    pub priority: PendingPriority,
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
    pub retry_count: u32,
    pub manual_review_required: bool,
    pub suggested_actions: Vec<SuggestedAction>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEvent {
    pub pending_id: Uuid,
    pub identifiers: Vec<Identifier>,
    pub enriched_data: Option<HashMap<String, serde_json::Value>>,
    pub source_entry: Uuid,
    pub created_at: DateTime<Utc>,
    pub reason: PendingReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingReason {
    NoIdentifiers,
    InvalidIdentifiers(String),
    ConflictingDFIDs {
        identifier: Identifier,
        conflicting_dfids: Vec<String>,
        confidence_scores: Option<Vec<f64>>,
    },
    IdentifierMappingConflict {
        conflicting_mappings: Vec<String>,
        resolution_strategy: Option<ResolutionStrategy>,
    },
    DataQualityIssue {
        issue_type: String,
        severity: QualitySeverity,
        details: String,
    },
    ProcessingError(String),
    ValidationError(String),
    DuplicateDetectionAmbiguous {
        potential_matches: Vec<String>,
        similarity_scores: Vec<f64>,
    },
    CrossSystemConflict {
        external_system: String,
        conflict_type: String,
    },
}

impl PendingEvent {
    pub fn new(
        identifiers: Vec<Identifier>,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        source_entry: Uuid,
        reason: PendingReason,
    ) -> Self {
        Self {
            pending_id: Uuid::new_v4(),
            identifiers,
            enriched_data,
            source_entry,
            created_at: Utc::now(),
            reason,
        }
    }
}

impl PendingItem {
    pub fn new(
        identifiers: Vec<Identifier>,
        enriched_data: Option<HashMap<String, serde_json::Value>>,
        source_entry: Uuid,
        reason: PendingReason,
        user_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Self {
        let priority = Self::calculate_priority(&reason);
        let manual_review_required = reason.requires_manual_review();
        let suggested_actions = Self::generate_suggested_actions(&reason);

        Self {
            pending_id: Uuid::new_v4(),
            identifiers,
            enriched_data,
            source_entry,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            reason,
            priority,
            user_id,
            workspace_id,
            retry_count: 0,
            manual_review_required,
            suggested_actions,
            metadata: HashMap::new(),
        }
    }

    pub fn update_last_modified(&mut self) {
        self.last_updated = Utc::now();
    }

    pub fn increment_retry_count(&mut self) {
        self.retry_count += 1;
        self.update_last_modified();
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.update_last_modified();
    }

    pub fn update_priority(&mut self, priority: PendingPriority) {
        self.priority = priority;
        self.update_last_modified();
    }

    pub fn add_suggested_action(&mut self, action: SuggestedAction) {
        self.suggested_actions.push(action);
        self.update_last_modified();
    }

    fn calculate_priority(reason: &PendingReason) -> PendingPriority {
        match reason {
            PendingReason::NoIdentifiers => PendingPriority::High,
            PendingReason::ConflictingDFIDs { .. } => PendingPriority::Critical,
            PendingReason::DataQualityIssue { severity, .. } => match severity {
                QualitySeverity::Critical => PendingPriority::Critical,
                QualitySeverity::High => PendingPriority::High,
                QualitySeverity::Medium => PendingPriority::Normal,
                QualitySeverity::Low => PendingPriority::Low,
            },
            PendingReason::IdentifierMappingConflict { .. } => PendingPriority::High,
            PendingReason::DuplicateDetectionAmbiguous { .. } => PendingPriority::Normal,
            PendingReason::CrossSystemConflict { .. } => PendingPriority::High,
            PendingReason::ValidationError(_) => PendingPriority::Normal,
            PendingReason::InvalidIdentifiers(_) => PendingPriority::Normal,
            PendingReason::ProcessingError(_) => PendingPriority::Low,
        }
    }

    fn generate_suggested_actions(reason: &PendingReason) -> Vec<SuggestedAction> {
        match reason {
            PendingReason::NoIdentifiers => vec![
                SuggestedAction {
                    action_type: "add_identifiers".to_string(),
                    description: "Add one or more identifiers to enable processing".to_string(),
                    confidence: 0.9,
                    automated: false,
                },
                SuggestedAction {
                    action_type: "generate_dfid".to_string(),
                    description: "Generate a new DFID for this item".to_string(),
                    confidence: 0.8,
                    automated: true,
                },
            ],
            PendingReason::ConflictingDFIDs { .. } => vec![
                SuggestedAction {
                    action_type: "manual_merge".to_string(),
                    description: "Manually review and merge conflicting identities".to_string(),
                    confidence: 0.7,
                    automated: false,
                },
                SuggestedAction {
                    action_type: "create_separate".to_string(),
                    description: "Create as separate item with new DFID".to_string(),
                    confidence: 0.6,
                    automated: true,
                },
            ],
            PendingReason::DuplicateDetectionAmbiguous { .. } => vec![SuggestedAction {
                action_type: "review_duplicates".to_string(),
                description: "Review potential duplicate matches".to_string(),
                confidence: 0.8,
                automated: false,
            }],
            _ => vec![SuggestedAction {
                action_type: "manual_review".to_string(),
                description: "Requires manual review for resolution".to_string(),
                confidence: 0.7,
                automated: false,
            }],
        }
    }
}

impl PendingReason {
    pub fn requires_manual_review(&self) -> bool {
        match self {
            PendingReason::ConflictingDFIDs { .. } => true,
            PendingReason::DataQualityIssue { severity, .. } => {
                matches!(severity, QualitySeverity::High | QualitySeverity::Critical)
            }
            PendingReason::IdentifierMappingConflict { .. } => true,
            PendingReason::DuplicateDetectionAmbiguous { .. } => true,
            PendingReason::CrossSystemConflict { .. } => true,
            _ => false,
        }
    }

    pub fn get_description(&self) -> String {
        match self {
            PendingReason::NoIdentifiers => {
                "Item has no identifiers for entity resolution".to_string()
            }
            PendingReason::InvalidIdentifiers(details) => {
                format!("Invalid identifiers: {details}")
            }
            PendingReason::ConflictingDFIDs {
                identifier,
                conflicting_dfids,
                ..
            } => {
                format!("Identifier {identifier:?} maps to multiple DFIDs: {conflicting_dfids:?}")
            }
            PendingReason::IdentifierMappingConflict {
                conflicting_mappings,
                ..
            } => {
                format!("Conflicting identifier mappings: {conflicting_mappings:?}")
            }
            PendingReason::DataQualityIssue {
                issue_type,
                details,
                ..
            } => {
                format!("Data quality issue ({issue_type}): {details}")
            }
            PendingReason::ProcessingError(error) => format!("Processing error: {error}"),
            PendingReason::ValidationError(error) => format!("Validation error: {error}"),
            PendingReason::DuplicateDetectionAmbiguous {
                potential_matches, ..
            } => {
                format!(
                    "Ambiguous duplicate detection: {} potential matches",
                    potential_matches.len()
                )
            }
            PendingReason::CrossSystemConflict {
                external_system,
                conflict_type,
            } => {
                format!("Conflict with external system {external_system}: {conflict_type}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub description: String,
    pub affected_identifiers: Vec<Identifier>,
    pub suggested_resolution: Option<ResolutionStrategy>,
    pub confidence: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    IdentifierDFIDMapping,
    DuplicateDetection,
    DataQuality,
    CrossSystem,
    ValidationFailure,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ConflictAnalysisResult {
    pub conflicts: Vec<ConflictInfo>,
    pub severity: ConflictSeverity,
    pub can_auto_resolve: bool,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub circuit_id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub default_namespace: String,                // NOVO
    pub alias_config: Option<CircuitAliasConfig>, // NOVO
    pub created_timestamp: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub members: Vec<CircuitMember>,
    pub permissions: CircuitPermissions,
    pub status: CircuitStatus,
    pub pending_requests: Vec<JoinRequest>,
    pub custom_roles: Vec<CustomRole>,
    pub public_settings: Option<PublicSettings>,
    pub adapter_config: Option<CircuitAdapterConfig>,
    pub post_action_settings: Option<PostActionSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicSettings {
    pub access_mode: PublicAccessMode,
    pub scheduled_date: Option<DateTime<Utc>>,
    pub access_password: Option<String>,
    pub public_name: Option<String>,
    pub public_description: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub logo_url: Option<String>,
    pub tagline: Option<String>,
    pub footer_text: Option<String>,
    pub published_items: Vec<String>,
    pub auto_approve_members: bool,
    pub auto_publish_pushed_items: bool,
    pub show_encrypted_events: bool,
    pub required_event_types: Option<String>,
    pub data_quality_rules: Option<String>,
    pub export_permissions: Option<ExportPermissionLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublicAccessMode {
    Public,
    Protected,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportPermissionLevel {
    Admin,
    Members,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub requester_id: String,
    pub requested_at: DateTime<Utc>,
    pub message: Option<String>,
    pub status: JoinRequestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinRequestStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicItemWithEvents {
    pub dfid: String,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCircuitInfo {
    pub circuit_id: Uuid,
    pub public_name: String,
    pub public_description: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub logo_url: Option<String>,
    pub tagline: Option<String>,
    pub footer_text: Option<String>,
    pub member_count: usize,
    pub access_mode: PublicAccessMode,
    pub requires_password: bool,
    pub is_currently_accessible: bool,
    pub published_items: Vec<String>,
    pub auto_publish_pushed_items: bool,
    pub published_items_with_events: Vec<PublicItemWithEvents>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitMember {
    pub member_id: String,
    pub role: MemberRole,
    pub custom_role_name: Option<String>,
    pub permissions: Vec<Permission>,
    pub joined_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Push,
    Pull,
    Invite,
    ManageMembers,
    ManagePermissions,
    Delete,
    Certify,
    Audit,
    ManageRoles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CircuitPermissions {
    pub require_approval_for_push: bool,
    pub require_approval_for_pull: bool,
    pub allow_public_visibility: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitStatus {
    Active,
    Inactive,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitOperation {
    pub operation_id: Uuid,
    pub circuit_id: Uuid,
    pub dfid: String,
    pub operation_type: OperationType,
    pub requester_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: OperationStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Push,
    Pull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Approved,
    Rejected,
    Completed,
    Failed,
}

impl Event {
    pub fn new(
        dfid: String,
        event_type: EventType,
        source: String,
        visibility: EventVisibility,
    ) -> Self {
        let timestamp = Utc::now();
        let metadata = HashMap::new();
        let content_hash =
            Self::calculate_content_hash(&event_type, &source, &timestamp, &metadata);

        Self {
            event_id: Uuid::new_v4(),
            dfid,
            event_type,
            timestamp,
            source,
            metadata,
            is_encrypted: false,
            visibility,
            content_hash,
        }
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        // Recalculate hash when metadata changes
        self.content_hash = Self::calculate_content_hash(
            &self.event_type,
            &self.source,
            &self.timestamp,
            &self.metadata,
        );
    }

    pub fn encrypt(&mut self) {
        self.is_encrypted = true;
    }

    /// Calculate content hash using BLAKE3 for event deduplication
    /// Hash includes: event_type + source + timestamp + metadata
    fn calculate_content_hash(
        event_type: &EventType,
        source: &str,
        timestamp: &DateTime<Utc>,
        metadata: &HashMap<String, serde_json::Value>,
    ) -> String {
        let mut hasher = blake3::Hasher::new();

        // Add event_type to hash
        hasher.update(format!("{event_type:?}").as_bytes());

        // Add source to hash
        hasher.update(source.as_bytes());

        // Add timestamp to hash (as RFC3339 string for consistency)
        hasher.update(timestamp.to_rfc3339().as_bytes());

        // Add metadata to hash (sorted keys for deterministic hashing)
        let metadata_json = serde_json::to_string(metadata).unwrap_or_default();
        hasher.update(metadata_json.as_bytes());

        hasher.finalize().to_hex().to_string()
    }

    /// Check if a specific user can view this event based on visibility and metadata
    /// Returns true if the user has permission to see the event
    pub fn can_user_view(&self, user_id: &str, current_circuit_id: Option<&str>) -> bool {
        match self.visibility {
            EventVisibility::Public => true,
            EventVisibility::Private => {
                // Private events only visible to creator
                self.source == user_id
            }
            EventVisibility::Direct => {
                // Direct events visible to creator or recipient
                if self.source == user_id {
                    return true;
                }
                // Check metadata for recipient_id
                if let Some(serde_json::Value::String(recipient)) =
                    self.metadata.get("recipient_id")
                {
                    recipient == user_id
                } else {
                    false
                }
            }
            EventVisibility::CircuitOnly => {
                // Circuit events only visible in specific circuit
                if let Some(circuit_id) = current_circuit_id {
                    if let Some(serde_json::Value::String(event_circuit)) =
                        self.metadata.get("circuit_id")
                    {
                        return event_circuit == circuit_id;
                    }
                }
                // If no circuit context or no circuit_id in metadata, not visible
                false
            }
        }
    }

    /// Set recipient for Direct visibility events
    pub fn set_recipient(&mut self, recipient_id: String) {
        self.metadata.insert(
            "recipient_id".to_string(),
            serde_json::Value::String(recipient_id),
        );
        // Recalculate hash after metadata change
        self.content_hash = Self::calculate_content_hash(
            &self.event_type,
            &self.source,
            &self.timestamp,
            &self.metadata,
        );
    }

    /// Set circuit for CircuitOnly visibility events
    pub fn set_circuit(&mut self, circuit_id: String) {
        self.metadata.insert(
            "circuit_id".to_string(),
            serde_json::Value::String(circuit_id),
        );
        // Recalculate hash after metadata change
        self.content_hash = Self::calculate_content_hash(
            &self.event_type,
            &self.source,
            &self.timestamp,
            &self.metadata,
        );
    }
}

impl Circuit {
    pub fn new(name: String, description: String, owner_id: String) -> Self {
        let circuit_id = Uuid::new_v4();
        let now = Utc::now();

        let owner_member = CircuitMember {
            member_id: owner_id.clone(),
            role: MemberRole::Owner,
            custom_role_name: None,
            permissions: vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
                Permission::ManagePermissions,
                Permission::ManageRoles,
                Permission::Delete,
                Permission::Certify,
                Permission::Audit,
            ],
            joined_timestamp: now,
        };

        let default_roles = vec![
            CustomRole::default_owner(circuit_id),
            CustomRole::default_member(circuit_id),
        ];

        // Initialize adapter_config with default "none" configuration
        let default_adapter_config = CircuitAdapterConfig {
            circuit_id,
            adapter_type: None,
            configured_by: "system".to_string(),
            configured_at: now,
            requires_approval: false,
            auto_migrate_existing: false,
            sponsor_adapter_access: false,
        };

        Self {
            circuit_id,
            name,
            description,
            owner_id,
            default_namespace: "generic".to_string(),
            alias_config: None,
            created_timestamp: now,
            last_modified: now,
            members: vec![owner_member],
            permissions: CircuitPermissions::default(),
            status: CircuitStatus::Active,
            pending_requests: Vec::new(),
            custom_roles: default_roles,
            public_settings: None,
            adapter_config: Some(default_adapter_config),
            post_action_settings: None,
        }
    }

    pub fn add_member(&mut self, member_id: String, role: MemberRole) {
        let permissions = match role {
            MemberRole::Owner => vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
                Permission::ManagePermissions,
                Permission::ManageRoles,
                Permission::Delete,
                Permission::Certify,
                Permission::Audit,
            ],
            MemberRole::Admin => vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
                Permission::ManageRoles,
                Permission::Certify,
            ],
            MemberRole::Member => vec![Permission::Push, Permission::Pull],
            MemberRole::Viewer => vec![Permission::Pull],
        };

        let member = CircuitMember {
            member_id,
            role,
            custom_role_name: None,
            permissions,
            joined_timestamp: Utc::now(),
        };

        self.members.push(member);
        self.last_modified = Utc::now();
    }

    pub fn has_permission(&self, member_id: &str, permission: &Permission) -> bool {
        self.members
            .iter()
            .find(|m| m.member_id == member_id)
            .map(|m| m.permissions.contains(permission))
            .unwrap_or(false)
    }

    pub fn get_member(&self, member_id: &str) -> Option<&CircuitMember> {
        self.members.iter().find(|m| m.member_id == member_id)
    }

    pub fn is_member(&self, member_id: &str) -> bool {
        // Owner is always considered a member
        if self.owner_id == member_id {
            return true;
        }

        self.members.iter().any(|m| m.member_id == member_id)
    }

    pub fn has_pending_request(&self, requester_id: &str) -> bool {
        self.pending_requests.iter().any(|r| {
            r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending)
        })
    }

    pub fn add_join_request(
        &mut self,
        requester_id: String,
        message: Option<String>,
    ) -> Result<(), String> {
        // Check if user is already a member
        if self.is_member(&requester_id) {
            return Err("User is already a member of this circuit".to_string());
        }

        // Check if there's already a pending request
        if self.has_pending_request(&requester_id) {
            return Err("User already has a pending request for this circuit".to_string());
        }

        let request = JoinRequest {
            requester_id,
            requested_at: Utc::now(),
            message,
            status: JoinRequestStatus::Pending,
        };

        self.pending_requests.push(request);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn approve_join_request(
        &mut self,
        requester_id: &str,
        role: MemberRole,
    ) -> Result<(), String> {
        // Find and update the request
        let request = self
            .pending_requests
            .iter_mut()
            .find(|r| {
                r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending)
            })
            .ok_or("No pending request found for this user")?;

        request.status = JoinRequestStatus::Approved;

        // Add the user as a member
        self.add_member(requester_id.to_string(), role);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn reject_join_request(&mut self, requester_id: &str) -> Result<(), String> {
        let request = self
            .pending_requests
            .iter_mut()
            .find(|r| {
                r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending)
            })
            .ok_or("No pending request found for this user")?;

        request.status = JoinRequestStatus::Rejected;
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn get_pending_requests(&self) -> Vec<&JoinRequest> {
        self.pending_requests
            .iter()
            .filter(|r| matches!(r.status, JoinRequestStatus::Pending))
            .collect()
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
        self.last_modified = Utc::now();
    }

    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.last_modified = Utc::now();
    }

    pub fn update_permissions(&mut self, permissions: CircuitPermissions) {
        self.permissions = permissions;
        self.last_modified = Utc::now();
    }

    pub fn add_custom_role(
        &mut self,
        role_name: String,
        permissions: Vec<Permission>,
        description: String,
        color: Option<String>,
        created_by: String,
    ) -> Result<(), String> {
        // Check if role name already exists
        if self.custom_roles.iter().any(|r| r.role_name == role_name) {
            return Err(format!("Role '{role_name}' already exists"));
        }

        let mut custom_role = CustomRole::new(
            self.circuit_id,
            role_name,
            permissions,
            description,
            created_by,
        );
        if let Some(color) = color {
            custom_role.set_color(color);
        }
        self.custom_roles.push(custom_role);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn get_custom_role(&self, role_name: &str) -> Option<&CustomRole> {
        self.custom_roles.iter().find(|r| r.role_name == role_name)
    }

    pub fn assign_custom_role(&mut self, member_id: &str, role_name: &str) -> Result<(), String> {
        // Check if custom role exists and get the permissions
        let custom_role_permissions = self
            .get_custom_role(role_name)
            .ok_or_else(|| format!("Custom role '{role_name}' not found"))?
            .permissions
            .clone();

        // Find member to verify existence
        let _member = self
            .members
            .iter()
            .find(|m| m.member_id == member_id)
            .ok_or_else(|| format!("Member '{member_id}' not found"))?;

        // Custom role REPLACES permissions entirely (no combination with base role)
        let final_permissions = custom_role_permissions;

        // Now update the member
        let member = self
            .members
            .iter_mut()
            .find(|m| m.member_id == member_id)
            .unwrap(); // Safe because we already checked existence above

        member.custom_role_name = Some(role_name.to_string());
        member.permissions = final_permissions;
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn remove_custom_role(&mut self, role_name: &str) -> Result<(), String> {
        // Check if it's a default role
        if role_name == "Owner" || role_name == "Member" {
            return Err("Cannot remove default roles".to_string());
        }

        // Check if any members are using this role
        let members_using_role: Vec<&str> = self
            .members
            .iter()
            .filter(|m| m.custom_role_name.as_ref() == Some(&role_name.to_string()))
            .map(|m| m.member_id.as_str())
            .collect();

        if !members_using_role.is_empty() {
            return Err(format!(
                "Cannot remove role '{}' - it is assigned to members: {}",
                role_name,
                members_using_role.join(", ")
            ));
        }

        // Remove the role
        self.custom_roles.retain(|r| r.role_name != role_name);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn update_custom_role(
        &mut self,
        role_name: &str,
        new_permissions: Option<Vec<Permission>>,
        new_description: Option<String>,
        new_color: Option<String>,
    ) -> Result<(), String> {
        // Check if it's a default role
        if role_name == "Owner" || role_name == "Member" {
            return Err("Cannot update default roles".to_string());
        }

        // Find the role
        let role = self
            .custom_roles
            .iter_mut()
            .find(|r| r.role_name == role_name)
            .ok_or_else(|| format!("Custom role '{role_name}' not found"))?;

        // Update fields if provided
        if let Some(permissions) = new_permissions {
            role.permissions = permissions;

            // Update permissions for all members with this role
            for member in self.members.iter_mut() {
                if member.custom_role_name.as_ref() == Some(&role_name.to_string()) {
                    member.permissions = role.permissions.clone();
                }
            }
        }

        if let Some(description) = new_description {
            role.description = description;
        }

        if let Some(color) = new_color {
            role.color = Some(color);
        }

        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn update_public_settings(&mut self, settings: PublicSettings) -> Result<(), String> {
        // Validate scheduled date if access mode is scheduled
        if let PublicAccessMode::Scheduled = settings.access_mode {
            if settings.scheduled_date.is_none() {
                return Err("Scheduled date is required for scheduled access mode".to_string());
            }
        }

        // Validate password if access mode is protected
        if let PublicAccessMode::Protected = settings.access_mode {
            if settings.access_password.is_none() {
                return Err("Password is required for protected access mode".to_string());
            }
        }

        self.public_settings = Some(settings);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn is_publicly_accessible(&self) -> bool {
        // First check if public visibility is enabled
        if !self.permissions.allow_public_visibility {
            return false;
        }

        // Check if public settings exist and determine accessibility
        if let Some(ref settings) = self.public_settings {
            match settings.access_mode {
                PublicAccessMode::Public => true,
                PublicAccessMode::Protected => true, // Accessible with password
                PublicAccessMode::Scheduled => {
                    if let Some(scheduled_date) = settings.scheduled_date {
                        Utc::now() >= scheduled_date
                    } else {
                        false
                    }
                }
            }
        } else {
            // Basic public visibility without advanced settings
            true
        }
    }

    pub fn get_public_info(&self) -> Option<PublicCircuitInfo> {
        if !self.is_publicly_accessible() {
            return None;
        }

        let settings = self.public_settings.as_ref();

        Some(PublicCircuitInfo {
            circuit_id: self.circuit_id,
            public_name: settings
                .and_then(|s| s.public_name.clone())
                .unwrap_or_else(|| self.name.clone()),
            public_description: settings.and_then(|s| s.public_description.clone()),
            primary_color: settings.and_then(|s| s.primary_color.clone()),
            secondary_color: settings.and_then(|s| s.secondary_color.clone()),
            logo_url: settings.and_then(|s| s.logo_url.clone()),
            tagline: settings.and_then(|s| s.tagline.clone()),
            footer_text: settings.and_then(|s| s.footer_text.clone()),
            member_count: self.members.len(),
            access_mode: settings
                .map(|s| s.access_mode.clone())
                .unwrap_or(PublicAccessMode::Public),
            requires_password: matches!(
                settings.map(|s| &s.access_mode),
                Some(PublicAccessMode::Protected)
            ),
            is_currently_accessible: self.is_publicly_accessible(),
            published_items: settings
                .map(|s| s.published_items.clone())
                .unwrap_or_default(),
            auto_publish_pushed_items: settings
                .map(|s| s.auto_publish_pushed_items)
                .unwrap_or(false),
            published_items_with_events: Vec::new(), // Will be populated by circuits_engine
        })
    }

    pub fn get_member_count_by_role(&self) -> std::collections::HashMap<String, usize> {
        let mut role_counts = std::collections::HashMap::new();

        for member in &self.members {
            let role_name = member
                .custom_role_name
                .as_deref()
                .unwrap_or(match member.role {
                    MemberRole::Owner => "Owner",
                    MemberRole::Admin => "Admin",
                    MemberRole::Member => "Member",
                    MemberRole::Viewer => "Viewer",
                });

            *role_counts.entry(role_name.to_string()).or_insert(0) += 1;
        }

        role_counts
    }
}

impl CircuitOperation {
    pub fn new(
        circuit_id: Uuid,
        dfid: String,
        operation_type: OperationType,
        requester_id: String,
    ) -> Self {
        Self {
            operation_id: Uuid::new_v4(),
            circuit_id,
            dfid,
            operation_type,
            requester_id,
            timestamp: Utc::now(),
            status: OperationStatus::Pending,
            metadata: HashMap::new(),
        }
    }

    pub fn approve(&mut self) {
        self.status = OperationStatus::Approved;
    }

    pub fn complete(&mut self) {
        self.status = OperationStatus::Completed;
    }

    pub fn fail(&mut self, reason: String) {
        self.status = OperationStatus::Failed;
        self.metadata.insert(
            "failure_reason".to_string(),
            serde_json::Value::String(reason),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRole {
    pub role_id: Uuid,
    pub circuit_id: Uuid,
    pub role_name: String,
    pub permissions: Vec<Permission>,
    pub description: String,
    pub color: Option<String>,
    pub created_timestamp: DateTime<Utc>,
    pub created_by: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoleType {
    Default(MemberRole),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemShare {
    pub share_id: String,
    pub dfid: String,
    pub shared_by: String,
    pub recipient_user_id: String,
    pub shared_at: DateTime<Utc>,
    pub permissions: Option<Vec<String>>,
    pub source_entry: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedItemResponse {
    pub share_id: String,
    pub item: Item,
    pub shared_by: String,
    pub shared_at: DateTime<Utc>,
    pub permissions: Option<Vec<String>>,
}

impl ItemShare {
    pub fn new(
        dfid: String,
        shared_by: String,
        recipient_user_id: String,
        permissions: Option<Vec<String>>,
    ) -> Self {
        Self {
            share_id: format!(
                "SHARE-{}-{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4().to_string()[0..8].to_uppercase()
            ),
            dfid,
            shared_by,
            recipient_user_id,
            shared_at: Utc::now(),
            permissions,
            source_entry: Uuid::new_v4(),
        }
    }
}

impl CustomRole {
    pub fn new(
        circuit_id: Uuid,
        role_name: String,
        permissions: Vec<Permission>,
        description: String,
        created_by: String,
    ) -> Self {
        Self {
            role_id: Uuid::new_v4(),
            circuit_id,
            role_name,
            permissions,
            description,
            color: None,
            created_timestamp: Utc::now(),
            created_by,
            is_default: false,
        }
    }

    pub fn default_member(circuit_id: Uuid) -> Self {
        Self {
            role_id: Uuid::new_v4(),
            circuit_id,
            role_name: "Member".to_string(),
            permissions: vec![Permission::Push, Permission::Pull],
            description: "Default member role".to_string(),
            color: Some("#gray".to_string()),
            created_timestamp: Utc::now(),
            created_by: "system".to_string(),
            is_default: true,
        }
    }

    pub fn default_owner(circuit_id: Uuid) -> Self {
        Self {
            role_id: Uuid::new_v4(),
            circuit_id,
            role_name: "Owner".to_string(),
            permissions: vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
                Permission::ManagePermissions,
                Permission::ManageRoles,
                Permission::Delete,
                Permission::Certify,
                Permission::Audit,
            ],
            description: "Circuit owner with full permissions".to_string(),
            color: Some("#gold".to_string()),
            created_timestamp: Utc::now(),
            created_by: "system".to_string(),
            is_default: true,
        }
    }

    pub fn set_color(&mut self, color: String) {
        self.color = Some(color);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    Push,
    Pull,
    Enrich,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStatus {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDetails {
    pub items_affected: usize,
    pub enrichments_made: Option<usize>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub activity_id: String,
    pub activity_type: ActivityType,
    pub circuit_id: Uuid,
    pub circuit_name: String,
    pub item_dfids: Vec<String>,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: ActivityStatus,
    pub details: ActivityDetails,
}

impl Activity {
    pub fn new(
        activity_type: ActivityType,
        circuit_id: Uuid,
        circuit_name: String,
        item_dfids: Vec<String>,
        user_id: String,
        status: ActivityStatus,
        details: ActivityDetails,
    ) -> Self {
        Self {
            activity_id: format!(
                "ACTIVITY-{}-{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .nth(0)
                    .unwrap()
                    .to_uppercase()
            ),
            activity_type,
            circuit_id,
            circuit_name,
            item_dfids,
            user_id,
            timestamp: Utc::now(),
            status,
            details,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitItem {
    pub dfid: String,
    pub circuit_id: Uuid,
    pub pushed_by: String,
    pub pushed_at: DateTime<Utc>,
    pub permissions: Vec<String>,
}

impl CircuitItem {
    pub fn new(
        dfid: String,
        circuit_id: Uuid,
        pushed_by: String,
        permissions: Vec<String>,
    ) -> Self {
        Self {
            dfid,
            circuit_id,
            pushed_by,
            pushed_at: Utc::now(),
            permissions,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStats {
    pub total_items: usize,
    pub unique_identifiers: usize,
    pub enrichable_fields: Vec<String>,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentMatch {
    pub item_dfid: String,
    pub enrichments_available: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPushResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub results: Vec<BatchPushItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPushItemResult {
    pub dfid: String,
    pub success: bool,
    pub error_message: Option<String>,
}

// ============================================================================
// AUDIT SYSTEM TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: Uuid,
    pub user_id: String,
    pub event_type: AuditEventType,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub outcome: AuditOutcome,
    pub severity: AuditSeverity,
    pub timestamp: DateTime<Utc>,
    pub details: HashMap<String, serde_json::Value>,
    pub metadata: AuditEventMetadata,
    pub signature: Option<String>,
    pub compliance: ComplianceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventType {
    Security,
    Data,
    Access,
    Compliance,
    System,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditOutcome {
    Success,
    Failure,
    Warning,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditEventMetadata {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub location: Option<String>,
    pub device_id: Option<String>,
    pub session_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplianceInfo {
    pub gdpr: Option<bool>,
    pub ccpa: Option<bool>,
    pub hipaa: Option<bool>,
    pub sox: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncident {
    pub incident_id: Uuid,
    pub title: String,
    pub description: String,
    pub severity: AuditSeverity,
    pub category: IncidentCategory,
    pub status: IncidentStatus,
    pub affected_users: Vec<String>,
    pub affected_resources: Vec<String>,
    pub related_event_ids: Vec<Uuid>,
    pub confidential: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentCategory {
    UnauthorizedAccess,
    DataBreach,
    SystemCompromise,
    PolicyViolation,
    DenialOfService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IncidentStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: Uuid,
    pub report_type: ComplianceReportType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub scope: ComplianceScope,
    pub export_format: ExportFormat,
    pub include_evidence: bool,
    pub status: ReportStatus,
    pub generated_at: Option<DateTime<Utc>>,
    pub file_path: Option<String>,
    pub findings: Vec<ComplianceFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceReportType {
    GdprDataSubject,
    CcpaConsumer,
    SoxFinancial,
    AuditTrail,
    SecurityIncident,
    FoodSafety,
    GDPR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    Pdf,
    Xml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportStatus {
    Pending,
    Generating,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScope {
    pub user_id: Option<String>,
    pub resource_types: Vec<String>,
    pub event_types: Vec<AuditEventType>,
    pub regulations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub finding_id: Uuid,
    pub finding_type: String,
    pub description: String,
    pub severity: AuditSeverity,
    pub evidence: Vec<Uuid>, // Event IDs
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDashboardMetrics {
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub total_events: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub events_last_24h: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub events_last_7d: u64,
    pub security_incidents: SecurityIncidentSummary,
    pub compliance_status: ComplianceStatus,
    pub top_users: Vec<UserRiskProfile>,
    pub anomalies: Vec<SecurityAnomaly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncidentSummary {
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub open: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub critical: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub resolved: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub gdpr_events: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub ccpa_events: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub hipaa_events: u64,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub sox_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRiskProfile {
    pub user_id: String,
    #[serde(with = "crate::safe_json_numbers::u64_safe")]
    pub event_count: u64,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAnomaly {
    pub anomaly_type: String,
    pub description: String,
    pub severity: AuditSeverity,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    pub user_id: Option<String>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub actions: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
    pub outcomes: Option<Vec<AuditOutcome>>,
    pub severities: Option<Vec<AuditSeverity>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub compliance: Option<ComplianceInfo>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<AuditSortBy>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSortBy {
    Timestamp,
    Severity,
    EventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

// Implementation blocks for audit types
impl AuditEvent {
    pub fn new(
        user_id: String,
        event_type: AuditEventType,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        severity: AuditSeverity,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            user_id,
            event_type,
            action,
            resource,
            resource_id: None,
            outcome,
            severity,
            timestamp: Utc::now(),
            details: HashMap::new(),
            metadata: AuditEventMetadata::default(),
            signature: None,
            compliance: ComplianceInfo::default(),
        }
    }

    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        self.details = details;
        self
    }

    pub fn with_metadata(mut self, metadata: AuditEventMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_compliance(mut self, compliance: ComplianceInfo) -> Self {
        self.compliance = compliance;
        self
    }

    pub fn add_detail(&mut self, key: String, value: serde_json::Value) {
        self.details.insert(key, value);
    }
}

impl SecurityIncident {
    pub fn new(
        title: String,
        description: String,
        severity: AuditSeverity,
        category: IncidentCategory,
    ) -> Self {
        Self {
            incident_id: Uuid::new_v4(),
            title,
            description,
            severity,
            category,
            status: IncidentStatus::Open,
            affected_users: Vec::new(),
            affected_resources: Vec::new(),
            related_event_ids: Vec::new(),
            confidential: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            assigned_to: None,
        }
    }

    pub fn add_affected_user(&mut self, user_id: String) {
        if !self.affected_users.contains(&user_id) {
            self.affected_users.push(user_id);
        }
        self.updated_at = Utc::now();
    }

    pub fn add_affected_resource(&mut self, resource: String) {
        if !self.affected_resources.contains(&resource) {
            self.affected_resources.push(resource);
        }
        self.updated_at = Utc::now();
    }

    pub fn add_related_event(&mut self, event_id: Uuid) {
        if !self.related_event_ids.contains(&event_id) {
            self.related_event_ids.push(event_id);
        }
        self.updated_at = Utc::now();
    }

    pub fn assign_to(&mut self, assignee: String) {
        self.assigned_to = Some(assignee);
        self.status = IncidentStatus::InProgress;
        self.updated_at = Utc::now();
    }

    pub fn resolve(&mut self) {
        self.status = IncidentStatus::Resolved;
        self.updated_at = Utc::now();
    }

    pub fn close(&mut self) {
        self.status = IncidentStatus::Closed;
        self.updated_at = Utc::now();
    }
}

impl ComplianceReport {
    pub fn new(
        report_type: ComplianceReportType,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scope: ComplianceScope,
        export_format: ExportFormat,
    ) -> Self {
        Self {
            report_id: Uuid::new_v4(),
            report_type,
            period_start,
            period_end,
            scope,
            export_format,
            include_evidence: false,
            status: ReportStatus::Pending,
            generated_at: None,
            file_path: None,
            findings: Vec::new(),
        }
    }

    pub fn start_generation(&mut self) {
        self.status = ReportStatus::Generating;
    }

    pub fn complete_generation(&mut self, file_path: String) {
        self.status = ReportStatus::Completed;
        self.generated_at = Some(Utc::now());
        self.file_path = Some(file_path);
    }

    pub fn fail_generation(&mut self) {
        self.status = ReportStatus::Failed;
    }

    pub fn add_finding(&mut self, finding: ComplianceFinding) {
        self.findings.push(finding);
    }
}

// ============================================================================
// ZK PROOF TYPES
// ============================================================================

// Re-export ZK proof types from the engine module
pub use crate::zk_proof_engine::{
    AgriculturalContext, CircuitInput, CircuitTemplate, CircuitType, ProofStatus,
    VerificationResult as ZkVerificationResult, ZkProof, ZkProofError,
};

// ============================================================================
// ADAPTER TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AdapterType {
    None,
    IpfsIpfs,
    StellarTestnetIpfs,
    StellarMainnetIpfs,
    EthereumGoerliIpfs,
    PolygonArweave,
    Custom(String),
}

impl Default for AdapterType {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for AdapterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterType::None => write!(f, "none"),
            AdapterType::IpfsIpfs => write!(f, "ipfs-ipfs"),
            AdapterType::StellarTestnetIpfs => write!(f, "stellar_testnet-ipfs"),
            AdapterType::StellarMainnetIpfs => write!(f, "stellar_mainnet-ipfs"),
            AdapterType::EthereumGoerliIpfs => write!(f, "ethereum_goerli-ipfs"),
            AdapterType::PolygonArweave => write!(f, "polygon-arweave"),
            AdapterType::Custom(name) => write!(f, "custom-{name}"),
        }
    }
}

impl AdapterType {
    pub fn from_string(s: &str) -> Result<Self, String> {
        let normalized = s.trim();
        match normalized {
            "none" | "None" => Ok(AdapterType::None),
            "ipfs-ipfs" | "IpfsIpfs" => Ok(AdapterType::IpfsIpfs),
            "stellar_testnet-ipfs" | "StellarTestnetIpfs" => Ok(AdapterType::StellarTestnetIpfs),
            "stellar_mainnet-ipfs" | "StellarMainnetIpfs" => Ok(AdapterType::StellarMainnetIpfs),
            "ethereum_goerli-ipfs" | "EthereumGoerliIpfs" => Ok(AdapterType::EthereumGoerliIpfs),
            "polygon-arweave" | "PolygonArweave" => Ok(AdapterType::PolygonArweave),
            custom if custom.starts_with("custom-") => {
                Ok(AdapterType::Custom(custom[7..].to_string()))
            }
            _ => Err(format!("Unknown adapter type: {s}")),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AdapterType::None => "No storage adapter - circuit data stays local only",
            AdapterType::IpfsIpfs => "Full IPFS storage - decentralized with no blockchain",
            AdapterType::StellarTestnetIpfs => {
                "Stellar testnet NFTs + IPFS events - for testing blockchain integration"
            }
            AdapterType::StellarMainnetIpfs => {
                "Stellar mainnet NFTs + IPFS events - production blockchain + IPFS"
            }
            AdapterType::EthereumGoerliIpfs => {
                "Ethereum Goerli testnet + IPFS - Ethereum ecosystem testing"
            }
            AdapterType::PolygonArweave => {
                "Polygon NFTs + Arweave permanent storage - low cost + permanence"
            }
            AdapterType::Custom(_) => "Custom adapter configuration",
        }
    }

    #[allow(clippy::match_like_matches_macro)]
    pub fn requires_blockchain(&self) -> bool {
        match self {
            AdapterType::None => false,
            AdapterType::IpfsIpfs => false,
            _ => true,
        }
    }

    pub fn storage_locations(&self) -> (StorageBackendType, StorageBackendType) {
        match self {
            AdapterType::None => (StorageBackendType::Local, StorageBackendType::Local),
            AdapterType::IpfsIpfs => (StorageBackendType::IPFS, StorageBackendType::IPFS),
            AdapterType::StellarTestnetIpfs => {
                (StorageBackendType::StellarTestnet, StorageBackendType::IPFS)
            }
            AdapterType::StellarMainnetIpfs => {
                (StorageBackendType::StellarMainnet, StorageBackendType::IPFS)
            }
            AdapterType::EthereumGoerliIpfs => {
                (StorageBackendType::EthereumGoerli, StorageBackendType::IPFS)
            }
            AdapterType::PolygonArweave => {
                (StorageBackendType::Polygon, StorageBackendType::Arweave)
            }
            AdapterType::Custom(_) => (StorageBackendType::Custom, StorageBackendType::Custom),
        }
    }
}

impl serde::Serialize for AdapterType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for AdapterType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        AdapterType::from_string(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageBackendType {
    Local,
    IPFS,
    StellarTestnet,
    StellarMainnet,
    EthereumMainnet,
    EthereumGoerli,
    Polygon,
    Arweave,
    Custom,
}

// Storage History Tracking System

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStorageHistory {
    pub dfid: String,
    pub storage_records: Vec<StorageRecord>,
    pub current_primary: Option<StorageLocation>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRecord {
    pub adapter_type: AdapterType,
    pub storage_location: StorageLocation,
    pub stored_at: DateTime<Utc>,
    pub triggered_by: String, // "item_creation", "circuit_push", "migration"
    pub triggered_by_id: Option<String>, // circuit_id, user_id, etc.
    pub events_range: Option<(DateTime<Utc>, Option<DateTime<Utc>>)>, // Which events are in this storage
    pub is_active: bool,
    pub metadata: HashMap<String, serde_json::Value>, // Additional storage-specific metadata
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemWithHistory {
    pub item: Item,
    pub events: Vec<Event>,
    pub storage_history: ItemStorageHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitAdapterConfig {
    pub circuit_id: Uuid,
    pub adapter_type: Option<AdapterType>, // None = use client default
    pub configured_by: String,
    pub configured_at: DateTime<Utc>,
    pub requires_approval: bool,
    pub auto_migrate_existing: bool, // Whether to migrate existing items when circuit adapter changes
    pub sponsor_adapter_access: bool, // When true, circuit sponsors adapter access for all members
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAdapterConfig {
    pub client_id: String,
    pub default_adapter: AdapterType,
    pub available_adapters: Vec<AdapterType>, // Based on tier
    pub circuit_overrides: HashMap<Uuid, AdapterType>,
    pub tier: String, // "basic", "professional", "enterprise"
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// WEBHOOK & POST-ACTION SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostActionSettings {
    pub enabled: bool,
    pub webhooks: Vec<WebhookConfig>,
    pub trigger_events: Vec<PostActionTrigger>,
    pub include_storage_details: bool,
    pub include_item_metadata: bool,
}

impl Default for PostActionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            webhooks: vec![],
            trigger_events: vec![PostActionTrigger::ItemPushed],
            include_storage_details: true,
            include_item_metadata: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub auth_type: WebhookAuthType,
    pub auth_credentials: Option<String>, // encrypted at rest
    pub enabled: bool,
    pub retry_config: RetryConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WebhookConfig {
    pub fn new(name: String, url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            url,
            method: HttpMethod::Post,
            headers: HashMap::new(),
            auth_type: WebhookAuthType::None,
            auth_credentials: None,
            enabled: true,
            retry_config: RetryConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    Post,
    Put,
    Patch,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookAuthType {
    None,
    BearerToken,
    ApiKey,
    BasicAuth,
    CustomHeader,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PostActionTrigger {
    ItemPushed,
    ItemApproved,
    ItemTokenized,
    ItemPublished,
}

impl PostActionTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            PostActionTrigger::ItemPushed => "item_pushed",
            PostActionTrigger::ItemApproved => "item_approved",
            PostActionTrigger::ItemTokenized => "item_tokenized",
            PostActionTrigger::ItemPublished => "item_published",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub circuit_id: Uuid,
    pub trigger_event: PostActionTrigger,
    pub payload: serde_json::Value,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
}

impl WebhookDelivery {
    pub fn new(
        webhook_id: Uuid,
        circuit_id: Uuid,
        trigger_event: PostActionTrigger,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            webhook_id,
            circuit_id,
            trigger_event,
            payload,
            status: DeliveryStatus::Pending,
            attempts: 0,
            response_code: None,
            response_body: None,
            created_at: Utc::now(),
            delivered_at: None,
            error_message: None,
            next_retry_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    InProgress,
    Delivered,
    Failed,
    Retrying,
}

// Webhook payload structure sent to configured endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub circuit_id: String,
    pub circuit_name: String,
    pub timestamp: DateTime<Utc>,
    pub item: WebhookItemData,
    pub storage: Option<WebhookStorageData>,
    pub operation_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookItemData {
    pub dfid: String,
    pub local_id: Option<String>,
    pub identifiers: Vec<HashMap<String, String>>,
    pub pushed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookStorageData {
    pub adapter_type: String,
    pub location: String,
    pub hash: String,
    pub cid: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

// ============================================================================
// USER MANAGEMENT & TIER SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UserTier {
    Basic,
    Professional,
    Enterprise,
    Admin,
}

impl UserTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserTier::Basic => "basic",
            UserTier::Professional => "professional",
            UserTier::Enterprise => "enterprise",
            UserTier::Admin => "admin",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "basic" => Ok(UserTier::Basic),
            "professional" => Ok(UserTier::Professional),
            "enterprise" => Ok(UserTier::Enterprise),
            "admin" => Ok(UserTier::Admin),
            _ => Err(format!("Invalid tier: {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Suspended,
    Banned,
    PendingVerification,
    TrialExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub tier: UserTier,
    pub status: AccountStatus,
    pub credits: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub subscription: Option<Subscription>,
    pub limits: TierLimits,
    pub is_admin: bool,
    pub workspace_id: Option<String>,
    pub available_adapters: Option<Vec<AdapterType>>, // None = use tier defaults
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subscription_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub auto_renew: bool,
    pub billing_cycle: BillingCycle,
    pub price_per_cycle: i64, // in cents
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Suspended,
    Trial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BillingCycle {
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierLimits {
    pub max_items_per_month: Option<i64>,
    pub max_circuits: Option<i64>,
    pub max_storage_locations: Option<i64>,
    pub max_api_requests_per_hour: Option<i64>,
    pub max_workspace_members: Option<i64>,
    pub available_adapters: Vec<AdapterType>,
    pub can_use_premium_adapters: bool,
    pub max_audit_retention_days: i64,
    pub priority_support: bool,
}

impl TierLimits {
    pub fn for_tier(tier: &UserTier) -> Self {
        match tier {
            UserTier::Basic => TierLimits {
                max_items_per_month: Some(1000),
                max_circuits: Some(5),
                max_storage_locations: Some(1),
                max_api_requests_per_hour: Some(100),
                max_workspace_members: Some(3),
                available_adapters: vec![AdapterType::IpfsIpfs],
                can_use_premium_adapters: false,
                max_audit_retention_days: 30,
                priority_support: false,
            },
            UserTier::Professional => TierLimits {
                max_items_per_month: Some(10000),
                max_circuits: Some(25),
                max_storage_locations: Some(3),
                max_api_requests_per_hour: Some(1000),
                max_workspace_members: Some(10),
                available_adapters: vec![AdapterType::IpfsIpfs, AdapterType::StellarTestnetIpfs],
                can_use_premium_adapters: false,
                max_audit_retention_days: 90,
                priority_support: true,
            },
            UserTier::Enterprise => TierLimits {
                max_items_per_month: None, // Unlimited
                max_circuits: None,
                max_storage_locations: None,
                max_api_requests_per_hour: Some(10000),
                max_workspace_members: None,
                available_adapters: vec![
                    AdapterType::IpfsIpfs,
                    AdapterType::StellarTestnetIpfs,
                    AdapterType::StellarMainnetIpfs,
                ],
                can_use_premium_adapters: true,
                max_audit_retention_days: 365,
                priority_support: true,
            },
            UserTier::Admin => TierLimits {
                max_items_per_month: None,
                max_circuits: None,
                max_storage_locations: None,
                max_api_requests_per_hour: None,
                max_workspace_members: None,
                available_adapters: vec![
                    AdapterType::IpfsIpfs,
                    AdapterType::StellarTestnetIpfs,
                    AdapterType::StellarMainnetIpfs,
                    AdapterType::EthereumGoerliIpfs,
                    AdapterType::PolygonArweave,
                ],
                can_use_premium_adapters: true,
                max_audit_retention_days: 3650, // 10 years
                priority_support: true,
            },
        }
    }
}

// ============================================================================
// CREDIT SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransaction {
    pub transaction_id: String,
    pub user_id: String,
    pub amount: i64, // Positive for credits added, negative for credits consumed
    pub transaction_type: CreditTransactionType,
    pub description: String,
    pub operation_type: Option<String>, // "item_creation", "circuit_push", etc.
    pub operation_id: Option<String>,   // Associated operation ID
    pub timestamp: DateTime<Utc>,
    pub balance_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditTransactionType {
    Purchase,     // User bought credits
    Grant,        // Admin granted credits
    Consumption,  // Credits used for operations
    Refund,       // Credits refunded
    Subscription, // Credits from subscription
    Penalty,      // Credits deducted as penalty
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditCosts {
    pub item_creation: i64,
    pub circuit_operation: i64,
    pub storage_migration: i64,
    pub audit_export: i64,
    pub premium_adapter_usage: i64,
    pub api_request: i64,
}

impl CreditCosts {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        CreditCosts {
            item_creation: 1,
            circuit_operation: 2,
            storage_migration: 5,
            audit_export: 10,
            premium_adapter_usage: 3,
            api_request: 0, // Free tier gets some API requests
        }
    }

    pub fn for_tier(tier: &UserTier) -> Self {
        match tier {
            UserTier::Basic => CreditCosts {
                item_creation: 2,
                circuit_operation: 3,
                storage_migration: 10,
                audit_export: 20,
                premium_adapter_usage: 10, // Expensive for basic users
                api_request: 1,
            },
            UserTier::Professional => CreditCosts {
                item_creation: 1,
                circuit_operation: 2,
                storage_migration: 5,
                audit_export: 10,
                premium_adapter_usage: 5,
                api_request: 0, // Free API requests
            },
            UserTier::Enterprise | UserTier::Admin => CreditCosts {
                item_creation: 1,
                circuit_operation: 1,
                storage_migration: 3,
                audit_export: 5,
                premium_adapter_usage: 2,
                api_request: 0,
            },
        }
    }
}

// ============================================================================
// USAGE TRACKING & ANALYTICS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub user_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub items_created: i64,
    pub circuits_used: i64,
    pub storage_operations: i64,
    pub api_requests: i64,
    pub credits_consumed: i64,
    pub adapter_usage: HashMap<AdapterType, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatistics {
    pub total_users: i64,
    pub active_users_24h: i64,
    pub active_users_30d: i64,
    pub total_items: i64,
    pub total_circuits: i64,
    pub total_storage_operations: i64,
    pub credits_consumed_24h: i64,
    pub tier_distribution: HashMap<UserTier, i64>,
    pub adapter_usage_stats: HashMap<AdapterType, i64>,
    pub generated_at: DateTime<Utc>,
}

// ============================================================================
// NOTIFICATION SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    JoinRequestReceived,
    JoinRequestApproved,
    JoinRequestRejected,
    CircuitInvite,
    ItemShared,
    MemberAdded,
    MemberRemoved,
    RoleChanged,
    CircuitUpdated,
    AccountUpdated,
    CreditsAdjusted,
    AccountFrozen,
    AccountUnfrozen,
    AdaptersUpdated,
    CircuitAdapterConfigUpdated,
    CircuitItemPendingApproval,
    CircuitItemApproved,
    CircuitItemRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

impl Notification {
    pub fn new(
        user_id: String,
        notification_type: NotificationType,
        title: String,
        message: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: format!(
                "NOTIF-{}-{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4().to_string()[..8].to_uppercase()
            ),
            user_id,
            notification_type,
            title,
            message,
            read: false,
            timestamp: Utc::now(),
            data,
        }
    }

    pub fn mark_as_read(&mut self) {
        self.read = true;
    }
}

// ============================================================================
// ADMIN SYSTEM
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminRole {
    SuperAdmin,       // Full system access
    UserManager,      // User management only
    ContentModerator, // Content oversight
    SystemMonitor,    // Read-only system access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAction {
    pub action_id: String,
    pub admin_user_id: String,
    pub action_type: AdminActionType,
    pub target_user_id: Option<String>,
    pub target_resource_id: Option<String>,
    pub details: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminActionType {
    UserCreated,
    UserUpdated,
    UserDeleted,
    UserSuspended,
    UserBanned,
    UserUnbanned,
    TierChanged,
    CreditsAdded,
    CreditsRemoved,
    CreditsAdjusted,
    BulkCreditsGranted,
    SystemConfigChanged,
    DataExported,
    WorkspaceDeleted,
    CircuitDeleted,
    SecurityIncidentResolved,
    AdapterCreated,
    AdapterUpdated,
    AdapterDeleted,
}

// ============================================================================
// Adapter Configuration Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub config_id: Uuid,
    pub name: String,
    pub description: String,
    pub adapter_type: AdapterType,
    pub connection_details: AdapterConnectionDetails,
    pub contract_configs: Option<ContractConfigs>,
    pub is_active: bool,
    pub is_default: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub test_status: Option<TestStatus>,
}

impl AdapterConfig {
    pub fn new(
        name: String,
        description: String,
        adapter_type: AdapterType,
        connection_details: AdapterConnectionDetails,
        created_by: String,
    ) -> Self {
        Self {
            config_id: Uuid::new_v4(),
            name,
            description,
            adapter_type,
            connection_details,
            contract_configs: None,
            is_active: true,
            is_default: false,
            created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_tested_at: None,
            test_status: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConnectionDetails {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub secret_key: Option<String>,
    pub auth_type: AuthType,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
    pub max_concurrent_requests: u32,
    pub custom_headers: HashMap<String, String>,
}

impl Default for AdapterConnectionDetails {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: None,
            secret_key: None,
            auth_type: AuthType::None,
            timeout_ms: 30000,
            retry_attempts: 3,
            max_concurrent_requests: 10,
            custom_headers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    None,
    ApiKey,
    Bearer,
    BasicAuth,
    OAuth2,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfigs {
    pub mint_contract: Option<ContractInfo>,
    pub ipcm_contract: Option<ContractInfo>,
    pub network: String,
    pub chain_id: Option<String>,
}

impl ContractConfigs {
    pub fn new(network: String) -> Self {
        Self {
            mint_contract: None,
            ipcm_contract: None,
            network,
            chain_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub contract_address: String,
    pub contract_name: String,
    pub abi: Option<String>,
    pub methods: HashMap<String, MethodConfig>,
}

impl ContractInfo {
    pub fn new(contract_address: String, contract_name: String) -> Self {
        Self {
            contract_address,
            contract_name,
            abi: None,
            methods: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodConfig {
    pub method_name: String,
    pub description: String,
    pub parameters: Vec<ParameterMapping>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMapping {
    pub param_name: String,
    pub param_type: String,
    pub source: ParameterSource,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterSource {
    Static(String),
    FromDfid,
    FromItem(String),
    FromEvent(String),
    FromCircuit(String),
    FromTimestamp,
    FromUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Warning,
    NotTested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterTestResult {
    pub config_id: Uuid,
    pub tested_at: DateTime<Utc>,
    pub status: TestStatus,
    pub connection_test: ConnectionTestResult,
    pub contract_tests: Vec<ContractTestResult>,
    pub error_message: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub endpoint_reachable: bool,
    pub authentication_valid: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTestResult {
    pub contract_type: String,
    pub contract_address: String,
    pub is_valid: bool,
    pub methods_verified: Vec<String>,
    pub error: Option<String>,
}

// ============================================================================
// CID Timeline Types
// ============================================================================

/// Timeline entry representing a single CID version for an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub dfid: String,
    pub cid: String,
    pub event_sequence: i32,
    pub blockchain_timestamp: i64,
    pub ipcm_transaction_hash: String,
    pub network: String,
    pub created_at: DateTime<Utc>,
}

/// Maps an event to the CID where it first appeared
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCidMapping {
    pub id: Uuid,
    pub event_id: Uuid,
    pub dfid: String,
    pub first_cid: String,
    pub appeared_in_sequence: i32,
    pub created_at: DateTime<Utc>,
}

/// Tracks blockchain event indexing progress per network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingProgress {
    pub network: String,
    pub last_indexed_ledger: i64,
    pub last_confirmed_ledger: i64,
    pub last_indexed_at: DateTime<Utc>,
    pub status: String,
    pub error_message: Option<String>,
    pub total_events_indexed: i64,
    pub last_error_at: Option<DateTime<Utc>>,
}

// ============================================================================
// User Activity Tracking System
// ============================================================================

/// Activity action type for user actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum UserActivityType {
    Create,
    Read,
    Update,
    Delete,
    Login,
    Logout,
    Export,
    Import,
    Share,
    Approve,
    Reject,
}

/// Activity category for grouping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum UserActivityCategory {
    Authentication,
    Items,
    Circuits,
    Events,
    Admin,
    ApiKeys,
    Workspaces,
}

/// Resource type for the activity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum UserResourceType {
    Item,
    Circuit,
    Event,
    ApiKey,
    User,
    Workspace,
    Adapter,
}

/// User activity record - tracks all user actions in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    pub activity_id: String,
    pub user_id: String,
    pub workspace_id: String,
    pub timestamp: DateTime<Utc>,
    pub activity_type: UserActivityType,
    pub category: UserActivityCategory,
    pub resource_type: UserResourceType,
    pub resource_id: String,
    pub action: String,
    pub description: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub success: bool,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl UserActivity {
    pub fn new(
        user_id: String,
        workspace_id: String,
        activity_type: UserActivityType,
        category: UserActivityCategory,
        resource_type: UserResourceType,
        resource_id: String,
        action: String,
        description: String,
    ) -> Self {
        Self {
            activity_id: format!(
                "ACT-{}-{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap()
                    .to_uppercase()
            ),
            user_id,
            workspace_id,
            timestamp: Utc::now(),
            activity_type,
            category,
            resource_type,
            resource_id,
            action,
            description,
            metadata: serde_json::Value::Null,
            success: true,
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn with_ip_address(mut self, ip_address: String) -> Self {
        self.ip_address = Some(ip_address);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
}

/// Activity query filters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserActivityFilters {
    pub category: Option<UserActivityCategory>,
    pub activity_type: Option<UserActivityType>,
    pub resource_type: Option<UserResourceType>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub search_query: Option<String>,
    pub user_id: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Activity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivityStats {
    pub total_actions: usize,
    pub by_category: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub most_active_hours: Vec<(usize, usize)>,
    pub success_rate: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Activity list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivityListResponse {
    pub activities: Vec<UserActivity>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}
