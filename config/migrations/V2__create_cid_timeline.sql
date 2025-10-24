-- ============================================================================
-- CID Timeline Tables for IPCM Event Tracking
-- ============================================================================
--
-- This migration creates tables to track the complete timeline of IPFS CIDs
-- for each DFID, enabling reconstruction of item history from blockchain events.
--
-- Architecture:
-- 1. item_cid_timeline: Chronological sequence of all CIDs for each DFID
-- 2. event_cid_mapping: Maps which events first appeared in which CID
-- 3. blockchain_indexing_progress: Tracks event listener progress per network
--
-- Flow:
-- - Push operation → IPFS upload → NFT mint (first time) → IPCM update
-- - Event listener polls Soroban → Detects IPCM events → Populates timeline
-- - Frontend queries timeline → Displays chronological story with CIDs
-- ============================================================================

-- ============================================================================
-- Table: item_cid_timeline
-- Purpose: Store chronological sequence of all CIDs for each DFID
-- ============================================================================
CREATE TABLE IF NOT EXISTS item_cid_timeline (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(100) NOT NULL,
    cid VARCHAR(100) NOT NULL,
    event_sequence INTEGER NOT NULL,
    blockchain_timestamp BIGINT NOT NULL,
    ipcm_transaction_hash VARCHAR(100) NOT NULL,
    network VARCHAR(50) NOT NULL DEFAULT 'stellar-testnet',
    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Ensure unique sequence per DFID
    UNIQUE(dfid, event_sequence)
);

CREATE INDEX IF NOT EXISTS idx_cid_timeline_dfid ON item_cid_timeline(dfid);
CREATE INDEX IF NOT EXISTS idx_cid_timeline_cid ON item_cid_timeline(cid);
CREATE INDEX IF NOT EXISTS idx_cid_timeline_sequence ON item_cid_timeline(dfid, event_sequence);
CREATE INDEX IF NOT EXISTS idx_cid_timeline_timestamp ON item_cid_timeline(blockchain_timestamp);
CREATE INDEX IF NOT EXISTS idx_cid_timeline_network ON item_cid_timeline(network);

-- ============================================================================
-- Function: Auto-increment event_sequence per DFID
-- ============================================================================
CREATE OR REPLACE FUNCTION set_event_sequence()
RETURNS TRIGGER AS $$
BEGIN
    -- Get next sequence number for this DFID
    SELECT COALESCE(MAX(event_sequence), 0) + 1
    INTO NEW.event_sequence
    FROM item_cid_timeline
    WHERE dfid = NEW.dfid;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Trigger: Auto-set sequence before insert
-- ============================================================================
DROP TRIGGER IF EXISTS trigger_auto_sequence ON item_cid_timeline;

CREATE TRIGGER trigger_auto_sequence
BEFORE INSERT ON item_cid_timeline
FOR EACH ROW
WHEN (NEW.event_sequence IS NULL OR NEW.event_sequence = 0)
EXECUTE FUNCTION set_event_sequence();

-- ============================================================================
-- Table: event_cid_mapping
-- Purpose: Track which events first appeared in which CID
-- ============================================================================
CREATE TABLE IF NOT EXISTS event_cid_mapping (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_id UUID NOT NULL,
    dfid VARCHAR(100) NOT NULL,
    first_cid VARCHAR(100) NOT NULL,
    appeared_in_sequence INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- One event appears in exactly one CID first
    UNIQUE(event_id)

    -- Foreign key to events table (if it exists)
    -- CONSTRAINT fk_event_cid_event FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_event_cid_dfid ON event_cid_mapping(dfid);
CREATE INDEX IF NOT EXISTS idx_event_cid_first_cid ON event_cid_mapping(first_cid);
CREATE INDEX IF NOT EXISTS idx_event_cid_sequence ON event_cid_mapping(dfid, appeared_in_sequence);

-- ============================================================================
-- Table: blockchain_indexing_progress
-- Purpose: Track event listener progress for each blockchain network
-- ============================================================================
CREATE TABLE IF NOT EXISTS blockchain_indexing_progress (
    network VARCHAR(100) PRIMARY KEY,
    last_indexed_ledger BIGINT NOT NULL DEFAULT 0,
    last_confirmed_ledger BIGINT NOT NULL DEFAULT 0,
    last_indexed_at TIMESTAMPTZ DEFAULT NOW(),
    status VARCHAR(50) DEFAULT 'active',
    error_message TEXT,

    -- Metadata about indexing health
    total_events_indexed BIGINT DEFAULT 0,
    last_error_at TIMESTAMPTZ
);

-- Insert default progress for supported networks
INSERT INTO blockchain_indexing_progress (network, last_indexed_ledger, status)
VALUES
    ('stellar-testnet', 0, 'active'),
    ('stellar-mainnet', 0, 'active')
ON CONFLICT (network) DO NOTHING;

-- ============================================================================
-- View: item_timeline_with_events
-- Purpose: Convenient view combining timeline with event information
-- ============================================================================
DROP VIEW IF EXISTS item_timeline_with_events CASCADE;

CREATE OR REPLACE VIEW item_timeline_with_events AS
SELECT
    t.dfid,
    t.event_sequence,
    t.cid,
    t.blockchain_timestamp,
    t.ipcm_transaction_hash,
    t.network,
    t.created_at as timeline_created_at,
    COALESCE(
        json_agg(
            json_build_object(
                'event_id', e.event_id,
                'appeared_in_sequence', e.appeared_in_sequence
            )
            ORDER BY e.appeared_in_sequence
        ) FILTER (WHERE e.event_id IS NOT NULL),
        '[]'::json
    ) as events
FROM item_cid_timeline t
LEFT JOIN event_cid_mapping e ON t.dfid = e.dfid AND t.event_sequence = e.appeared_in_sequence
GROUP BY t.id, t.dfid, t.event_sequence, t.cid, t.blockchain_timestamp, t.ipcm_transaction_hash, t.network, t.created_at
ORDER BY t.dfid, t.event_sequence;

-- ============================================================================
-- Comments for documentation
-- ============================================================================
COMMENT ON TABLE item_cid_timeline IS 'Chronological timeline of IPFS CIDs for each DFID, populated by blockchain event listener';
COMMENT ON TABLE event_cid_mapping IS 'Maps which events first appeared in which CID version';
COMMENT ON TABLE blockchain_indexing_progress IS 'Tracks blockchain event listener progress per network';
COMMENT ON COLUMN item_cid_timeline.event_sequence IS 'Auto-incrementing sequence number per DFID (1, 2, 3, ...)';
COMMENT ON COLUMN item_cid_timeline.blockchain_timestamp IS 'Ledger close timestamp from Stellar (Unix timestamp)';
COMMENT ON COLUMN item_cid_timeline.ipcm_transaction_hash IS 'Stellar transaction hash for IPCM update_ipcm call';

-- ============================================================================
-- Grant permissions (adjust as needed for your deployment)
-- ============================================================================
-- GRANT SELECT, INSERT, UPDATE ON item_cid_timeline TO defarm_app;
-- GRANT SELECT, INSERT, UPDATE ON event_cid_mapping TO defarm_app;
-- GRANT SELECT, UPDATE ON blockchain_indexing_progress TO defarm_app;
-- GRANT SELECT ON item_timeline_with_events TO defarm_app;
