-- Add profile information to users
ALTER TABLE users ADD COLUMN first_name VARCHAR(50);
ALTER TABLE users ADD COLUMN last_name VARCHAR(50);
ALTER TABLE users ADD COLUMN profile_picture_url VARCHAR(255);