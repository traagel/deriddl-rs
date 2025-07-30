-- Add performance indexes
CREATE INDEX idx_users_name ON users(first_name, last_name);
CREATE INDEX idx_users_created_at ON users(created_at);