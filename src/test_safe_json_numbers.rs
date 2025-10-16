/// Test module for verifying safe JSON number serialization
#[cfg(test)]
mod tests {
    use crate::safe_json_numbers::JS_MAX_SAFE_INTEGER;
    use crate::types::{
        AuditDashboardMetrics, ComplianceStatus, SecurityIncidentSummary, UserRiskProfile,
    };

    #[test]
    fn test_audit_metrics_serialization() {
        let metrics = AuditDashboardMetrics {
            total_events: 1000,
            events_last_24h: JS_MAX_SAFE_INTEGER,
            events_last_7d: JS_MAX_SAFE_INTEGER + 1, // This should be serialized as string
            security_incidents: SecurityIncidentSummary {
                open: 5,
                critical: 2,
                resolved: JS_MAX_SAFE_INTEGER + 100, // This should be serialized as string
            },
            compliance_status: ComplianceStatus {
                gdpr_events: 100,
                ccpa_events: 200,
                hipaa_events: JS_MAX_SAFE_INTEGER,
                sox_events: u64::MAX, // This should definitely be serialized as string
            },
            top_users: vec![UserRiskProfile {
                user_id: "test_user".to_string(),
                event_count: JS_MAX_SAFE_INTEGER + 1, // This should be serialized as string
                risk_score: 0.75,
            }],
            anomalies: vec![],
        };

        let json_str = serde_json::to_string(&metrics).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Check that safe values are numbers
        assert_eq!(json["total_events"], 1000);
        assert_eq!(json["events_last_24h"], JS_MAX_SAFE_INTEGER);

        // Check that unsafe values are strings
        assert_eq!(
            json["events_last_7d"],
            serde_json::Value::String((JS_MAX_SAFE_INTEGER + 1).to_string())
        );
        assert_eq!(
            json["security_incidents"]["resolved"],
            serde_json::Value::String((JS_MAX_SAFE_INTEGER + 100).to_string())
        );
        assert_eq!(
            json["compliance_status"]["sox_events"],
            serde_json::Value::String(u64::MAX.to_string())
        );
        assert_eq!(
            json["top_users"][0]["event_count"],
            serde_json::Value::String((JS_MAX_SAFE_INTEGER + 1).to_string())
        );

        // Verify we can deserialize back
        let deserialized: AuditDashboardMetrics = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.total_events, metrics.total_events);
        assert_eq!(deserialized.events_last_7d, metrics.events_last_7d);
        assert_eq!(
            deserialized.compliance_status.sox_events,
            metrics.compliance_status.sox_events
        );
    }

    #[test]
    fn test_api_response_serialization() {
        use crate::api::admin::AdminDashboardStats;
        use chrono::Utc;
        use std::collections::HashMap;

        let mut users_by_tier = HashMap::new();
        users_by_tier.insert("free".to_string(), 100);
        users_by_tier.insert("premium".to_string(), JS_MAX_SAFE_INTEGER);
        users_by_tier.insert("enterprise".to_string(), JS_MAX_SAFE_INTEGER + 1);

        let stats = AdminDashboardStats {
            total_users: 1000,
            users_by_tier,
            users_by_status: HashMap::new(),
            total_credits_issued: i64::MAX, // This should be serialized as string
            total_credits_consumed: 5000,
            active_users_last_30_days: JS_MAX_SAFE_INTEGER,
            new_users_last_30_days: 50,
            generated_at: Utc::now(),
        };

        let json_str = serde_json::to_string(&stats).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Check that safe values are numbers
        assert_eq!(json["total_users"], 1000);
        assert_eq!(json["total_credits_consumed"], 5000);
        assert_eq!(json["new_users_last_30_days"], 50);

        // Check that unsafe values are strings
        assert_eq!(
            json["total_credits_issued"],
            serde_json::Value::String(i64::MAX.to_string())
        );

        // Note: HashMap values aren't automatically converted, only the direct fields
        // This is intentional as HashMap<String, u64> would need custom serialization
    }

    #[test]
    fn test_edge_cases() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            #[serde(with = "crate::safe_json_numbers::u64_safe")]
            value: u64,
        }

        // Test exact boundary value
        let test = TestStruct {
            value: JS_MAX_SAFE_INTEGER,
        };
        let json = serde_json::to_string(&test).unwrap();
        assert!(!json.contains("\"9007199254740991\"")); // Should be a number, not a string
        assert!(json.contains("9007199254740991"));

        // Test boundary + 1
        let test = TestStruct {
            value: JS_MAX_SAFE_INTEGER + 1,
        };
        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("\"9007199254740992\"")); // Should be a string

        // Test zero
        let test = TestStruct { value: 0 };
        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("0"));
    }
}
