-- Migration V4: Add System Statistics
-- Created: 2025-10-23
-- Purpose: Complete PostgreSQL storage implementation - add system statistics table
-- NOTE: Timeline and CID mapping tables already exist from V2__create_cid_timeline.sql

-- ============================================================================
-- SYSTEM STATISTICS - Aggregated system-wide statistics
-- ============================================================================

CREATE TABLE IF NOT EXISTS system_statistics (
    id SERIAL PRIMARY KEY,
    total_users BIGINT NOT NULL DEFAULT 0,
    active_users_24h BIGINT NOT NULL DEFAULT 0,
    active_users_30d BIGINT NOT NULL DEFAULT 0,
    total_items BIGINT NOT NULL DEFAULT 0,
    total_circuits BIGINT NOT NULL DEFAULT 0,
    total_storage_operations BIGINT NOT NULL DEFAULT 0,
    credits_consumed_24h BIGINT NOT NULL DEFAULT 0,
    tier_distribution JSONB,
    adapter_usage_stats JSONB,
    generated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT single_stats_row CHECK (id = 1)
);

-- Insert initial statistics row
INSERT INTO system_statistics (id, total_users, total_items, total_circuits, tier_distribution, adapter_usage_stats)
VALUES (1, 0, 0, 0, '{}'::jsonb, '{}'::jsonb)
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE system_statistics IS 'Aggregated system-wide statistics for admin dashboard and monitoring';
COMMENT ON COLUMN system_statistics.tier_distribution IS 'JSON map of UserTier -> count (e.g., {"Basic": 100, "Professional": 50})';
COMMENT ON COLUMN system_statistics.adapter_usage_stats IS 'JSON map of AdapterType -> usage count (e.g., {"IPFS": 500, "Stellar": 200})';
