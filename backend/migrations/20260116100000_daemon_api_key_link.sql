-- Add plaintext column to api_keys for ServerPoll mode
-- and api_key_id to daemons for linking daemon to its API key

-- Add plaintext column for ServerPoll keys
-- plaintext: Stores plaintext API key for ServerPoll mode daemons only
--            NULL for DaemonPoll daemons (server doesn't need to send key)
ALTER TABLE api_keys ADD COLUMN plaintext TEXT DEFAULT NULL;

-- Add api_key_id to daemons for ServerPoll lookup
-- api_key_id: Links daemon to its API key for ServerPoll
--             NULL for daemons not yet associated with a key
ALTER TABLE daemons ADD COLUMN api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL;

-- Create index for efficient lookup
CREATE INDEX idx_daemons_api_key ON daemons(api_key_id) WHERE api_key_id IS NOT NULL;

-- Allow last_seen to be NULL for provisioned ServerPoll daemons not yet contacted
-- This enables detecting first contact (last_seen transitions from NULL to a value)
ALTER TABLE daemons ALTER COLUMN last_seen DROP NOT NULL;

-- Track unreachability for ServerPoll circuit breaker
-- When a daemon becomes unreachable after repeated failures, polling stops
-- until manually retried via the retry-connection endpoint
ALTER TABLE daemons ADD COLUMN is_unreachable BOOLEAN NOT NULL DEFAULT false;
