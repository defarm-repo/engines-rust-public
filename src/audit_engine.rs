use crate::storage::{StorageBackend, StorageError};
use crate::types::{
    AuditEvent, AuditEventType, AuditOutcome, AuditSeverity, AuditEventMetadata, ComplianceInfo,
    SecurityIncident, IncidentCategory, ComplianceReport, ComplianceReportType,
    ComplianceScope, ExportFormat, AuditQuery, AuditDashboardMetrics
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub enum AuditError {
    StorageError(StorageError),
    ValidationError(String),
    ProcessingError(String),
}

impl From<StorageError> for AuditError {
    fn from(err: StorageError) -> Self {
        AuditError::StorageError(err)
    }
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::StorageError(e) => write!(f, "Storage error: {}", e),
            AuditError::ValidationError(e) => write!(f, "Validation error: {}", e),
            AuditError::ProcessingError(e) => write!(f, "Processing error: {}", e),
        }
    }
}

impl std::error::Error for AuditError {}

#[derive(Clone)]
pub struct AuditEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
}

impl<S: StorageBackend> AuditEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self { storage }
    }

    pub fn get_storage(&self) -> &Arc<std::sync::Mutex<S>> {
        &self.storage
    }

    // Core audit event logging
    pub fn log_event(
        &self,
        user_id: String,
        event_type: AuditEventType,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        severity: AuditSeverity,
        details: Option<HashMap<String, serde_json::Value>>,
        metadata: Option<AuditEventMetadata>,
        compliance: Option<ComplianceInfo>,
    ) -> Result<Uuid, AuditError> {
        let mut event = AuditEvent::new(user_id, event_type, action, resource, outcome, severity);

        if let Some(details) = details {
            event = event.with_details(details);
        }

        if let Some(metadata) = metadata {
            event = event.with_metadata(metadata);
        }

        if let Some(compliance) = compliance {
            event = event.with_compliance(compliance);
        }

        let event_id = event.event_id;
        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_audit_event(&event)?;

        Ok(event_id)
    }

    // Security event logging with automatic incident creation for critical events
    pub fn log_security_event(
        &self,
        user_id: String,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        severity: AuditSeverity,
        details: HashMap<String, serde_json::Value>,
        metadata: Option<AuditEventMetadata>,
    ) -> Result<(Uuid, Option<Uuid>), AuditError> {
        let event_id = self.log_event(
            user_id.clone(),
            AuditEventType::Security,
            action.clone(),
            resource.clone(),
            outcome.clone(),
            severity.clone(),
            Some(details.clone()),
            metadata,
            None,
        )?;

        // Create security incident for critical security events or failures
        let incident_id = if matches!(severity, AuditSeverity::Critical) ||
                            matches!(outcome, AuditOutcome::Failure | AuditOutcome::Blocked) {
            let category = self.determine_incident_category(&action, &details);
            let title = format!("Security Event: {} on {}", action, resource);
            let description = format!(
                "Automatic incident created for {} security event. User: {}, Resource: {}, Outcome: {:?}",
                severity_to_string(&severity), user_id, resource, outcome
            );

            let mut incident = SecurityIncident::new(title, description, severity, category);
            incident.add_affected_user(user_id);
            incident.add_affected_resource(resource);
            incident.add_related_event(event_id);

            let incident_id = incident.incident_id;
            self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_security_incident(&incident)?;
            Some(incident_id)
        } else {
            None
        };

        Ok((event_id, incident_id))
    }

    // Data access logging with compliance tracking
    pub fn log_data_access(
        &self,
        user_id: String,
        action: String,
        resource: String,
        resource_id: Option<String>,
        outcome: AuditOutcome,
        data_classification: Option<String>,
        compliance_flags: Option<ComplianceInfo>,
        metadata: Option<AuditEventMetadata>,
    ) -> Result<Uuid, AuditError> {
        let severity = match data_classification.as_deref() {
            Some("confidential") | Some("restricted") => AuditSeverity::High,
            Some("internal") => AuditSeverity::Medium,
            _ => AuditSeverity::Low,
        };

        let mut details = HashMap::new();
        if let Some(classification) = data_classification {
            details.insert("data_classification".to_string(), serde_json::Value::String(classification));
        }
        if let Some(resource_id) = &resource_id {
            details.insert("resource_id".to_string(), serde_json::Value::String(resource_id.clone()));
        }

        let mut event = AuditEvent::new(user_id, AuditEventType::Data, action, resource, outcome, severity);
        event = event.with_details(details);

        if let Some(resource_id) = resource_id {
            event = event.with_resource_id(resource_id);
        }

        if let Some(metadata) = metadata {
            event = event.with_metadata(metadata);
        }

        if let Some(compliance) = compliance_flags {
            event = event.with_compliance(compliance);
        }

        let event_id = event.event_id;
        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_audit_event(&event)?;

        Ok(event_id)
    }

    // User activity logging
    pub fn log_user_activity(
        &self,
        user_id: String,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        session_info: Option<HashMap<String, serde_json::Value>>,
        metadata: Option<AuditEventMetadata>,
    ) -> Result<Uuid, AuditError> {
        let severity = match outcome {
            AuditOutcome::Failure | AuditOutcome::Blocked => AuditSeverity::Medium,
            _ => AuditSeverity::Low,
        };

        let details = session_info.unwrap_or_default();

        self.log_event(
            user_id,
            AuditEventType::User,
            action,
            resource,
            outcome,
            severity,
            Some(details),
            metadata,
            None,
        )
    }

    // System events logging
    pub fn log_system_event(
        &self,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        severity: AuditSeverity,
        system_details: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Uuid, AuditError> {
        self.log_event(
            "system".to_string(),
            AuditEventType::System,
            action,
            resource,
            outcome,
            severity,
            system_details,
            None,
            None,
        )
    }

    // Compliance event logging
    pub fn log_compliance_event(
        &self,
        user_id: String,
        action: String,
        resource: String,
        outcome: AuditOutcome,
        compliance_type: &str,
        compliance_details: HashMap<String, serde_json::Value>,
        metadata: Option<AuditEventMetadata>,
    ) -> Result<Uuid, AuditError> {
        let mut compliance = ComplianceInfo::default();
        match compliance_type.to_lowercase().as_str() {
            "gdpr" => compliance.gdpr = Some(true),
            "ccpa" => compliance.ccpa = Some(true),
            "hipaa" => compliance.hipaa = Some(true),
            "sox" => compliance.sox = Some(true),
            _ => {}
        }

        let severity = AuditSeverity::Medium; // Compliance events are generally medium severity

        self.log_event(
            user_id,
            AuditEventType::Compliance,
            action,
            resource,
            outcome,
            severity,
            Some(compliance_details),
            metadata,
            Some(compliance),
        )
    }

    // Query audit events
    pub fn query_events(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.query_audit_events(query)?)
    }

    // Get events by user
    pub fn get_user_events(&self, user_id: &str) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_audit_events_by_user(user_id)?)
    }

    // Get events by type
    pub fn get_events_by_type(&self, event_type: AuditEventType) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_audit_events_by_type(event_type)?)
    }

    // Get events in time range
    pub fn get_events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_audit_events_in_time_range(start, end)?)
    }

    // Security incident management
    pub fn create_security_incident(
        &self,
        title: String,
        description: String,
        severity: AuditSeverity,
        category: IncidentCategory,
        affected_users: Vec<String>,
        affected_resources: Vec<String>,
        related_events: Vec<Uuid>,
    ) -> Result<Uuid, AuditError> {
        let mut incident = SecurityIncident::new(title, description, severity, category);

        for user in affected_users {
            incident.add_affected_user(user);
        }

        for resource in affected_resources {
            incident.add_affected_resource(resource);
        }

        for event_id in related_events {
            incident.add_related_event(event_id);
        }

        let incident_id = incident.incident_id;
        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_security_incident(&incident)?;

        Ok(incident_id)
    }

    pub fn assign_incident(&self, incident_id: &Uuid, assignee: String) -> Result<(), AuditError> {
        let mut storage = self.storage.lock()
        .map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?;
        if let Some(mut incident) = storage.get_security_incident(incident_id)? {
            incident.assign_to(assignee);
            storage.update_security_incident(&incident)?;
        }
        Ok(())
    }

    pub fn resolve_incident(&self, incident_id: &Uuid) -> Result<(), AuditError> {
        let mut storage = self.storage.lock()
        .map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?;
        if let Some(mut incident) = storage.get_security_incident(incident_id)? {
            incident.resolve();
            storage.update_security_incident(&incident)?;
        }
        Ok(())
    }

    pub fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_open_incidents()?)
    }

    // Dashboard and metrics
    pub fn get_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_audit_dashboard_metrics()?)
    }

    pub fn get_event_count_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u64, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.get_event_count_by_time_range(start, end)?)
    }

    // Compliance reporting
    pub fn create_compliance_report(
        &self,
        report_type: ComplianceReportType,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        scope: ComplianceScope,
        export_format: ExportFormat,
    ) -> Result<Uuid, AuditError> {
        let report = ComplianceReport::new(report_type, period_start, period_end, scope, export_format);
        let report_id = report.report_id;
        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_compliance_report(&report)?;
        Ok(report_id)
    }

    pub fn get_compliance_reports(&self) -> Result<Vec<ComplianceReport>, AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.list_compliance_reports()?)
    }

    // Enhanced compliance reporting methods
    pub fn generate_gdpr_compliance_report(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<ComplianceReport, AuditError> {
        let scope = ComplianceScope {
            user_id: None,
            resource_types: vec!["all".to_string()],
            event_types: vec![AuditEventType::Data, AuditEventType::Access],
            regulations: vec!["GDPR".to_string()],
        };
        let report = ComplianceReport::new(
            ComplianceReportType::GDPR,
            start_date,
            end_date,
            scope,
            ExportFormat::Json,
        );

        // Store the report for audit trail
        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_compliance_report(&report)?;
        Ok(report)
    }

    pub fn generate_food_safety_report(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<ComplianceReport, AuditError> {
        let scope = ComplianceScope {
            user_id: None,
            resource_types: vec!["supply_chain".to_string(), "food_items".to_string()],
            event_types: vec![AuditEventType::System, AuditEventType::Compliance],
            regulations: vec!["FDA_FSMA".to_string(), "EU_Food_Law".to_string()],
        };
        let report = ComplianceReport::new(
            ComplianceReportType::FoodSafety,
            start_date,
            end_date,
            scope,
            ExportFormat::Pdf,
        );

        self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.store_compliance_report(&report)?;
        Ok(report)
    }

    pub fn get_compliance_events_for_period(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<AuditEvent>, AuditError> {
        let all_events = self.get_events_in_range(start_date, end_date)?;
        let compliance_events: Vec<AuditEvent> = all_events
            .into_iter()
            .filter(|event| matches!(event.event_type, AuditEventType::Compliance) || event.compliance.gdpr.is_some() || event.compliance.ccpa.is_some() || event.compliance.hipaa.is_some() || event.compliance.sox.is_some())
            .collect();
        Ok(compliance_events)
    }

    pub fn get_security_incidents_for_compliance(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<SecurityIncident>, AuditError> {
        let all_incidents = self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.list_security_incidents()?;
        let filtered_incidents: Vec<SecurityIncident> = all_incidents
            .into_iter()
            .filter(|incident| incident.created_at >= start_date && incident.created_at <= end_date)
            .collect();
        Ok(filtered_incidents)
    }

    // Event synchronization for distributed systems
    pub fn sync_events(&self, events: Vec<AuditEvent>) -> Result<(), AuditError> {
        Ok(self.storage.lock().map_err(|_| AuditError::StorageError(StorageError::IoError("Storage mutex poisoned".to_string())))?.sync_audit_events(events)?)
    }

    // Helper methods
    fn determine_incident_category(&self, action: &str, _details: &HashMap<String, serde_json::Value>) -> IncidentCategory {
        let action_lower = action.to_lowercase();

        if action_lower.contains("breach") || action_lower.contains("leak") || action_lower.contains("exposure") {
            IncidentCategory::DataBreach
        } else if action_lower.contains("unauthorized") || action_lower.contains("intrusion") {
            IncidentCategory::UnauthorizedAccess
        } else if action_lower.contains("malware") || action_lower.contains("compromise") {
            IncidentCategory::SystemCompromise
        } else if action_lower.contains("policy") || action_lower.contains("violation") {
            IncidentCategory::PolicyViolation
        } else if action_lower.contains("dos") || action_lower.contains("denial") {
            IncidentCategory::DenialOfService
        } else {
            // Default to unauthorized access for unknown security events
            IncidentCategory::UnauthorizedAccess
        }
    }
}

// Helper function to convert severity to string
fn severity_to_string(severity: &AuditSeverity) -> &'static str {
    match severity {
        AuditSeverity::Low => "low",
        AuditSeverity::Medium => "medium",
        AuditSeverity::High => "high",
        AuditSeverity::Critical => "critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;

    #[test]
    fn test_log_event() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let audit_engine = AuditEngine::new(storage.clone());

        let event_id = audit_engine.log_event(
            "user123".to_string(),
            AuditEventType::Security,
            "login".to_string(),
            "authentication_system".to_string(),
            AuditOutcome::Success,
            AuditSeverity::Low,
            None,
            None,
            None,
        ).unwrap();

        assert!(!event_id.is_nil());

        // Verify the event was stored
        let events = audit_engine.get_user_events("user123").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event_id);
        assert_eq!(events[0].action, "login");
    }

    #[test]
    fn test_security_incident_creation() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let audit_engine = AuditEngine::new(storage.clone());

        let mut details = HashMap::new();
        details.insert("ip".to_string(), serde_json::Value::String("192.168.1.1".to_string()));

        // Log a critical security event
        let (event_id, incident_id) = audit_engine.log_security_event(
            "user123".to_string(),
            "unauthorized_access".to_string(),
            "admin_panel".to_string(),
            AuditOutcome::Blocked,
            AuditSeverity::Critical,
            details,
            None,
        ).unwrap();

        assert!(!event_id.is_nil());
        assert!(incident_id.is_some());

        // Verify incident was created
        let incidents = audit_engine.get_open_incidents().unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].incident_id, incident_id.unwrap());
        assert!(incidents[0].related_event_ids.contains(&event_id));
    }

    #[test]
    fn test_dashboard_metrics() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let audit_engine = AuditEngine::new(storage.clone());

        // Log some events
        audit_engine.log_event(
            "user1".to_string(),
            AuditEventType::User,
            "login".to_string(),
            "app".to_string(),
            AuditOutcome::Success,
            AuditSeverity::Low,
            None,
            None,
            None,
        ).unwrap();

        audit_engine.log_event(
            "user2".to_string(),
            AuditEventType::Data,
            "read".to_string(),
            "database".to_string(),
            AuditOutcome::Success,
            AuditSeverity::Medium,
            None,
            None,
            None,
        ).unwrap();

        let metrics = audit_engine.get_dashboard_metrics().unwrap();
        assert_eq!(metrics.total_events, 2);
        assert_eq!(metrics.events_last_24h, 2);
        assert_eq!(metrics.top_users.len(), 2);
    }
}