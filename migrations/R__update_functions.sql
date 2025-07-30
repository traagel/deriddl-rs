-- Repeatable migration: Simple SQLite operations
-- Avoiding complex triggers due to SQL parser limitations

-- Ensure we have our demo table
CREATE TABLE IF NOT EXISTS migration_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT,
    executed_at TEXT,
    execution_count INTEGER DEFAULT 1
);

-- Log this migration execution
INSERT OR REPLACE INTO migration_log (
    id,
    migration_name, 
    executed_at,
    execution_count
)
SELECT 
    COALESCE(existing.id, NULL) as id,
    'R__update_functions' as migration_name,
    datetime('now') as executed_at,
    COALESCE(existing.execution_count + 1, 1) as execution_count
FROM (SELECT 1) dummy
LEFT JOIN migration_log existing ON existing.migration_name = 'R__update_functions';