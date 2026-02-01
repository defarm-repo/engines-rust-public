-- Enable pg_trgm extension for fuzzy search (must be first)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create dfid_locations table
CREATE TABLE IF NOT EXISTS dfid_locations (
    location_id UUID PRIMARY KEY,
    dfid VARCHAR(255) NOT NULL,
    location_type JSONB NOT NULL,
    location_url TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    registered_by VARCHAR(255) NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    last_verified TIMESTAMPTZ,

    CONSTRAINT dfid_format_check CHECK (dfid ~ '^DFID-[0-9]{8}-[0-9]{6}-[A-F0-9]{6}$')
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_dfid_locations_dfid ON dfid_locations(dfid);
CREATE INDEX IF NOT EXISTS idx_dfid_locations_registered_at ON dfid_locations(registered_at DESC);
CREATE INDEX IF NOT EXISTS idx_dfid_locations_verified ON dfid_locations(verified) WHERE verified = true;
CREATE INDEX IF NOT EXISTS idx_dfid_locations_location_type ON dfid_locations USING GIN (location_type);

-- Create index for search queries (requires pg_trgm extension)
CREATE INDEX IF NOT EXISTS idx_dfid_locations_dfid_trgm ON dfid_locations USING gin (dfid gin_trgm_ops);

-- Add comment
COMMENT ON TABLE dfid_locations IS 'Index of DFID locations across different storage backends';
COMMENT ON COLUMN dfid_locations.location_type IS 'Type of location: circuit, blockchain, ipfs, or registry';
COMMENT ON COLUMN dfid_locations.verified IS 'Whether this location has been verified by the system';
