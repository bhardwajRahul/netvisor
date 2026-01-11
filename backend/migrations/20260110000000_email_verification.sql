-- Email verification columns
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verification_token TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verification_expires TIMESTAMPTZ;

-- Password reset columns (migrate from in-memory to database)
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_reset_token TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_reset_expires TIMESTAMPTZ;

-- Grandfather existing users as verified
UPDATE users SET email_verified = TRUE WHERE email_verified = FALSE;

-- Indexes for token lookups
CREATE INDEX IF NOT EXISTS idx_users_email_verification_token
  ON users(email_verification_token) WHERE email_verification_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_password_reset_token
  ON users(password_reset_token) WHERE password_reset_token IS NOT NULL;
