use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Namespaces padrão do sistema
pub mod namespaces {
    pub const BOVINO: &str = "bovino";
    pub const AVES: &str = "aves";
    pub const SUINO: &str = "suino";
    pub const SOJA: &str = "soja";
    pub const MILHO: &str = "milho";
    pub const ALGODAO: &str = "algodao";
    pub const CAFE: &str = "cafe";
    pub const LEITE: &str = "leite";
    pub const GENERIC: &str = "generic";

    pub fn is_valid(namespace: &str) -> bool {
        matches!(
            namespace,
            "bovino"
                | "aves"
                | "suino"
                | "soja"
                | "milho"
                | "algodao"
                | "cafe"
                | "leite"
                | "generic"
        )
    }

    pub fn all() -> Vec<&'static str> {
        vec![
            BOVINO, AVES, SUINO, SOJA, MILHO, ALGODAO, CAFE, LEITE, GENERIC,
        ]
    }
}

// Registries canônicos reconhecidos
pub mod registries {
    pub const SISBOV: &str = "sisbov";
    pub const CPF: &str = "cpf";
    pub const CNPJ: &str = "cnpj";
    pub const CAR: &str = "car";
    pub const NIRF: &str = "nirf";
    pub const IE: &str = "ie";
    pub const RFID: &str = "rfid";

    pub fn validate(registry: &str, value: &str) -> bool {
        match registry {
            "sisbov" => validate_sisbov(value),
            "cpf" => validate_cpf(value),
            "cnpj" => validate_cnpj(value),
            "car" => validate_car(value),
            "nirf" => validate_nirf(value),
            "ie" => value.len() >= 8,    // Basic validation
            "rfid" => value.len() >= 10, // Basic validation
            _ => true,
        }
    }

    fn validate_sisbov(value: &str) -> bool {
        value.starts_with("BR") && value.len() == 14 && value[2..].chars().all(char::is_numeric)
    }

    fn validate_cpf(cpf: &str) -> bool {
        let cpf = cpf.chars().filter(|c| c.is_numeric()).collect::<String>();
        if cpf.len() != 11 {
            return false;
        }

        // Basic CPF validation (could be enhanced with checksum)
        !cpf.chars().all(|c| c == cpf.chars().next().unwrap())
    }

    fn validate_cnpj(cnpj: &str) -> bool {
        let cnpj = cnpj.chars().filter(|c| c.is_numeric()).collect::<String>();
        if cnpj.len() != 14 {
            return false;
        }

        // Basic CNPJ validation (could be enhanced with checksum)
        !cnpj.chars().all(|c| c == cnpj.chars().next().unwrap())
    }

    fn validate_car(value: &str) -> bool {
        value.starts_with("BR-") && value.len() >= 9 && value.len() <= 44
    }

    fn validate_nirf(value: &str) -> bool {
        let numbers = value.chars().filter(|c| c.is_numeric()).collect::<String>();
        numbers.len() >= 8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnhancedIdentifier {
    pub namespace: String,
    pub key: String,
    pub value: String,
    pub id_type: IdentifierType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IdentifierType {
    Canonical {
        registry: String,
        verified: bool,
        verification_date: Option<DateTime<Utc>>,
    },
    Contextual {
        scope: String, // "user", "organization", "circuit"
    },
}

impl EnhancedIdentifier {
    pub fn canonical(namespace: &str, registry: &str, value: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: registry.to_string(),
            value: value.to_string(),
            id_type: IdentifierType::Canonical {
                registry: registry.to_string(),
                verified: false,
                verification_date: None,
            },
        }
    }

    pub fn canonical_verified(
        namespace: &str,
        registry: &str,
        value: &str,
        verification_date: DateTime<Utc>,
    ) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: registry.to_string(),
            value: value.to_string(),
            id_type: IdentifierType::Canonical {
                registry: registry.to_string(),
                verified: true,
                verification_date: Some(verification_date),
            },
        }
    }

    pub fn contextual(namespace: &str, key: &str, value: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            id_type: IdentifierType::Contextual {
                scope: "user".to_string(),
            },
        }
    }

    pub fn contextual_with_scope(namespace: &str, key: &str, value: &str, scope: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            id_type: IdentifierType::Contextual {
                scope: scope.to_string(),
            },
        }
    }

    /// Generate unique key for lookups
    pub fn unique_key(&self) -> String {
        format!("{}:{}:{}", self.namespace, self.key, self.value)
    }

    /// Check if this is a canonical identifier
    pub fn is_canonical(&self) -> bool {
        matches!(self.id_type, IdentifierType::Canonical { .. })
    }

    /// Check if this is a contextual identifier
    pub fn is_contextual(&self) -> bool {
        matches!(self.id_type, IdentifierType::Contextual { .. })
    }

    /// Get the registry name if this is a canonical identifier
    pub fn get_registry(&self) -> Option<String> {
        match &self.id_type {
            IdentifierType::Canonical { registry, .. } => Some(registry.clone()),
            _ => None,
        }
    }

    /// Validate the identifier format
    pub fn validate(&self) -> bool {
        // Validate namespace
        if !namespaces::is_valid(&self.namespace) {
            return false;
        }

        // Validate key and value are not empty
        if self.key.trim().is_empty() || self.value.trim().is_empty() {
            return false;
        }

        // If canonical, validate against registry rules
        if let IdentifierType::Canonical { ref registry, .. } = self.id_type {
            registries::validate(registry, &self.value)
        } else {
            // Contextual identifiers are valid if non-empty
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAlias {
    pub scheme: String, // "certA:animal", "erp:lote", "user:123"
    pub value: String,
    pub issuer_id: String, // user_id or api_key_id
    pub issued_at: DateTime<Utc>,
    pub evidence_hash: String, // hash of the original receipt/proof
}

impl ExternalAlias {
    pub fn new(scheme: &str, value: &str, issuer_id: &str, evidence_hash: &str) -> Self {
        Self {
            scheme: scheme.to_string(),
            value: value.to_string(),
            issuer_id: issuer_id.to_string(),
            issued_at: Utc::now(),
            evidence_hash: evidence_hash.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitAliasConfig {
    pub required_canonical: Vec<String>,         // ["sisbov", "cpf"]
    pub required_contextual: Vec<String>,        // ["lote", "safra"]
    pub use_fingerprint: bool,                   // use fingerprint for dedup
    pub allowed_namespaces: Option<Vec<String>>, // None = all allowed
    pub auto_apply_namespace: bool,              // apply default_namespace if missing
}

impl Default for CircuitAliasConfig {
    fn default() -> Self {
        Self {
            required_canonical: vec![],
            required_contextual: vec![],
            use_fingerprint: true,
            auto_apply_namespace: true,
            allowed_namespaces: None,
        }
    }
}

impl CircuitAliasConfig {
    /// Create a config for bovine traceability
    pub fn bovine_traceability() -> Self {
        Self {
            required_canonical: vec!["sisbov".to_string()],
            required_contextual: vec![],
            use_fingerprint: false,
            allowed_namespaces: Some(vec!["bovino".to_string()]),
            auto_apply_namespace: true,
        }
    }

    /// Create a config for grain lots
    pub fn grain_lots() -> Self {
        Self {
            required_canonical: vec![],
            required_contextual: vec!["lote".to_string(), "safra".to_string()],
            use_fingerprint: true,
            allowed_namespaces: Some(vec!["soja".to_string(), "milho".to_string()]),
            auto_apply_namespace: true,
        }
    }

    /// Create a config for poultry
    pub fn poultry() -> Self {
        Self {
            required_canonical: vec![],
            required_contextual: vec!["lote".to_string(), "granja".to_string()],
            use_fingerprint: true,
            allowed_namespaces: Some(vec!["aves".to_string()]),
            auto_apply_namespace: true,
        }
    }

    /// Create an open config that accepts anything
    pub fn open() -> Self {
        Self {
            required_canonical: vec![],
            required_contextual: vec![],
            use_fingerprint: true,
            allowed_namespaces: None,
            auto_apply_namespace: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_validation() {
        assert!(namespaces::is_valid("bovino"));
        assert!(namespaces::is_valid("soja"));
        assert!(namespaces::is_valid("generic"));
        assert!(!namespaces::is_valid("invalid"));
        assert!(!namespaces::is_valid(""));
    }

    #[test]
    fn test_sisbov_validation() {
        assert!(registries::validate("sisbov", "BR123456789012"));
        assert!(!registries::validate("sisbov", "12345678901234")); // Missing BR
        assert!(!registries::validate("sisbov", "BR12345678901")); // Too short
        assert!(!registries::validate("sisbov", "BR12345678901A")); // Contains letter
    }

    #[test]
    fn test_cpf_validation() {
        assert!(registries::validate("cpf", "12345678901"));
        assert!(registries::validate("cpf", "123.456.789-01")); // With formatting
        assert!(!registries::validate("cpf", "1234567890")); // Too short
        assert!(!registries::validate("cpf", "11111111111")); // All same digit
    }

    #[test]
    fn test_canonical_identifier() {
        let id = EnhancedIdentifier::canonical("bovino", "sisbov", "BR123456789012");
        assert!(id.is_canonical());
        assert!(!id.is_contextual());
        assert_eq!(id.get_registry(), Some("sisbov".to_string()));
        assert!(id.validate());
    }

    #[test]
    fn test_contextual_identifier() {
        let id = EnhancedIdentifier::contextual("soja", "lote", "123");
        assert!(!id.is_canonical());
        assert!(id.is_contextual());
        assert_eq!(id.get_registry(), None);
        assert!(id.validate());
    }

    #[test]
    fn test_unique_key() {
        let id1 = EnhancedIdentifier::canonical("bovino", "sisbov", "BR123456789012");
        assert_eq!(id1.unique_key(), "bovino:sisbov:BR123456789012");

        let id2 = EnhancedIdentifier::contextual("soja", "lote", "123");
        assert_eq!(id2.unique_key(), "soja:lote:123");
    }

    #[test]
    fn test_invalid_identifier() {
        let id1 = EnhancedIdentifier::canonical("invalid_namespace", "sisbov", "BR123456789012");
        assert!(!id1.validate());

        let id2 = EnhancedIdentifier::canonical("bovino", "sisbov", "invalid");
        assert!(!id2.validate());

        let id3 = EnhancedIdentifier::contextual("soja", "", "123");
        assert!(!id3.validate());
    }

    #[test]
    fn test_circuit_alias_configs() {
        let bovine = CircuitAliasConfig::bovine_traceability();
        assert_eq!(bovine.required_canonical, vec!["sisbov"]);
        assert!(!bovine.use_fingerprint);

        let grain = CircuitAliasConfig::grain_lots();
        assert_eq!(grain.required_contextual, vec!["lote", "safra"]);
        assert!(grain.use_fingerprint);

        let open = CircuitAliasConfig::open();
        assert!(open.required_canonical.is_empty());
        assert!(open.use_fingerprint);
        assert!(open.allowed_namespaces.is_none());
    }
}
