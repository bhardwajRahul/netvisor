-- Add FK constraints from {hosts, ip_addresses, ports, services, interfaces,
-- bindings, subnets, vlans, subnet_vlans} → discoveries(id) for the new
-- last_discovery_id / first_discovery_id columns. Following the
-- expand-and-contract pattern from the project migration guidelines:
-- ADD CONSTRAINT ... NOT VALID here; VALIDATE in the next migration.
--
-- Rationale: NOT VALID skips the table scan that would otherwise hold a
-- SHARE ROW EXCLUSIVE lock. Existing rows are all NULL for these columns
-- (just added in the previous migration) so the validate pass is trivial,
-- but the project linter still requires the two-step pattern uniformly.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE hosts
    ADD CONSTRAINT hosts_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT hosts_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE ip_addresses
    ADD CONSTRAINT ip_addresses_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT ip_addresses_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE ports
    ADD CONSTRAINT ports_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT ports_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE services
    ADD CONSTRAINT services_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT services_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE interfaces
    ADD CONSTRAINT interfaces_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT interfaces_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE bindings
    ADD CONSTRAINT bindings_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT bindings_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE subnets
    ADD CONSTRAINT subnets_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT subnets_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

ALTER TABLE vlans
    ADD CONSTRAINT vlans_last_discovery_fk
        FOREIGN KEY (last_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID,
    ADD CONSTRAINT vlans_first_discovery_fk
        FOREIGN KEY (first_discovery_id) REFERENCES discovery(id) ON DELETE SET NULL NOT VALID;

-- subnet_vlans intentionally omitted: junction is Snapshotable but not
-- DiscoveryTracked. SCD2 valid_from/valid_to capture link lifetimes; per-
-- link discovery FKs aren't tracked.
