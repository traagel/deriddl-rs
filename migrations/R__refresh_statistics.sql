-- Repeatable migration: Simple test operations (SQLite compatible)
-- This runs every time to demonstrate repeatable migration functionality

-- This is a simple demo that doesn't rely on complex schema
-- Just create a temporary table and populate it with current timestamp
CREATE TABLE IF NOT EXISTS migration_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT,
    executed_at TEXT,
    execution_count INTEGER DEFAULT 1
);

-- Update or insert execution log
INSERT OR REPLACE INTO migration_log (
    id,
    migration_name, 
    executed_at,
    execution_count
)
SELECT 
    COALESCE(existing.id, NULL) as id,
    'R__refresh_statistics' as migration_name,
    datetime('now') as executed_at,
    COALESCE(existing.execution_count + 1, 1) as execution_count
FROM (SELECT 1) dummy
LEFT JOIN migration_log existing ON existing.migration_name = 'R__refresh_statistics';