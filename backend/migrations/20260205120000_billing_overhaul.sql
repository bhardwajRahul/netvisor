ALTER TABLE organizations ADD COLUMN IF NOT EXISTS has_payment_method BOOLEAN NOT NULL DEFAULT false;
-- Existing customers with an active or trialing subscription had to provide payment upfront
UPDATE organizations SET has_payment_method = true WHERE plan_status IN ('active', 'trialing');
ALTER TABLE organizations ADD COLUMN IF NOT EXISTS trial_end_date TIMESTAMPTZ;
ALTER TABLE daemons ADD COLUMN IF NOT EXISTS standby BOOLEAN NOT NULL DEFAULT false;
