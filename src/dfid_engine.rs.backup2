use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct DfidEngine {
    sequence_counter: Arc<AtomicU64>,
}

impl DfidEngine {
    pub fn new() -> Self {
        Self {
            sequence_counter: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn generate_dfid(&self) -> String {
        let timestamp = Utc::now();
        let sequence = self.sequence_counter.fetch_add(1, Ordering::SeqCst);

        let timestamp_str = timestamp.format("%Y%m%d").to_string();
        let sequence_str = format!("{sequence:06}");
        let checksum = self.calculate_checksum(&timestamp_str, &sequence_str);

        format!("DFID-{timestamp_str}-{sequence_str}-{checksum}")
    }

    pub fn validate_dfid(&self, dfid: &str) -> bool {
        if !dfid.starts_with("DFID-") {
            return false;
        }

        let parts: Vec<&str> = dfid.split('-').collect();
        if parts.len() != 4 {
            return false;
        }

        let timestamp_str = parts[1];
        let sequence_str = parts[2];
        let provided_checksum = parts[3];

        if timestamp_str.len() != 8 || sequence_str.len() != 6 {
            return false;
        }

        if !timestamp_str.chars().all(|c| c.is_ascii_digit())
            || !sequence_str.chars().all(|c| c.is_ascii_digit())
        {
            return false;
        }

        let calculated_checksum = self.calculate_checksum(timestamp_str, sequence_str);
        calculated_checksum == provided_checksum
    }

    pub fn extract_metadata(&self, dfid: &str) -> Option<DfidMetadata> {
        if !self.validate_dfid(dfid) {
            return None;
        }

        let parts: Vec<&str> = dfid.split('-').collect();
        let timestamp_str = parts[1];
        let sequence_str = parts[2];

        let year = timestamp_str[0..4].parse::<i32>().ok()?;
        let month = timestamp_str[4..6].parse::<u32>().ok()?;
        let day = timestamp_str[6..8].parse::<u32>().ok()?;

        let sequence = sequence_str.parse::<u64>().ok()?;

        Some(DfidMetadata {
            year,
            month,
            day,
            sequence,
            full_dfid: dfid.to_string(),
        })
    }

    fn calculate_checksum(&self, timestamp: &str, sequence: &str) -> String {
        let combined = format!("{timestamp}{sequence}");
        let mut hash = 0u32;

        for byte in combined.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }

        format!("{:X}", hash % 0xFFFF)
    }

    pub fn reset_sequence(&self) {
        self.sequence_counter.store(1, Ordering::SeqCst);
    }

    pub fn get_current_sequence(&self) -> u64 {
        self.sequence_counter.load(Ordering::SeqCst)
    }

    pub fn ensure_min_sequence(&self, next: u64) {
        let mut current = self.sequence_counter.load(Ordering::SeqCst);
        while current < next {
            match self.sequence_counter.compare_exchange(
                current,
                next,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }
}

impl Default for DfidEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DfidMetadata {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub sequence: u64,
    pub full_dfid: String,
}

impl DfidMetadata {
    pub fn creation_date(&self) -> Option<DateTime<Utc>> {
        use chrono::{NaiveDate, TimeZone};

        let naive_date = NaiveDate::from_ymd_opt(self.year, self.month, self.day)?;
        Some(Utc.from_utc_datetime(&naive_date.and_hms_opt(0, 0, 0)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfid_generation() {
        let engine = DfidEngine::new();
        let dfid = engine.generate_dfid();

        assert!(dfid.starts_with("DFID-"));
        assert!(engine.validate_dfid(&dfid));
    }

    #[test]
    fn test_dfid_format() {
        let engine = DfidEngine::new();
        let dfid = engine.generate_dfid();

        let parts: Vec<&str> = dfid.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "DFID");
        assert_eq!(parts[1].len(), 8); // YYYYMMDD
        assert_eq!(parts[2].len(), 6); // 6-digit sequence
        assert!(!parts[3].is_empty()); // checksum
    }

    #[test]
    fn test_dfid_validation() {
        let engine = DfidEngine::new();

        assert!(!engine.validate_dfid(""));
        assert!(!engine.validate_dfid("DFID"));
        assert!(!engine.validate_dfid("DFID-20240926"));
        assert!(!engine.validate_dfid("INVALID-20240926-000001-ABC"));
        assert!(!engine.validate_dfid("DFID-2024926-000001-ABC")); // Invalid date format
        assert!(!engine.validate_dfid("DFID-20240926-1-ABC")); // Invalid sequence format
    }

    #[test]
    fn test_sequential_dfids() {
        let engine = DfidEngine::new();

        let dfid1 = engine.generate_dfid();
        let dfid2 = engine.generate_dfid();

        assert_ne!(dfid1, dfid2);
        assert!(engine.validate_dfid(&dfid1));
        assert!(engine.validate_dfid(&dfid2));

        let meta1 = engine.extract_metadata(&dfid1).unwrap();
        let meta2 = engine.extract_metadata(&dfid2).unwrap();

        assert_eq!(meta2.sequence, meta1.sequence + 1);
    }

    #[test]
    fn test_metadata_extraction() {
        let engine = DfidEngine::new();
        let dfid = engine.generate_dfid();

        let metadata = engine.extract_metadata(&dfid).unwrap();
        assert_eq!(metadata.full_dfid, dfid);
        assert!(metadata.year >= 2024);
        assert!(metadata.month >= 1 && metadata.month <= 12);
        assert!(metadata.day >= 1 && metadata.day <= 31);
        assert!(metadata.sequence > 0);
    }

    #[test]
    fn test_metadata_creation_date() {
        let engine = DfidEngine::new();
        let dfid = engine.generate_dfid();

        let metadata = engine.extract_metadata(&dfid).unwrap();
        let creation_date = metadata.creation_date().unwrap();

        let now = Utc::now();
        assert_eq!(creation_date.date_naive(), now.date_naive());
    }

    #[test]
    fn test_sequence_reset() {
        let engine = DfidEngine::new();

        engine.generate_dfid();
        engine.generate_dfid();
        assert!(engine.get_current_sequence() > 2);

        engine.reset_sequence();
        assert_eq!(engine.get_current_sequence(), 1);
    }

    #[test]
    fn test_checksum_consistency() {
        let engine = DfidEngine::new();
        let timestamp = "20240926";
        let sequence = "000001";

        let checksum1 = engine.calculate_checksum(timestamp, sequence);
        let checksum2 = engine.calculate_checksum(timestamp, sequence);

        assert_eq!(checksum1, checksum2);
        assert!(!checksum1.is_empty());
    }

    #[test]
    fn test_invalid_metadata_extraction() {
        let engine = DfidEngine::new();

        assert!(engine.extract_metadata("invalid-dfid").is_none());
        assert!(engine.extract_metadata("DFID-invalid-000001-ABC").is_none());
        assert!(engine
            .extract_metadata("DFID-20240926-invalid-ABC")
            .is_none());
    }
}
