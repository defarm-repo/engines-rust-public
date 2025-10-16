use crate::logging::{LogEntry, LoggingEngine};
use crate::storage::{InMemoryStorage, StorageBackend, StorageError};
use crate::types::{DataLakeEntry, Identifier, Receipt};
use blake3;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug)]
pub enum ReceiptError {
    NoIdentifiers,
    StorageError(StorageError),
}

impl std::fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReceiptError::NoIdentifiers => write!(f, "At least one identifier is required"),
            ReceiptError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for ReceiptError {}

pub struct ReceiptEngine<S: StorageBackend> {
    storage: S,
    logger: LoggingEngine,
}

impl<S: StorageBackend> ReceiptEngine<S> {
    pub fn new(storage: S) -> Self {
        let mut logger = LoggingEngine::new();
        logger.info(
            "ReceiptEngine",
            "initialization",
            "Receipt engine initialized",
        );

        Self { storage, logger }
    }

    pub fn process_data(
        &mut self,
        data: &[u8],
        identifiers: Vec<Identifier>,
    ) -> Result<Receipt, ReceiptError> {
        self.logger
            .info(
                "ReceiptEngine",
                "data_reception_attempt",
                "Processing data reception",
            )
            .with_context("data_size", data.len().to_string())
            .with_context("identifiers_count", identifiers.len().to_string());

        if identifiers.is_empty() {
            self.logger
                .error(
                    "ReceiptEngine",
                    "validation_failure",
                    "Data rejected: no identifiers provided",
                )
                .with_context("data_size", data.len().to_string());
            return Err(ReceiptError::NoIdentifiers);
        }

        let hash = blake3::hash(data);
        let receipt = Receipt {
            id: Uuid::new_v4(),
            hash: hash.to_hex().to_string(),
            timestamp: Utc::now(),
            data_size: data.len(),
            identifiers: identifiers.clone(),
        };

        if let Err(e) = self.storage.store_receipt(&receipt) {
            self.logger
                .error(
                    "ReceiptEngine",
                    "storage_failure",
                    "Failed to store receipt",
                )
                .with_context("receipt_id", receipt.id.to_string())
                .with_context("error", e.to_string());
            return Err(ReceiptError::StorageError(e));
        }

        // Create data lake entry for verification processing
        let data_lake_entry = DataLakeEntry::new(
            receipt.id,
            identifiers.clone(),
            receipt.hash.clone(),
            data.len(),
        );

        if let Err(e) = self.storage.store_data_lake_entry(&data_lake_entry) {
            self.logger
                .error(
                    "ReceiptEngine",
                    "data_lake_storage_failure",
                    "Failed to store data lake entry",
                )
                .with_context("receipt_id", receipt.id.to_string())
                .with_context("error", e.to_string());
        } else {
            self.logger
                .info(
                    "ReceiptEngine",
                    "data_lake_entry_created",
                    "Data lake entry created",
                )
                .with_context("entry_id", data_lake_entry.entry_id.to_string())
                .with_context("receipt_id", receipt.id.to_string());
        }

        self.logger
            .info(
                "ReceiptEngine",
                "receipt_created",
                "Receipt successfully created",
            )
            .with_context("receipt_id", receipt.id.to_string())
            .with_context("hash", receipt.hash.clone())
            .with_context("data_size", data.len().to_string());

        Ok(receipt)
    }

    pub fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError> {
        self.storage.get_receipt(id)
    }

    pub fn verify_data(&self, id: &Uuid, data: &[u8]) -> Result<bool, StorageError> {
        if let Some(receipt) = self.storage.get_receipt(id)? {
            let hash = blake3::hash(data);
            Ok(receipt.hash == hash.to_hex().to_string())
        } else {
            Ok(false)
        }
    }

    pub fn find_receipts_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Vec<Receipt>, StorageError> {
        self.storage.find_receipts_by_identifier(identifier)
    }

    pub fn find_receipts_by_key(&self, key: &str) -> Result<Vec<Receipt>, StorageError> {
        let receipts = self.storage.list_receipts()?;
        Ok(receipts
            .into_iter()
            .filter(|receipt| receipt.identifiers.iter().any(|id| id.key == key))
            .collect())
    }

    pub fn find_receipts_by_value(&self, value: &str) -> Result<Vec<Receipt>, StorageError> {
        let receipts = self.storage.list_receipts()?;
        Ok(receipts
            .into_iter()
            .filter(|receipt| receipt.identifiers.iter().any(|id| id.value == value))
            .collect())
    }

    pub fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError> {
        self.storage.list_receipts()
    }

    pub fn list_identifiers(&self) -> Result<Vec<Identifier>, StorageError> {
        let receipts = self.storage.list_receipts()?;
        let mut identifiers = Vec::new();
        for receipt in receipts {
            for identifier in receipt.identifiers {
                if !identifiers.contains(&identifier) {
                    identifiers.push(identifier);
                }
            }
        }
        Ok(identifiers)
    }

    pub fn get_logs(&self) -> &[LogEntry] {
        self.logger.get_logs()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<&LogEntry> {
        self.logger.get_logs_by_event_type(event_type)
    }
}

impl Default for ReceiptEngine<InMemoryStorage> {
    fn default() -> Self {
        Self::new(InMemoryStorage::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedFileStorage, EncryptionKey, InMemoryStorage};

    #[test]
    fn test_process_data_with_identifiers() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let data = b"test data";
        let identifiers = vec![
            Identifier::new("user_id", "12345"),
            Identifier::new("transaction_id", "tx_abc123"),
        ];

        let receipt = engine.process_data(data, identifiers.clone()).unwrap();

        assert!(!receipt.hash.is_empty());
        assert_eq!(receipt.data_size, data.len());
        assert_eq!(receipt.identifiers.len(), 2);
        assert!(engine.get_receipt(&receipt.id).unwrap().is_some());
    }

    #[test]
    fn test_process_data_no_identifiers_fails() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let data = b"test data";
        let result = engine.process_data(data, vec![]);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReceiptError::NoIdentifiers));
    }

    #[test]
    fn test_verify_data() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let data = b"test data";
        let identifiers = vec![Identifier::new("test", "value")];
        let receipt = engine.process_data(data, identifiers).unwrap();

        assert!(engine.verify_data(&receipt.id, data).unwrap());
        assert!(!engine.verify_data(&receipt.id, b"different data").unwrap());
    }

    #[test]
    fn test_find_receipts_by_identifier() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let user_id = Identifier::new("user_id", "12345");

        engine
            .process_data(b"data 1", vec![user_id.clone()])
            .unwrap();
        engine
            .process_data(b"data 2", vec![user_id.clone()])
            .unwrap();
        engine
            .process_data(b"data 3", vec![Identifier::new("user_id", "67890")])
            .unwrap();

        let receipts = engine.find_receipts_by_identifier(&user_id).unwrap();
        assert_eq!(receipts.len(), 2);
    }

    #[test]
    fn test_find_receipts_by_key() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());

        engine
            .process_data(b"data 1", vec![Identifier::new("user_id", "12345")])
            .unwrap();
        engine
            .process_data(b"data 2", vec![Identifier::new("user_id", "67890")])
            .unwrap();
        engine
            .process_data(b"data 3", vec![Identifier::new("order_id", "order123")])
            .unwrap();

        let receipts = engine.find_receipts_by_key("user_id").unwrap();
        assert_eq!(receipts.len(), 2);
    }

    #[test]
    fn test_find_receipts_by_value() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());

        engine
            .process_data(b"data 1", vec![Identifier::new("user_id", "12345")])
            .unwrap();
        engine
            .process_data(b"data 2", vec![Identifier::new("customer_id", "12345")])
            .unwrap();
        engine
            .process_data(b"data 3", vec![Identifier::new("user_id", "67890")])
            .unwrap();

        let receipts = engine.find_receipts_by_value("12345").unwrap();
        assert_eq!(receipts.len(), 2);
    }

    #[test]
    fn test_multiple_identifiers_per_receipt() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let identifiers = vec![
            Identifier::new("user_id", "12345"),
            Identifier::new("session_id", "sess_abc"),
            Identifier::new("transaction_id", "tx_123"),
        ];

        let receipt = engine
            .process_data(b"transaction data", identifiers)
            .unwrap();

        assert_eq!(receipt.identifiers.len(), 3);

        let by_user = engine.find_receipts_by_key("user_id").unwrap();
        let by_session = engine.find_receipts_by_key("session_id").unwrap();
        let by_transaction = engine.find_receipts_by_key("transaction_id").unwrap();

        assert_eq!(by_user.len(), 1);
        assert_eq!(by_session.len(), 1);
        assert_eq!(by_transaction.len(), 1);
        assert_eq!(by_user[0].id, receipt.id);
    }

    #[test]
    fn test_same_data_different_identifiers() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let data = b"identical data";

        let receipt1 = engine
            .process_data(data, vec![Identifier::new("user", "alice")])
            .unwrap();
        let receipt2 = engine
            .process_data(data, vec![Identifier::new("user", "bob")])
            .unwrap();

        assert_eq!(receipt1.hash, receipt2.hash);
        assert_ne!(receipt1.id, receipt2.id);
        assert_ne!(receipt1.identifiers, receipt2.identifiers);
    }

    #[test]
    fn test_logging_initialization() {
        let engine = ReceiptEngine::new(InMemoryStorage::new());
        let logs = engine.get_logs();

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].engine, "ReceiptEngine");
        assert_eq!(logs[0].event_type, "initialization");
    }

    #[test]
    fn test_logging_successful_receipt() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());
        let identifiers = vec![Identifier::new("test", "value")];

        engine.process_data(b"test data", identifiers).unwrap();

        let logs = engine.get_logs();
        assert!(logs.len() >= 3);

        let attempt_logs = engine.get_logs_by_event_type("data_reception_attempt");
        assert_eq!(attempt_logs.len(), 1);

        let created_logs = engine.get_logs_by_event_type("receipt_created");
        assert_eq!(created_logs.len(), 1);
    }

    #[test]
    fn test_logging_validation_failure() {
        let mut engine = ReceiptEngine::new(InMemoryStorage::new());

        let result = engine.process_data(b"test data", vec![]);
        assert!(result.is_err());

        let error_logs = engine.get_logs_by_event_type("validation_failure");
        assert_eq!(error_logs.len(), 1);
        assert_eq!(
            error_logs[0].message,
            "Data rejected: no identifiers provided"
        );
    }

    #[test]
    fn test_encrypted_file_storage() {
        use std::env;
        let temp_dir = env::temp_dir().join("receipt_engine_test");
        let key = EncryptionKey::generate();
        let storage = EncryptedFileStorage::new(temp_dir.to_str().unwrap()).with_encryption(key);
        let mut engine = ReceiptEngine::new(storage);

        let identifiers = vec![Identifier::new("test", "encrypted")];
        let receipt = engine.process_data(b"sensitive data", identifiers).unwrap();

        let retrieved = engine.get_receipt(&receipt.id).unwrap().unwrap();
        assert_eq!(retrieved.hash, receipt.hash);
        assert_eq!(retrieved.identifiers, receipt.identifiers);

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_unencrypted_file_storage() {
        use std::env;
        let temp_dir = env::temp_dir().join("receipt_engine_test_plain");
        let storage = EncryptedFileStorage::new(temp_dir.to_str().unwrap());
        let mut engine = ReceiptEngine::new(storage);

        let identifiers = vec![Identifier::new("test", "plain")];
        let receipt = engine.process_data(b"plain data", identifiers).unwrap();

        let retrieved = engine.get_receipt(&receipt.id).unwrap().unwrap();
        assert_eq!(retrieved.hash, receipt.hash);

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
