-- Migration 003: Add chain_id to file_conversation_index
-- Purpose: Link file accesses to chains for context-aware queries
-- Version: 2.1 → 2.2
-- Created: 2025-12-16

-- ============================================================================
-- EXTEND FILE INDEX WITH CHAIN CONTEXT
-- ============================================================================
-- The inverted_index module now tracks chain_id for each file access.
-- This allows queries like "What files did this chain touch?"

-- Add chain_id column to existing table
ALTER TABLE file_conversation_index ADD COLUMN chain_id TEXT;

-- Index for chain-based queries
CREATE INDEX IF NOT EXISTS idx_file_conv_chain ON file_conversation_index(chain_id);

-- Foreign key cannot be added via ALTER in SQLite, but we document the relationship
-- chain_id references chains(chain_id) from migration 002

-- ============================================================================
-- CHAIN-FILE INTEGRATION VIEW
-- ============================================================================
-- View: Files touched by a chain (aggregated across all sessions in chain)

CREATE VIEW IF NOT EXISTS chain_files AS
SELECT
    fci.chain_id,
    fci.file_path,
    COUNT(DISTINCT fci.session_id) as session_count,
    SUM(fci.access_count) as total_accesses,
    GROUP_CONCAT(DISTINCT fci.access_type) as access_types,
    MIN(fci.first_accessed_at) as first_accessed,
    MAX(fci.first_accessed_at) as last_accessed
FROM file_conversation_index fci
WHERE fci.chain_id IS NOT NULL
GROUP BY fci.chain_id, fci.file_path
ORDER BY fci.chain_id, session_count DESC;

-- ============================================================================
-- Update metadata
-- ============================================================================
INSERT OR REPLACE INTO _metadata (key, value, updated_at)
VALUES ('schema_version', '2.2', CURRENT_TIMESTAMP);
