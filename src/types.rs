use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub key: String,
    pub value: String,
}

impl Identifier {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub canonical_identifiers: Vec<Identifier>,
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
}

impl DataLakeEntry {
    pub fn new(receipt_id: Uuid, identifiers: Vec<Identifier>, data_hash: String, data_size: usize) -> Self {
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
            canonical_identifiers: identifiers,
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
            if !self.canonical_identifiers.contains(&identifier) {
                self.canonical_identifiers.push(identifier);
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub circuit_id: Uuid,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub created_timestamp: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub members: Vec<CircuitMember>,
    pub permissions: CircuitPermissions,
    pub status: CircuitStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitMember {
    pub member_id: String,
    pub role: MemberRole,
    pub permissions: Vec<Permission>,
    pub joined_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitPermissions {
    pub default_push: bool,
    pub default_pull: bool,
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
    pub fn new(dfid: String, event_type: EventType, source: String, visibility: EventVisibility) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            dfid,
            event_type,
            timestamp: Utc::now(),
            source,
            metadata: HashMap::new(),
            is_encrypted: false,
            visibility,
        }
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    pub fn encrypt(&mut self) {
        self.is_encrypted = true;
    }
}

impl Circuit {
    pub fn new(name: String, description: String, owner_id: String) -> Self {
        let owner_member = CircuitMember {
            member_id: owner_id.clone(),
            role: MemberRole::Owner,
            permissions: vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
                Permission::ManagePermissions,
                Permission::Delete,
            ],
            joined_timestamp: Utc::now(),
        };

        Self {
            circuit_id: Uuid::new_v4(),
            name,
            description,
            owner_id,
            created_timestamp: Utc::now(),
            last_modified: Utc::now(),
            members: vec![owner_member],
            permissions: CircuitPermissions::default(),
            status: CircuitStatus::Active,
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
                Permission::Delete,
            ],
            MemberRole::Admin => vec![
                Permission::Push,
                Permission::Pull,
                Permission::Invite,
                Permission::ManageMembers,
            ],
            MemberRole::Member => vec![Permission::Push, Permission::Pull],
            MemberRole::Viewer => vec![Permission::Pull],
        };

        let member = CircuitMember {
            member_id,
            role,
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
}

impl Default for CircuitPermissions {
    fn default() -> Self {
        Self {
            default_push: false,
            default_pull: true,
            require_approval_for_push: true,
            require_approval_for_pull: false,
            allow_public_visibility: false,
        }
    }
}

impl CircuitOperation {
    pub fn new(circuit_id: Uuid, dfid: String, operation_type: OperationType, requester_id: String) -> Self {
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
        self.metadata.insert("failure_reason".to_string(), serde_json::Value::String(reason));
    }
}