use blake3;

/// Generates privacy-preserving hashes for personal and company identifiers
/// All names, documents, and identifiable information are hashed with BLAKE3
pub struct HashGenerator;

impl HashGenerator {
    /// Hash an owner name/company and return formatted identifier
    /// Format: hash:owner:{blake3_hex}
    pub fn hash_owner(name: &str) -> String {
        let hash = blake3::hash(name.as_bytes());
        format!("hash:owner:{}", hash.to_hex())
    }

    /// Hash a veterinarian name
    /// Format: hash:vet:{blake3_hex}
    pub fn hash_vet(name: &str) -> String {
        let hash = blake3::hash(name.as_bytes());
        format!("hash:vet:{}", hash.to_hex())
    }

    /// Hash a CPF (Brazilian individual tax ID)
    /// Format: hash:cpf:{blake3_hex}
    pub fn hash_cpf(cpf: &str) -> String {
        let hash = blake3::hash(cpf.as_bytes());
        format!("hash:cpf:{}", hash.to_hex())
    }

    /// Hash a CNPJ (Brazilian company tax ID)
    /// Format: hash:cnpj:{blake3_hex}
    pub fn hash_cnpj(cnpj: &str) -> String {
        let hash = blake3::hash(cnpj.as_bytes());
        format!("hash:cnpj:{}", hash.to_hex())
    }

    /// Hash a company name
    /// Format: hash:company:{blake3_hex}
    pub fn hash_company(name: &str) -> String {
        let hash = blake3::hash(name.as_bytes());
        format!("hash:company:{}", hash.to_hex())
    }

    /// Hash a farm/ranch name
    /// Format: hash:farm:{blake3_hex}
    pub fn hash_farm(name: &str) -> String {
        let hash = blake3::hash(name.as_bytes());
        format!("hash:farm:{}", hash.to_hex())
    }

    /// Generic hash for any identifier
    pub fn hash_generic(prefix: &str, value: &str) -> String {
        let hash = blake3::hash(value.as_bytes());
        format!("hash:{}:{}", prefix, hash.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_owner() {
        let hash1 = HashGenerator::hash_owner("Fazenda São José");
        let hash2 = HashGenerator::hash_owner("Fazenda São José");
        let hash3 = HashGenerator::hash_owner("Fazenda Santa Maria");

        // Same input produces same hash
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("hash:owner:"));

        // Different input produces different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_vet() {
        let hash = HashGenerator::hash_vet("Dr. Maria Silva");
        assert!(hash.starts_with("hash:vet:"));
        assert_eq!(hash.len(), "hash:vet:".len() + 64); // BLAKE3 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_hash_formats() {
        let cpf_hash = HashGenerator::hash_cpf("12345678901");
        let cnpj_hash = HashGenerator::hash_cnpj("12345678000190");
        let company_hash = HashGenerator::hash_company("Cooperativa Agrícola MS");
        let farm_hash = HashGenerator::hash_farm("Fazenda Três Lagoas");

        assert!(cpf_hash.starts_with("hash:cpf:"));
        assert!(cnpj_hash.starts_with("hash:cnpj:"));
        assert!(company_hash.starts_with("hash:company:"));
        assert!(farm_hash.starts_with("hash:farm:"));
    }

    #[test]
    fn test_deterministic() {
        // Same input always produces same output
        let input = "Test Owner Name";
        let results: Vec<String> = (0..100).map(|_| HashGenerator::hash_owner(input)).collect();

        assert!(results.windows(2).all(|w| w[0] == w[1]));
    }
}
