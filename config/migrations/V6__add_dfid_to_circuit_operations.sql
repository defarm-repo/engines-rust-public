-- Add dfid column to circuit_operations table for item association
ALTER TABLE circuit_operations
    ADD COLUMN IF NOT EXISTS dfid VARCHAR(255);

-- Index to speed up lookups by DFID
CREATE INDEX IF NOT EXISTS idx_circuit_operations_dfid ON circuit_operations(dfid);
