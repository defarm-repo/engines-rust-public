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
    pub identifiers: Vec<Identifier>,
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
            identifiers,
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
    pub pending_requests: Vec<JoinRequest>,
    pub custom_roles: Vec<CustomRole>,
    pub public_settings: Option<PublicSettings>,
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
pub struct PublicCircuitInfo {
    pub circuit_id: Uuid,
    pub public_name: String,
    pub public_description: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub member_count: usize,
    pub access_mode: PublicAccessMode,
    pub requires_password: bool,
    pub is_currently_accessible: bool,
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
        let circuit_id = Uuid::new_v4();

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
            joined_timestamp: Utc::now(),
        };

        let default_roles = vec![
            CustomRole::default_owner(circuit_id),
            CustomRole::default_member(circuit_id),
        ];

        Self {
            circuit_id,
            name,
            description,
            owner_id,
            created_timestamp: Utc::now(),
            last_modified: Utc::now(),
            members: vec![owner_member],
            permissions: CircuitPermissions::default(),
            status: CircuitStatus::Active,
            pending_requests: Vec::new(),
            custom_roles: default_roles,
            public_settings: None,
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
        self.members.iter().any(|m| m.member_id == member_id)
    }

    pub fn has_pending_request(&self, requester_id: &str) -> bool {
        self.pending_requests.iter().any(|r| r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending))
    }

    pub fn add_join_request(&mut self, requester_id: String, message: Option<String>) -> Result<(), String> {
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

    pub fn approve_join_request(&mut self, requester_id: &str, role: MemberRole) -> Result<(), String> {
        // Find and update the request
        let request = self.pending_requests.iter_mut()
            .find(|r| r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending))
            .ok_or("No pending request found for this user")?;

        request.status = JoinRequestStatus::Approved;

        // Add the user as a member
        self.add_member(requester_id.to_string(), role);
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn reject_join_request(&mut self, requester_id: &str) -> Result<(), String> {
        let request = self.pending_requests.iter_mut()
            .find(|r| r.requester_id == requester_id && matches!(r.status, JoinRequestStatus::Pending))
            .ok_or("No pending request found for this user")?;

        request.status = JoinRequestStatus::Rejected;
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn get_pending_requests(&self) -> Vec<&JoinRequest> {
        self.pending_requests.iter()
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

    pub fn add_custom_role(&mut self, role_name: String, permissions: Vec<Permission>, description: String, color: Option<String>, created_by: String) -> Result<(), String> {
        // Check if role name already exists
        if self.custom_roles.iter().any(|r| r.role_name == role_name) {
            return Err(format!("Role '{}' already exists", role_name));
        }

        let mut custom_role = CustomRole::new(self.circuit_id, role_name, permissions, description, created_by);
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
        let custom_role_permissions = self.get_custom_role(role_name)
            .ok_or_else(|| format!("Custom role '{}' not found", role_name))?
            .permissions.clone();

        // Find member first to get the role, then update
        let member_role = self.members.iter()
            .find(|m| m.member_id == member_id)
            .ok_or_else(|| format!("Member '{}' not found", member_id))?
            .role;

        // Get base role permissions
        let base_role_permissions = match member_role {
            MemberRole::Owner => self.get_custom_role("Owner").map(|r| r.permissions.clone()).unwrap_or_else(|| {
                vec![Permission::Push, Permission::Pull, Permission::Invite, Permission::ManageMembers,
                     Permission::ManagePermissions, Permission::ManageRoles, Permission::Delete,
                     Permission::Certify, Permission::Audit]
            }),
            MemberRole::Admin => vec![Permission::Push, Permission::Pull, Permission::Invite, Permission::ManageMembers],
            MemberRole::Member => vec![Permission::Push, Permission::Pull],
            MemberRole::Viewer => vec![Permission::Pull],
        };

        // Combine base role permissions with custom role permissions
        let mut combined_permissions = base_role_permissions;
        for perm in custom_role_permissions {
            if !combined_permissions.contains(&perm) {
                combined_permissions.push(perm);
            }
        }

        // Now update the member
        let member = self.members.iter_mut()
            .find(|m| m.member_id == member_id)
            .unwrap(); // Safe because we already checked existence above

        member.custom_role_name = Some(role_name.to_string());
        member.permissions = combined_permissions;
        self.last_modified = Utc::now();
        Ok(())
    }

    pub fn remove_custom_role(&mut self, role_name: &str) -> Result<(), String> {
        // Check if it's a default role
        if role_name == "Owner" || role_name == "Member" {
            return Err("Cannot remove default roles".to_string());
        }

        // Check if any members are using this role
        let members_using_role: Vec<&str> = self.members.iter()
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
            public_name: settings.and_then(|s| s.public_name.as_ref().map(|n| n.clone())).unwrap_or_else(|| self.name.clone()),
            public_description: settings.and_then(|s| s.public_description.as_ref().map(|d| d.clone())),
            primary_color: settings.and_then(|s| s.primary_color.as_ref().map(|c| c.clone())),
            secondary_color: settings.and_then(|s| s.secondary_color.as_ref().map(|c| c.clone())),
            member_count: self.members.len(),
            access_mode: settings.map(|s| s.access_mode.clone()).unwrap_or(PublicAccessMode::Public),
            requires_password: matches!(
                settings.map(|s| &s.access_mode),
                Some(PublicAccessMode::Protected)
            ),
            is_currently_accessible: self.is_publicly_accessible(),
        })
    }

    pub fn get_member_count_by_role(&self) -> std::collections::HashMap<String, usize> {
        let mut role_counts = std::collections::HashMap::new();

        for member in &self.members {
            let role_name = member.custom_role_name.as_deref()
                .unwrap_or_else(|| match member.role {
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
    pub fn new(dfid: String, shared_by: String, recipient_user_id: String, permissions: Option<Vec<String>>) -> Self {
        Self {
            share_id: format!("SHARE-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), Uuid::new_v4().to_string()[0..8].to_uppercase()),
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
    pub fn new(circuit_id: Uuid, role_name: String, permissions: Vec<Permission>, description: String, created_by: String) -> Self {
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
            activity_id: format!("ACTIVITY-{}-{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4().to_string().split('-').nth(0).unwrap().to_uppercase()
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
    pub fn new(dfid: String, circuit_id: Uuid, pushed_by: String, permissions: Vec<String>) -> Self {
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