-- DeFarm Engines Initial Database Schema
-- Production PostgreSQL schema for all storage entities

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- RECEIPTS
-- ============================================================================

CREATE TABLE IF NOT EXISTS receipts (
    id UUID PRIMARY KEY,
    data_hash VARCHAR(64) NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS receipt_identifiers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    receipt_id UUID NOT NULL REFERENCES receipts(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_receipt_identifiers_key_value ON receipt_identifiers(key, value);
CREATE INDEX IF NOT EXISTS idx_receipt_identifiers_receipt_id ON receipt_identifiers(receipt_id);

-- ============================================================================
-- LOGS
-- ============================================================================

CREATE TABLE IF NOT EXISTS logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp BIGINT NOT NULL,
    level VARCHAR(20) NOT NULL,
    engine VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    message TEXT NOT NULL,
    context_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_logs_engine ON logs(engine);
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp DESC);

-- ============================================================================
-- DATA LAKE
-- ============================================================================

CREATE TABLE IF NOT EXISTS data_lake_entries (
    entry_id UUID PRIMARY KEY,
    data_hash VARCHAR(64) NOT NULL,
    receipt_id UUID NOT NULL REFERENCES receipts(id),
    timestamp BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL,
    processing_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_data_lake_status ON data_lake_entries(status);
CREATE INDEX IF NOT EXISTS idx_data_lake_receipt_id ON data_lake_entries(receipt_id);

-- ============================================================================
-- ITEMS
-- ============================================================================

CREATE TABLE IF NOT EXISTS items (
    dfid VARCHAR(255) PRIMARY KEY,
    item_hash VARCHAR(64) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    last_updated_ts BIGINT NOT NULL,
    enriched_data JSONB,
    legacy_mode BOOLEAN NOT NULL DEFAULT TRUE,
    fingerprint TEXT,
    aliases JSONB,
    confidence_score DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add missing columns to existing items table (idempotent migration)
ALTER TABLE items ADD COLUMN IF NOT EXISTS legacy_mode BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE items ADD COLUMN IF NOT EXISTS fingerprint TEXT;
ALTER TABLE items ADD COLUMN IF NOT EXISTS aliases JSONB;
ALTER TABLE items ADD COLUMN IF NOT EXISTS confidence_score DOUBLE PRECISION NOT NULL DEFAULT 1.0;

CREATE INDEX IF NOT EXISTS idx_items_status ON items(status);
CREATE INDEX IF NOT EXISTS idx_items_created_at ON items(created_at_ts DESC);

CREATE TABLE IF NOT EXISTS item_identifiers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid) ON DELETE CASCADE,
    namespace VARCHAR(255) NOT NULL DEFAULT 'generic',
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    id_type VARCHAR(50) NOT NULL DEFAULT 'Contextual',
    type_metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add missing columns to existing item_identifiers table (idempotent migration)
ALTER TABLE item_identifiers ADD COLUMN IF NOT EXISTS namespace VARCHAR(255) NOT NULL DEFAULT 'generic';
ALTER TABLE item_identifiers ADD COLUMN IF NOT EXISTS id_type VARCHAR(50) NOT NULL DEFAULT 'Contextual';
ALTER TABLE item_identifiers ADD COLUMN IF NOT EXISTS type_metadata JSONB;

CREATE INDEX IF NOT EXISTS idx_item_identifiers_dfid ON item_identifiers(dfid);
CREATE INDEX IF NOT EXISTS idx_item_identifiers_key_value ON item_identifiers(key, value);
CREATE INDEX IF NOT EXISTS idx_item_identifiers_namespace ON item_identifiers(namespace);

CREATE TABLE IF NOT EXISTS item_source_entries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid) ON DELETE CASCADE,
    entry_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_item_source_entries_dfid ON item_source_entries(dfid);

-- ============================================================================
-- LID-DFID MAPPINGS
-- ============================================================================

CREATE TABLE IF NOT EXISTS lid_dfid_mappings (
    local_id UUID PRIMARY KEY,
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lid_dfid_mappings_dfid ON lid_dfid_mappings(dfid);

-- ============================================================================
-- IDENTIFIER MAPPINGS
-- ============================================================================

CREATE TABLE IF NOT EXISTS identifier_mappings (
    mapping_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    identifier_key VARCHAR(255) NOT NULL,
    identifier_value TEXT NOT NULL,
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid),
    confidence_score REAL NOT NULL,
    source_entry_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_identifier_mappings_key_value ON identifier_mappings(identifier_key, identifier_value);
CREATE INDEX IF NOT EXISTS idx_identifier_mappings_dfid ON identifier_mappings(dfid);

-- ============================================================================
-- CONFLICT RESOLUTIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS conflict_resolutions (
    conflict_id UUID PRIMARY KEY,
    identifier_key VARCHAR(255) NOT NULL,
    identifier_value TEXT NOT NULL,
    conflicting_dfids TEXT[] NOT NULL,
    resolution_strategy VARCHAR(100),
    resolved_dfid VARCHAR(255),
    status VARCHAR(50) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    resolved_at_ts BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_status ON conflict_resolutions(status);

-- ============================================================================
-- EVENTS
-- ============================================================================

CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    dfid VARCHAR(255) REFERENCES items(dfid),
    timestamp BIGINT NOT NULL,
    visibility VARCHAR(50) NOT NULL,
    encrypted_data BYTEA,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_dfid ON events(dfid);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_visibility ON events(visibility);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC);

-- ============================================================================
-- CIRCUITS
-- ============================================================================

CREATE TABLE IF NOT EXISTS circuits (
    circuit_id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    owner_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    last_modified_ts BIGINT NOT NULL,
    permissions JSONB NOT NULL,
    alias_config JSONB,
    adapter_config JSONB,
    public_settings JSONB,
    post_action_settings JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add missing columns to existing circuits table (idempotent migration)
ALTER TABLE circuits ADD COLUMN IF NOT EXISTS alias_config JSONB;
ALTER TABLE circuits ADD COLUMN IF NOT EXISTS adapter_config JSONB;
ALTER TABLE circuits ADD COLUMN IF NOT EXISTS public_settings JSONB;
ALTER TABLE circuits ADD COLUMN IF NOT EXISTS post_action_settings JSONB;

CREATE INDEX IF NOT EXISTS idx_circuits_owner_id ON circuits(owner_id);
CREATE INDEX IF NOT EXISTS idx_circuits_status ON circuits(status);

CREATE TABLE IF NOT EXISTS circuit_members (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    member_id VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL,
    permissions TEXT[] NOT NULL,
    joined_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(circuit_id, member_id)
);

CREATE INDEX IF NOT EXISTS idx_circuit_members_circuit_id ON circuit_members(circuit_id);
CREATE INDEX IF NOT EXISTS idx_circuit_members_member_id ON circuit_members(member_id);

CREATE TABLE IF NOT EXISTS circuit_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid),
    added_at_ts BIGINT NOT NULL,
    added_by VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(circuit_id, dfid)
);

CREATE INDEX IF NOT EXISTS idx_circuit_items_circuit_id ON circuit_items(circuit_id);
CREATE INDEX IF NOT EXISTS idx_circuit_items_dfid ON circuit_items(dfid);

CREATE TABLE IF NOT EXISTS circuit_operations (
    operation_id UUID PRIMARY KEY,
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    operation_type VARCHAR(50) NOT NULL,
    requester_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    approved_at_ts BIGINT,
    approver_id VARCHAR(255),
    completed_at_ts BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_circuit_operations_circuit_id ON circuit_operations(circuit_id);
CREATE INDEX IF NOT EXISTS idx_circuit_operations_status ON circuit_operations(status);

CREATE TABLE IF NOT EXISTS circuit_pending_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid),
    pushed_by VARCHAR(255) NOT NULL,
    pushed_at_ts BIGINT NOT NULL,
    status VARCHAR(50) NOT NULL,
    identifiers JSONB NOT NULL,
    enriched_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(circuit_id, dfid)
);

CREATE INDEX IF NOT EXISTS idx_circuit_pending_items_circuit_id ON circuit_pending_items(circuit_id);
CREATE INDEX IF NOT EXISTS idx_circuit_pending_items_status ON circuit_pending_items(status);

CREATE TABLE IF NOT EXISTS circuit_custom_roles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    role_name VARCHAR(100) NOT NULL,
    permissions TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(circuit_id, role_name)
);

CREATE INDEX IF NOT EXISTS idx_circuit_custom_roles_circuit_id ON circuit_custom_roles(circuit_id);

-- ============================================================================
-- USERS & AUTHENTICATION
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_accounts (
    user_id VARCHAR(255) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    tier VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    workspace_id VARCHAR(255),
    created_at_ts BIGINT NOT NULL,
    last_login_ts BIGINT,
    available_adapters TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add missing columns to existing user_accounts table (idempotent migration)
ALTER TABLE user_accounts ADD COLUMN IF NOT EXISTS available_adapters TEXT[];

CREATE INDEX IF NOT EXISTS idx_user_accounts_username ON user_accounts(username);
CREATE INDEX IF NOT EXISTS idx_user_accounts_email ON user_accounts(email);
CREATE INDEX IF NOT EXISTS idx_user_accounts_tier ON user_accounts(tier);

CREATE TABLE IF NOT EXISTS credit_balances (
    user_id VARCHAR(255) PRIMARY KEY REFERENCES user_accounts(user_id) ON DELETE CASCADE,
    credits BIGINT NOT NULL,
    updated_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS credit_transactions (
    transaction_id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES user_accounts(user_id),
    amount BIGINT NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    description TEXT,
    balance_after BIGINT NOT NULL,
    created_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_credit_transactions_user_id ON credit_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_credit_transactions_created_at ON credit_transactions(created_at_ts DESC);

-- ============================================================================
-- API KEYS
-- ============================================================================

CREATE TABLE IF NOT EXISTS api_keys (
    key_id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES user_accounts(user_id) ON DELETE CASCADE,
    key_hash VARCHAR(64) NOT NULL UNIQUE,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(255) NOT NULL,
    permissions TEXT[] NOT NULL,
    rate_limit_per_minute INT,
    rate_limit_per_hour INT,
    rate_limit_per_day INT,
    expires_at_ts BIGINT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    allowed_ips TEXT[],
    allowed_endpoints TEXT[],
    created_at_ts BIGINT NOT NULL,
    last_used_at_ts BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_is_active ON api_keys(is_active);

CREATE TABLE IF NOT EXISTS api_key_usage (
    usage_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_id UUID NOT NULL REFERENCES api_keys(key_id) ON DELETE CASCADE,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    status_code INT NOT NULL,
    timestamp_ts BIGINT NOT NULL,
    ip_address VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_key_usage_key_id ON api_key_usage(key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp_ts DESC);

-- ============================================================================
-- ADAPTERS
-- ============================================================================

CREATE TABLE IF NOT EXISTS adapter_configs (
    config_id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    adapter_type VARCHAR(100) NOT NULL,
    connection_details JSONB NOT NULL,
    contract_configs JSONB,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(255) NOT NULL,
    created_at_ts BIGINT NOT NULL,
    updated_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_adapter_configs_adapter_type ON adapter_configs(adapter_type);
CREATE INDEX IF NOT EXISTS idx_adapter_configs_is_active ON adapter_configs(is_active);
CREATE INDEX IF NOT EXISTS idx_adapter_configs_is_default ON adapter_configs(is_default);

-- ============================================================================
-- STORAGE HISTORY
-- ============================================================================

CREATE TABLE IF NOT EXISTS storage_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dfid VARCHAR(255) NOT NULL REFERENCES items(dfid) ON DELETE CASCADE,
    adapter_type VARCHAR(100) NOT NULL,
    storage_location JSONB NOT NULL,
    stored_at_ts BIGINT NOT NULL,
    triggered_by VARCHAR(255) NOT NULL,
    triggered_by_id VARCHAR(255),
    events_range_start BIGINT,
    events_range_end BIGINT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_storage_history_dfid ON storage_history(dfid);
CREATE INDEX IF NOT EXISTS idx_storage_history_adapter_type ON storage_history(adapter_type);
CREATE INDEX IF NOT EXISTS idx_storage_history_is_active ON storage_history(is_active);

-- ============================================================================
-- ACTIVITIES
-- ============================================================================

CREATE TABLE IF NOT EXISTS activities (
    activity_id UUID PRIMARY KEY,
    activity_type VARCHAR(100) NOT NULL,
    circuit_id UUID REFERENCES circuits(circuit_id),
    circuit_name VARCHAR(255),
    dfids TEXT[] NOT NULL,
    performed_by VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    details JSONB NOT NULL,
    timestamp_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_activities_circuit_id ON activities(circuit_id);
CREATE INDEX IF NOT EXISTS idx_activities_performed_by ON activities(performed_by);
CREATE INDEX IF NOT EXISTS idx_activities_timestamp ON activities(timestamp_ts DESC);

-- ============================================================================
-- USER ACTIVITIES (API activity logs)
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_activities (
    activity_id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES user_accounts(user_id) ON DELETE CASCADE,
    workspace_id VARCHAR(255) NOT NULL,
    activity_type VARCHAR(100) NOT NULL,
    category VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    action VARCHAR(255) NOT NULL,
    description TEXT,
    metadata JSONB,
    success BOOLEAN NOT NULL DEFAULT TRUE,
    ip_address VARCHAR(255),
    user_agent VARCHAR(255),
    timestamp_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_activities_user_id ON user_activities(user_id);
CREATE INDEX IF NOT EXISTS idx_user_activities_workspace_id ON user_activities(workspace_id);
CREATE INDEX IF NOT EXISTS idx_user_activities_timestamp ON user_activities(timestamp_ts DESC);

-- ============================================================================
-- NOTIFICATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS notifications (
    notification_id UUID PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES user_accounts(user_id) ON DELETE CASCADE,
    notification_type VARCHAR(100) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    data JSONB,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_ts BIGINT NOT NULL,
    read_at_ts BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_is_read ON notifications(is_read);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at_ts DESC);

-- ============================================================================
-- WEBHOOKS
-- ============================================================================

CREATE TABLE IF NOT EXISTS webhook_configs (
    webhook_id UUID PRIMARY KEY,
    circuit_id UUID NOT NULL REFERENCES circuits(circuit_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    trigger_events TEXT[] NOT NULL,
    auth_type VARCHAR(50),
    auth_config JSONB,
    retry_config JSONB,
    created_at_ts BIGINT NOT NULL,
    updated_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_webhook_configs_circuit_id ON webhook_configs(circuit_id);
CREATE INDEX IF NOT EXISTS idx_webhook_configs_enabled ON webhook_configs(enabled);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    delivery_id UUID PRIMARY KEY,
    webhook_id UUID NOT NULL REFERENCES webhook_configs(webhook_id) ON DELETE CASCADE,
    trigger_event VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(50) NOT NULL,
    http_status_code INT,
    response_body TEXT,
    error_message TEXT,
    attempt_count INT NOT NULL DEFAULT 1,
    delivered_at_ts BIGINT,
    created_at_ts BIGINT NOT NULL,
    next_retry_at_ts BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries(status);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_created_at ON webhook_deliveries(created_at_ts DESC);

-- ============================================================================
-- ADMIN ACTIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS admin_actions (
    action_id UUID PRIMARY KEY,
    admin_id VARCHAR(255) NOT NULL REFERENCES user_accounts(user_id),
    action_type VARCHAR(100) NOT NULL,
    target_id VARCHAR(255),
    action_data JSONB,
    performed_at_ts BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_admin_actions_admin_id ON admin_actions(admin_id);
CREATE INDEX IF NOT EXISTS idx_admin_actions_action_type ON admin_actions(action_type);
CREATE INDEX IF NOT EXISTS idx_admin_actions_performed_at ON admin_actions(performed_at_ts DESC);
