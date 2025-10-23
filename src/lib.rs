pub mod activity_engine;
pub mod adapters;
pub mod audit_engine;
pub mod blockchain_event_listener;
pub mod circuits_engine;
pub mod conflict_detection;
pub mod dfid_engine;
pub mod events_engine;
pub mod identifier_types;
pub mod ipfs_client;
pub mod items_engine;
pub mod logging;
pub mod receipt_engine;
pub mod stellar_client;
pub mod storage;
pub mod types;
pub mod verification_engine;
pub mod zk_proof_engine;
// Stellar health check disabled - using SDK not CLI
// pub mod stellar_health_check;
// pub mod postgres_storage; // Disabled - has type incompatibilities. Use PostgresPersistence instead.
// Redis cache infrastructure - ACTIVE
pub mod redis_cache;
pub mod redis_postgres_storage; // Production-ready: Redis + PostgreSQL Primary Storage
                                // pub mod cached_postgres_storage; // DEPRECATED - replaced by redis_postgres_storage
pub mod adapter_manager;
pub mod api;
pub mod api_key_engine;
pub mod api_key_middleware;
pub mod api_key_storage;
pub mod auth_middleware;
pub mod credit_manager;
pub mod db_init;
pub mod error_handling;
pub mod notification_engine;
pub mod postgres_persistence;
pub mod rate_limiter;
pub mod safe_json_numbers;
pub mod storage_factory;
pub mod storage_history_manager; // Deprecated - use storage_history_reader
pub mod storage_history_reader;
pub mod tier_permission_system;
pub mod webhook_delivery_worker;
pub mod webhook_engine;

#[cfg(test)]
mod test_safe_json_numbers;

pub use activity_engine::*;
pub use api_key_engine::*;
pub use api_key_middleware::*;
pub use api_key_storage::*;
pub use audit_engine::*;
pub use circuits_engine::*;
pub use conflict_detection::*;
pub use dfid_engine::*;
pub use error_handling::*;
pub use events_engine::*;
pub use items_engine::*;
pub use logging::*;
pub use notification_engine::*;
pub use rate_limiter::*;
pub use receipt_engine::*;
pub use storage::*;
pub use storage_history_manager::*; // Deprecated - use storage_history_reader
pub use storage_history_reader::*;
pub use types::*;
pub use verification_engine::{VerificationEngine, VerificationError, VerificationResult};
pub use webhook_engine::*;
pub use zk_proof_engine::{
    AgriculturalContext, CircuitInput, CircuitTemplate, CircuitType, ProofStatus,
    VerificationResult as ZkVerificationResult, ZkProof, ZkProofEngine, ZkProofError,
};
