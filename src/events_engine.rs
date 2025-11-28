use crate::logging::LoggingEngine;
use crate::postgres_persistence::PostgresPersistence;
use crate::storage::StorageBackend;
use crate::types::{Event, EventCreationResult, EventType, EventVisibility};
use chrono::{DateTime, Utc};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub enum EventsError {
    StorageError(String),
    EncryptionError(String),
    ValidationError(String),
    NotFound,
}

impl std::fmt::Display for EventsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventsError::StorageError(e) => write!(f, "Storage error: {e}"),
            EventsError::EncryptionError(e) => write!(f, "Encryption error: {e}"),
            EventsError::ValidationError(e) => write!(f, "Validation error: {e}"),
            EventsError::NotFound => write!(f, "Event not found"),
        }
    }
}

impl std::error::Error for EventsError {}

pub struct EventsEngine<S: StorageBackend> {
    storage: S,
    logger: Arc<std::sync::Mutex<LoggingEngine>>,
    postgres: Option<Arc<RwLock<Option<PostgresPersistence>>>>,
}

impl<S: StorageBackend + 'static> EventsEngine<S> {
    pub fn new(storage: S) -> Self {
        let logger = LoggingEngine::new();
        Self {
            storage,
            logger: Arc::new(std::sync::Mutex::new(logger)),
            postgres: None,
        }
    }

    pub fn with_postgres(mut self, postgres: Arc<RwLock<Option<PostgresPersistence>>>) -> Self {
        self.postgres = Some(postgres);
        self
    }

    /// Create event without metadata (backward compatible)
    pub fn create_event(
        &mut self,
        dfid: String,
        event_type: EventType,
        source: String,
        visibility: EventVisibility,
    ) -> Result<Event, EventsError> {
        // Delegate to create_event_with_metadata with empty metadata
        let result =
            self.create_event_with_metadata(dfid, event_type, source, visibility, HashMap::new())?;
        Ok(result.event)
    }

    /// Create event with metadata and automatic deduplication
    /// Returns EventCreationResult with deduplication info
    pub fn create_event_with_metadata(
        &mut self,
        dfid: String,
        event_type: EventType,
        source: String,
        visibility: EventVisibility,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<EventCreationResult, EventsError> {
        // Calculate dedup hash BEFORE creating the event
        let dedup_hash = Event::calculate_dedup_hash(&dfid, &event_type, &source, &metadata);

        self.logger
            .lock()
            .unwrap()
            .info(
                "events_engine",
                "event_creation_started",
                format!("Creating event for DFID: {dfid}"),
            )
            .with_context("dfid", dfid.clone())
            .with_context("event_type", format!("{event_type:?}"))
            .with_context("source", source.clone())
            .with_context("dedup_hash", dedup_hash.clone());

        // Check for existing event with same content hash (deduplication)
        if let Ok(Some(existing_event)) = self.storage.get_event_by_content_hash(&dedup_hash) {
            self.logger
                .lock()
                .unwrap()
                .info(
                    "events_engine",
                    "event_deduplicated",
                    format!("Event deduplicated for DFID: {dfid}"),
                )
                .with_context("existing_event_id", existing_event.event_id.to_string())
                .with_context("content_hash", dedup_hash);

            return Ok(EventCreationResult {
                event: existing_event.clone(),
                was_deduplicated: true,
                original_event_id: Some(existing_event.event_id),
                was_merged: false,
                merged_keys: vec![],
            });
        }

        // No duplicate found, create new event
        let mut event = Event::new_with_metadata(
            dfid.clone(),
            event_type.clone(),
            source.clone(),
            visibility.clone(),
            metadata,
        );

        if matches!(visibility, EventVisibility::Private) {
            event.encrypt();
            self.logger
                .lock()
                .unwrap()
                .info(
                    "events_engine",
                    "event_encrypted",
                    format!("Event encrypted for DFID: {dfid}"),
                )
                .with_context("event_id", event.event_id.to_string());
        }

        // Store in storage first
        self.storage
            .store_event(&event)
            .map_err(|e| EventsError::StorageError(e.to_string()))?;

        self.logger
            .lock()
            .unwrap()
            .info(
                "events_engine",
                "event_created",
                "Event created successfully",
            )
            .with_context("event_id", event.event_id.to_string())
            .with_context("dfid", dfid.clone())
            .with_context("was_deduplicated", "false".to_string());

        // Write-through cache: Persist to PostgreSQL asynchronously (non-blocking)
        if let Some(pg_ref) = &self.postgres {
            let pg = Arc::clone(pg_ref);
            let event_clone = event.clone();
            tokio::spawn(async move {
                let pg_guard = pg.read().await;
                if let Some(pg_persistence) = &*pg_guard {
                    if let Err(e) = pg_persistence.persist_event(&event_clone).await {
                        tracing::warn!("Failed to persist event to PostgreSQL: {}", e);
                        // Don't fail the request - in-memory write succeeded
                    }
                }
            });
        }

        Ok(EventCreationResult {
            event,
            was_deduplicated: false,
            original_event_id: None,
            was_merged: false,
            merged_keys: vec![],
        })
    }

    /// Create a local event (without DFID yet)
    /// Local events are stored with a temporary DFID until pushed to a circuit
    pub fn create_local_event(
        &mut self,
        event_type: EventType,
        source: String,
        visibility: EventVisibility,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<EventCreationResult, EventsError> {
        let event = Event::new_local(event_type.clone(), source.clone(), visibility, metadata);

        self.logger
            .lock()
            .unwrap()
            .info(
                "events_engine",
                "local_event_created",
                format!("Creating local event (no DFID yet)"),
            )
            .with_context("local_event_id", event.local_event_id.unwrap().to_string())
            .with_context("event_type", format!("{event_type:?}"))
            .with_context("source", source);

        // Store in storage
        self.storage
            .store_event(&event)
            .map_err(|e| EventsError::StorageError(e.to_string()))?;

        // Write-through cache: Persist to PostgreSQL asynchronously
        if let Some(pg_ref) = &self.postgres {
            let pg = Arc::clone(pg_ref);
            let event_clone = event.clone();
            tokio::spawn(async move {
                let pg_guard = pg.read().await;
                if let Some(pg_persistence) = &*pg_guard {
                    if let Err(e) = pg_persistence.persist_event(&event_clone).await {
                        tracing::warn!("Failed to persist local event to PostgreSQL: {}", e);
                    }
                }
            });
        }

        Ok(EventCreationResult {
            event,
            was_deduplicated: false,
            original_event_id: None,
            was_merged: false,
            merged_keys: vec![],
        })
    }

    /// Get a local event by its local_event_id
    pub fn get_local_event(&self, local_event_id: &Uuid) -> Result<Option<Event>, EventsError> {
        // Local events use the format LOCAL-EVENT-{uuid} as their DFID
        let temp_dfid = format!("LOCAL-EVENT-{}", local_event_id);
        let events = self
            .storage
            .get_events_by_dfid(&temp_dfid)
            .map_err(|e| EventsError::StorageError(e.to_string()))?;

        // Return the first (should be only) event with this local_event_id
        Ok(events
            .into_iter()
            .find(|e| e.local_event_id == Some(*local_event_id)))
    }

    /// Push a local event to a circuit, assigning it a real DFID
    /// Returns the updated event with the new DFID
    pub fn push_local_event_to_circuit(
        &mut self,
        local_event_id: &Uuid,
        circuit_id: Uuid,
        new_dfid: String,
    ) -> Result<EventCreationResult, EventsError> {
        // Get the local event
        let mut event = self
            .get_local_event(local_event_id)?
            .ok_or_else(|| EventsError::NotFound)?;

        // Validate it's a local event
        if !event.is_local {
            return Err(EventsError::ValidationError(
                "Event is not a local event".to_string(),
            ));
        }

        // Calculate dedup hash with the new DFID to check for duplicates
        let dedup_hash = Event::calculate_dedup_hash(
            &new_dfid,
            &event.event_type,
            &event.source,
            &event.metadata,
        );

        // Check for existing event with same content (deduplication)
        if let Ok(Some(mut existing_event)) = self.storage.get_event_by_content_hash(&dedup_hash) {
            // Auto-merge: merge new metadata into existing event
            let merged_keys = existing_event.merge_metadata(event.metadata.clone());

            if !merged_keys.is_empty() {
                // Update the existing event with merged metadata
                self.storage
                    .update_event(&existing_event)
                    .map_err(|e| EventsError::StorageError(e.to_string()))?;

                self.logger
                    .lock()
                    .unwrap()
                    .info(
                        "events_engine",
                        "local_event_merged",
                        format!("Local event auto-merged during push"),
                    )
                    .with_context("local_event_id", local_event_id.to_string())
                    .with_context("existing_event_id", existing_event.event_id.to_string())
                    .with_context("dfid", new_dfid.clone())
                    .with_context("merged_keys", merged_keys.join(", "));

                // Persist merged event to PostgreSQL
                if let Some(pg_ref) = &self.postgres {
                    let pg = Arc::clone(pg_ref);
                    let event_clone = existing_event.clone();
                    tokio::spawn(async move {
                        let pg_guard = pg.read().await;
                        if let Some(pg_persistence) = &*pg_guard {
                            if let Err(e) = pg_persistence.persist_event(&event_clone).await {
                                tracing::warn!(
                                    "Failed to persist merged event to PostgreSQL: {}",
                                    e
                                );
                            }
                        }
                    });
                }

                return Ok(EventCreationResult {
                    event: existing_event.clone(),
                    was_deduplicated: true,
                    original_event_id: Some(existing_event.event_id),
                    was_merged: true,
                    merged_keys,
                });
            } else {
                // No new metadata to merge, just return existing event
                self.logger
                    .lock()
                    .unwrap()
                    .info(
                        "events_engine",
                        "local_event_deduplicated",
                        format!("Local event deduplicated during push (no merge needed)"),
                    )
                    .with_context("local_event_id", local_event_id.to_string())
                    .with_context("existing_event_id", existing_event.event_id.to_string())
                    .with_context("dfid", new_dfid);

                return Ok(EventCreationResult {
                    event: existing_event.clone(),
                    was_deduplicated: true,
                    original_event_id: Some(existing_event.event_id),
                    was_merged: false,
                    merged_keys: vec![],
                });
            }
        }

        // Push to circuit (updates DFID and marks as non-local)
        event.push_to_circuit(circuit_id, new_dfid.clone());

        self.logger
            .lock()
            .unwrap()
            .info(
                "events_engine",
                "local_event_pushed",
                format!("Local event pushed to circuit"),
            )
            .with_context("local_event_id", local_event_id.to_string())
            .with_context("circuit_id", circuit_id.to_string())
            .with_context("new_dfid", new_dfid);

        // Update in storage
        self.storage
            .update_event(&event)
            .map_err(|e| EventsError::StorageError(e.to_string()))?;

        // Persist to PostgreSQL
        if let Some(pg_ref) = &self.postgres {
            let pg = Arc::clone(pg_ref);
            let event_clone = event.clone();
            tokio::spawn(async move {
                let pg_guard = pg.read().await;
                if let Some(pg_persistence) = &*pg_guard {
                    if let Err(e) = pg_persistence.persist_event(&event_clone).await {
                        tracing::warn!("Failed to persist pushed event to PostgreSQL: {}", e);
                    }
                }
            });
        }

        Ok(EventCreationResult {
            event,
            was_deduplicated: false,
            original_event_id: None,
            was_merged: false,
            merged_keys: vec![],
        })
    }

    pub fn add_event_metadata(
        &mut self,
        event_id: &Uuid,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Event, EventsError> {
        let mut event = self
            .storage
            .get_event(event_id)
            .map_err(|e| EventsError::StorageError(e.to_string()))?
            .ok_or(EventsError::NotFound)?;

        for (key, value) in metadata {
            event.add_metadata(key.clone(), value.clone());
            self.logger
                .lock()
                .unwrap()
                .info("events_engine", "metadata_added", "Metadata added to event")
                .with_context("event_id", event_id.to_string())
                .with_context("metadata_key", key);
        }

        self.storage
            .update_event(&event)
            .map_err(|e| EventsError::StorageError(e.to_string()))?;

        Ok(event)
    }

    pub fn get_events_for_item(&self, dfid: &str) -> Result<Vec<Event>, EventsError> {
        self.storage
            .get_events_by_dfid(dfid)
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn get_events_by_type(&self, event_type: EventType) -> Result<Vec<Event>, EventsError> {
        self.storage
            .get_events_by_type(event_type)
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn get_events_by_visibility(
        &self,
        visibility: EventVisibility,
    ) -> Result<Vec<Event>, EventsError> {
        self.storage
            .get_events_by_visibility(visibility)
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn get_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>, EventsError> {
        self.storage
            .get_events_in_time_range(start, end)
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn get_public_events(&self) -> Result<Vec<Event>, EventsError> {
        self.get_events_by_visibility(EventVisibility::Public)
    }

    pub fn get_private_events(&self) -> Result<Vec<Event>, EventsError> {
        self.get_events_by_visibility(EventVisibility::Private)
    }

    pub fn list_all_events(&self) -> Result<Vec<Event>, EventsError> {
        self.storage
            .list_events()
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn get_event(&self, event_id: &Uuid) -> Result<Option<Event>, EventsError> {
        self.storage
            .get_event(event_id)
            .map_err(|e| EventsError::StorageError(e.to_string()))
    }

    pub fn create_item_created_event(
        &mut self,
        dfid: String,
        source: String,
        identifiers: Vec<String>,
    ) -> Result<Event, EventsError> {
        let event = self.create_event(
            dfid.clone(),
            EventType::Created,
            source,
            EventVisibility::Public,
        )?;

        let identifiers_json: Vec<serde_json::Value> = identifiers
            .into_iter()
            .map(serde_json::Value::String)
            .collect();

        self.add_event_metadata(
            &event.event_id,
            [(
                "identifiers".to_string(),
                serde_json::Value::Array(identifiers_json),
            )]
            .iter()
            .cloned()
            .collect(),
        )
    }

    pub fn create_item_enriched_event(
        &mut self,
        dfid: String,
        source: String,
        data_keys: Vec<String>,
    ) -> Result<Event, EventsError> {
        let event = self.create_event(
            dfid.clone(),
            EventType::Enriched,
            source,
            EventVisibility::Public,
        )?;

        let keys_json: Vec<serde_json::Value> = data_keys
            .into_iter()
            .map(serde_json::Value::String)
            .collect();

        self.add_event_metadata(
            &event.event_id,
            [(
                "enriched_keys".to_string(),
                serde_json::Value::Array(keys_json),
            )]
            .iter()
            .cloned()
            .collect(),
        )
    }

    pub fn create_item_merged_event(
        &mut self,
        primary_dfid: String,
        secondary_dfid: String,
        source: String,
    ) -> Result<Event, EventsError> {
        let event = self.create_event(
            primary_dfid.clone(),
            EventType::Merged,
            source,
            EventVisibility::Public,
        )?;

        self.add_event_metadata(
            &event.event_id,
            [(
                "merged_from".to_string(),
                serde_json::Value::String(secondary_dfid),
            )]
            .iter()
            .cloned()
            .collect(),
        )
    }

    pub fn create_circuit_operation_event(
        &mut self,
        dfid: String,
        circuit_id: String,
        operation: String,
        requester_id: String,
        visibility: EventVisibility,
    ) -> Result<Event, EventsError> {
        let event_type = match operation.as_str() {
            "push" => EventType::PushedToCircuit,
            "pull" => EventType::PulledFromCircuit,
            _ => {
                return Err(EventsError::ValidationError(
                    "Invalid operation type".to_string(),
                ))
            }
        };

        let event =
            self.create_event(dfid.clone(), event_type, requester_id.clone(), visibility)?;

        let metadata = [
            (
                "circuit_id".to_string(),
                serde_json::Value::String(circuit_id),
            ),
            (
                "requester_id".to_string(),
                serde_json::Value::String(requester_id),
            ),
            (
                "operation".to_string(),
                serde_json::Value::String(operation),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        self.add_event_metadata(&event.event_id, metadata)
    }

    pub fn get_logs(&self) -> Vec<crate::logging::LogEntry> {
        self.logger.lock().unwrap().get_logs().to_vec()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<crate::logging::LogEntry> {
        self.logger
            .lock()
            .unwrap()
            .get_logs_by_event_type(event_type)
            .into_iter()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorage;
    use std::sync::Arc;

    #[test]
    fn test_create_event() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut events_engine = EventsEngine::new(storage);

        let result = events_engine.create_event(
            "DFID-123".to_string(),
            EventType::Created,
            "test_source".to_string(),
            EventVisibility::Public,
        );

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.dfid, "DFID-123");
        assert_eq!(event.source, "test_source");
        assert_eq!(event.visibility, EventVisibility::Public);
    }

    #[test]
    fn test_create_private_event() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut events_engine = EventsEngine::new(storage);

        let result = events_engine.create_event(
            "DFID-123".to_string(),
            EventType::Created,
            "test_source".to_string(),
            EventVisibility::Private,
        );

        assert!(result.is_ok());
        let event = result.unwrap();
        assert!(event.is_encrypted);
        assert_eq!(event.visibility, EventVisibility::Private);
    }

    #[test]
    fn test_add_metadata() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut events_engine = EventsEngine::new(storage);

        let event = events_engine
            .create_event(
                "DFID-123".to_string(),
                EventType::Created,
                "test_source".to_string(),
                EventVisibility::Public,
            )
            .unwrap();

        let metadata = [(
            "key1".to_string(),
            serde_json::Value::String("value1".to_string()),
        )]
        .iter()
        .cloned()
        .collect();

        let result = events_engine.add_event_metadata(&event.event_id, metadata);
        assert!(result.is_ok());

        let updated_event = result.unwrap();
        assert_eq!(
            updated_event.metadata.get("key1").unwrap(),
            &serde_json::Value::String("value1".to_string())
        );
    }

    #[test]
    fn test_get_events_for_item() {
        let storage = Arc::new(std::sync::Mutex::new(InMemoryStorage::new()));
        let mut events_engine = EventsEngine::new(storage);

        events_engine
            .create_event(
                "DFID-123".to_string(),
                EventType::Created,
                "source1".to_string(),
                EventVisibility::Public,
            )
            .unwrap();

        events_engine
            .create_event(
                "DFID-123".to_string(),
                EventType::Enriched,
                "source2".to_string(),
                EventVisibility::Public,
            )
            .unwrap();

        let events = events_engine.get_events_for_item("DFID-123").unwrap();
        assert_eq!(events.len(), 2);
    }
}
