use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::{
    DfidLocation, GetLocationsResponse, IndexError, LocationInput, LocationType,
    RegisterLocationRequest, RegisterLocationResponse, SearchResponse,
};

pub struct IndexEngine {
    pool: PgPool,
}

impl IndexEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register a new location for a DFID
    pub async fn register_location(
        &self,
        request: RegisterLocationRequest,
        registered_by: &str,
    ) -> Result<RegisterLocationResponse, IndexError> {
        let location_id = Uuid::new_v4();
        let registered_at = Utc::now();

        let (location_type, location_url) = match &request.location {
            LocationInput::Circuit {
                circuit_id,
                circuit_name,
                url,
            } => (
                serde_json::to_value(LocationType::Circuit {
                    circuit_id: *circuit_id,
                    circuit_name: circuit_name.clone(),
                })
                .unwrap(),
                url.clone(),
            ),
            LocationInput::Blockchain { chain, tx_hash, url } => (
                serde_json::to_value(LocationType::Blockchain {
                    chain: chain.clone(),
                    tx_hash: tx_hash.clone(),
                })
                .unwrap(),
                url.clone(),
            ),
            LocationInput::Ipfs { cid, url } => (
                serde_json::to_value(LocationType::Ipfs { cid: cid.clone() }).unwrap(),
                url.clone(),
            ),
            LocationInput::Registry {
                registry_name,
                registry_url,
            } => (
                serde_json::to_value(LocationType::Registry {
                    registry_name: registry_name.clone(),
                    registry_url: registry_url.clone(),
                })
                .unwrap(),
                registry_url.clone(),
            ),
        };

        sqlx::query(
            r#"
            INSERT INTO dfid_locations
            (location_id, dfid, location_type, location_url, metadata, registered_by, registered_at, verified)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(location_id)
        .bind(&request.dfid)
        .bind(location_type)
        .bind(location_url)
        .bind(&request.metadata)
        .bind(registered_by)
        .bind(registered_at)
        .bind(false) // New locations are unverified by default
        .execute(&self.pool)
        .await?;

        Ok(RegisterLocationResponse {
            success: true,
            location_id,
            dfid: request.dfid,
            registered_at,
        })
    }

    /// Get all locations for a specific DFID
    pub async fn get_locations(&self, dfid: &str) -> Result<GetLocationsResponse, IndexError> {
        let rows = sqlx::query_as::<_, (Uuid, String, serde_json::Value, String, serde_json::Value, String, chrono::DateTime<Utc>, bool, Option<chrono::DateTime<Utc>>)>(
            r#"
            SELECT location_id, dfid, location_type, location_url, metadata,
                   registered_by, registered_at, verified, last_verified
            FROM dfid_locations
            WHERE dfid = $1
            ORDER BY registered_at DESC
            "#
        )
        .bind(dfid)
        .fetch_all(&self.pool)
        .await?;

        let locations: Vec<DfidLocation> = rows
            .into_iter()
            .map(|(location_id, dfid, location_type_json, location_url, metadata, registered_by, registered_at, verified, last_verified)| {
                let location_type: LocationType =
                    serde_json::from_value(location_type_json).unwrap();

                DfidLocation {
                    location_id,
                    dfid,
                    location_type,
                    location_url,
                    metadata,
                    registered_by,
                    registered_at,
                    verified,
                    last_verified,
                }
            })
            .collect();

        let total = locations.len();

        Ok(GetLocationsResponse {
            dfid: dfid.to_string(),
            locations,
            total_locations: total,
        })
    }

    /// Search for DFIDs by partial match
    pub async fn search_dfids(&self, query: &str) -> Result<SearchResponse, IndexError> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT DISTINCT dfid
            FROM dfid_locations
            WHERE dfid ILIKE $1
            ORDER BY dfid
            LIMIT 100
            "#
        )
        .bind(search_pattern)
        .fetch_all(&self.pool)
        .await?;

        let dfids: Vec<String> = rows.into_iter().map(|(dfid,)| dfid).collect();
        let total = dfids.len();

        Ok(SearchResponse {
            query: query.to_string(),
            dfids,
            total,
        })
    }

    /// Verify a location (admin only)
    pub async fn verify_location(&self, location_id: Uuid) -> Result<(), IndexError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE dfid_locations
            SET verified = true, last_verified = $1
            WHERE location_id = $2
            "#
        )
        .bind(now)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a location (admin only)
    pub async fn delete_location(&self, location_id: Uuid) -> Result<(), IndexError> {
        sqlx::query(
            r#"
            DELETE FROM dfid_locations
            WHERE location_id = $1
            "#
        )
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get statistics about the index
    pub async fn get_stats(&self) -> Result<serde_json::Value, IndexError> {
        let total_dfids: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT dfid)
            FROM dfid_locations
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_locations: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM dfid_locations
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let verified_locations: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM dfid_locations
            WHERE verified = true
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(serde_json::json!({
            "total_dfids": total_dfids.0,
            "total_locations": total_locations.0,
            "verified_locations": verified_locations.0,
            "unverified_locations": total_locations.0 - verified_locations.0
        }))
    }
}
