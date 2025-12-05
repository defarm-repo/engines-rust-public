use tokio_postgres::{NoTls, Error as PgError};
use deadpool_postgres::{Pool, Manager, ManagerConfig, RecyclingMethod, Runtime};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json;

use crate::storage::{StorageBackend, StorageError};
use crate::types::*;
use crate::logging::{LogEntry, LogLevel};
use crate::identifier_types::EnhancedIdentifier;

/// PostgreSQL-backed storage implementation
/// Implements all StorageBackend methods with connection pooling
pub struct PostgresStorage {
    pool: Pool,
}

impl PostgresStorage {
    /// Create a new PostgreSQL storage with connection pool
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let config = database_url.parse::<tokio_postgres::Config>()
            .map_err(|e| StorageError::ConfigurationError(format!("Invalid database URL: {}", e)))?;

        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, manager_config);

        let pool = Pool::builder(manager)
            .max_size(16)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| StorageError::ConnectionError(format!("Failed to create pool: {}", e)))?;

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool_postgres::Client, StorageError> {
        self.pool.get().await
            .map_err(|e| StorageError::ConnectionError(format!("Failed to get connection: {}", e)))
    }

    /// Convert PostgreSQL error to StorageError
    fn map_pg_error(e: PgError) -> StorageError {
        StorageError::ReadError(format!("PostgreSQL error: {}", e))
    }
}

// Convert common types to/from database format
impl PostgresStorage {
    fn identifier_to_key_value(identifier: &Identifier) -> (&str, &str) {
        (&identifier.key, &identifier.value)
    }

    fn row_to_receipt(row: &tokio_postgres::Row) -> Result<Receipt, StorageError> {
        Ok(Receipt {
            id: row.get("id"),
            hash: row.get("data_hash"),
            timestamp: row.get("timestamp"),
            data_size: row.get::<_, i64>("data_size") as usize,
            identifiers: Vec::new(), // Loaded separately
        })
    }

    fn row_to_log(row: &tokio_postgres::Row) -> Result<LogEntry, StorageError> {
        let level_str: String = row.get("level");
        let level = match level_str.as_str() {
            "Info" => LogLevel::Info,
            "Warn" => LogLevel::Warn,
            "Error" => LogLevel::Error,
            _ => LogLevel::Info,
        };

        let context_data: Option<serde_json::Value> = row.get("context_data");
        let context = context_data.and_then(|v| {
            serde_json::from_value(v).ok()
        }).unwrap_or_else(std::collections::HashMap::new);

        Ok(LogEntry {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            level,
            engine: row.get("engine"),
            event_type: row.get("event_type"),
            message: row.get("message"),
            context,
        })
    }

    fn row_to_item(row: &tokio_postgres::Row) -> Result<Item, StorageError> {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Active" => ItemStatus::Active,
            "Merged" => ItemStatus::Merged,
            "Split" => ItemStatus::Split,
            "Deprecated" => ItemStatus::Deprecated,
            _ => ItemStatus::Active,
        };

        let enriched_data: Option<serde_json::Value> = row.get("enriched_data");
        let enriched_data_map = enriched_data.and_then(|v| {
            serde_json::from_value(v).ok()
        }).unwrap_or_else(std::collections::HashMap::new);

        // Convert bigint timestamps to DateTime<Utc>
        let created_at_ts: i64 = row.get("created_at_ts");
        let last_updated_ts: i64 = row.get("last_updated_ts");

        use chrono::TimeZone;
        let creation_timestamp = Utc.timestamp_millis_opt(created_at_ts)
            .single()
            .unwrap_or_else(|| Utc::now());
        let last_modified = Utc.timestamp_millis_opt(last_updated_ts)
            .single()
            .unwrap_or_else(|| Utc::now());

        // Use item_hash as fingerprint for now
        let item_hash: String = row.get("item_hash");

        Ok(Item {
            dfid: row.get("dfid"),
            local_id: None, // Not in DB schema yet
            legacy_mode: true, // Assume legacy mode for existing items
            identifiers: Vec::new(), // Loaded separately
            aliases: Vec::new(), // Not in DB schema yet
            fingerprint: Some(item_hash),
            enriched_data: enriched_data_map,
            creation_timestamp,
            last_modified,
            source_entries: Vec::new(), // Loaded separately
            confidence_score: 1.0, // Not in DB schema yet
            status,
        })
    }
}

impl StorageBackend for PostgresStorage {
    // ============================================================================
    // RECEIPTS (5 methods)
    // ============================================================================

    fn store_receipt(&mut self, receipt: &Receipt) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            // Insert receipt
            client.execute(
                "INSERT INTO receipts (id, data_hash, timestamp, data_size) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (id) DO UPDATE SET data_hash = $2, timestamp = $3, data_size = $4",
                &[&receipt.id, &receipt.hash, &receipt.timestamp, &(receipt.data_size as i64)]
            ).await.map_err(Self::map_pg_error)?;

            // Insert identifiers
            for identifier in &receipt.identifiers {
                client.execute(
                    "INSERT INTO receipt_identifiers (receipt_id, key, value)
                     VALUES ($1, $2, $3)",
                    &[&receipt.id, &identifier.key, &identifier.value]
                ).await.map_err(Self::map_pg_error)?;
            }

            Ok(())
        })
    }

    fn get_receipt(&self, id: &Uuid) -> Result<Option<Receipt>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            // Get receipt
            let row = client.query_opt(
                "SELECT id, data_hash, timestamp, data_size FROM receipts WHERE id = $1",
                &[id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let mut receipt = Self::row_to_receipt(&row)?;

            // Get identifiers
            let rows = client.query(
                "SELECT key, value FROM receipt_identifiers WHERE receipt_id = $1",
                &[id]
            ).await.map_err(Self::map_pg_error)?;

            receipt.identifiers = rows.iter().map(|row| {
                Identifier {
                    key: row.get("key"),
                    value: row.get("value"),
                }
            }).collect();

            Ok(Some(receipt))
        })
    }

    fn find_receipts_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Receipt>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT DISTINCT r.id, r.data_hash, r.timestamp, r.data_size
                 FROM receipts r
                 JOIN receipt_identifiers ri ON r.id = ri.receipt_id
                 WHERE ri.key = $1 AND ri.value = $2",
                &[&identifier.key, &identifier.value]
            ).await.map_err(Self::map_pg_error)?;

            let mut receipts = Vec::new();
            for row in rows {
                let mut receipt = Self::row_to_receipt(&row)?;

                // Get all identifiers for this receipt
                let id_rows = client.query(
                    "SELECT key, value FROM receipt_identifiers WHERE receipt_id = $1",
                    &[&receipt.id]
                ).await.map_err(Self::map_pg_error)?;

                receipt.identifiers = id_rows.iter().map(|row| {
                    Identifier {
                        key: row.get("key"),
                        value: row.get("value"),
                    }
                }).collect();

                receipts.push(receipt);
            }

            Ok(receipts)
        })
    }

    fn list_receipts(&self) -> Result<Vec<Receipt>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT id, data_hash, timestamp, data_size FROM receipts ORDER BY timestamp DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut receipts = Vec::new();
            for row in rows {
                let mut receipt = Self::row_to_receipt(&row)?;

                // Get identifiers
                let id_rows = client.query(
                    "SELECT key, value FROM receipt_identifiers WHERE receipt_id = $1",
                    &[&receipt.id]
                ).await.map_err(Self::map_pg_error)?;

                receipt.identifiers = id_rows.iter().map(|row| {
                    Identifier {
                        key: row.get("key"),
                        value: row.get("value"),
                    }
                }).collect();

                receipts.push(receipt);
            }

            Ok(receipts)
        })
    }

    // ============================================================================
    // LOGS (2 methods)
    // ============================================================================

    fn store_log(&mut self, log: &LogEntry) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let level_str = format!("{:?}", log.level);
            let context_json = serde_json::to_value(&log.context)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            client.execute(
                "INSERT INTO logs (timestamp, level, engine, event_type, message, context_data)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[&log.timestamp, &level_str, &log.engine, &log.event_type, &log.message, &context_json]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_logs(&self) -> Result<Vec<LogEntry>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT timestamp, level, engine, event_type, message, context_data
                 FROM logs ORDER BY timestamp DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            rows.iter()
                .map(|row| Self::row_to_log(row))
                .collect()
        })
    }

    // ============================================================================
    // DATA LAKE (5 methods)
    // ============================================================================

    fn store_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", entry.status);

            client.execute(
                "INSERT INTO data_lake_entries
                 (entry_id, data_hash, receipt_id, timestamp, status, processing_notes)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (entry_id) DO UPDATE SET
                 status = $5, processing_notes = $6, updated_at = NOW()",
                &[&entry.entry_id, &entry.data_hash, &entry.receipt_id,
                  &entry.timestamp, &status_str, &entry.processing_notes]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_data_lake_entry(&self, entry_id: &Uuid) -> Result<Option<DataLakeEntry>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT entry_id, data_hash, receipt_id, timestamp, status, processing_notes
                 FROM data_lake_entries WHERE entry_id = $1",
                &[entry_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => ProcessingStatus::Pending,
                "Processing" => ProcessingStatus::Processing,
                "Completed" => ProcessingStatus::Completed,
                "Failed" => ProcessingStatus::Failed,
                _ => ProcessingStatus::Pending,
            };

            Ok(Some(DataLakeEntry {
                entry_id: row.get("entry_id"),
                data_hash: row.get("data_hash"),
                receipt_id: row.get("receipt_id"),
                timestamp: row.get("timestamp"),
                status,
                processing_notes: row.get("processing_notes"),
            }))
        })
    }

    fn update_data_lake_entry(&mut self, entry: &DataLakeEntry) -> Result<(), StorageError> {
        self.store_data_lake_entry(entry)
    }

    fn get_data_lake_entries_by_status(&self, status: ProcessingStatus) -> Result<Vec<DataLakeEntry>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", status);

            let rows = client.query(
                "SELECT entry_id, data_hash, receipt_id, timestamp, status, processing_notes
                 FROM data_lake_entries WHERE status = $1 ORDER BY timestamp ASC",
                &[&status_str]
            ).await.map_err(Self::map_pg_error)?;

            let mut entries = Vec::new();
            for row in rows {
                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Pending" => ProcessingStatus::Pending,
                    "Processing" => ProcessingStatus::Processing,
                    "Completed" => ProcessingStatus::Completed,
                    "Failed" => ProcessingStatus::Failed,
                    _ => ProcessingStatus::Pending,
                };

                entries.push(DataLakeEntry {
                    entry_id: row.get("entry_id"),
                    data_hash: row.get("data_hash"),
                    receipt_id: row.get("receipt_id"),
                    timestamp: row.get("timestamp"),
                    status,
                    processing_notes: row.get("processing_notes"),
                });
            }

            Ok(entries)
        })
    }

    fn list_data_lake_entries(&self) -> Result<Vec<DataLakeEntry>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT entry_id, data_hash, receipt_id, timestamp, status, processing_notes
                 FROM data_lake_entries ORDER BY timestamp DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut entries = Vec::new();
            for row in rows {
                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Pending" => ProcessingStatus::Pending,
                    "Processing" => ProcessingStatus::Processing,
                    "Completed" => ProcessingStatus::Completed,
                    "Failed" => ProcessingStatus::Failed,
                    _ => ProcessingStatus::Pending,
                };

                entries.push(DataLakeEntry {
                    entry_id: row.get("entry_id"),
                    data_hash: row.get("data_hash"),
                    receipt_id: row.get("receipt_id"),
                    timestamp: row.get("timestamp"),
                    status,
                    processing_notes: row.get("processing_notes"),
                });
            }

            Ok(entries)
        })
    }

    // ============================================================================
    // ITEMS (7 methods)
    // ============================================================================

    fn store_item(&mut self, item: &Item) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", item.status);
            let enriched_json = serde_json::to_value(&item.enriched_data).ok();

            // Use fingerprint as item_hash, or compute from dfid if not available
            let item_hash = item.fingerprint.as_ref()
                .map(|s| s.clone())
                .unwrap_or_else(|| blake3::hash(item.dfid.as_bytes()).to_string());

            // Convert DateTime<Utc> to timestamp (milliseconds since epoch)
            let created_at_ts = item.creation_timestamp.timestamp_millis();
            let last_updated_ts = item.last_modified.timestamp_millis();

            // Insert item
            client.execute(
                "INSERT INTO items
                 (dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (dfid) DO UPDATE SET
                 item_hash = $2, status = $3, last_updated_ts = $5, enriched_data = $6, updated_at = NOW()",
                &[&item.dfid, &item_hash, &status_str, &created_at_ts, &last_updated_ts, &enriched_json]
            ).await.map_err(Self::map_pg_error)?;

            // Delete old identifiers and source entries
            client.execute(
                "DELETE FROM item_identifiers WHERE dfid = $1",
                &[&item.dfid]
            ).await.map_err(Self::map_pg_error)?;

            client.execute(
                "DELETE FROM item_source_entries WHERE dfid = $1",
                &[&item.dfid]
            ).await.map_err(Self::map_pg_error)?;

            // Insert identifiers
            for identifier in &item.identifiers {
                client.execute(
                    "INSERT INTO item_identifiers (dfid, key, value) VALUES ($1, $2, $3)",
                    &[&item.dfid, &identifier.key, &identifier.value]
                ).await.map_err(Self::map_pg_error)?;
            }

            // Insert source entries
            for entry_id in &item.source_entries {
                client.execute(
                    "INSERT INTO item_source_entries (dfid, entry_id) VALUES ($1, $2)",
                    &[&item.dfid, entry_id]
                ).await.map_err(Self::map_pg_error)?;
            }

            Ok(())
        })
    }

    fn get_item_by_dfid(&self, dfid: &str) -> Result<Option<Item>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data
                 FROM items WHERE dfid = $1",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let mut item = Self::row_to_item(&row)?;

            // Get identifiers
            let id_rows = client.query(
                "SELECT key, value FROM item_identifiers WHERE dfid = $1",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            item.identifiers = id_rows.iter().map(|row| {
                Identifier {
                    key: row.get("key"),
                    value: row.get("value"),
                }
            }).collect();

            // Get source entries
            let source_rows = client.query(
                "SELECT entry_id FROM item_source_entries WHERE dfid = $1",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            item.source_entries = source_rows.iter().map(|row| row.get("entry_id")).collect();

            Ok(Some(item))
        })
    }

    fn update_item(&mut self, item: &Item) -> Result<(), StorageError> {
        self.store_item(item)
    }

    fn list_items(&self) -> Result<Vec<Item>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data
                 FROM items ORDER BY created_at_ts DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut items = Vec::new();
            for row in rows {
                let mut item = Self::row_to_item(&row)?;
                let dfid = item.dfid.clone();

                // Get identifiers
                let id_rows = client.query(
                    "SELECT key, value FROM item_identifiers WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.identifiers = id_rows.iter().map(|row| {
                    Identifier {
                        key: row.get("key"),
                        value: row.get("value"),
                    }
                }).collect();

                // Get source entries
                let source_rows = client.query(
                    "SELECT entry_id FROM item_source_entries WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.source_entries = source_rows.iter().map(|row| row.get("entry_id")).collect();

                items.push(item);
            }

            Ok(items)
        })
    }

    fn find_items_by_identifier(&self, identifier: &Identifier) -> Result<Vec<Item>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT DISTINCT i.dfid, i.item_hash, i.status, i.created_at_ts, i.last_updated_ts, i.enriched_data
                 FROM items i
                 JOIN item_identifiers ii ON i.dfid = ii.dfid
                 WHERE ii.key = $1 AND ii.value = $2",
                &[&identifier.key, &identifier.value]
            ).await.map_err(Self::map_pg_error)?;

            let mut items = Vec::new();
            for row in rows {
                let mut item = Self::row_to_item(&row)?;
                let dfid = item.dfid.clone();

                // Get all identifiers
                let id_rows = client.query(
                    "SELECT key, value FROM item_identifiers WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.identifiers = id_rows.iter().map(|row| {
                    Identifier {
                        key: row.get("key"),
                        value: row.get("value"),
                    }
                }).collect();

                // Get source entries
                let source_rows = client.query(
                    "SELECT entry_id FROM item_source_entries WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.source_entries = source_rows.iter().map(|row| row.get("entry_id")).collect();

                items.push(item);
            }

            Ok(items)
        })
    }

    fn find_items_by_status(&self, status: ItemStatus) -> Result<Vec<Item>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", status);

            let rows = client.query(
                "SELECT dfid, item_hash, status, created_at_ts, last_updated_ts, enriched_data
                 FROM items WHERE status = $1 ORDER BY created_at_ts DESC",
                &[&status_str]
            ).await.map_err(Self::map_pg_error)?;

            let mut items = Vec::new();
            for row in rows {
                let mut item = Self::row_to_item(&row)?;
                let dfid = item.dfid.clone();

                // Get identifiers
                let id_rows = client.query(
                    "SELECT key, value FROM item_identifiers WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.identifiers = id_rows.iter().map(|row| {
                    Identifier {
                        key: row.get("key"),
                        value: row.get("value"),
                    }
                }).collect();

                // Get source entries
                let source_rows = client.query(
                    "SELECT entry_id FROM item_source_entries WHERE dfid = $1",
                    &[&dfid]
                ).await.map_err(Self::map_pg_error)?;

                item.source_entries = source_rows.iter().map(|row| row.get("entry_id")).collect();

                items.push(item);
            }

            Ok(items)
        })
    }

    fn delete_item(&mut self, dfid: &str) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "DELETE FROM items WHERE dfid = $1",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    // ============================================================================
    // IDENTIFIER MAPPINGS (4 methods)
    // ============================================================================

    fn store_identifier_mapping(&mut self, mapping: &IdentifierMapping) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", mapping.status);

            client.execute(
                "INSERT INTO identifier_mappings
                 (mapping_id, identifier_key, identifier_value, dfid, confidence_score, source_entry_id)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (mapping_id) DO UPDATE SET
                 dfid = $4, confidence_score = $5, source_entry_id = $6",
                &[&mapping.mapping_id, &mapping.identifier.key, &mapping.identifier.value,
                  &mapping.dfid, &mapping.confidence_score, &mapping.source_entry_id]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_identifier_mappings(&self, identifier: &Identifier) -> Result<Vec<IdentifierMapping>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT mapping_id, identifier_key, identifier_value, dfid, confidence_score, source_entry_id
                 FROM identifier_mappings
                 WHERE identifier_key = $1 AND identifier_value = $2",
                &[&identifier.key, &identifier.value]
            ).await.map_err(Self::map_pg_error)?;

            let mappings = rows.iter().map(|row| {
                IdentifierMapping {
                    mapping_id: row.get("mapping_id"),
                    identifier: Identifier {
                        key: row.get("identifier_key"),
                        value: row.get("identifier_value"),
                    },
                    dfid: row.get("dfid"),
                    confidence_score: row.get("confidence_score"),
                    source_entry_id: row.get("source_entry_id"),
                    status: MappingStatus::Confirmed, // Default
                }
            }).collect();

            Ok(mappings)
        })
    }

    fn update_identifier_mapping(&mut self, mapping: &IdentifierMapping) -> Result<(), StorageError> {
        self.store_identifier_mapping(mapping)
    }

    fn list_identifier_mappings(&self) -> Result<Vec<IdentifierMapping>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT mapping_id, identifier_key, identifier_value, dfid, confidence_score, source_entry_id
                 FROM identifier_mappings ORDER BY created_at DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mappings = rows.iter().map(|row| {
                IdentifierMapping {
                    mapping_id: row.get("mapping_id"),
                    identifier: Identifier {
                        key: row.get("identifier_key"),
                        value: row.get("identifier_value"),
                    },
                    dfid: row.get("dfid"),
                    confidence_score: row.get("confidence_score"),
                    source_entry_id: row.get("source_entry_id"),
                    status: MappingStatus::Confirmed,
                }
            }).collect();

            Ok(mappings)
        })
    }

    // ============================================================================
    // CONFLICT RESOLUTION (3 methods)
    // ============================================================================

    fn store_conflict_resolution(&mut self, conflict: &ConflictResolution) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", conflict.status);
            let strategy_str = conflict.resolution_strategy.as_ref().map(|s| format!("{:?}", s));

            client.execute(
                "INSERT INTO conflict_resolutions
                 (conflict_id, identifier_key, identifier_value, conflicting_dfids,
                  resolution_strategy, resolved_dfid, status, created_at_ts, resolved_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (conflict_id) DO UPDATE SET
                 resolution_strategy = $5, resolved_dfid = $6, status = $7, resolved_at_ts = $9",
                &[&conflict.conflict_id, &conflict.identifier.key, &conflict.identifier.value,
                  &conflict.conflicting_dfids, &strategy_str, &conflict.resolved_dfid,
                  &status_str, &conflict.created_at, &conflict.resolved_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_conflict_resolution(&self, conflict_id: &Uuid) -> Result<Option<ConflictResolution>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT conflict_id, identifier_key, identifier_value, conflicting_dfids,
                 resolution_strategy, resolved_dfid, status, created_at_ts, resolved_at_ts
                 FROM conflict_resolutions WHERE conflict_id = $1",
                &[conflict_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => ResolutionStatus::Pending,
                "Resolved" => ResolutionStatus::Resolved,
                "Rejected" => ResolutionStatus::Rejected,
                _ => ResolutionStatus::Pending,
            };

            let strategy_str: Option<String> = row.get("resolution_strategy");
            let resolution_strategy = strategy_str.and_then(|s| match s.as_str() {
                "AutoConfidence" => Some(ResolutionStrategy::AutoConfidence),
                "ManualReview" => Some(ResolutionStrategy::ManualReview),
                "TemporalPrecedence" => Some(ResolutionStrategy::TemporalPrecedence),
                _ => None,
            });

            Ok(Some(ConflictResolution {
                conflict_id: row.get("conflict_id"),
                identifier: Identifier {
                    key: row.get("identifier_key"),
                    value: row.get("identifier_value"),
                },
                conflicting_dfids: row.get("conflicting_dfids"),
                resolution_strategy,
                resolved_dfid: row.get("resolved_dfid"),
                status,
                created_at: row.get("created_at_ts"),
                resolved_at: row.get("resolved_at_ts"),
            }))
        })
    }

    fn get_pending_conflicts(&self) -> Result<Vec<ConflictResolution>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT conflict_id, identifier_key, identifier_value, conflicting_dfids,
                 resolution_strategy, resolved_dfid, status, created_at_ts, resolved_at_ts
                 FROM conflict_resolutions WHERE status = 'Pending' ORDER BY created_at_ts ASC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let conflicts = rows.iter().map(|row| {
                let strategy_str: Option<String> = row.get("resolution_strategy");
                let resolution_strategy = strategy_str.and_then(|s| match s.as_str() {
                    "AutoConfidence" => Some(ResolutionStrategy::AutoConfidence),
                    "ManualReview" => Some(ResolutionStrategy::ManualReview),
                    "TemporalPrecedence" => Some(ResolutionStrategy::TemporalPrecedence),
                    _ => None,
                });

                ConflictResolution {
                    conflict_id: row.get("conflict_id"),
                    identifier: Identifier {
                        key: row.get("identifier_key"),
                        value: row.get("identifier_value"),
                    },
                    conflicting_dfids: row.get("conflicting_dfids"),
                    resolution_strategy,
                    resolved_dfid: row.get("resolved_dfid"),
                    status: ResolutionStatus::Pending,
                    created_at: row.get("created_at_ts"),
                    resolved_at: row.get("resolved_at_ts"),
                }
            }).collect();

            Ok(conflicts)
        })
    }

    // ============================================================================
    // EVENTS (8 methods)
    // ============================================================================

    fn store_event(&mut self, event: &Event) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let event_type_str = format!("{:?}", event.event_type);
            let visibility_str = format!("{:?}", event.visibility);
            let metadata_json = serde_json::to_value(&event.metadata)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            client.execute(
                "INSERT INTO events
                 (event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (event_id) DO UPDATE SET
                 event_type = $2, visibility = $5, encrypted_data = $6, metadata = $7",
                &[&event.event_id, &event_type_str, &event.dfid,
                  &event.timestamp, &visibility_str, &event.encrypted_data, &metadata_json]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_event(&self, event_id: &Uuid) -> Result<Option<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events WHERE event_id = $1",
                &[event_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let event_type_str: String = row.get("event_type");
            let event_type = match event_type_str.as_str() {
                "Created" => EventType::Created,
                "Enriched" => EventType::Enriched,
                "Merged" => EventType::Merged,
                "Split" => EventType::Split,
                _ => EventType::Created,
            };

            let visibility_str: String = row.get("visibility");
            let visibility = match visibility_str.as_str() {
                "Public" => EventVisibility::Public,
                "Private" => EventVisibility::Private,
                "CircuitOnly" => EventVisibility::CircuitOnly,
                "Direct" => EventVisibility::Direct,
                _ => EventVisibility::Public,
            };

            let metadata_json: serde_json::Value = row.get("metadata");
            let metadata = serde_json::from_value(metadata_json)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
            let is_encrypted = encrypted_data.is_some();

            Ok(Some(Event {
                event_id: row.get("event_id"),
                dfid: row.get("dfid"),
                event_type,
                timestamp: row.get("timestamp"),
                source: "system".to_string(),
                metadata,
                is_encrypted,
                visibility,
                content_hash: String::new(),
                local_event_id: None,
                is_local: false,
                pushed_to_circuit: None,
                snapshot_id: None,
                snapshot_cid: None,
            }))
        })
    }

    fn update_event(&mut self, event: &Event) -> Result<(), StorageError> {
        self.store_event(event)
    }

    fn list_events(&self) -> Result<Vec<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events ORDER BY timestamp DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut events = Vec::new();
            for row in rows {
                let event_type_str: String = row.get("event_type");
                let event_type = match event_type_str.as_str() {
                    "Created" => EventType::Created,
                    "Enriched" => EventType::Enriched,
                    "Merged" => EventType::Merged,
                    "Split" => EventType::Split,
                    _ => EventType::Created,
                };

                let visibility_str: String = row.get("visibility");
                let visibility = match visibility_str.as_str() {
                    "Public" => EventVisibility::Public,
                    "Private" => EventVisibility::Private,
                    "CircuitOnly" => EventVisibility::CircuitOnly,
                    "Direct" => EventVisibility::Direct,
                    _ => EventVisibility::Public,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
                let is_encrypted = encrypted_data.is_some();

                events.push(Event {
                    event_id: row.get("event_id"),
                    dfid: row.get("dfid"),
                    event_type,
                    timestamp: row.get("timestamp"),
                    source: "system".to_string(),
                    metadata,
                    is_encrypted,
                    visibility,
                    content_hash: String::new(),
                    local_event_id: None,
                    is_local: false,
                    pushed_to_circuit: None,
                    snapshot_id: None,
                    snapshot_cid: None,
                });
            }

            Ok(events)
        })
    }

    fn get_events_by_dfid(&self, dfid: &str) -> Result<Vec<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events WHERE dfid = $1 ORDER BY timestamp DESC",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            let mut events = Vec::new();
            for row in rows {
                let event_type_str: String = row.get("event_type");
                let event_type = match event_type_str.as_str() {
                    "Created" => EventType::Created,
                    "Enriched" => EventType::Enriched,
                    "Merged" => EventType::Merged,
                    "Split" => EventType::Split,
                    _ => EventType::Created,
                };

                let visibility_str: String = row.get("visibility");
                let visibility = match visibility_str.as_str() {
                    "Public" => EventVisibility::Public,
                    "Private" => EventVisibility::Private,
                    "CircuitOnly" => EventVisibility::CircuitOnly,
                    "Direct" => EventVisibility::Direct,
                    _ => EventVisibility::Public,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
                let is_encrypted = encrypted_data.is_some();

                events.push(Event {
                    event_id: row.get("event_id"),
                    dfid: row.get("dfid"),
                    event_type,
                    timestamp: row.get("timestamp"),
                    source: "system".to_string(),
                    metadata,
                    is_encrypted,
                    visibility,
                    content_hash: String::new(),
                    local_event_id: None,
                    is_local: false,
                    pushed_to_circuit: None,
                    snapshot_id: None,
                    snapshot_cid: None,
                });
            }

            Ok(events)
        })
    }

    fn get_events_by_type(&self, event_type: EventType) -> Result<Vec<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let event_type_str = format!("{:?}", event_type);

            let rows = client.query(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events WHERE event_type = $1 ORDER BY timestamp DESC",
                &[&event_type_str]
            ).await.map_err(Self::map_pg_error)?;

            let mut events = Vec::new();
            for row in rows {
                let visibility_str: String = row.get("visibility");
                let visibility = match visibility_str.as_str() {
                    "Public" => EventVisibility::Public,
                    "Private" => EventVisibility::Private,
                    "CircuitOnly" => EventVisibility::CircuitOnly,
                    "Direct" => EventVisibility::Direct,
                    _ => EventVisibility::Public,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
                let is_encrypted = encrypted_data.is_some();

                events.push(Event {
                    event_id: row.get("event_id"),
                    dfid: row.get("dfid"),
                    event_type: event_type.clone(),
                    timestamp: row.get("timestamp"),
                    source: "system".to_string(),
                    metadata,
                    is_encrypted,
                    visibility,
                    content_hash: String::new(),
                    local_event_id: None,
                    is_local: false,
                    pushed_to_circuit: None,
                    snapshot_id: None,
                    snapshot_cid: None,
                });
            }

            Ok(events)
        })
    }

    fn get_events_by_visibility(&self, visibility: EventVisibility) -> Result<Vec<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let visibility_str = format!("{:?}", visibility);

            let rows = client.query(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events WHERE visibility = $1 ORDER BY timestamp DESC",
                &[&visibility_str]
            ).await.map_err(Self::map_pg_error)?;

            let mut events = Vec::new();
            for row in rows {
                let event_type_str: String = row.get("event_type");
                let event_type = match event_type_str.as_str() {
                    "Created" => EventType::Created,
                    "Enriched" => EventType::Enriched,
                    "Merged" => EventType::Merged,
                    "Split" => EventType::Split,
                    _ => EventType::Created,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
                let is_encrypted = encrypted_data.is_some();

                events.push(Event {
                    event_id: row.get("event_id"),
                    dfid: row.get("dfid"),
                    event_type,
                    timestamp: row.get("timestamp"),
                    source: "system".to_string(),
                    metadata,
                    is_encrypted,
                    visibility: visibility.clone(),
                    content_hash: String::new(),
                    local_event_id: None,
                    is_local: false,
                    pushed_to_circuit: None,
                    snapshot_id: None,
                    snapshot_cid: None,
                });
            }

            Ok(events)
        })
    }

    fn get_events_in_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Event>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let start_ts = start.timestamp_millis();
            let end_ts = end.timestamp_millis();

            let rows = client.query(
                "SELECT event_id, event_type, dfid, timestamp, visibility, encrypted_data, metadata
                 FROM events WHERE timestamp >= $1 AND timestamp <= $2 ORDER BY timestamp DESC",
                &[&start_ts, &end_ts]
            ).await.map_err(Self::map_pg_error)?;

            let mut events = Vec::new();
            for row in rows {
                let event_type_str: String = row.get("event_type");
                let event_type = match event_type_str.as_str() {
                    "Created" => EventType::Created,
                    "Enriched" => EventType::Enriched,
                    "Merged" => EventType::Merged,
                    "Split" => EventType::Split,
                    _ => EventType::Created,
                };

                let visibility_str: String = row.get("visibility");
                let visibility = match visibility_str.as_str() {
                    "Public" => EventVisibility::Public,
                    "Private" => EventVisibility::Private,
                    "CircuitOnly" => EventVisibility::CircuitOnly,
                    "Direct" => EventVisibility::Direct,
                    _ => EventVisibility::Public,
                };

                let metadata_json: serde_json::Value = row.get("metadata");
                let metadata = serde_json::from_value(metadata_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let encrypted_data: Option<Vec<u8>> = row.get("encrypted_data");
                let is_encrypted = encrypted_data.is_some();

                events.push(Event {
                    event_id: row.get("event_id"),
                    dfid: row.get("dfid"),
                    event_type,
                    timestamp: row.get("timestamp"),
                    source: "system".to_string(),
                    metadata,
                    is_encrypted,
                    visibility,
                    content_hash: String::new(),
                    local_event_id: None,
                    is_local: false,
                    pushed_to_circuit: None,
                    snapshot_id: None,
                    snapshot_cid: None,
                });
            }

            Ok(events)
        })
    }

    // ============================================================================
    // CIRCUITS (15 methods)
    // ============================================================================

    fn store_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", circuit.status);
            let permissions_json = serde_json::to_value(&circuit.permissions)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            let alias_config_json = circuit.alias_config.as_ref()
                .map(|c| serde_json::to_value(c).ok())
                .flatten();
            let adapter_config_json = circuit.adapter_config.as_ref()
                .map(|c| serde_json::to_value(c).ok())
                .flatten();
            let public_settings_json = circuit.public_settings.as_ref()
                .map(|c| serde_json::to_value(c).ok())
                .flatten();
            let post_action_settings_json = circuit.post_action_settings.as_ref()
                .map(|c| serde_json::to_value(c).ok())
                .flatten();

            client.execute(
                "INSERT INTO circuits
                 (circuit_id, name, description, owner_id, status, created_at_ts, last_modified_ts,
                  permissions, alias_config, adapter_config, public_settings, post_action_settings)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (circuit_id) DO UPDATE SET
                 name = $2, description = $3, status = $5, last_modified_ts = $7,
                 permissions = $8, alias_config = $9, adapter_config = $10,
                 public_settings = $11, post_action_settings = $12, updated_at = NOW()",
                &[&circuit.circuit_id, &circuit.name, &circuit.description, &circuit.owner_id,
                  &status_str, &circuit.created_at, &circuit.last_modified,
                  &permissions_json, &alias_config_json, &adapter_config_json,
                  &public_settings_json, &post_action_settings_json]
            ).await.map_err(Self::map_pg_error)?;

            // Delete and re-insert members
            client.execute(
                "DELETE FROM circuit_members WHERE circuit_id = $1",
                &[&circuit.circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            for member in &circuit.members {
                let role_str = format!("{:?}", member.role);
                let permissions: Vec<String> = member.permissions.iter()
                    .map(|p| format!("{:?}", p))
                    .collect();

                client.execute(
                    "INSERT INTO circuit_members
                     (circuit_id, member_id, role, permissions, joined_at_ts)
                     VALUES ($1, $2, $3, $4, $5)",
                    &[&circuit.circuit_id, &member.member_id, &role_str, &permissions, &member.joined_at]
                ).await.map_err(Self::map_pg_error)?;
            }

            Ok(())
        })
    }

    fn get_circuit(&self, circuit_id: &Uuid) -> Result<Option<Circuit>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT circuit_id, name, description, owner_id, status, created_at_ts, last_modified_ts,
                 permissions, alias_config, adapter_config, public_settings, post_action_settings
                 FROM circuits WHERE circuit_id = $1",
                &[circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Active" => CircuitStatus::Active,
                "Inactive" => CircuitStatus::Inactive,
                "Suspended" => CircuitStatus::Suspended,
                _ => CircuitStatus::Active,
            };

            let permissions_json: serde_json::Value = row.get("permissions");
            let permissions = serde_json::from_value(permissions_json)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            let alias_config_json: Option<serde_json::Value> = row.get("alias_config");
            let alias_config = alias_config_json.and_then(|v| serde_json::from_value(v).ok());

            let adapter_config_json: Option<serde_json::Value> = row.get("adapter_config");
            let adapter_config = adapter_config_json.and_then(|v| serde_json::from_value(v).ok());

            let public_settings_json: Option<serde_json::Value> = row.get("public_settings");
            let public_settings = public_settings_json.and_then(|v| serde_json::from_value(v).ok());

            let post_action_settings_json: Option<serde_json::Value> = row.get("post_action_settings");
            let post_action_settings = post_action_settings_json.and_then(|v| serde_json::from_value(v).ok());

            // Get members
            let member_rows = client.query(
                "SELECT member_id, role, permissions, joined_at_ts
                 FROM circuit_members WHERE circuit_id = $1",
                &[circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            let members = member_rows.iter().map(|row| {
                let role_str: String = row.get("role");
                let role = match role_str.as_str() {
                    "Owner" => MemberRole::Owner,
                    "Admin" => MemberRole::Admin,
                    "Member" => MemberRole::Member,
                    "Viewer" => MemberRole::Viewer,
                    "Custom" => MemberRole::Custom("Custom".to_string()),
                    _ => MemberRole::Member,
                };

                let permission_strs: Vec<String> = row.get("permissions");
                let permissions = permission_strs.iter().filter_map(|p| match p.as_str() {
                    "Push" => Some(Permission::Push),
                    "Pull" => Some(Permission::Pull),
                    "ManageMembers" => Some(Permission::ManageMembers),
                    "ApproveOperations" => Some(Permission::ApproveOperations),
                    "ViewOperations" => Some(Permission::ViewOperations),
                    "ConfigureCircuit" => Some(Permission::ConfigureCircuit),
                    _ => None,
                }).collect();

                CircuitMember {
                    member_id: row.get("member_id"),
                    role,
                    permissions,
                    joined_at: row.get("joined_at_ts"),
                }
            }).collect();

            Ok(Some(Circuit {
                circuit_id: row.get("circuit_id"),
                name: row.get("name"),
                description: row.get("description"),
                owner_id: row.get("owner_id"),
                members,
                status,
                created_at: row.get("created_at_ts"),
                last_modified: row.get("last_modified_ts"),
                permissions,
                alias_config,
                adapter_config,
                public_settings,
                post_action_settings,
            }))
        })
    }

    fn update_circuit(&mut self, circuit: &Circuit) -> Result<(), StorageError> {
        self.store_circuit(circuit)
    }

    fn list_circuits(&self) -> Result<Vec<Circuit>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT circuit_id FROM circuits ORDER BY created_at_ts DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut circuits = Vec::new();
            for row in rows {
                let circuit_id: Uuid = row.get("circuit_id");
                if let Some(circuit) = self.get_circuit(&circuit_id)? {
                    circuits.push(circuit);
                }
            }

            Ok(circuits)
        })
    }

    fn get_circuits_for_member(&self, member_id: &str) -> Result<Vec<Circuit>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT DISTINCT c.circuit_id
                 FROM circuits c
                 JOIN circuit_members cm ON c.circuit_id = cm.circuit_id
                 WHERE cm.member_id = $1 OR c.owner_id = $1
                 ORDER BY c.created_at_ts DESC",
                &[&member_id]
            ).await.map_err(Self::map_pg_error)?;

            let mut circuits = Vec::new();
            for row in rows {
                let circuit_id: Uuid = row.get("circuit_id");
                if let Some(circuit) = self.get_circuit(&circuit_id)? {
                    circuits.push(circuit);
                }
            }

            Ok(circuits)
        })
    }

    fn store_circuit_operation(&mut self, operation: &CircuitOperation) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let operation_type_str = format!("{:?}", operation.operation_type);
            let status_str = format!("{:?}", operation.status);

            client.execute(
                "INSERT INTO circuit_operations
                 (operation_id, circuit_id, operation_type, requester_id, status,
                  created_at_ts, approved_at_ts, approver_id, completed_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (operation_id) DO UPDATE SET
                 status = $5, approved_at_ts = $7, approver_id = $8, completed_at_ts = $9",
                &[&operation.operation_id, &operation.circuit_id, &operation_type_str,
                  &operation.requester_id, &status_str, &operation.created_at,
                  &operation.approved_at, &operation.approver_id, &operation.completed_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_circuit_operation(&self, operation_id: &Uuid) -> Result<Option<CircuitOperation>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT operation_id, circuit_id, operation_type, requester_id, status,
                 created_at_ts, approved_at_ts, approver_id, completed_at_ts
                 FROM circuit_operations WHERE operation_id = $1",
                &[operation_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let operation_type_str: String = row.get("operation_type");
            let operation_type = match operation_type_str.as_str() {
                "Push" => OperationType::Push,
                "Pull" => OperationType::Pull,
                _ => OperationType::Push,
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => OperationStatus::Pending,
                "Approved" => OperationStatus::Approved,
                "Rejected" => OperationStatus::Rejected,
                "Completed" => OperationStatus::Completed,
                "Failed" => OperationStatus::Failed,
                _ => OperationStatus::Pending,
            };

            Ok(Some(CircuitOperation {
                operation_id: row.get("operation_id"),
                circuit_id: row.get("circuit_id"),
                operation_type,
                requester_id: row.get("requester_id"),
                status,
                created_at: row.get("created_at_ts"),
                approved_at: row.get("approved_at_ts"),
                approver_id: row.get("approver_id"),
                completed_at: row.get("completed_at_ts"),
            }))
        })
    }

    fn update_circuit_operation(&mut self, operation: &CircuitOperation) -> Result<(), StorageError> {
        self.store_circuit_operation(operation)
    }

    fn get_circuit_operations(&self, circuit_id: &Uuid) -> Result<Vec<CircuitOperation>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT operation_id FROM circuit_operations
                 WHERE circuit_id = $1 ORDER BY created_at_ts DESC",
                &[circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            let mut operations = Vec::new();
            for row in rows {
                let operation_id: Uuid = row.get("operation_id");
                if let Some(operation) = self.get_circuit_operation(&operation_id)? {
                    operations.push(operation);
                }
            }

            Ok(operations)
        })
    }

    fn store_circuit_item(&mut self, circuit_item: &CircuitItem) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "INSERT INTO circuit_items (circuit_id, dfid, added_at_ts, added_by)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (circuit_id, dfid) DO NOTHING",
                &[&circuit_item.circuit_id, &circuit_item.dfid, &circuit_item.added_at, &circuit_item.added_by]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_circuit_items(&self, circuit_id: &Uuid) -> Result<Vec<CircuitItem>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT circuit_id, dfid, added_at_ts, added_by
                 FROM circuit_items WHERE circuit_id = $1 ORDER BY added_at_ts DESC",
                &[circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            let items = rows.iter().map(|row| {
                CircuitItem {
                    circuit_id: row.get("circuit_id"),
                    dfid: row.get("dfid"),
                    added_at: row.get("added_at_ts"),
                    added_by: row.get("added_by"),
                }
            }).collect();

            Ok(items)
        })
    }

    fn remove_circuit_item(&mut self, circuit_id: &Uuid, dfid: &str) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "DELETE FROM circuit_items WHERE circuit_id = $1 AND dfid = $2",
                &[circuit_id, &dfid]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    // ============================================================================
    // USER ACCOUNTS & AUTHENTICATION (~20 methods)
    // ============================================================================

    fn store_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let tier_str = format!("{:?}", user.tier);
            let status_str = format!("{:?}", user.status);
            let available_adapters: Vec<String> = user.available_adapters.iter()
                .map(|a| format!("{:?}", a))
                .collect();

            client.execute(
                "INSERT INTO user_accounts
                 (user_id, username, email, password_hash, tier, status, is_admin,
                  workspace_id, created_at_ts, last_login_ts, available_adapters)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (user_id) DO UPDATE SET
                 username = $2, email = $3, password_hash = $4, tier = $5, status = $6,
                 is_admin = $7, workspace_id = $8, last_login_ts = $10,
                 available_adapters = $11, updated_at = NOW()",
                &[&user.user_id, &user.username, &user.email, &user.password_hash,
                  &tier_str, &status_str, &user.is_admin, &user.workspace_id,
                  &user.created_at, &user.last_login, &available_adapters]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_user_account(&self, user_id: &str) -> Result<Option<UserAccount>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT user_id, username, email, password_hash, tier, status, is_admin,
                 workspace_id, created_at_ts, last_login_ts, available_adapters
                 FROM user_accounts WHERE user_id = $1",
                &[&user_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let tier_str: String = row.get("tier");
            let tier = match tier_str.as_str() {
                "Free" => UserTier::Free,
                "Starter" => UserTier::Starter,
                "Professional" => UserTier::Professional,
                "Enterprise" => UserTier::Enterprise,
                _ => UserTier::Free,
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Active" => AccountStatus::Active,
                "Suspended" => AccountStatus::Suspended,
                "Deactivated" => AccountStatus::Deactivated,
                _ => AccountStatus::Active,
            };

            let adapter_strs: Vec<String> = row.get("available_adapters");
            let available_adapters = adapter_strs
                .iter()
                .filter_map(|a| AdapterType::from_string(a).ok())
                .collect();

            Ok(Some(UserAccount {
                user_id: row.get("user_id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                tier,
                status,
                is_admin: row.get("is_admin"),
                workspace_id: row.get("workspace_id"),
                created_at: row.get("created_at_ts"),
                last_login: row.get("last_login_ts"),
                available_adapters,
            }))
        })
    }

    fn get_user_by_username(&self, username: &str) -> Result<Option<UserAccount>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT user_id FROM user_accounts WHERE username = $1",
                &[&username]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let user_id: String = row.get("user_id");
            self.get_user_account(&user_id)
        })
    }

    fn get_user_by_email(&self, email: &str) -> Result<Option<UserAccount>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT user_id FROM user_accounts WHERE email = $1",
                &[&email]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let user_id: String = row.get("user_id");
            self.get_user_account(&user_id)
        })
    }

    fn update_user_account(&mut self, user: &UserAccount) -> Result<(), StorageError> {
        self.store_user_account(user)
    }

    fn list_user_accounts(&self) -> Result<Vec<UserAccount>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT user_id FROM user_accounts ORDER BY created_at_ts DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut users = Vec::new();
            for row in rows {
                let user_id: String = row.get("user_id");
                if let Some(user) = self.get_user_account(&user_id)? {
                    users.push(user);
                }
            }

            Ok(users)
        })
    }

    fn delete_user_account(&mut self, user_id: &str) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "DELETE FROM user_accounts WHERE user_id = $1",
                &[&user_id]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn record_credit_transaction(&mut self, transaction: &CreditTransaction) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let transaction_type_str = format!("{:?}", transaction.transaction_type);

            client.execute(
                "INSERT INTO credit_transactions
                 (transaction_id, user_id, amount, transaction_type, description, balance_after, created_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[&transaction.transaction_id, &transaction.user_id, &transaction.amount,
                  &transaction_type_str, &transaction.description, &transaction.balance_after,
                  &transaction.created_at]
            ).await.map_err(Self::map_pg_error)?;

            // Update balance
            client.execute(
                "INSERT INTO credit_balances (user_id, credits, updated_at_ts)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (user_id) DO UPDATE SET
                 credits = $2, updated_at_ts = $3, updated_at = NOW()",
                &[&transaction.user_id, &transaction.balance_after, &transaction.created_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_credit_transaction(&self, transaction_id: &str) -> Result<Option<CreditTransaction>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let transaction_uuid = Uuid::parse_str(transaction_id)
                .map_err(|e| StorageError::ValidationError(format!("Invalid UUID: {}", e)))?;

            let row = client.query_opt(
                "SELECT transaction_id, user_id, amount, transaction_type, description, balance_after, created_at_ts
                 FROM credit_transactions WHERE transaction_id = $1",
                &[&transaction_uuid]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let transaction_type_str: String = row.get("transaction_type");
            let transaction_type = match transaction_type_str.as_str() {
                "Purchase" => TransactionType::Purchase,
                "Usage" => TransactionType::Usage,
                "Refund" => TransactionType::Refund,
                "Grant" => TransactionType::Grant,
                _ => TransactionType::Usage,
            };

            Ok(Some(CreditTransaction {
                transaction_id: row.get("transaction_id"),
                user_id: row.get("user_id"),
                amount: row.get("amount"),
                transaction_type,
                description: row.get("description"),
                balance_after: row.get("balance_after"),
                created_at: row.get("created_at_ts"),
            }))
        })
    }

    fn get_credit_transactions(&self, user_id: &str, limit: Option<usize>) -> Result<Vec<CreditTransaction>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let limit_i64 = limit.unwrap_or(100) as i64;

            let rows = client.query(
                "SELECT transaction_id, user_id, amount, transaction_type, description, balance_after, created_at_ts
                 FROM credit_transactions WHERE user_id = $1
                 ORDER BY created_at_ts DESC LIMIT $2",
                &[&user_id, &limit_i64]
            ).await.map_err(Self::map_pg_error)?;

            let transactions = rows.iter().map(|row| {
                let transaction_type_str: String = row.get("transaction_type");
                let transaction_type = match transaction_type_str.as_str() {
                    "Purchase" => TransactionType::Purchase,
                    "Usage" => TransactionType::Usage,
                    "Refund" => TransactionType::Refund,
                    "Grant" => TransactionType::Grant,
                    _ => TransactionType::Usage,
                };

                CreditTransaction {
                    transaction_id: row.get("transaction_id"),
                    user_id: row.get("user_id"),
                    amount: row.get("amount"),
                    transaction_type,
                    description: row.get("description"),
                    balance_after: row.get("balance_after"),
                    created_at: row.get("created_at_ts"),
                }
            }).collect();

            Ok(transactions)
        })
    }

    fn get_credit_transactions_by_operation(&self, operation_type: &str) -> Result<Vec<CreditTransaction>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT transaction_id, user_id, amount, transaction_type, description, balance_after, created_at_ts
                 FROM credit_transactions WHERE transaction_type = $1
                 ORDER BY created_at_ts DESC",
                &[&operation_type]
            ).await.map_err(Self::map_pg_error)?;

            let transactions = rows.iter().map(|row| {
                let transaction_type_str: String = row.get("transaction_type");
                let transaction_type = match transaction_type_str.as_str() {
                    "Purchase" => TransactionType::Purchase,
                    "Usage" => TransactionType::Usage,
                    "Refund" => TransactionType::Refund,
                    "Grant" => TransactionType::Grant,
                    _ => TransactionType::Usage,
                };

                CreditTransaction {
                    transaction_id: row.get("transaction_id"),
                    user_id: row.get("user_id"),
                    amount: row.get("amount"),
                    transaction_type,
                    description: row.get("description"),
                    balance_after: row.get("balance_after"),
                    created_at: row.get("created_at_ts"),
                }
            }).collect();

            Ok(transactions)
        })
    }

    fn record_admin_action(&mut self, action: &AdminAction) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let action_type_str = format!("{:?}", action.action_type);
            let action_data_json = serde_json::to_value(&action.action_data)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            client.execute(
                "INSERT INTO admin_actions
                 (action_id, admin_id, action_type, target_id, action_data, performed_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[&action.action_id, &action.admin_id, &action_type_str,
                  &action.target_id, &action_data_json, &action.performed_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_admin_actions(&self, admin_id: Option<&str>, limit: Option<usize>) -> Result<Vec<AdminAction>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let limit_i64 = limit.unwrap_or(100) as i64;

            let rows = if let Some(aid) = admin_id {
                client.query(
                    "SELECT action_id, admin_id, action_type, target_id, action_data, performed_at_ts
                     FROM admin_actions WHERE admin_id = $1
                     ORDER BY performed_at_ts DESC LIMIT $2",
                    &[&aid, &limit_i64]
                ).await.map_err(Self::map_pg_error)?
            } else {
                client.query(
                    "SELECT action_id, admin_id, action_type, target_id, action_data, performed_at_ts
                     FROM admin_actions ORDER BY performed_at_ts DESC LIMIT $1",
                    &[&limit_i64]
                ).await.map_err(Self::map_pg_error)?
            };

            let actions = rows.iter().map(|row| {
                let action_type_str: String = row.get("action_type");
                let action_type = match action_type_str.as_str() {
                    "UserCreated" => AdminActionType::UserCreated,
                    "UserSuspended" => AdminActionType::UserSuspended,
                    "TierChanged" => AdminActionType::TierChanged,
                    "CreditsGranted" => AdminActionType::CreditsGranted,
                    "AdapterAccessGranted" => AdminActionType::AdapterAccessGranted,
                    _ => AdminActionType::UserCreated,
                };

                let action_data_json: serde_json::Value = row.get("action_data");
                let action_data = serde_json::from_value(action_data_json).unwrap_or_default();

                AdminAction {
                    action_id: row.get("action_id"),
                    admin_id: row.get("admin_id"),
                    action_type,
                    target_id: row.get("target_id"),
                    action_data,
                    performed_at: row.get("performed_at_ts"),
                }
            }).collect();

            Ok(actions)
        })
    }

    fn get_admin_actions_by_type(&self, action_type: &str) -> Result<Vec<AdminAction>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT action_id, admin_id, action_type, target_id, action_data, performed_at_ts
                 FROM admin_actions WHERE action_type = $1
                 ORDER BY performed_at_ts DESC",
                &[&action_type]
            ).await.map_err(Self::map_pg_error)?;

            let actions = rows.iter().map(|row| {
                let action_type_str: String = row.get("action_type");
                let action_type = match action_type_str.as_str() {
                    "UserCreated" => AdminActionType::UserCreated,
                    "UserSuspended" => AdminActionType::UserSuspended,
                    "TierChanged" => AdminActionType::TierChanged,
                    "CreditsGranted" => AdminActionType::CreditsGranted,
                    "AdapterAccessGranted" => AdminActionType::AdapterAccessGranted,
                    _ => AdminActionType::UserCreated,
                };

                let action_data_json: serde_json::Value = row.get("action_data");
                let action_data = serde_json::from_value(action_data_json).unwrap_or_default();

                AdminAction {
                    action_id: row.get("action_id"),
                    admin_id: row.get("admin_id"),
                    action_type,
                    target_id: row.get("target_id"),
                    action_data,
                    performed_at: row.get("performed_at_ts"),
                }
            }).collect();

            Ok(actions)
        })
    }

    // ===========================================================================
    // REMAINING METHODS - STUB IMPLEMENTATIONS
    // These methods return NotImplemented errors and need full implementation
    // ===========================================================================

    // Item Shares (6 methods)
    fn store_item_share(&mut self, _share: &ItemShare) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_item_share - need item_shares table".to_string()))
    }

    fn get_item_share(&self, _share_id: &str) -> Result<Option<ItemShare>, StorageError> {
        Err(StorageError::NotImplemented("get_item_share".to_string()))
    }

    fn get_shares_for_user(&self, _user_id: &str) -> Result<Vec<ItemShare>, StorageError> {
        Err(StorageError::NotImplemented("get_shares_for_user".to_string()))
    }

    fn get_shares_for_item(&self, _dfid: &str) -> Result<Vec<ItemShare>, StorageError> {
        Err(StorageError::NotImplemented("get_shares_for_item".to_string()))
    }

    fn is_item_shared_with_user(&self, _dfid: &str, _user_id: &str) -> Result<bool, StorageError> {
        Err(StorageError::NotImplemented("is_item_shared_with_user".to_string()))
    }

    fn delete_item_share(&mut self, _share_id: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("delete_item_share".to_string()))
    }

    // Activity methods (3 methods)
    fn store_activity(&mut self, activity: &Activity) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let activity_type_str = format!("{:?}", activity.activity_type);
            let status_str = format!("{:?}", activity.status);
            let details_json = serde_json::to_value(&activity.details)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            client.execute(
                "INSERT INTO activities
                 (activity_id, activity_type, circuit_id, circuit_name, dfids, performed_by,
                  status, details, timestamp_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[&activity.activity_id, &activity_type_str, &activity.circuit_id,
                  &activity.circuit_name, &activity.dfids, &activity.performed_by,
                  &status_str, &details_json, &activity.timestamp]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_activities_for_user(&self, user_id: &str) -> Result<Vec<Activity>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT activity_id, activity_type, circuit_id, circuit_name, dfids, performed_by,
                 status, details, timestamp_ts
                 FROM activities WHERE performed_by = $1
                 ORDER BY timestamp_ts DESC",
                &[&user_id]
            ).await.map_err(Self::map_pg_error)?;

            let activities = rows.iter().map(|row| {
                let activity_type_str: String = row.get("activity_type");
                let activity_type = match activity_type_str.as_str() {
                    "Push" => ActivityType::Push,
                    "Pull" => ActivityType::Pull,
                    "Share" => ActivityType::Share,
                    "CircuitCreated" => ActivityType::CircuitCreated,
                    _ => ActivityType::Push,
                };

                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Completed" => ActivityStatus::Completed,
                    "Failed" => ActivityStatus::Failed,
                    "Pending" => ActivityStatus::Pending,
                    _ => ActivityStatus::Completed,
                };

                let details_json: serde_json::Value = row.get("details");
                let details = serde_json::from_value(details_json).unwrap_or(ActivityDetails {
                    item_count: 0,
                    adapter_type: None,
                    storage_location: None,
                    error_message: None,
                    additional_info: std::collections::HashMap::new(),
                });

                Activity {
                    activity_id: row.get("activity_id"),
                    activity_type,
                    circuit_id: row.get("circuit_id"),
                    circuit_name: row.get("circuit_name"),
                    dfids: row.get("dfids"),
                    performed_by: row.get("performed_by"),
                    status,
                    details,
                    timestamp: row.get("timestamp_ts"),
                }
            }).collect();

            Ok(activities)
        })
    }

    fn get_activities_for_circuit(&self, circuit_id: &Uuid) -> Result<Vec<Activity>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT activity_id, activity_type, circuit_id, circuit_name, dfids, performed_by,
                 status, details, timestamp_ts
                 FROM activities WHERE circuit_id = $1
                 ORDER BY timestamp_ts DESC",
                &[circuit_id]
            ).await.map_err(Self::map_pg_error)?;

            let activities = rows.iter().map(|row| {
                let activity_type_str: String = row.get("activity_type");
                let activity_type = match activity_type_str.as_str() {
                    "Push" => ActivityType::Push,
                    "Pull" => ActivityType::Pull,
                    "Share" => ActivityType::Share,
                    "CircuitCreated" => ActivityType::CircuitCreated,
                    _ => ActivityType::Push,
                };

                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Completed" => ActivityStatus::Completed,
                    "Failed" => ActivityStatus::Failed,
                    "Pending" => ActivityStatus::Pending,
                    _ => ActivityStatus::Completed,
                };

                let details_json: serde_json::Value = row.get("details");
                let details = serde_json::from_value(details_json).unwrap_or(ActivityDetails {
                    item_count: 0,
                    adapter_type: None,
                    storage_location: None,
                    error_message: None,
                    additional_info: std::collections::HashMap::new(),
                });

                Activity {
                    activity_id: row.get("activity_id"),
                    activity_type,
                    circuit_id: row.get("circuit_id"),
                    circuit_name: row.get("circuit_name"),
                    dfids: row.get("dfids"),
                    performed_by: row.get("performed_by"),
                    status,
                    details,
                    timestamp: row.get("timestamp_ts"),
                }
            }).collect();

            Ok(activities)
        })
    }

    fn get_all_activities(&self) -> Result<Vec<Activity>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT activity_id, activity_type, circuit_id, circuit_name, dfids, performed_by,
                 status, details, timestamp_ts
                 FROM activities ORDER BY timestamp_ts DESC LIMIT 1000",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let activities = rows.iter().map(|row| {
                let activity_type_str: String = row.get("activity_type");
                let activity_type = match activity_type_str.as_str() {
                    "Push" => ActivityType::Push,
                    "Pull" => ActivityType::Pull,
                    "Share" => ActivityType::Share,
                    "CircuitCreated" => ActivityType::CircuitCreated,
                    _ => ActivityType::Push,
                };

                let status_str: String = row.get("status");
                let status = match status_str.as_str() {
                    "Completed" => ActivityStatus::Completed,
                    "Failed" => ActivityStatus::Failed,
                    "Pending" => ActivityStatus::Pending,
                    _ => ActivityStatus::Completed,
                };

                let details_json: serde_json::Value = row.get("details");
                let details = serde_json::from_value(details_json).unwrap_or(ActivityDetails {
                    item_count: 0,
                    adapter_type: None,
                    storage_location: None,
                    error_message: None,
                    additional_info: std::collections::HashMap::new(),
                });

                Activity {
                    activity_id: row.get("activity_id"),
                    activity_type,
                    circuit_id: row.get("circuit_id"),
                    circuit_name: row.get("circuit_name"),
                    dfids: row.get("dfids"),
                    performed_by: row.get("performed_by"),
                    status,
                    details,
                    timestamp: row.get("timestamp_ts"),
                }
            }).collect();

            Ok(activities)
        })
    }

    // Audit Events (9 methods) - Need separate audit_events table
    fn store_audit_event(&mut self, _event: &AuditEvent) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_audit_event - need audit_events table".to_string()))
    }

    fn get_audit_event(&self, _event_id: &Uuid) -> Result<Option<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("get_audit_event".to_string()))
    }

    fn query_audit_events(&self, _query: &AuditQuery) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("query_audit_events".to_string()))
    }

    fn list_audit_events(&self) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("list_audit_events".to_string()))
    }

    fn get_audit_events_by_user(&self, _user_id: &str) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("get_audit_events_by_user".to_string()))
    }

    fn get_audit_events_by_type(&self, _event_type: AuditEventType) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("get_audit_events_by_type".to_string()))
    }

    fn get_audit_events_by_severity(&self, _severity: AuditSeverity) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("get_audit_events_by_severity".to_string()))
    }

    fn get_audit_events_in_time_range(&self, _start: DateTime<Utc>, _end: DateTime<Utc>) -> Result<Vec<AuditEvent>, StorageError> {
        Err(StorageError::NotImplemented("get_audit_events_in_time_range".to_string()))
    }

    fn sync_audit_events(&mut self, _events: Vec<AuditEvent>) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("sync_audit_events".to_string()))
    }

    // Security Incidents (7 methods) - Need security_incidents table
    fn store_security_incident(&mut self, _incident: &SecurityIncident) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_security_incident - need security_incidents table".to_string()))
    }

    fn get_security_incident(&self, _incident_id: &Uuid) -> Result<Option<SecurityIncident>, StorageError> {
        Err(StorageError::NotImplemented("get_security_incident".to_string()))
    }

    fn update_security_incident(&mut self, _incident: &SecurityIncident) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("update_security_incident".to_string()))
    }

    fn list_security_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Err(StorageError::NotImplemented("list_security_incidents".to_string()))
    }

    fn get_incidents_by_severity(&self, _severity: AuditSeverity) -> Result<Vec<SecurityIncident>, StorageError> {
        Err(StorageError::NotImplemented("get_incidents_by_severity".to_string()))
    }

    fn get_open_incidents(&self) -> Result<Vec<SecurityIncident>, StorageError> {
        Err(StorageError::NotImplemented("get_open_incidents".to_string()))
    }

    fn get_incidents_by_assignee(&self, _assignee: &str) -> Result<Vec<SecurityIncident>, StorageError> {
        Err(StorageError::NotImplemented("get_incidents_by_assignee".to_string()))
    }

    // Compliance Reports (6 methods) - Need compliance_reports table
    fn store_compliance_report(&mut self, _report: &ComplianceReport) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_compliance_report - need compliance_reports table".to_string()))
    }

    fn get_compliance_report(&self, _report_id: &Uuid) -> Result<Option<ComplianceReport>, StorageError> {
        Err(StorageError::NotImplemented("get_compliance_report".to_string()))
    }

    fn update_compliance_report(&mut self, _report: &ComplianceReport) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("update_compliance_report".to_string()))
    }

    fn list_compliance_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Err(StorageError::NotImplemented("list_compliance_reports".to_string()))
    }

    fn get_reports_by_type(&self, _report_type: &str) -> Result<Vec<ComplianceReport>, StorageError> {
        Err(StorageError::NotImplemented("get_reports_by_type".to_string()))
    }

    fn get_pending_reports(&self) -> Result<Vec<ComplianceReport>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_reports".to_string()))
    }

    // Audit Dashboard (2 methods)
    fn get_audit_dashboard_metrics(&self) -> Result<AuditDashboardMetrics, StorageError> {
        Err(StorageError::NotImplemented("get_audit_dashboard_metrics".to_string()))
    }

    fn get_event_count_by_time_range(&self, _start: DateTime<Utc>, _end: DateTime<Utc>) -> Result<u64, StorageError> {
        Err(StorageError::NotImplemented("get_event_count_by_time_range".to_string()))
    }

    // Pending Items (9 methods) - Need pending_items table
    fn store_pending_item(&mut self, _item: &PendingItem) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_pending_item - need pending_items table".to_string()))
    }

    fn get_pending_item(&self, _pending_id: &Uuid) -> Result<Option<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_item".to_string()))
    }

    fn list_pending_items(&self) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("list_pending_items".to_string()))
    }

    fn get_pending_items_by_reason(&self, _reason_type: &str) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_items_by_reason".to_string()))
    }

    fn get_pending_items_by_user(&self, _user_id: &str) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_items_by_user".to_string()))
    }

    fn get_pending_items_by_workspace(&self, _workspace_id: &str) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_items_by_workspace".to_string()))
    }

    fn get_pending_items_by_priority(&self, _priority: PendingPriority) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_items_by_priority".to_string()))
    }

    fn update_pending_item(&mut self, _item: &PendingItem) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("update_pending_item".to_string()))
    }

    fn delete_pending_item(&mut self, _pending_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("delete_pending_item".to_string()))
    }

    fn get_pending_items_requiring_manual_review(&self) -> Result<Vec<PendingItem>, StorageError> {
        Err(StorageError::NotImplemented("get_pending_items_requiring_manual_review".to_string()))
    }

    // ZK Proofs (10 methods) - Need zk_proofs table
    fn store_zk_proof(&mut self, _proof: &crate::zk_proof_engine::ZkProof) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_zk_proof - need zk_proofs table".to_string()))
    }

    fn get_zk_proof(&self, _proof_id: &Uuid) -> Result<Option<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("get_zk_proof".to_string()))
    }

    fn update_zk_proof(&mut self, _proof: &crate::zk_proof_engine::ZkProof) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("update_zk_proof".to_string()))
    }

    fn query_zk_proofs(&self, _query: &crate::api::zk_proofs::ZkProofQuery) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("query_zk_proofs".to_string()))
    }

    fn list_zk_proofs(&self) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("list_zk_proofs".to_string()))
    }

    fn get_zk_proofs_by_user(&self, _user_id: &str) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("get_zk_proofs_by_user".to_string()))
    }

    fn get_zk_proofs_by_circuit_type(&self, _circuit_type: CircuitType) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("get_zk_proofs_by_circuit_type".to_string()))
    }

    fn get_zk_proofs_by_status(&self, _status: crate::zk_proof_engine::ProofStatus) -> Result<Vec<crate::zk_proof_engine::ZkProof>, StorageError> {
        Err(StorageError::NotImplemented("get_zk_proofs_by_status".to_string()))
    }

    fn get_zk_proof_statistics(&self) -> Result<crate::api::zk_proofs::ZkProofStatistics, StorageError> {
        Err(StorageError::NotImplemented("get_zk_proof_statistics".to_string()))
    }

    fn delete_zk_proof(&mut self, _proof_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("delete_zk_proof".to_string()))
    }

    // Storage History (3 methods)
    fn store_storage_history(&mut self, history: &ItemStorageHistory) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            // Store each record in the history
            for record in &history.storage_records {
                let storage_location_json = serde_json::to_value(&record.storage_location)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                let metadata_json = record.metadata.as_ref()
                    .map(|m| serde_json::to_value(m).ok())
                    .flatten();

                client.execute(
                    "INSERT INTO storage_history
                     (dfid, adapter_type, storage_location, stored_at_ts, triggered_by,
                      triggered_by_id, events_range_start, events_range_end, is_active, metadata)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                    &[&history.dfid, &record.adapter_type, &storage_location_json,
                      &record.stored_at, &record.triggered_by, &record.triggered_by_id,
                      &record.events_range.as_ref().map(|r| r.0),
                      &record.events_range.as_ref().map(|r| r.1),
                      &true, &metadata_json]
                ).await.map_err(Self::map_pg_error)?;
            }

            Ok(())
        })
    }

    fn get_storage_history(&self, dfid: &str) -> Result<Option<ItemStorageHistory>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT id, dfid, adapter_type, storage_location, stored_at_ts, triggered_by,
                 triggered_by_id, events_range_start, events_range_end, is_active, metadata
                 FROM storage_history WHERE dfid = $1 ORDER BY stored_at_ts DESC",
                &[&dfid]
            ).await.map_err(Self::map_pg_error)?;

            if rows.is_empty() {
                return Ok(None);
            }

            let mut storage_records = Vec::new();
            for row in rows {
                let storage_location_json: serde_json::Value = row.get("storage_location");
                let storage_location = serde_json::from_value(storage_location_json)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;

                let metadata_json: Option<serde_json::Value> = row.get("metadata");
                let metadata = metadata_json.and_then(|v| serde_json::from_value(v).ok());

                let events_start: Option<i64> = row.get("events_range_start");
                let events_end: Option<i64> = row.get("events_range_end");
                let events_range = if let (Some(start), Some(end)) = (events_start, events_end) {
                    Some((start, end))
                } else {
                    None
                };

                storage_records.push(StorageRecord {
                    adapter_type: row.get("adapter_type"),
                    storage_location,
                    stored_at: row.get("stored_at_ts"),
                    triggered_by: row.get("triggered_by"),
                    triggered_by_id: row.get("triggered_by_id"),
                    events_range,
                    metadata,
                });
            }

            Ok(Some(ItemStorageHistory {
                dfid: dfid.to_string(),
                storage_records,
            }))
        })
    }

    fn add_storage_record(&mut self, dfid: &str, record: StorageRecord) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let storage_location_json = serde_json::to_value(&record.storage_location)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            let metadata_json = record.metadata.as_ref()
                .map(|m| serde_json::to_value(m).ok())
                .flatten();

            client.execute(
                "INSERT INTO storage_history
                 (dfid, adapter_type, storage_location, stored_at_ts, triggered_by,
                  triggered_by_id, events_range_start, events_range_end, is_active, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[&dfid, &record.adapter_type, &storage_location_json,
                  &record.stored_at, &record.triggered_by, &record.triggered_by_id,
                  &record.events_range.as_ref().map(|r| r.0),
                  &record.events_range.as_ref().map(|r| r.1),
                  &true, &metadata_json]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    // Circuit Adapter Config (4 methods)
    fn store_circuit_adapter_config(&mut self, _config: &CircuitAdapterConfig) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_circuit_adapter_config - stored in circuits table adapter_config field".to_string()))
    }

    fn get_circuit_adapter_config(&self, circuit_id: &Uuid) -> Result<Option<CircuitAdapterConfig>, StorageError> {
        let circuit = self.get_circuit(circuit_id)?;
        Ok(circuit.and_then(|c| c.adapter_config))
    }

    fn update_circuit_adapter_config(&mut self, _config: &CircuitAdapterConfig) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("update_circuit_adapter_config - update circuit instead".to_string()))
    }

    fn list_circuit_adapter_configs(&self) -> Result<Vec<CircuitAdapterConfig>, StorageError> {
        let circuits = self.list_circuits()?;
        Ok(circuits.into_iter().filter_map(|c| c.adapter_config).collect())
    }

    // Notification methods (7 methods)
    fn store_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let notification_type_str = format!("{:?}", notification.notification_type);
            let data_json = notification.data.as_ref()
                .map(|d| serde_json::to_value(d).ok())
                .flatten();

            client.execute(
                "INSERT INTO notifications
                 (notification_id, user_id, notification_type, title, message, data,
                  is_read, created_at_ts, read_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (notification_id) DO UPDATE SET
                 is_read = $7, read_at_ts = $9",
                &[&notification.notification_id, &notification.user_id, &notification_type_str,
                  &notification.title, &notification.message, &data_json,
                  &notification.is_read, &notification.created_at, &notification.read_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_notification(&self, notification_id: &str) -> Result<Option<Notification>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let notification_uuid = Uuid::parse_str(notification_id)
                .map_err(|e| StorageError::ValidationError(format!("Invalid UUID: {}", e)))?;

            let row = client.query_opt(
                "SELECT notification_id, user_id, notification_type, title, message, data,
                 is_read, created_at_ts, read_at_ts
                 FROM notifications WHERE notification_id = $1",
                &[&notification_uuid]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let notification_type_str: String = row.get("notification_type");
            let notification_type = match notification_type_str.as_str() {
                "ItemPushed" => NotificationType::ItemPushed,
                "ItemPulled" => NotificationType::ItemPulled,
                "CircuitInvitation" => NotificationType::CircuitInvitation,
                "OperationApproved" => NotificationType::OperationApproved,
                "OperationRejected" => NotificationType::OperationRejected,
                _ => NotificationType::ItemPushed,
            };

            let data_json: Option<serde_json::Value> = row.get("data");
            let data = data_json.and_then(|v| serde_json::from_value(v).ok());

            Ok(Some(Notification {
                notification_id: row.get("notification_id"),
                user_id: row.get("user_id"),
                notification_type,
                title: row.get("title"),
                message: row.get("message"),
                data,
                is_read: row.get("is_read"),
                created_at: row.get("created_at_ts"),
                read_at: row.get("read_at_ts"),
            }))
        })
    }

    fn get_user_notifications(&self, user_id: &str, since: Option<DateTime<Utc>>, limit: Option<usize>, unread_only: bool) -> Result<Vec<Notification>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let limit_i64 = limit.unwrap_or(100) as i64;
            let since_ts = since.map(|d| d.timestamp_millis());

            let rows = if let Some(ts) = since_ts {
                if unread_only {
                    client.query(
                        "SELECT notification_id FROM notifications
                         WHERE user_id = $1 AND created_at_ts >= $2 AND is_read = false
                         ORDER BY created_at_ts DESC LIMIT $3",
                        &[&user_id, &ts, &limit_i64]
                    ).await.map_err(Self::map_pg_error)?
                } else {
                    client.query(
                        "SELECT notification_id FROM notifications
                         WHERE user_id = $1 AND created_at_ts >= $2
                         ORDER BY created_at_ts DESC LIMIT $3",
                        &[&user_id, &ts, &limit_i64]
                    ).await.map_err(Self::map_pg_error)?
                }
            } else {
                if unread_only {
                    client.query(
                        "SELECT notification_id FROM notifications
                         WHERE user_id = $1 AND is_read = false
                         ORDER BY created_at_ts DESC LIMIT $2",
                        &[&user_id, &limit_i64]
                    ).await.map_err(Self::map_pg_error)?
                } else {
                    client.query(
                        "SELECT notification_id FROM notifications
                         WHERE user_id = $1 ORDER BY created_at_ts DESC LIMIT $2",
                        &[&user_id, &limit_i64]
                    ).await.map_err(Self::map_pg_error)?
                }
            };

            let mut notifications = Vec::new();
            for row in rows {
                let notification_id: Uuid = row.get("notification_id");
                if let Some(notification) = self.get_notification(&notification_id.to_string())? {
                    notifications.push(notification);
                }
            }

            Ok(notifications)
        })
    }

    fn update_notification(&mut self, notification: &Notification) -> Result<(), StorageError> {
        self.store_notification(notification)
    }

    fn delete_notification(&mut self, notification_id: &str) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let notification_uuid = Uuid::parse_str(notification_id)
                .map_err(|e| StorageError::ValidationError(format!("Invalid UUID: {}", e)))?;

            client.execute(
                "DELETE FROM notifications WHERE notification_id = $1",
                &[&notification_uuid]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn mark_all_notifications_read(&mut self, user_id: &str) -> Result<usize, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let now = chrono::Utc::now().timestamp_millis();

            let result = client.execute(
                "UPDATE notifications SET is_read = true, read_at_ts = $2
                 WHERE user_id = $1 AND is_read = false",
                &[&user_id, &now]
            ).await.map_err(Self::map_pg_error)?;

            Ok(result as usize)
        })
    }

    fn get_unread_notification_count(&self, user_id: &str) -> Result<usize, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_one(
                "SELECT COUNT(*) as count FROM notifications
                 WHERE user_id = $1 AND is_read = false",
                &[&user_id]
            ).await.map_err(Self::map_pg_error)?;

            let count: i64 = row.get("count");
            Ok(count as usize)
        })
    }

    // Adapter Config methods (10 methods)
    fn store_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let adapter_type_str = config.adapter_type.to_string();
            let connection_details_json = serde_json::to_value(&config.connection_details)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            let contract_configs_json = config.contract_configs.as_ref()
                .map(|c| serde_json::to_value(c).ok())
                .flatten();

            client.execute(
                "INSERT INTO adapter_configs
                 (config_id, name, description, adapter_type, connection_details, contract_configs,
                  is_active, is_default, created_by, created_at_ts, updated_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (config_id) DO UPDATE SET
                 name = $2, description = $3, connection_details = $5, contract_configs = $6,
                 is_active = $7, is_default = $8, updated_at_ts = $11, updated_at = NOW()",
                &[&config.config_id, &config.name, &config.description, &adapter_type_str,
                  &connection_details_json, &contract_configs_json, &config.is_active,
                  &config.is_default, &config.created_by, &config.created_at, &config.updated_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_adapter_config(&self, config_id: &Uuid) -> Result<Option<AdapterConfig>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT config_id, name, description, adapter_type, connection_details, contract_configs,
                 is_active, is_default, created_by, created_at_ts, updated_at_ts
                 FROM adapter_configs WHERE config_id = $1",
                &[config_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let adapter_type_str: String = row.get("adapter_type");
            let adapter_type =
                AdapterType::from_string(&adapter_type_str).unwrap_or(AdapterType::IpfsIpfs);

            let connection_details_json: serde_json::Value = row.get("connection_details");
            let connection_details = serde_json::from_value(connection_details_json)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            let contract_configs_json: Option<serde_json::Value> = row.get("contract_configs");
            let contract_configs = contract_configs_json.and_then(|v| serde_json::from_value(v).ok());

            Ok(Some(AdapterConfig {
                config_id: row.get("config_id"),
                name: row.get("name"),
                description: row.get("description"),
                adapter_type,
                connection_details,
                contract_configs,
                is_active: row.get("is_active"),
                is_default: row.get("is_default"),
                created_by: row.get("created_by"),
                created_at: row.get("created_at_ts"),
                updated_at: row.get("updated_at_ts"),
            }))
        })
    }

    fn update_adapter_config(&mut self, config: &AdapterConfig) -> Result<(), StorageError> {
        self.store_adapter_config(config)
    }

    fn delete_adapter_config(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "DELETE FROM adapter_configs WHERE config_id = $1",
                &[config_id]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn list_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT config_id FROM adapter_configs ORDER BY created_at_ts DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut configs = Vec::new();
            for row in rows {
                let config_id: Uuid = row.get("config_id");
                if let Some(config) = self.get_adapter_config(&config_id)? {
                    configs.push(config);
                }
            }

            Ok(configs)
        })
    }

    fn list_active_adapter_configs(&self) -> Result<Vec<AdapterConfig>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let rows = client.query(
                "SELECT config_id FROM adapter_configs WHERE is_active = true ORDER BY created_at_ts DESC",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut configs = Vec::new();
            for row in rows {
                let config_id: Uuid = row.get("config_id");
                if let Some(config) = self.get_adapter_config(&config_id)? {
                    configs.push(config);
                }
            }

            Ok(configs)
        })
    }

    fn get_adapter_configs_by_type(&self, adapter_type: &AdapterType) -> Result<Vec<AdapterConfig>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let adapter_type_str = adapter_type.to_string();

            let rows = client.query(
                "SELECT config_id FROM adapter_configs WHERE adapter_type = $1 ORDER BY created_at_ts DESC",
                &[&adapter_type_str]
            ).await.map_err(Self::map_pg_error)?;

            let mut configs = Vec::new();
            for row in rows {
                let config_id: Uuid = row.get("config_id");
                if let Some(config) = self.get_adapter_config(&config_id)? {
                    configs.push(config);
                }
            }

            Ok(configs)
        })
    }

    fn get_default_adapter_config(&self) -> Result<Option<AdapterConfig>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT config_id FROM adapter_configs WHERE is_default = true LIMIT 1",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let config_id: Uuid = row.get("config_id");
            self.get_adapter_config(&config_id)
        })
    }

    fn set_default_adapter(&mut self, config_id: &Uuid) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            // Unset all default flags
            client.execute(
                "UPDATE adapter_configs SET is_default = false",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            // Set the new default
            client.execute(
                "UPDATE adapter_configs SET is_default = true WHERE config_id = $1",
                &[config_id]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn store_adapter_test_result(&mut self, _result: &AdapterTestResult) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_adapter_test_result - need adapter_test_results table".to_string()))
    }

    fn get_adapter_test_result(&self, _config_id: &Uuid) -> Result<Option<AdapterTestResult>, StorageError> {
        Err(StorageError::NotImplemented("get_adapter_test_result".to_string()))
    }

    // LID-DFID Mapping (2 methods)
    fn store_lid_dfid_mapping(&mut self, lid: &Uuid, dfid: &str) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            client.execute(
                "INSERT INTO lid_dfid_mappings (local_id, dfid)
                 VALUES ($1, $2)
                 ON CONFLICT (local_id) DO UPDATE SET dfid = $2",
                &[lid, &dfid]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_dfid_by_lid(&self, lid: &Uuid) -> Result<Option<String>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT dfid FROM lid_dfid_mappings WHERE local_id = $1",
                &[lid]
            ).await.map_err(Self::map_pg_error)?;

            Ok(row.map(|r| r.get("dfid")))
        })
    }

    // Canonical Identifier Lookup (1 method) - Need enhanced_identifiers table
    fn get_dfid_by_canonical(&self, _namespace: &str, _registry: &str, _value: &str) -> Result<Option<String>, StorageError> {
        Err(StorageError::NotImplemented("get_dfid_by_canonical - need enhanced_identifiers table".to_string()))
    }

    // Fingerprint Mapping (2 methods) - Need fingerprint_mappings table
    fn store_fingerprint_mapping(&mut self, _fingerprint: &str, _dfid: &str, _circuit_id: &Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_fingerprint_mapping - need fingerprint_mappings table".to_string()))
    }

    fn get_dfid_by_fingerprint(&self, _fingerprint: &str, _circuit_id: &Uuid) -> Result<Option<String>, StorageError> {
        Err(StorageError::NotImplemented("get_dfid_by_fingerprint - need fingerprint_mappings table".to_string()))
    }

    // Enhanced Identifier Mapping (1 method) - Need enhanced_identifiers table
    fn store_enhanced_identifier_mapping(&mut self, _identifier: &EnhancedIdentifier, _dfid: &str) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented("store_enhanced_identifier_mapping - need enhanced_identifiers table".to_string()))
    }

    // Webhook Deliveries (4 methods)
    fn store_webhook_delivery(&mut self, delivery: &WebhookDelivery) -> Result<(), StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let status_str = format!("{:?}", delivery.status);
            let payload_json = serde_json::to_value(&delivery.payload)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            client.execute(
                "INSERT INTO webhook_deliveries
                 (delivery_id, webhook_id, trigger_event, payload, status, http_status_code,
                  response_body, error_message, attempt_count, delivered_at_ts, created_at_ts,
                  next_retry_at_ts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (delivery_id) DO UPDATE SET
                 status = $5, http_status_code = $6, response_body = $7, error_message = $8,
                 attempt_count = $9, delivered_at_ts = $10, next_retry_at_ts = $12",
                &[&delivery.delivery_id, &delivery.webhook_id, &delivery.trigger_event,
                  &payload_json, &status_str, &delivery.http_status_code, &delivery.response_body,
                  &delivery.error_message, &delivery.attempt_count, &delivery.delivered_at,
                  &delivery.created_at, &delivery.next_retry_at]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn get_webhook_delivery(&self, delivery_id: &Uuid) -> Result<Option<WebhookDelivery>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let row = client.query_opt(
                "SELECT delivery_id, webhook_id, trigger_event, payload, status, http_status_code,
                 response_body, error_message, attempt_count, delivered_at_ts, created_at_ts,
                 next_retry_at_ts
                 FROM webhook_deliveries WHERE delivery_id = $1",
                &[delivery_id]
            ).await.map_err(Self::map_pg_error)?;

            let Some(row) = row else {
                return Ok(None);
            };

            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Pending" => DeliveryStatus::Pending,
                "Delivered" => DeliveryStatus::Delivered,
                "Failed" => DeliveryStatus::Failed,
                "Retrying" => DeliveryStatus::Retrying,
                _ => DeliveryStatus::Pending,
            };

            let payload_json: serde_json::Value = row.get("payload");
            let payload = serde_json::from_value(payload_json)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;

            Ok(Some(WebhookDelivery {
                delivery_id: row.get("delivery_id"),
                webhook_id: row.get("webhook_id"),
                trigger_event: row.get("trigger_event"),
                payload,
                status,
                http_status_code: row.get("http_status_code"),
                response_body: row.get("response_body"),
                error_message: row.get("error_message"),
                attempt_count: row.get("attempt_count"),
                delivered_at: row.get("delivered_at_ts"),
                created_at: row.get("created_at_ts"),
                next_retry_at: row.get("next_retry_at_ts"),
            }))
        })
    }

    fn get_webhook_deliveries_by_circuit(&self, circuit_id: &Uuid, limit: Option<usize>) -> Result<Vec<WebhookDelivery>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let limit_i64 = limit.unwrap_or(100) as i64;

            let rows = client.query(
                "SELECT wd.delivery_id
                 FROM webhook_deliveries wd
                 JOIN webhook_configs wc ON wd.webhook_id = wc.webhook_id
                 WHERE wc.circuit_id = $1
                 ORDER BY wd.created_at_ts DESC LIMIT $2",
                &[circuit_id, &limit_i64]
            ).await.map_err(Self::map_pg_error)?;

            let mut deliveries = Vec::new();
            for row in rows {
                let delivery_id: Uuid = row.get("delivery_id");
                if let Some(delivery) = self.get_webhook_delivery(&delivery_id)? {
                    deliveries.push(delivery);
                }
            }

            Ok(deliveries)
        })
    }

    fn get_webhook_deliveries_by_webhook(&self, webhook_id: &Uuid, limit: Option<usize>) -> Result<Vec<WebhookDelivery>, StorageError> {
        tokio::runtime::Handle::current().block_on(async {
            let client = self.get_conn().await?;

            let limit_i64 = limit.unwrap_or(100) as i64;

            let rows = client.query(
                "SELECT delivery_id FROM webhook_deliveries
                 WHERE webhook_id = $1 ORDER BY created_at_ts DESC LIMIT $2",
                &[webhook_id, &limit_i64]
            ).await.map_err(Self::map_pg_error)?;

            let mut deliveries = Vec::new();
            for row in rows {
                let delivery_id: Uuid = row.get("delivery_id");
                if let Some(delivery) = self.get_webhook_delivery(&delivery_id)? {
                    deliveries.push(delivery);
                }
            }

            Ok(deliveries)
        })
    }

    // ============================================================================
    // Implementation pending
    // ============================================================================

    fn add_cid_to_timeline(
        &mut self,
        _dfid: &str,
        _cid: &str,
        _ipcm_tx: &str,
        _timestamp: i64,
        _network: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_item_timeline(&self, _dfid: &str) -> Result<Vec<TimelineEntry>, StorageError> {
        Ok(Vec::new())
    }

    fn get_timeline_by_sequence(
        &self,
        _dfid: &str,
        _sequence: i32,
    ) -> Result<Option<TimelineEntry>, StorageError> {
        Ok(None)
    }

    fn map_event_to_cid(
        &mut self,
        _event_id: &Uuid,
        _dfid: &str,
        _cid: &str,
        _sequence: i32,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_event_first_cid(&self, _event_id: &Uuid) -> Result<Option<EventCidMapping>, StorageError> {
        Ok(None)
    }

    fn get_events_in_cid(&self, _cid: &str) -> Result<Vec<EventCidMapping>, StorageError> {
        Ok(Vec::new())
    }

    fn update_indexing_progress(
        &mut self,
        _network: &str,
        _last_ledger: i64,
        _confirmed_ledger: i64,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_indexing_progress(&self, _network: &str) -> Result<Option<IndexingProgress>, StorageError> {
        Ok(None)
    }

    fn increment_events_indexed(&mut self, _network: &str, _count: i64) -> Result<(), StorageError> {
        Ok(())
    }

    fn get_system_statistics(&self) -> Result<SystemStatistics, StorageError> {
        Ok(SystemStatistics {
            total_items: 0,
            total_events: 0,
            total_circuits: 0,
            total_users: 0,
            total_receipts: 0,
        })
    }

    fn update_system_statistics(&mut self, _stats: &SystemStatistics) -> Result<(), StorageError> {
        Ok(())
    }

    fn store_user_activity(&mut self, activity: &UserActivity) -> Result<(), StorageError> {
        let client = tokio::runtime::Handle::current().block_on(self.get_conn())?;

        tokio::runtime::Handle::current().block_on(async {
            client.execute(
                "INSERT INTO user_activities (activity_id, user_id, workspace_id, activity_type, timestamp, description)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (activity_id) DO NOTHING",
                &[
                    &activity.activity_id,
                    &activity.user_id,
                    &activity.workspace_id,
                    &format!("{:?}", activity.activity_type),
                    &activity.timestamp.timestamp(),
                    &activity.description,
                ]
            ).await.map_err(Self::map_pg_error)?;

            Ok(())
        })
    }

    fn list_user_activities(&self) -> Result<Vec<UserActivity>, StorageError> {
        let client = tokio::runtime::Handle::current().block_on(self.get_conn())?;

        tokio::runtime::Handle::current().block_on(async {
            let rows = client.query(
                "SELECT activity_id, user_id, workspace_id, activity_type, timestamp, description
                 FROM user_activities
                 ORDER BY timestamp DESC
                 LIMIT 1000",
                &[]
            ).await.map_err(Self::map_pg_error)?;

            let mut activities = Vec::new();
            for row in rows {
                let activity_id: Uuid = row.get("activity_id");
                let user_id: String = row.get("user_id");
                let workspace_id: String = row.get("workspace_id");
                let activity_type_str: String = row.get("activity_type");
                let timestamp_i64: i64 = row.get("timestamp");
                let description: String = row.get("description");

                let timestamp = DateTime::<Utc>::from_timestamp(timestamp_i64, 0)
                    .ok_or_else(|| StorageError::ReadError("Invalid timestamp".to_string()))?;

                let activity_type = match activity_type_str.as_str() {
                    "ItemCreated" => UserActivityType::ItemCreated,
                    "ItemUpdated" => UserActivityType::ItemUpdated,
                    "CircuitJoined" => UserActivityType::CircuitJoined,
                    _ => UserActivityType::ItemCreated,
                };

                activities.push(UserActivity {
                    activity_id,
                    user_id,
                    workspace_id,
                    activity_type,
                    timestamp,
                    description,
                });
            }

            Ok(activities)
        })
    }

    fn clear_user_activities(&mut self) -> Result<(), StorageError> {
        let client = tokio::runtime::Handle::current().block_on(self.get_conn())?;

        tokio::runtime::Handle::current().block_on(async {
            client.execute("DELETE FROM user_activities", &[])
                .await
                .map_err(Self::map_pg_error)?;

            Ok(())
        })
    }
}
