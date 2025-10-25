/// Redis Cache Layer for High-Performance Distributed Caching
///
/// This module provides a Redis-based cache layer that sits in front of PostgreSQL,
/// enabling horizontal scaling with multiple API instances sharing the same cache.
///
/// Architecture:
/// ```
/// API Instance 1 ─┐
///                 ├──▶ Redis Cache ──▶ PostgreSQL Primary
/// API Instance 2 ─┘
/// ```
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing;

use crate::types::{Circuit, Event, Item};

/// Redis cache with connection pooling
pub struct RedisCache {
    pool: RedisPool,
    default_ttl: Duration,
}

impl RedisCache {
    /// Create a new Redis cache from connection URL
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `default_ttl` - Default TTL for cached items (e.g., 3600s = 1 hour)
    pub fn new(redis_url: &str, default_ttl: Duration) -> Result<Self, String> {
        let cfg = RedisConfig::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| format!("Failed to create Redis pool: {e}"))?;

        tracing::info!("✅ Redis cache initialized (TTL: {:?})", default_ttl);

        Ok(Self { pool, default_ttl })
    }

    /// Get a connection from the pool
    async fn get_conn(&self) -> Result<deadpool_redis::Connection, String> {
        self.pool
            .get()
            .await
            .map_err(|e| format!("Failed to get Redis connection: {e}"))
    }

    // ============================================================================
    // ITEM CACHE OPERATIONS
    // ============================================================================

    /// Get item from cache by DFID
    pub async fn get_item(&self, dfid: &str) -> Result<Option<Item>, String> {
        let mut conn = self.get_conn().await?;
        let key = format!("item:{dfid}");

        let cached: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| format!("Redis get failed: {e}"))?;

        if let Some(json) = cached {
            let item: Item = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize item: {e}"))?;
            tracing::debug!("🎯 Cache HIT: item {}", dfid);
            Ok(Some(item))
        } else {
            tracing::debug!("❌ Cache MISS: item {}", dfid);
            Ok(None)
        }
    }

    /// Store item in cache
    pub async fn set_item(&self, item: &Item) -> Result<(), String> {
        let mut conn = self.get_conn().await?;
        let key = format!("item:{}", item.dfid);
        let json =
            serde_json::to_string(item).map_err(|e| format!("Failed to serialize item: {e}"))?;

        let _: () = conn
            .set_ex(&key, json, self.default_ttl.as_secs())
            .await
            .map_err(|e| format!("Redis set failed: {e}"))?;

        tracing::debug!("✅ Cached item: {}", item.dfid);
        Ok(())
    }

    /// Delete item from cache
    pub async fn delete_item(&self, dfid: &str) -> Result<(), String> {
        let mut conn = self.get_conn().await?;
        let key = format!("item:{dfid}");

        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| format!("Redis delete failed: {e}"))?;

        tracing::debug!("🗑️  Deleted from cache: item {}", dfid);
        Ok(())
    }

    // ============================================================================
    // CIRCUIT CACHE OPERATIONS
    // ============================================================================

    /// Get circuit from cache by ID
    pub async fn get_circuit(&self, circuit_id: &str) -> Result<Option<Circuit>, String> {
        let mut conn = self.get_conn().await?;
        let key = format!("circuit:{circuit_id}");

        let cached: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| format!("Redis get failed: {e}"))?;

        if let Some(json) = cached {
            let circuit: Circuit = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize circuit: {e}"))?;
            tracing::debug!("🎯 Cache HIT: circuit {}", circuit_id);
            Ok(Some(circuit))
        } else {
            tracing::debug!("❌ Cache MISS: circuit {}", circuit_id);
            Ok(None)
        }
    }

    /// Store circuit in cache
    pub async fn set_circuit(&self, circuit: &Circuit) -> Result<(), String> {
        let mut conn = self.get_conn().await?;
        let key = format!("circuit:{}", circuit.circuit_id);
        let json = serde_json::to_string(circuit)
            .map_err(|e| format!("Failed to serialize circuit: {e}"))?;

        let _: () = conn
            .set_ex(&key, json, self.default_ttl.as_secs())
            .await
            .map_err(|e| format!("Redis set failed: {e}"))?;

        tracing::debug!("✅ Cached circuit: {}", circuit.circuit_id);
        Ok(())
    }

    /// Delete circuit from cache
    pub async fn delete_circuit(&self, circuit_id: &str) -> Result<(), String> {
        let mut conn = self.get_conn().await?;
        let key = format!("circuit:{circuit_id}");

        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| format!("Redis delete failed: {e}"))?;

        tracing::debug!("🗑️  Deleted from cache: circuit {}", circuit_id);
        Ok(())
    }

    // ============================================================================
    // EVENT CACHE OPERATIONS
    // ============================================================================

    /// Get event from cache by ID
    pub async fn get_event(&self, event_id: &uuid::Uuid) -> Result<Option<Event>, String> {
        let mut conn = self.get_conn().await?;
        let key = format!("event:{event_id}");

        let cached: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| format!("Redis get failed: {e}"))?;

        if let Some(json) = cached {
            let event: Event = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize event: {e}"))?;
            tracing::debug!("🎯 Cache HIT: event {}", event_id);
            Ok(Some(event))
        } else {
            tracing::debug!("❌ Cache MISS: event {}", event_id);
            Ok(None)
        }
    }

    /// Store event in cache
    pub async fn set_event(&self, event: &Event) -> Result<(), String> {
        let mut conn = self.get_conn().await?;
        let key = format!("event:{}", event.event_id);
        let json = serde_json::to_string(event)
            .map_err(|e| format!("Failed to serialize event: {e}"))?;

        let _: () = conn
            .set_ex(&key, json, self.default_ttl.as_secs())
            .await
            .map_err(|e| format!("Redis set failed: {e}"))?;

        tracing::debug!("✅ Cached event: {}", event.event_id);
        Ok(())
    }

    // ============================================================================
    // BULK OPERATIONS
    // ============================================================================

    /// Invalidate all cached items (use sparingly)
    pub async fn invalidate_items(&self) -> Result<(), String> {
        let mut conn = self.get_conn().await?;

        // Scan for all item keys and delete them
        let pattern = "item:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| format!("Redis KEYS failed: {e}"))?;

        if !keys.is_empty() {
            let _: () = conn
                .del(&keys)
                .await
                .map_err(|e| format!("Redis bulk delete failed: {e}"))?;

            tracing::info!("🗑️  Invalidated {} cached items", keys.len());
        }

        Ok(())
    }

    /// Invalidate all cached circuits
    pub async fn invalidate_circuits(&self) -> Result<(), String> {
        let mut conn = self.get_conn().await?;

        let pattern = "circuit:*";
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| format!("Redis KEYS failed: {e}"))?;

        if !keys.is_empty() {
            let _: () = conn
                .del(&keys)
                .await
                .map_err(|e| format!("Redis bulk delete failed: {e}"))?;

            tracing::info!("🗑️  Invalidated {} cached circuits", keys.len());
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats, String> {
        let mut conn = self.get_conn().await?;

        // Count keys by pattern
        let item_keys: Vec<String> = redis::cmd("KEYS")
            .arg("item:*")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();

        let circuit_keys: Vec<String> = redis::cmd("KEYS")
            .arg("circuit:*")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();

        let event_keys: Vec<String> = redis::cmd("KEYS")
            .arg("event:*")
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();

        Ok(CacheStats {
            cached_items: item_keys.len(),
            cached_circuits: circuit_keys.len(),
            cached_events: event_keys.len(),
        })
    }

    /// Health check - verify Redis is reachable
    pub async fn health_check(&self) -> Result<(), String> {
        let mut conn = self.get_conn().await?;

        let _: String = redis::cmd("PING")
            .query_async(&mut *conn)
            .await
            .map_err(|e| format!("Redis PING failed: {e}"))?;

        Ok(())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_items: usize,
    pub cached_circuits: usize,
    pub cached_events: usize,
}
