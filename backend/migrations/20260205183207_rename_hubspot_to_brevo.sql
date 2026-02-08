ALTER TABLE organizations RENAME COLUMN hubspot_company_id TO brevo_company_id;
UPDATE organizations SET brevo_company_id = NULL;
