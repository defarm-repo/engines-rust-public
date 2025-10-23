-- ============================================================================
-- Migration: Extend items and identifier schema to support unified identifiers
-- ============================================================================

ALTER TABLE items
    ADD COLUMN IF NOT EXISTS legacy_mode BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS fingerprint TEXT,
    ADD COLUMN IF NOT EXISTS aliases JSONB,
    ADD COLUMN IF NOT EXISTS confidence_score DOUBLE PRECISION NOT NULL DEFAULT 1.0;

ALTER TABLE item_identifiers
    ADD COLUMN IF NOT EXISTS namespace VARCHAR(255) NOT NULL DEFAULT 'generic',
    ADD COLUMN IF NOT EXISTS id_type VARCHAR(50) NOT NULL DEFAULT 'Contextual',
    ADD COLUMN IF NOT EXISTS type_metadata JSONB;

CREATE INDEX IF NOT EXISTS idx_item_identifiers_namespace ON item_identifiers(namespace);

