--
-- A device-level software revision column, splitting the version pair ENTITY-MIB reports.
--
-- Why: `hosts.firmware_revision` was defined by 20260827120000 as "firmware *or* software
-- revision of the device as a whole". That "or" makes the column a union with no discriminator,
-- and it has been harmless only because every writer so far has exactly one version concept, all
-- of them genuinely firmware — UniFi, Instant On, Modbus `0x2B` MajorMinorRevision, EtherNet/IP
-- revision.
--
-- ENTITY-MIB is the first source that reports two different versions at once.
-- `entPhysicalFirmwareRev` (.9) and `entPhysicalSoftwareRev` (.10) are distinct objects in
-- RFC 4133: on a Cisco chassis the first is the bootloader and the second is the IOS version.
-- Folding them into one column needs an arbitration rule that discards a real value and files the
-- survivor under a label that may be wrong — a stored "15.0(2)SE11" with nothing recording whether
-- it is the bootloader or the OS. So .9 keeps `firmware_revision` and .10 gets its own column.
--
-- No backfill. All four existing writers of `firmware_revision` write genuine firmware, so every
-- existing row is already correct where it stands and this column starts empty.
--
-- The `_source` sibling is the per-value provenance column 20260828120000 added for every other
-- discovered attribute, at the same `Unspecified` default and for the same reason: no UI or API
-- path lets a person type this value, so there is nothing to protect and a row left at
-- `Unspecified` is displaced by the first real reading.
--
-- Additive and prod-safe: both are `ADD COLUMN` with a non-volatile default, which is
-- metadata-only on PG11+ (no rewrite). No contract step — nothing is renamed or dropped, and
-- older servers ignore both columns.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts ADD COLUMN IF NOT EXISTS software_revision TEXT;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS software_revision_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
