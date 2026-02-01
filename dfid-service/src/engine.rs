use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DfidError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[cfg(feature = "redis-persistence")]
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

pub struct DfidEngine {
    sequence_counter: Arc<AtomicU64>,
    #[cfg(feature = "redis-persistence")]
    redis_client: Option<redis::Client>,
    last_date: Arc<std::sync::Mutex<String>>, // Track current day for sequence reset
}

impl DfidEngine {
    pub fn new() -> Self {
        let today = Utc::now().format("%Y%m%d").to_string();
        Self {
            sequence_counter: Arc::new(AtomicU64::new(1)),
            #[cfg(feature = "redis-persistence")]
            redis_client: None,
            last_date: Arc::new(std::sync::Mutex::new(today)),
        }
    }

    #[cfg(feature = "redis-persistence")]
    pub async fn new_with_redis(redis_url: &str) -> Result<Self, DfidError> {
        let client = redis::Client::open(redis_url)?;
        let mut conn = client.get_async_connection().await?;

        // Load sequence for today (per-day sequence)
        let today = Utc::now().format("%Y%m%d").to_string();
        let redis_key = format!("dfid:sequence:{}", today);

        let current_seq: Option<u64> = redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;

        let seq = current_seq.unwrap_or(1);

        tracing::info!("Initialized DFID sequence for {} from Redis: {}", today, seq);

        Ok(Self {
            sequence_counter: Arc::new(AtomicU64::new(seq)),
            redis_client: Some(client),
            last_date: Arc::new(std::sync::Mutex::new(today)),
        })
    }

    pub fn generate_dfid(&self) -> String {
        self.generate_dfid_with_context(None)
    }

    pub fn generate_dfid_with_context(&self, _context: Option<&str>) -> String {
        let timestamp = Utc::now();
        let timestamp_str = timestamp.format("%Y%m%d").to_string();

        // Check if day changed and reset sequence if needed
        {
            let mut last_date = self.last_date.lock().unwrap();
            if *last_date != timestamp_str {
                tracing::info!("Day changed from {} to {}, resetting sequence to 1", last_date, timestamp_str);
                self.sequence_counter.store(1, Ordering::SeqCst);
                *last_date = timestamp_str.clone();
            }
        }

        let sequence = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let sequence_str = format!("{sequence:06}");
        let checksum = self.calculate_checksum(&timestamp_str, &sequence_str);

        format!("DFID-{timestamp_str}-{sequence_str}-{checksum}")
    }

    pub fn generate_batch(&self, count: usize) -> Vec<String> {
        (0..count).map(|_| self.generate_dfid()).collect()
    }

    pub fn validate_dfid(&self, dfid: &str) -> Result<bool, DfidError> {
        if !dfid.starts_with("DFID-") {
            return Ok(false);
        }

        let parts: Vec<&str> = dfid.split('-').collect();
        if parts.len() != 4 {
            return Ok(false);
        }

        let timestamp_str = parts[1];
        let sequence_str = parts[2];
        let provided_checksum = parts[3];

        if timestamp_str.len() != 8 || sequence_str.len() != 6 {
            return Ok(false);
        }

        if !timestamp_str.chars().all(|c| c.is_ascii_digit())
            || !sequence_str.chars().all(|c| c.is_ascii_digit())
        {
            return Ok(false);
        }

        let calculated_checksum = self.calculate_checksum(timestamp_str, sequence_str);
        Ok(calculated_checksum == provided_checksum)
    }

    pub fn extract_metadata(&self, dfid: &str) -> Result<DfidMetadata, DfidError> {
        if !self.validate_dfid(dfid)? {
            return Err(DfidError::ValidationError(format!(
                "Invalid DFID format: {}",
                dfid
            )));
        }

        let parts: Vec<&str> = dfid.split('-').collect();
        let timestamp_str = parts[1];
        let sequence_str = parts[2];

        let year = timestamp_str[0..4]
            .parse::<i32>()
            .map_err(|e| DfidError::ValidationError(format!("Invalid year: {}", e)))?;
        let month = timestamp_str[4..6]
            .parse::<u32>()
            .map_err(|e| DfidError::ValidationError(format!("Invalid month: {}", e)))?;
        let day = timestamp_str[6..8]
            .parse::<u32>()
            .map_err(|e| DfidError::ValidationError(format!("Invalid day: {}", e)))?;

        let sequence = sequence_str
            .parse::<u64>()
            .map_err(|e| DfidError::ValidationError(format!("Invalid sequence: {}", e)))?;

        Ok(DfidMetadata {
            year,
            month,
            day,
            sequence,
            full_dfid: dfid.to_string(),
        })
    }

    fn calculate_checksum(&self, timestamp: &str, sequence: &str) -> String {
        use blake3::Hasher;

        let combined = format!("{timestamp}{sequence}");
        let hash = Hasher::new().update(combined.as_bytes()).finalize();

        // Use first 4 bytes and mod to get 24-bit value (0x000000 to 0xFFFFFF)
        let bytes = hash.as_bytes();
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 0x1000000;

        format!("{:06X}", value)
    }

    #[cfg(feature = "redis-persistence")]
    pub async fn persist_sequence(&self) -> Result<(), DfidError> {
        if let Some(ref client) = self.redis_client {
            let seq = self.sequence_counter.load(Ordering::SeqCst);
            let mut conn = client.get_async_connection().await?;

            // Per-day sequence key
            let today = Utc::now().format("%Y%m%d").to_string();
            let redis_key = format!("dfid:sequence:{}", today);

            // Save current sequence with 48h expiry (keep yesterday and today)
            redis::cmd("SET")
                .arg(&redis_key)
                .arg(seq)
                .arg("EX")
                .arg(172800) // 48 hours
                .query_async::<_, ()>(&mut conn)
                .await?;

            // Backup: also save to snapshot list (trimmed to last 30 days)
            let backup_key = "dfid:sequence:backups";
            let backup_entry = format!("{}:{}", today, seq);

            redis::cmd("ZADD")
                .arg(backup_key)
                .arg(today.parse::<i64>().unwrap_or(0))
                .arg(&backup_entry)
                .query_async::<_, ()>(&mut conn)
                .await?;

            // Trim old backups (keep last 30 days)
            let cutoff_date = (Utc::now() - chrono::Duration::days(30))
                .format("%Y%m%d")
                .to_string();
            redis::cmd("ZREMRANGEBYSCORE")
                .arg(backup_key)
                .arg("-inf")
                .arg(cutoff_date)
                .query_async::<_, ()>(&mut conn)
                .await?;

            tracing::debug!("Persisted sequence for {} to Redis: {} (with backup)", today, seq);
        }
        Ok(())
    }

    #[cfg(not(feature = "redis-persistence"))]
    pub async fn persist_sequence(&self) -> Result<(), DfidError> {
        // No-op when Redis persistence is disabled
        Ok(())
    }

    pub fn get_current_sequence(&self) -> u64 {
        self.sequence_counter.load(Ordering::SeqCst)
    }

    pub fn reset_sequence(&self) {
        self.sequence_counter.store(1, Ordering::SeqCst);
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

    #[cfg(feature = "redis-persistence")]
    pub async fn restore_from_backup(&self, date: &str) -> Result<u64, DfidError> {
        if let Some(ref client) = self.redis_client {
            let mut conn = client.get_async_connection().await?;
            let backup_key = "dfid:sequence:backups";

            // Get all backups and find the one for the requested date
            let backups: Vec<String> = redis::cmd("ZRANGEBYSCORE")
                .arg(backup_key)
                .arg(date)
                .arg(date)
                .query_async(&mut conn)
                .await?;

            if let Some(entry) = backups.first() {
                // Parse "YYYYMMDD:sequence" format
                if let Some((_, seq_str)) = entry.split_once(':') {
                    if let Ok(seq) = seq_str.parse::<u64>() {
                        self.sequence_counter.store(seq, Ordering::SeqCst);
                        tracing::info!("Restored sequence from backup for {}: {}", date, seq);
                        return Ok(seq);
                    }
                }
            }

            Err(DfidError::PersistenceError(format!(
                "No backup found for date: {}",
                date
            )))
        } else {
            Err(DfidError::PersistenceError(
                "Redis persistence not enabled".to_string(),
            ))
        }
    }
}

impl Default for DfidEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        assert!(engine.validate_dfid(&dfid).unwrap());
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
        assert_eq!(parts[3].len(), 6); // 24-bit checksum (6 hex chars)
    }

    #[test]
    fn test_blake3_checksum_consistency() {
        let engine = DfidEngine::new();
        let timestamp = "20250131";
        let sequence = "000001";

        let checksum1 = engine.calculate_checksum(timestamp, sequence);
        let checksum2 = engine.calculate_checksum(timestamp, sequence);

        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 6); // 6 hex characters for 24-bit
    }

    #[test]
    fn test_batch_generation() {
        let engine = DfidEngine::new();
        let batch = engine.generate_batch(10);

        assert_eq!(batch.len(), 10);
        for dfid in &batch {
            assert!(engine.validate_dfid(dfid).unwrap());
        }

        // Verify all DFIDs are unique
        let unique_count = batch.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 10);
    }

    #[test]
    fn test_metadata_extraction() {
        let engine = DfidEngine::new();
        let dfid = engine.generate_dfid();

        let metadata = engine.extract_metadata(&dfid).unwrap();
        assert_eq!(metadata.full_dfid, dfid);
        assert!(metadata.year >= 2025);
        assert!(metadata.month >= 1 && metadata.month <= 12);
        assert!(metadata.day >= 1 && metadata.day <= 31);
        assert!(metadata.sequence > 0);
    }
}
