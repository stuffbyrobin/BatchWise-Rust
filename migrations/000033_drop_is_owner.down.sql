ALTER TABLE users ADD COLUMN IF NOT EXISTS is_owner BOOLEAN NOT NULL DEFAULT false;
UPDATE users SET is_owner = (role = 'owner');
