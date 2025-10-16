use crate::storage::StorageBackend;
use crate::types::{
    ConflictAnalysisResult, ConflictInfo, ConflictSeverity, ConflictType, Identifier,
    PendingReason, QualitySeverity, ResolutionStrategy, SuggestedAction,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ConflictDetectionEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
    #[allow(dead_code)]
    confidence_threshold: f64,
    similarity_threshold: f64,
    dfid_conflict_threshold: usize,
}

impl<S: StorageBackend> ConflictDetectionEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self {
            storage,
            confidence_threshold: 0.8,
            similarity_threshold: 0.85,
            dfid_conflict_threshold: 2,
        }
    }

    pub fn with_thresholds(
        storage: Arc<std::sync::Mutex<S>>,
        confidence_threshold: f64,
        similarity_threshold: f64,
        dfid_conflict_threshold: usize,
    ) -> Self {
        Self {
            storage,
            confidence_threshold,
            similarity_threshold,
            dfid_conflict_threshold,
        }
    }

    pub fn analyze_identifiers(&self, identifiers: &[Identifier]) -> ConflictAnalysisResult {
        if identifiers.is_empty() {
            return ConflictAnalysisResult {
                conflicts: vec![],
                severity: ConflictSeverity::High,
                can_auto_resolve: false,
                suggested_actions: vec![SuggestedAction {
                    action_type: "add_identifiers".to_string(),
                    description: "Add identifiers to enable processing".to_string(),
                    confidence: 0.9,
                    automated: false,
                }],
            };
        }

        let mut conflicts = Vec::new();

        // 1. Check for DFID conflicts
        if let Some(dfid_conflicts) = self.detect_dfid_conflicts(identifiers) {
            conflicts.extend(dfid_conflicts);
        }

        // 2. Check for duplicate detection ambiguity
        if let Some(duplicate_conflicts) = self.detect_duplicate_ambiguity(identifiers) {
            conflicts.extend(duplicate_conflicts);
        }

        // 3. Check for data quality issues
        if let Some(quality_conflicts) = self.detect_data_quality_issues(identifiers) {
            conflicts.extend(quality_conflicts);
        }

        // 4. Check for cross-system conflicts
        if let Some(cross_conflicts) = self.detect_cross_system_conflicts(identifiers) {
            conflicts.extend(cross_conflicts);
        }

        // Determine overall severity and resolution capability
        let severity = self.calculate_overall_severity(&conflicts);
        let can_auto_resolve = self.can_auto_resolve(&conflicts);
        let suggested_actions = self.generate_resolution_actions(&conflicts);

        ConflictAnalysisResult {
            conflicts,
            severity,
            can_auto_resolve,
            suggested_actions,
        }
    }

    fn detect_dfid_conflicts(&self, identifiers: &[Identifier]) -> Option<Vec<ConflictInfo>> {
        let storage = self.storage.lock().unwrap();
        let mut conflicts = Vec::new();
        let mut dfid_mappings: HashMap<String, Vec<String>> = HashMap::new();

        // For each identifier, find all DFIDs it maps to
        for identifier in identifiers {
            if let Ok(items) = storage.find_items_by_identifier(identifier) {
                let dfids: Vec<String> = items.into_iter().map(|item| item.dfid).collect();
                if !dfids.is_empty() {
                    dfid_mappings.insert(format!("{identifier:?}"), dfids);
                }
            }
        }

        // Check for conflicts where one identifier maps to multiple DFIDs
        for (identifier_str, dfids) in dfid_mappings {
            if dfids.len() >= self.dfid_conflict_threshold {
                // Find the actual identifier from the string representation
                let affected_identifier = identifiers
                    .iter()
                    .find(|id| format!("{id:?}") == identifier_str)
                    .cloned()
                    .unwrap_or_else(|| identifiers[0].clone());

                let conflict = ConflictInfo {
                    conflict_type: ConflictType::IdentifierDFIDMapping,
                    severity: ConflictSeverity::Critical,
                    description: format!(
                        "Identifier maps to {} different DFIDs: {:?}",
                        dfids.len(),
                        dfids
                    ),
                    affected_identifiers: vec![affected_identifier],
                    suggested_resolution: Some(ResolutionStrategy::ManualReview),
                    confidence: 0.95,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("conflicting_dfids".to_string(), serde_json::json!(dfids));
                        metadata.insert("dfid_count".to_string(), serde_json::json!(dfids.len()));
                        metadata
                    },
                };
                conflicts.push(conflict);
            }
        }

        if conflicts.is_empty() {
            None
        } else {
            Some(conflicts)
        }
    }

    fn detect_duplicate_ambiguity(&self, identifiers: &[Identifier]) -> Option<Vec<ConflictInfo>> {
        let storage = self.storage.lock().unwrap();
        let mut conflicts = Vec::new();
        let mut potential_matches = Vec::new();
        let mut similarity_scores = Vec::new();

        // Simple similarity detection based on identifier matching
        // In a real implementation, this would use more sophisticated algorithms
        for identifier in identifiers {
            if let Ok(items) = storage.find_items_by_identifier(identifier) {
                for item in items {
                    // Calculate similarity score (simplified)
                    let similarity = self.calculate_similarity_score(identifier, &item.identifiers);
                    if similarity > self.similarity_threshold && similarity < 1.0 {
                        potential_matches.push(item.dfid);
                        similarity_scores.push(similarity);
                    }
                }
            }
        }

        if potential_matches.len() > 1 {
            let conflict = ConflictInfo {
                conflict_type: ConflictType::DuplicateDetection,
                severity: ConflictSeverity::Medium,
                description: format!(
                    "Found {} potential duplicate matches with high similarity",
                    potential_matches.len()
                ),
                affected_identifiers: identifiers.to_vec(),
                suggested_resolution: Some(ResolutionStrategy::ManualReview),
                confidence: 0.75,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "potential_matches".to_string(),
                        serde_json::json!(potential_matches),
                    );
                    metadata.insert(
                        "similarity_scores".to_string(),
                        serde_json::json!(similarity_scores),
                    );
                    metadata
                },
            };
            conflicts.push(conflict);
        }

        if conflicts.is_empty() {
            None
        } else {
            Some(conflicts)
        }
    }

    fn detect_data_quality_issues(&self, identifiers: &[Identifier]) -> Option<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();

        for identifier in identifiers {
            // Check for invalid identifier patterns
            let quality_issues = self.validate_identifier_quality(identifier);

            for (issue_type, severity, details) in quality_issues {
                let conflict = ConflictInfo {
                    conflict_type: ConflictType::DataQuality,
                    severity: match severity {
                        QualitySeverity::Critical => ConflictSeverity::Critical,
                        QualitySeverity::High => ConflictSeverity::High,
                        QualitySeverity::Medium => ConflictSeverity::Medium,
                        QualitySeverity::Low => ConflictSeverity::Low,
                    },
                    description: format!("Data quality issue ({issue_type}): {details}"),
                    affected_identifiers: vec![identifier.clone()],
                    suggested_resolution: match severity {
                        QualitySeverity::Critical | QualitySeverity::High => {
                            Some(ResolutionStrategy::ManualReview)
                        }
                        _ => Some(ResolutionStrategy::AutoMerge),
                    },
                    confidence: 0.85,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("issue_type".to_string(), serde_json::json!(issue_type));
                        metadata.insert(
                            "severity".to_string(),
                            serde_json::json!(format!("{:?}", severity)),
                        );
                        metadata.insert("details".to_string(), serde_json::json!(details));
                        metadata
                    },
                };
                conflicts.push(conflict);
            }
        }

        if conflicts.is_empty() {
            None
        } else {
            Some(conflicts)
        }
    }

    fn detect_cross_system_conflicts(
        &self,
        _identifiers: &[Identifier],
    ) -> Option<Vec<ConflictInfo>> {
        // Placeholder for cross-system conflict detection
        // In a real implementation, this would check against external systems
        None
    }

    fn validate_identifier_quality(
        &self,
        identifier: &Identifier,
    ) -> Vec<(String, QualitySeverity, String)> {
        let mut issues = Vec::new();

        // Check for empty or malformed identifiers
        match identifier.key.as_str() {
            "dfid" => {
                if identifier.value.is_empty() {
                    issues.push((
                        "empty_dfid".to_string(),
                        QualitySeverity::Critical,
                        "DFID cannot be empty".to_string(),
                    ));
                } else if identifier.value.len() < 10 {
                    issues.push((
                        "short_dfid".to_string(),
                        QualitySeverity::Medium,
                        "DFID appears to be too short".to_string(),
                    ));
                }
            }
            "email" => {
                if !identifier.value.contains('@') || !identifier.value.contains('.') {
                    issues.push((
                        "invalid_email".to_string(),
                        QualitySeverity::High,
                        "Email format appears invalid".to_string(),
                    ));
                }
            }
            "phone" => {
                if identifier.value.len() < 7 {
                    issues.push((
                        "short_phone".to_string(),
                        QualitySeverity::Medium,
                        "Phone number appears too short".to_string(),
                    ));
                }
            }
            "ssn" => {
                if identifier.value.len() != 11 && identifier.value.len() != 9 {
                    // 111-11-1111 or 111111111
                    issues.push((
                        "invalid_ssn_length".to_string(),
                        QualitySeverity::High,
                        "SSN has invalid length".to_string(),
                    ));
                }
            }
            _ => {
                if identifier.key.is_empty() || identifier.value.is_empty() {
                    issues.push((
                        "empty_custom_identifier".to_string(),
                        QualitySeverity::High,
                        "Custom identifier has empty name or value".to_string(),
                    ));
                }
            }
        }

        issues
    }

    fn calculate_similarity_score(
        &self,
        _identifier: &Identifier,
        _existing_identifiers: &[Identifier],
    ) -> f64 {
        // Simplified similarity calculation
        // In a real implementation, this would use sophisticated string similarity algorithms
        0.5
    }

    fn calculate_overall_severity(&self, conflicts: &[ConflictInfo]) -> ConflictSeverity {
        if conflicts.is_empty() {
            return ConflictSeverity::None;
        }

        let max_severity = conflicts
            .iter()
            .map(|c| &c.severity)
            .max_by(|a, b| {
                let a_val = match a {
                    ConflictSeverity::None => 0,
                    ConflictSeverity::Low => 1,
                    ConflictSeverity::Medium => 2,
                    ConflictSeverity::High => 3,
                    ConflictSeverity::Critical => 4,
                };
                let b_val = match b {
                    ConflictSeverity::None => 0,
                    ConflictSeverity::Low => 1,
                    ConflictSeverity::Medium => 2,
                    ConflictSeverity::High => 3,
                    ConflictSeverity::Critical => 4,
                };
                a_val.cmp(&b_val)
            })
            .unwrap_or(&ConflictSeverity::None);

        max_severity.clone()
    }

    fn can_auto_resolve(&self, conflicts: &[ConflictInfo]) -> bool {
        conflicts.iter().all(|conflict| {
            matches!(
                conflict.severity,
                ConflictSeverity::None | ConflictSeverity::Low
            ) && conflict.suggested_resolution.as_ref().is_some_and(|res| {
                matches!(
                    res,
                    ResolutionStrategy::AutoMerge | ResolutionStrategy::SkipProcessing
                )
            })
        })
    }

    fn generate_resolution_actions(&self, conflicts: &[ConflictInfo]) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();

        if conflicts.is_empty() {
            actions.push(SuggestedAction {
                action_type: "proceed".to_string(),
                description: "No conflicts detected, proceed with processing".to_string(),
                confidence: 1.0,
                automated: true,
            });
            return actions;
        }

        // Group conflicts by type and suggest appropriate actions
        let has_critical = conflicts
            .iter()
            .any(|c| matches!(c.severity, ConflictSeverity::Critical));
        let has_dfid_conflicts = conflicts
            .iter()
            .any(|c| matches!(c.conflict_type, ConflictType::IdentifierDFIDMapping));
        let has_quality_issues = conflicts
            .iter()
            .any(|c| matches!(c.conflict_type, ConflictType::DataQuality));

        if has_critical || has_dfid_conflicts {
            actions.push(SuggestedAction {
                action_type: "manual_review".to_string(),
                description: "Critical conflicts detected, requires manual review".to_string(),
                confidence: 0.95,
                automated: false,
            });
        }

        if has_quality_issues {
            actions.push(SuggestedAction {
                action_type: "data_cleanup".to_string(),
                description: "Clean up data quality issues before processing".to_string(),
                confidence: 0.8,
                automated: true,
            });
        }

        // Add conflict-specific actions
        for conflict in conflicts {
            if let Some(resolution) = &conflict.suggested_resolution {
                let action_type = match resolution {
                    ResolutionStrategy::AutoMerge => "auto_merge",
                    ResolutionStrategy::ManualReview => "manual_review",
                    ResolutionStrategy::SkipProcessing => "skip_processing",
                    ResolutionStrategy::CreateSeparate => "create_separate",
                    ResolutionStrategy::MatchBest => "match_best",
                    _ => "review_required",
                };

                actions.push(SuggestedAction {
                    action_type: action_type.to_string(),
                    description: format!(
                        "Resolve {} conflict: {}",
                        format!("{:?}", conflict.conflict_type).to_lowercase(),
                        conflict.description
                    ),
                    confidence: conflict.confidence,
                    automated: matches!(
                        resolution,
                        ResolutionStrategy::AutoMerge | ResolutionStrategy::SkipProcessing
                    ),
                });
            }
        }

        actions
    }

    pub fn convert_to_pending_reason(
        &self,
        analysis: &ConflictAnalysisResult,
    ) -> Option<PendingReason> {
        if analysis.conflicts.is_empty() {
            return None;
        }

        // Convert the most severe conflict to a PendingReason
        let primary_conflict = analysis.conflicts.iter().max_by(|a, b| {
            let a_val = match a.severity {
                ConflictSeverity::None => 0,
                ConflictSeverity::Low => 1,
                ConflictSeverity::Medium => 2,
                ConflictSeverity::High => 3,
                ConflictSeverity::Critical => 4,
            };
            let b_val = match b.severity {
                ConflictSeverity::None => 0,
                ConflictSeverity::Low => 1,
                ConflictSeverity::Medium => 2,
                ConflictSeverity::High => 3,
                ConflictSeverity::Critical => 4,
            };
            a_val.cmp(&b_val)
        })?;

        match primary_conflict.conflict_type {
            ConflictType::IdentifierDFIDMapping => {
                if let Some(identifier) = primary_conflict.affected_identifiers.first() {
                    let conflicting_dfids = primary_conflict
                        .metadata
                        .get("conflicting_dfids")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    Some(PendingReason::ConflictingDFIDs {
                        identifier: identifier.clone(),
                        conflicting_dfids,
                        confidence_scores: None,
                    })
                } else {
                    Some(PendingReason::ValidationError(
                        "DFID mapping conflict with no identifiers".to_string(),
                    ))
                }
            }
            ConflictType::DuplicateDetection => {
                let potential_matches = primary_conflict
                    .metadata
                    .get("potential_matches")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let similarity_scores = primary_conflict
                    .metadata
                    .get("similarity_scores")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default();

                Some(PendingReason::DuplicateDetectionAmbiguous {
                    potential_matches,
                    similarity_scores,
                })
            }
            ConflictType::DataQuality => {
                let issue_type = primary_conflict
                    .metadata
                    .get("issue_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let severity = primary_conflict
                    .metadata
                    .get("severity")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Critical" => Some(QualitySeverity::Critical),
                        "High" => Some(QualitySeverity::High),
                        "Medium" => Some(QualitySeverity::Medium),
                        "Low" => Some(QualitySeverity::Low),
                        _ => None,
                    })
                    .unwrap_or(QualitySeverity::Medium);

                Some(PendingReason::DataQualityIssue {
                    issue_type,
                    severity,
                    details: primary_conflict.description.clone(),
                })
            }
            ConflictType::CrossSystem => Some(PendingReason::CrossSystemConflict {
                external_system: "unknown".to_string(),
                conflict_type: primary_conflict.description.clone(),
            }),
            ConflictType::ValidationFailure => Some(PendingReason::ValidationError(
                primary_conflict.description.clone(),
            )),
        }
    }
}
