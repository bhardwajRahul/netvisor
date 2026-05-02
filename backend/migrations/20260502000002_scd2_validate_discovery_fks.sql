-- Validate the discovery FK constraints added in the previous migration.
-- Existing rows are all NULL for these columns so VALIDATE is trivial; this
-- step exists to satisfy the project's NOT VALID + VALIDATE pattern and
-- ensure the constraints are fully enforced going forward.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE hosts VALIDATE CONSTRAINT hosts_last_discovery_fk;
ALTER TABLE hosts VALIDATE CONSTRAINT hosts_first_discovery_fk;

ALTER TABLE ip_addresses VALIDATE CONSTRAINT ip_addresses_last_discovery_fk;
ALTER TABLE ip_addresses VALIDATE CONSTRAINT ip_addresses_first_discovery_fk;

ALTER TABLE ports VALIDATE CONSTRAINT ports_last_discovery_fk;
ALTER TABLE ports VALIDATE CONSTRAINT ports_first_discovery_fk;

ALTER TABLE services VALIDATE CONSTRAINT services_last_discovery_fk;
ALTER TABLE services VALIDATE CONSTRAINT services_first_discovery_fk;

ALTER TABLE interfaces VALIDATE CONSTRAINT interfaces_last_discovery_fk;
ALTER TABLE interfaces VALIDATE CONSTRAINT interfaces_first_discovery_fk;

ALTER TABLE bindings VALIDATE CONSTRAINT bindings_last_discovery_fk;
ALTER TABLE bindings VALIDATE CONSTRAINT bindings_first_discovery_fk;

ALTER TABLE subnets VALIDATE CONSTRAINT subnets_last_discovery_fk;
ALTER TABLE subnets VALIDATE CONSTRAINT subnets_first_discovery_fk;

ALTER TABLE vlans VALIDATE CONSTRAINT vlans_last_discovery_fk;
ALTER TABLE vlans VALIDATE CONSTRAINT vlans_first_discovery_fk;

-- subnet_vlans intentionally omitted: see scd2_add_discovery_fks_not_valid.sql.
