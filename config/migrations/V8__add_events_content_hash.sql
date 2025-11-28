-- V8: Add content_hash column to events table for deduplication
-- This enables efficient lookup of events by their content hash

-- Add content_hash column (nullable for existing events)
ALTER TABLE events ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64);

-- Create index for fast content_hash lookups
CREATE INDEX IF NOT EXISTS idx_events_content_hash ON events(content_hash);

-- Add source column (was being derived from metadata, now explicit)
ALTER TABLE events ADD COLUMN IF NOT EXISTS source VARCHAR(255);

COMMENT ON COLUMN events.content_hash IS 'BLAKE3 hash of event content for deduplication';
COMMENT ON COLUMN events.source IS 'Source of the event (user_id or system)';
