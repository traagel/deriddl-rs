-- Repeatable migration: Create simple demo views (SQLite compatible)
-- This will run whenever the content changes (checksum differs)

-- Create a view that works with existing test tables
-- First ensure we have the migration_log table from other repeatable migration
CREATE TABLE IF NOT EXISTS migration_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT,
    executed_at TEXT,
    execution_count INTEGER DEFAULT 1
);

-- Drop and recreate a simple view that shows migration execution history
DROP VIEW IF EXISTS migration_execution_summary;
CREATE VIEW migration_execution_summary AS
SELECT 
    migration_name,
    executed_at,
    execution_count,
    'R__create_views executed at ' || executed_at as description
FROM migration_log
ORDER BY executed_at DESC;