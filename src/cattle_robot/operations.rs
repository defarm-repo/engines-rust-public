use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::api_client::{ApiError, EnhancedIdentifier, RailwayApiClient};
use super::data_generator::{CattleData, DataGenerator, EventData};

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("API error: {0}")]
    ApiError(#[from] ApiError),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Circuit not configured")]
    CircuitNotConfigured,

    #[error("No cattle available for update")]
    NoCattleAvailable,

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

pub struct MintResult {
    pub cattle_id: Uuid,
    pub sisbov: String,
    pub dfid: String,
    pub local_id: String,
    pub cid: Option<String>,
    pub stellar_tx: Option<String>,
}

pub struct UpdateResult {
    pub cattle_id: Uuid,
    pub event_type: String,
    pub dfid: String,
    pub local_id: String,
}

/// Execute a new cattle mint operation
pub async fn mint_new_cattle(
    api_client: &RailwayApiClient,
    data_generator: &mut DataGenerator,
    pool: &PgPool,
    circuit_id: &str,
    requester_id: &str,
) -> Result<MintResult, OperationError> {
    // Generate cattle data
    let cattle = data_generator.generate_cattle();

    log::info!(
        "Minting new cattle: SISBOV={}, Breed={}, State={}, Owner={}",
        cattle.sisbov,
        cattle.breed,
        cattle.state,
        &cattle.owner_hash[..20]
    );

    // Generate birth event
    let birth_event = data_generator.generate_birth_event(&cattle);

    // Build enriched data
    let enriched_data = build_enriched_data(&cattle, &birth_event);

    // Build identifiers
    let identifiers = vec![
        EnhancedIdentifier::canonical("bovino", "sisbov", &cattle.sisbov),
        EnhancedIdentifier::contextual("bovino", "owner", &cattle.owner_hash),
    ];

    // Step 1: Create local item
    log::debug!("Creating local item...");
    let local_response = api_client
        .create_local_item(identifiers, enriched_data)
        .await?;

    let local_id = Uuid::parse_str(&local_response.local_id)
        .map_err(|e| OperationError::OperationFailed(format!("Invalid local_id: {e}")))?;

    log::debug!("Local item created: {local_id}");

    // Step 2: Push to circuit for tokenization
    log::debug!("Pushing to circuit {circuit_id}...");
    let push_response = api_client
        .push_local_to_circuit(circuit_id, &local_response.local_id, requester_id)
        .await?;

    log::info!(
        "✓ Cattle minted: DFID={}, CID={:?}",
        push_response.dfid,
        push_response
            .storage_metadata
            .as_ref()
            .and_then(|m| m.cid.as_ref())
    );

    // Extract storage metadata
    let (cid, stellar_tx) = match push_response.storage_metadata {
        Some(ref metadata) => (metadata.cid.clone(), metadata.stellar_tx.clone()),
        None => (None, None),
    };

    // Step 3: Store in database
    let cattle_id = store_cattle_in_db(pool, &cattle, local_id).await?;
    store_event_in_db(pool, cattle_id, &birth_event, &push_response.dfid, local_id).await?;
    store_mint_in_db(
        pool,
        cattle_id,
        &push_response.dfid,
        local_id,
        cid.as_deref(),
        stellar_tx.as_deref(),
        &push_response.operation_id,
    )
    .await?;

    Ok(MintResult {
        cattle_id,
        sisbov: cattle.sisbov,
        dfid: push_response.dfid,
        local_id: local_response.local_id,
        cid,
        stellar_tx,
    })
}

/// Execute an update operation on existing cattle
pub async fn update_existing_cattle(
    api_client: &RailwayApiClient,
    data_generator: &mut DataGenerator,
    pool: &PgPool,
    circuit_id: &str,
    requester_id: &str,
) -> Result<UpdateResult, OperationError> {
    // Select random cattle from database
    let cattle_record = select_random_cattle(pool).await?;

    log::info!(
        "Updating cattle: SISBOV={}, ID={}",
        cattle_record.sisbov,
        cattle_record.id
    );

    // Generate event based on type
    let event_type = data_generator.select_event_type();
    let event_data = generate_event_for_update(data_generator, &cattle_record, event_type);

    log::debug!("Generated {} event", event_data.event_type);

    // Build enriched data with event
    let enriched_data = build_enriched_data_for_event(&event_data);

    // Build identifiers (SISBOV for lookup)
    let identifiers = vec![EnhancedIdentifier::canonical(
        "bovino",
        "sisbov",
        &cattle_record.sisbov,
    )];

    // Step 1: Create local item with event data
    log::debug!("Creating local item for update...");
    let local_response = api_client
        .create_local_item(identifiers, enriched_data)
        .await?;

    let local_id = Uuid::parse_str(&local_response.local_id)
        .map_err(|e| OperationError::OperationFailed(format!("Invalid local_id: {e}")))?;

    // Step 2: Push to circuit
    log::debug!("Pushing update to circuit {circuit_id}...");
    let push_response = api_client
        .push_local_to_circuit(circuit_id, &local_response.local_id, requester_id)
        .await?;

    log::info!(
        "✓ Cattle updated: DFID={}, Event={}",
        push_response.dfid,
        event_data.event_type
    );

    // Step 3: Store event in database
    store_event_in_db(
        pool,
        cattle_record.id,
        &event_data,
        &push_response.dfid,
        local_id,
    )
    .await?;

    // Update cattle owner if transfer event
    if event_data.event_type == "transfer" {
        if let Some(new_owner) = &event_data.to_owner_hash {
            update_cattle_owner(pool, cattle_record.id, new_owner).await?;
        }
    }

    Ok(UpdateResult {
        cattle_id: cattle_record.id,
        event_type: event_data.event_type,
        dfid: push_response.dfid,
        local_id: local_response.local_id,
    })
}

// Helper functions

fn build_enriched_data(cattle: &CattleData, birth_event: &EventData) -> serde_json::Value {
    json!({
        "sisbov": cattle.sisbov,
        "breed": cattle.breed,
        "gender": cattle.gender,
        "birth_date": cattle.birth_date.to_string(),
        "state": cattle.state,
        "municipality": cattle.municipality_name,
        "municipality_code": cattle.municipality_code,
        "owner_type": cattle.owner_type,
        "birth_weight_kg": birth_event.metadata.get("birth_weight_kg"),
        "mother_sisbov": birth_event.metadata.get("mother_sisbov"),
        "location": birth_event.metadata.get("location"),
    })
}

fn build_enriched_data_for_event(event: &EventData) -> serde_json::Value {
    let mut data = json!({
        "event_type": event.event_type,
        "event_date": event.event_date.to_string(),
    });

    // Add event-specific metadata
    if let Some(obj) = data.as_object_mut() {
        for (key, value) in &event.metadata {
            obj.insert(key.clone(), value.clone());
        }
    }

    data
}

fn generate_event_for_update(
    data_generator: &mut DataGenerator,
    cattle_record: &CattleRecord,
    event_type: &str,
) -> EventData {
    let current_date = Utc::now().naive_utc().date();

    // Convert CattleRecord to CattleData for event generation
    let cattle_data = CattleData {
        sisbov: cattle_record.sisbov.clone(),
        birth_date: cattle_record.birth_date,
        breed: cattle_record.breed.clone(),
        gender: cattle_record.gender.clone(),
        state: cattle_record.state.clone(),
        municipality_code: cattle_record.municipality_code.clone().unwrap_or_default(),
        municipality_name: "".to_string(), // Not used
        owner_hash: cattle_record.owner_hash.clone(),
        owner_type: "".to_string(), // Not used
    };

    match event_type {
        "weight" => data_generator.generate_weight_event(&cattle_data, current_date),
        "transfer" => data_generator.generate_transfer_event(&cattle_data, current_date),
        "vaccination" => data_generator.generate_vaccination_event(&cattle_data, current_date),
        "movement" => data_generator.generate_movement_event(&cattle_data, current_date),
        _ => data_generator.generate_weight_event(&cattle_data, current_date), // Default
    }
}

// Database operations

#[derive(Debug, sqlx::FromRow)]
struct CattleRecord {
    id: Uuid,
    sisbov: String,
    birth_date: NaiveDate,
    breed: String,
    gender: String,
    state: String,
    municipality_code: Option<String>,
    owner_hash: String,
}

async fn store_cattle_in_db(
    pool: &PgPool,
    cattle: &CattleData,
    local_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO robot_cattle (id, sisbov, birth_date, breed, gender, state, municipality_code, owner_hash, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active')
        "#
    )
    .bind(id)
    .bind(&cattle.sisbov)
    .bind(cattle.birth_date)
    .bind(&cattle.breed)
    .bind(&cattle.gender)
    .bind(&cattle.state)
    .bind(&cattle.municipality_code)
    .bind(&cattle.owner_hash)
    .execute(pool)
    .await?;

    Ok(id)
}

async fn store_event_in_db(
    pool: &PgPool,
    cattle_id: Uuid,
    event: &EventData,
    dfid: &str,
    local_id: Uuid,
) -> Result<(), sqlx::Error> {
    let metadata_json = serde_json::to_value(&event.metadata).unwrap_or(json!({}));

    sqlx::query(
        r#"
        INSERT INTO robot_events (cattle_id, event_type, event_date, from_owner_hash, to_owner_hash, vet_hash, metadata, dfid, local_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#
    )
    .bind(cattle_id)
    .bind(&event.event_type)
    .bind(event.event_date)
    .bind(&event.from_owner_hash)
    .bind(&event.to_owner_hash)
    .bind(&event.vet_hash)
    .bind(metadata_json)
    .bind(dfid)
    .bind(local_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn store_mint_in_db(
    pool: &PgPool,
    cattle_id: Uuid,
    dfid: &str,
    local_id: Uuid,
    cid: Option<&str>,
    stellar_tx: Option<&str>,
    operation_id: &str,
) -> Result<(), sqlx::Error> {
    let operation_uuid = Uuid::parse_str(operation_id).ok();

    sqlx::query(
        r#"
        INSERT INTO robot_mints (cattle_id, dfid, local_id, cid, stellar_tx, operation_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(cattle_id)
    .bind(dfid)
    .bind(local_id)
    .bind(cid)
    .bind(stellar_tx)
    .bind(operation_uuid)
    .execute(pool)
    .await?;

    Ok(())
}

async fn select_random_cattle(pool: &PgPool) -> Result<CattleRecord, OperationError> {
    let record = sqlx::query_as::<_, CattleRecord>(
        r#"
        SELECT id, sisbov, birth_date, breed, gender, state, municipality_code, owner_hash
        FROM robot_cattle
        WHERE status = 'active'
        ORDER BY RANDOM()
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    record.ok_or(OperationError::NoCattleAvailable)
}

async fn update_cattle_owner(
    pool: &PgPool,
    cattle_id: Uuid,
    new_owner_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE robot_cattle
        SET owner_hash = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(new_owner_hash)
    .bind(cattle_id)
    .execute(pool)
    .await?;

    Ok(())
}
