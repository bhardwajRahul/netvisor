--
-- PostgreSQL database dump
--

\restrict hbNTv58f56aZ5lFHyvn9F1SZwldTyIikVblkNh76S4QYKdYflsVqgPWu4oI0TzQ

-- Dumped from database version 17.10
-- Dumped by pg_dump version 17.10

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_vlan_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_created_by_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS interfaces_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS interfaces_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_neighbor_if_entry_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_neighbor_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_native_vlan_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS groups_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_tag_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS dependency_members_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS dependency_members_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS dependencies_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_network_id_fkey;
DROP TRIGGER IF EXISTS reassign_daemons_before_user_delete ON public.users;
DROP INDEX IF EXISTS public.idx_vlans_snapshot_id;
DROP INDEX IF EXISTS public.idx_vlans_organization;
DROP INDEX IF EXISTS public.idx_vlans_network_number_live;
DROP INDEX IF EXISTS public.idx_vlans_network;
DROP INDEX IF EXISTS public.idx_vlans_live;
DROP INDEX IF EXISTS public.idx_vlans_lineage;
DROP INDEX IF EXISTS public.idx_vlans_as_of;
DROP INDEX IF EXISTS public.idx_users_password_reset_token;
DROP INDEX IF EXISTS public.idx_users_organization;
DROP INDEX IF EXISTS public.idx_users_oidc_provider_subject;
DROP INDEX IF EXISTS public.idx_users_email_verification_token;
DROP INDEX IF EXISTS public.idx_users_email_lower;
DROP INDEX IF EXISTS public.idx_user_network_access_user;
DROP INDEX IF EXISTS public.idx_user_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_keys_user;
DROP INDEX IF EXISTS public.idx_user_api_keys_org;
DROP INDEX IF EXISTS public.idx_user_api_keys_key;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_key;
DROP INDEX IF EXISTS public.idx_topologies_network;
DROP INDEX IF EXISTS public.idx_tags_snapshot_id;
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name_live;
DROP INDEX IF EXISTS public.idx_tags_live;
DROP INDEX IF EXISTS public.idx_tags_lineage;
DROP INDEX IF EXISTS public.idx_tags_as_of;
DROP INDEX IF EXISTS public.idx_subnets_snapshot_id;
DROP INDEX IF EXISTS public.idx_subnets_network;
DROP INDEX IF EXISTS public.idx_subnets_live;
DROP INDEX IF EXISTS public.idx_subnets_lineage;
DROP INDEX IF EXISTS public.idx_subnets_as_of;
DROP INDEX IF EXISTS public.idx_subnet_vlans_vlan;
DROP INDEX IF EXISTS public.idx_subnet_vlans_unique_live;
DROP INDEX IF EXISTS public.idx_subnet_vlans_subnet;
DROP INDEX IF EXISTS public.idx_subnet_vlans_snapshot_id;
DROP INDEX IF EXISTS public.idx_subnet_vlans_live;
DROP INDEX IF EXISTS public.idx_subnet_vlans_lineage;
DROP INDEX IF EXISTS public.idx_subnet_vlans_as_of;
DROP INDEX IF EXISTS public.idx_snapshots_network_taken_at;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_network;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_snapshot_id;
DROP INDEX IF EXISTS public.idx_services_network;
DROP INDEX IF EXISTS public.idx_services_live;
DROP INDEX IF EXISTS public.idx_services_lineage;
DROP INDEX IF EXISTS public.idx_services_host_position;
DROP INDEX IF EXISTS public.idx_services_host_id;
DROP INDEX IF EXISTS public.idx_services_as_of;
DROP INDEX IF EXISTS public.idx_ports_unique_live;
DROP INDEX IF EXISTS public.idx_ports_snapshot_id;
DROP INDEX IF EXISTS public.idx_ports_number;
DROP INDEX IF EXISTS public.idx_ports_network;
DROP INDEX IF EXISTS public.idx_ports_live;
DROP INDEX IF EXISTS public.idx_ports_lineage;
DROP INDEX IF EXISTS public.idx_ports_host;
DROP INDEX IF EXISTS public.idx_ports_as_of;
DROP INDEX IF EXISTS public.idx_organizations_stripe_customer;
DROP INDEX IF EXISTS public.idx_networks_owner_organization;
DROP INDEX IF EXISTS public.idx_ip_addresses_unique_live;
DROP INDEX IF EXISTS public.idx_ip_addresses_subnet;
DROP INDEX IF EXISTS public.idx_ip_addresses_snapshot_id;
DROP INDEX IF EXISTS public.idx_ip_addresses_network;
DROP INDEX IF EXISTS public.idx_ip_addresses_live;
DROP INDEX IF EXISTS public.idx_ip_addresses_lineage;
DROP INDEX IF EXISTS public.idx_ip_addresses_host_mac;
DROP INDEX IF EXISTS public.idx_ip_addresses_host;
DROP INDEX IF EXISTS public.idx_ip_addresses_as_of;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_snapshot_id;
DROP INDEX IF EXISTS public.idx_interfaces_network;
DROP INDEX IF EXISTS public.idx_interfaces_neighbor_interface;
DROP INDEX IF EXISTS public.idx_interfaces_neighbor_host;
DROP INDEX IF EXISTS public.idx_interfaces_mac_address;
DROP INDEX IF EXISTS public.idx_interfaces_live;
DROP INDEX IF EXISTS public.idx_interfaces_lineage;
DROP INDEX IF EXISTS public.idx_interfaces_ip_address;
DROP INDEX IF EXISTS public.idx_interfaces_host_name_live;
DROP INDEX IF EXISTS public.idx_interfaces_host_if_index;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_interfaces_as_of;
DROP INDEX IF EXISTS public.idx_hosts_snapshot_id;
DROP INDEX IF EXISTS public.idx_hosts_network;
DROP INDEX IF EXISTS public.idx_hosts_live;
DROP INDEX IF EXISTS public.idx_hosts_lineage;
DROP INDEX IF EXISTS public.idx_hosts_chassis_id;
DROP INDEX IF EXISTS public.idx_hosts_as_of;
DROP INDEX IF EXISTS public.idx_groups_network;
DROP INDEX IF EXISTS public.idx_entity_tags_unique_live;
DROP INDEX IF EXISTS public.idx_entity_tags_tag_id;
DROP INDEX IF EXISTS public.idx_entity_tags_snapshot_id;
DROP INDEX IF EXISTS public.idx_entity_tags_live;
DROP INDEX IF EXISTS public.idx_entity_tags_lineage;
DROP INDEX IF EXISTS public.idx_entity_tags_entity;
DROP INDEX IF EXISTS public.idx_entity_tags_as_of;
DROP INDEX IF EXISTS public.idx_discovery_network;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_dependency_members_unique_live;
DROP INDEX IF EXISTS public.idx_dependency_members_snapshot_id;
DROP INDEX IF EXISTS public.idx_dependency_members_service;
DROP INDEX IF EXISTS public.idx_dependency_members_live;
DROP INDEX IF EXISTS public.idx_dependency_members_lineage;
DROP INDEX IF EXISTS public.idx_dependency_members_dependency;
DROP INDEX IF EXISTS public.idx_dependency_members_binding;
DROP INDEX IF EXISTS public.idx_dependency_members_as_of;
DROP INDEX IF EXISTS public.idx_dependencies_snapshot_id;
DROP INDEX IF EXISTS public.idx_dependencies_live;
DROP INDEX IF EXISTS public.idx_dependencies_lineage;
DROP INDEX IF EXISTS public.idx_dependencies_as_of;
DROP INDEX IF EXISTS public.idx_daemons_network;
DROP INDEX IF EXISTS public.idx_daemons_api_key;
DROP INDEX IF EXISTS public.idx_daemon_host_id;
DROP INDEX IF EXISTS public.idx_credentials_type;
DROP INDEX IF EXISTS public.idx_credentials_org;
DROP INDEX IF EXISTS public.idx_bindings_snapshot_id;
DROP INDEX IF EXISTS public.idx_bindings_service;
DROP INDEX IF EXISTS public.idx_bindings_port;
DROP INDEX IF EXISTS public.idx_bindings_network;
DROP INDEX IF EXISTS public.idx_bindings_live;
DROP INDEX IF EXISTS public.idx_bindings_lineage;
DROP INDEX IF EXISTS public.idx_bindings_ip_address;
DROP INDEX IF EXISTS public.idx_bindings_as_of;
DROP INDEX IF EXISTS public.idx_api_keys_network;
DROP INDEX IF EXISTS public.idx_api_keys_key;
ALTER TABLE IF EXISTS ONLY tower_sessions.session DROP CONSTRAINT IF EXISTS session_pkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_pkey;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_pkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_pkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_pkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_pkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_pkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_pkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS if_entries_pkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_pkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_pkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_pkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public._sqlx_migrations DROP CONSTRAINT IF EXISTS _sqlx_migrations_pkey;
DROP TABLE IF EXISTS tower_sessions.session;
DROP TABLE IF EXISTS public.vlans;
DROP TABLE IF EXISTS public.users;
DROP TABLE IF EXISTS public.user_network_access;
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.user_api_key_network_access;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.subnet_vlans;
DROP TABLE IF EXISTS public.snapshots;
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.networks;
DROP TABLE IF EXISTS public.network_credentials;
DROP TABLE IF EXISTS public.ip_addresses;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.host_credentials;
DROP TABLE IF EXISTS public.entity_tags;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.dependency_members;
DROP TABLE IF EXISTS public.dependencies;
DROP TABLE IF EXISTS public.daemons;
DROP TABLE IF EXISTS public.credentials;
DROP TABLE IF EXISTS public.bindings;
DROP TABLE IF EXISTS public.api_keys;
DROP TABLE IF EXISTS public._sqlx_migrations;
DROP FUNCTION IF EXISTS public.reassign_daemons_on_user_delete();
DROP EXTENSION IF EXISTS pgcrypto;
DROP SCHEMA IF EXISTS tower_sessions;
--
-- Name: tower_sessions; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA tower_sessions;


ALTER SCHEMA tower_sessions OWNER TO postgres;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: reassign_daemons_on_user_delete(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.reassign_daemons_on_user_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    new_owner_id UUID;
BEGIN
    SELECT id INTO new_owner_id
    FROM users
    WHERE organization_id = OLD.organization_id
      AND permissions = 'Owner'
      AND id != OLD.id
    ORDER BY created_at ASC
    LIMIT 1;

    IF new_owner_id IS NOT NULL THEN
        UPDATE daemons
        SET user_id = new_owner_id
        WHERE user_id = OLD.id;
    END IF;

    RETURN OLD;
END;
$$;


ALTER FUNCTION public.reassign_daemons_on_user_delete() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO postgres;

--
-- Name: api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.api_keys (
    id uuid NOT NULL,
    key text NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL,
    plaintext text
);


ALTER TABLE public.api_keys OWNER TO postgres;

--
-- Name: bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.bindings (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    service_id uuid NOT NULL,
    binding_type text NOT NULL,
    ip_address_id uuid,
    port_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    CONSTRAINT bindings_binding_type_check CHECK ((binding_type = ANY (ARRAY['IPAddress'::text, 'Port'::text]))),
    CONSTRAINT valid_binding CHECK ((((binding_type = 'IPAddress'::text) AND (ip_address_id IS NOT NULL) AND (port_id IS NULL)) OR ((binding_type = 'Port'::text) AND (port_id IS NOT NULL))))
);


ALTER TABLE public.bindings OWNER TO postgres;

--
-- Name: credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.credentials (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    credential_type jsonb NOT NULL,
    target_ips inet[],
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.credentials OWNER TO postgres;

--
-- Name: daemons; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.daemons (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    last_seen timestamp with time zone,
    capabilities jsonb DEFAULT '{}'::jsonb,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    mode text DEFAULT '"Push"'::text,
    url text NOT NULL,
    name text,
    version text,
    user_id uuid NOT NULL,
    api_key_id uuid,
    is_unreachable boolean DEFAULT false NOT NULL,
    standby boolean DEFAULT false NOT NULL,
    standby_cleared_at timestamp with time zone
);


ALTER TABLE public.daemons OWNER TO postgres;

--
-- Name: dependencies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.dependencies (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    dependency_type text NOT NULL,
    member_type text DEFAULT 'Bindings'::text NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.dependencies OWNER TO postgres;

--
-- Name: dependency_members; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.dependency_members (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    dependency_id uuid NOT NULL,
    binding_id uuid,
    "position" integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    service_id uuid NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.dependency_members OWNER TO postgres;

--
-- Name: discovery; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.discovery (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    daemon_id uuid NOT NULL,
    run_type jsonb NOT NULL,
    discovery_type jsonb NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    scan_count integer DEFAULT 0 NOT NULL,
    force_full_scan boolean DEFAULT false NOT NULL,
    pending_credential_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.discovery OWNER TO postgres;

--
-- Name: entity_tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.entity_tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    entity_id uuid NOT NULL,
    entity_type character varying(50) NOT NULL,
    tag_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.entity_tags OWNER TO postgres;

--
-- Name: host_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.host_credentials (
    host_id uuid NOT NULL,
    credential_id uuid NOT NULL,
    ip_address_ids uuid[]
);


ALTER TABLE public.host_credentials OWNER TO postgres;

--
-- Name: hosts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.hosts (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    hostname text,
    description text,
    source jsonb NOT NULL,
    virtualization jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    hidden boolean DEFAULT false,
    sys_descr text,
    sys_object_id text,
    sys_location text,
    sys_contact text,
    management_url text,
    chassis_id text,
    manufacturer text,
    model text,
    serial_number text,
    sys_name text,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.hosts OWNER TO postgres;

--
-- Name: COLUMN hosts.sys_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_descr IS 'SNMP sysDescr.0 - full system description';


--
-- Name: COLUMN hosts.sys_object_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_object_id IS 'SNMP sysObjectID.0 - vendor OID for device identification';


--
-- Name: COLUMN hosts.sys_location; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_location IS 'SNMP sysLocation.0 - physical location';


--
-- Name: COLUMN hosts.sys_contact; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_contact IS 'SNMP sysContact.0 - admin contact info';


--
-- Name: COLUMN hosts.management_url; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.management_url IS 'URL for device management interface (manual or discovered)';


--
-- Name: COLUMN hosts.chassis_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.chassis_id IS 'LLDP lldpLocChassisId - globally unique device identifier for deduplication';


--
-- Name: interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interfaces (
    id uuid NOT NULL,
    host_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    if_index integer NOT NULL,
    if_descr text NOT NULL,
    if_alias text,
    if_type integer NOT NULL,
    speed_bps bigint,
    admin_status integer NOT NULL,
    oper_status integer NOT NULL,
    mac_address macaddr,
    ip_address_id uuid,
    neighbor_interface_id uuid,
    neighbor_host_id uuid,
    lldp_chassis_id jsonb,
    lldp_port_id jsonb,
    lldp_sys_name text,
    lldp_port_desc text,
    lldp_mgmt_addr inet,
    lldp_sys_desc text,
    cdp_device_id text,
    cdp_port_id text,
    cdp_platform text,
    cdp_address inet,
    if_name text,
    fdb_macs jsonb,
    native_vlan_id uuid,
    vlan_ids jsonb,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    CONSTRAINT chk_neighbor_exclusive CHECK (((neighbor_interface_id IS NULL) OR (neighbor_host_id IS NULL)))
);


ALTER TABLE public.interfaces OWNER TO postgres;

--
-- Name: TABLE interfaces; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.interfaces IS 'SNMP ifTable entries - physical/logical interfaces on network devices';


--
-- Name: COLUMN interfaces.if_index; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_index IS 'SNMP ifIndex - stable identifier within device';


--
-- Name: COLUMN interfaces.if_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_descr IS 'SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)';


--
-- Name: COLUMN interfaces.if_alias; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_alias IS 'SNMP ifAlias - user-configured description';


--
-- Name: COLUMN interfaces.if_type; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_type IS 'SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)';


--
-- Name: COLUMN interfaces.speed_bps; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.speed_bps IS 'Interface speed from ifSpeed/ifHighSpeed in bits per second';


--
-- Name: COLUMN interfaces.admin_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.admin_status IS 'SNMP ifAdminStatus: 1=up, 2=down, 3=testing';


--
-- Name: COLUMN interfaces.oper_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.oper_status IS 'SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown';


--
-- Name: COLUMN interfaces.ip_address_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.ip_address_id IS 'FK to Interface entity when this ifEntry has an IP address (must be on same host)';


--
-- Name: COLUMN interfaces.neighbor_interface_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.neighbor_interface_id IS 'Full neighbor resolution: FK to remote IfEntry discovered via LLDP/CDP';


--
-- Name: COLUMN interfaces.neighbor_host_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.neighbor_host_id IS 'Partial neighbor resolution: FK to remote Host when specific port is unknown';


--
-- Name: COLUMN interfaces.lldp_mgmt_addr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.lldp_mgmt_addr IS 'LLDP remote management address (lldpRemManAddr)';


--
-- Name: COLUMN interfaces.lldp_sys_desc; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.lldp_sys_desc IS 'LLDP remote system description (lldpRemSysDesc)';


--
-- Name: COLUMN interfaces.cdp_device_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.cdp_device_id IS 'CDP cache remote device ID (typically hostname)';


--
-- Name: COLUMN interfaces.cdp_port_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.cdp_port_id IS 'CDP cache remote port ID string';


--
-- Name: COLUMN interfaces.cdp_platform; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.cdp_platform IS 'CDP cache remote device platform (e.g., Cisco IOS)';


--
-- Name: COLUMN interfaces.cdp_address; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.cdp_address IS 'CDP cache remote device management IP address';


--
-- Name: invites; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.invites (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text NOT NULL,
    network_ids uuid[] NOT NULL,
    url text NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    send_to text
);


ALTER TABLE public.invites OWNER TO postgres;

--
-- Name: ip_addresses; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ip_addresses (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    ip_address inet NOT NULL,
    mac_address macaddr,
    name text,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.ip_addresses OWNER TO postgres;

--
-- Name: TABLE ip_addresses; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.ip_addresses IS 'IP addresses assigned to hosts on subnets';


--
-- Name: network_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.network_credentials (
    network_id uuid NOT NULL,
    credential_id uuid NOT NULL
);


ALTER TABLE public.network_credentials OWNER TO postgres;

--
-- Name: networks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.networks (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    organization_id uuid NOT NULL
);


ALTER TABLE public.networks OWNER TO postgres;

--
-- Name: COLUMN networks.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.networks.organization_id IS 'The organization that owns and pays for this network';


--
-- Name: organizations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.organizations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    stripe_customer_id text,
    plan jsonb NOT NULL,
    plan_status text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    onboarding jsonb DEFAULT '[]'::jsonb,
    brevo_company_id text,
    has_payment_method boolean DEFAULT false NOT NULL,
    trial_end_date timestamp with time zone,
    plan_limit_notifications jsonb DEFAULT '{}'::jsonb NOT NULL,
    use_case text,
    last_paused_at timestamp with time zone,
    trial_extended_used boolean DEFAULT false NOT NULL,
    last_downgrade_at timestamp with time zone,
    last_downgrade_from_plan jsonb,
    last_discount_at timestamp with time zone,
    discount_save_offer_percent_off bigint,
    discount_save_offer_active_until timestamp with time zone,
    next_renewal_at timestamp with time zone
);


ALTER TABLE public.organizations OWNER TO postgres;

--
-- Name: TABLE organizations; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.organizations IS 'Organizations that own networks and have Stripe subscriptions';


--
-- Name: COLUMN organizations.plan; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.organizations.plan IS 'The current billing plan for the organization (e.g., Community, Pro)';


--
-- Name: ports; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ports (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    port_number integer NOT NULL,
    protocol text NOT NULL,
    port_type text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    CONSTRAINT ports_port_number_check CHECK (((port_number >= 0) AND (port_number <= 65535))),
    CONSTRAINT ports_protocol_check CHECK ((protocol = ANY (ARRAY['Tcp'::text, 'Udp'::text])))
);


ALTER TABLE public.ports OWNER TO postgres;

--
-- Name: services; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.services (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    name text NOT NULL,
    host_id uuid NOT NULL,
    service_definition text NOT NULL,
    virtualization jsonb,
    source jsonb NOT NULL,
    "position" integer DEFAULT 0 NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.services OWNER TO postgres;

--
-- Name: shares; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.shares (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    topology_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name text NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    password_hash text,
    allowed_domains text[],
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    enabled_views jsonb
);


ALTER TABLE public.shares OWNER TO postgres;

--
-- Name: snapshots; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.snapshots (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    taken_at timestamp with time zone NOT NULL,
    created_by_user_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.snapshots OWNER TO postgres;

--
-- Name: subnet_vlans; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnet_vlans (
    id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    vlan_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.subnet_vlans OWNER TO postgres;

--
-- Name: subnets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnets (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    cidr text NOT NULL,
    name text NOT NULL,
    description text,
    subnet_type text NOT NULL,
    source jsonb NOT NULL,
    virtualization jsonb,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.subnets OWNER TO postgres;

--
-- Name: tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tags (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    color text NOT NULL,
    is_application boolean DEFAULT false NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.tags OWNER TO postgres;

--
-- Name: topologies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.topologies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    network_id uuid NOT NULL,
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.topologies OWNER TO postgres;

--
-- Name: user_api_key_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_key_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    api_key_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_api_key_network_access OWNER TO postgres;

--
-- Name: user_api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_keys (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    key text NOT NULL,
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Viewer'::text NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL
);


ALTER TABLE public.user_api_keys OWNER TO postgres;

--
-- Name: user_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_network_access OWNER TO postgres;

--
-- Name: users; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.users (
    id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    password_hash text,
    oidc_provider text,
    oidc_subject text,
    oidc_linked_at timestamp with time zone,
    email text NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Member'::text NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    terms_accepted_at timestamp with time zone,
    email_verified boolean DEFAULT false NOT NULL,
    email_verification_token text,
    email_verification_expires timestamp with time zone,
    password_reset_token text,
    password_reset_expires timestamp with time zone,
    pending_email text,
    email_settings jsonb DEFAULT '{"discovery_digest": true}'::jsonb NOT NULL
);


ALTER TABLE public.users OWNER TO postgres;

--
-- Name: COLUMN users.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.organization_id IS 'The single organization this user belongs to';


--
-- Name: COLUMN users.permissions; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.permissions IS 'User role within their organization: Owner, Member, Viewer';


--
-- Name: vlans; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.vlans (
    id uuid NOT NULL,
    vlan_number smallint NOT NULL,
    name text NOT NULL,
    description text,
    network_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    source jsonb DEFAULT '"Manual"'::jsonb NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.vlans OWNER TO postgres;

--
-- Name: session; Type: TABLE; Schema: tower_sessions; Owner: postgres
--

CREATE TABLE tower_sessions.session (
    id text NOT NULL,
    data bytea NOT NULL,
    expiry_date timestamp with time zone NOT NULL
);


ALTER TABLE tower_sessions.session OWNER TO postgres;

--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
20251006215000	users	2026-06-22 22:39:33.02037+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-06-22 22:39:33.026093+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-06-22 22:39:33.030014+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-06-22 22:39:33.03412+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-06-22 22:39:33.038198+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-06-22 22:39:33.042714+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-06-22 22:39:33.047567+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-06-22 22:39:33.053951+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-06-22 22:39:33.055812+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-06-22 22:39:33.05732+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-06-22 22:39:33.067016+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-06-22 22:39:33.069333+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-06-22 22:39:33.07693+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-06-22 22:39:33.101257+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-06-22 22:39:33.111656+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-06-22 22:39:33.113261+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-06-22 22:39:33.114813+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-06-22 22:39:33.116571+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-06-22 22:39:33.124423+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-06-22 22:39:33.12782+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-06-22 22:39:33.129272+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-06-22 22:39:33.131263+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-06-22 22:39:33.132933+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-06-22 22:39:33.134848+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-06-22 22:39:33.136219+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-06-22 22:39:33.138851+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-06-22 22:39:33.146591+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-06-22 22:39:33.148015+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-06-22 22:39:33.15678+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-06-22 22:39:33.158686+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-06-22 22:39:33.159987+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-06-22 22:39:33.162299+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-06-22 22:39:33.16718+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-06-22 22:39:33.173476+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-06-22 22:39:33.175305+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-06-22 22:39:33.176656+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-06-22 22:39:33.17798+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-06-22 22:39:33.179451+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-06-22 22:39:33.186028+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-06-22 22:39:33.194716+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-06-22 22:39:33.202767+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-06-22 22:39:33.212207+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-06-22 22:39:33.218138+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-06-22 22:39:33.220112+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-06-22 22:39:33.22172+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-06-22 22:39:33.223564+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-06-22 22:39:33.228685+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-06-22 22:39:33.238765+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-06-22 22:39:33.241751+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-06-22 22:39:33.243984+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-06-22 22:39:33.245992+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-06-22 22:39:33.256804+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-06-22 22:39:33.258673+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-06-22 22:39:33.262305+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-06-22 22:39:33.263889+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-06-22 22:39:33.270353+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-06-22 22:39:33.274563+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-06-22 22:39:33.287007+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-06-22 22:39:33.290628+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-06-22 22:39:33.292143+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-06-22 22:39:33.294043+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-06-22 22:39:33.2958+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-06-22 22:39:33.298277+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-06-22 22:39:33.29977+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-06-22 22:39:33.301507+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-06-22 22:39:33.303166+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-06-22 22:39:33.304593+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-06-22 22:39:33.305998+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-06-22 22:39:33.307387+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-06-22 22:39:33.309498+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-06-22 22:39:33.326268+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-06-22 22:39:33.327679+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-06-22 22:39:33.328985+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-06-22 22:39:33.331098+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-06-22 22:39:33.332605+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-06-22 22:39:33.334976+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-06-22 22:39:33.337362+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-06-22 22:39:33.338661+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-06-22 22:39:33.340508+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-06-22 22:39:33.348644+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-06-22 22:39:33.350238+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-06-22 22:39:33.360381+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-06-22 22:39:33.362266+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-06-22 22:39:33.369292+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-06-22 22:39:33.370845+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-06-22 22:39:33.372151+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-06-22 22:39:33.376051+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-06-22 22:39:33.377534+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-06-22 22:39:33.380246+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-06-22 22:39:33.394782+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-06-22 22:39:33.406046+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-06-22 22:39:33.415655+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-06-22 22:39:33.420071+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-06-22 22:39:33.441948+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-06-22 22:39:33.445617+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-06-22 22:39:33.512751+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-06-22 22:39:33.516453+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-06-22 22:39:33.518305+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-06-22 22:39:33.522742+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-06-22 22:39:33.533645+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-06-22 22:39:33.557357+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-06-22 22:39:33.563453+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-06-22 22:39:33.572541+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext) FROM stdin;
46e870fd-87e5-4325-b0a5-5887f06daa1f	44e45cb974d36f7407865226486d1418b6bb69aa955dbf301870b07e1b7cdeca	7351e4ce-703f-4a0d-beac-b685d5456ce8	Integrated Daemon API Key	2026-06-22 22:39:35.759105+00	2026-06-22 22:39:35.759105+00	2026-06-22 22:44:14.425486+00	\N	t	\N
3824f5fe-8e89-4752-ba6c-aacb4af655a7	255edeb343e1102b6dc964f974547a7b2f37caa78e744b3764c2a4d9242ab7c2	7351e4ce-703f-4a0d-beac-b685d5456ce8	scanopy-daemon-serverpoll API Key	2026-06-22 22:40:36.996208+00	2026-06-22 22:40:36.996208+00	2026-06-22 22:44:32.385404+00	\N	t	scp_d_WZwYWCEgIIMX3cnMEnnjNK0B0yJaFqMk
1cdb1b1d-efd8-48ef-bf96-fae3480e88db	7b30c8ebf47dcbecd8eb7ea047bbdcd314b9b719a257676e8652a390e6ac3353	7351e4ce-703f-4a0d-beac-b685d5456ce8	Compat Test API Key	2026-06-22 22:44:05.503837+00	2026-06-22 22:44:05.503837+00	2026-06-22 22:44:20.668457+00	\N	t	\N
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
44e5e8b2-f9ac-42a8-bd24-7c3657106624	7351e4ce-703f-4a0d-beac-b685d5456ce8	556294d3-421e-4b45-8b34-a82624b6f937	Port	14ca8455-226d-4741-b90f-706df52ca3b4	ea73264e-cd98-465b-9536-9365f39e106c	2026-06-22 22:39:44.526284+00	2026-06-22 22:39:44.526284+00	2026-06-22 22:39:44.526284+00	\N	\N	2026-06-22 22:39:44.526284+00	\N	\N	\N
c7385e77-fac6-4cfd-aa01-80c9f98afccc	7351e4ce-703f-4a0d-beac-b685d5456ce8	556294d3-421e-4b45-8b34-a82624b6f937	Port	16b57a3c-dcc6-40c8-8de5-dafe68965767	ea73264e-cd98-465b-9536-9365f39e106c	2026-06-22 22:39:44.526285+00	2026-06-22 22:39:44.526285+00	2026-06-22 22:39:44.526285+00	\N	\N	2026-06-22 22:39:44.526285+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, target_ips, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at) FROM stdin;
d5482c01-a2a3-44cc-8d35-d114680dcfe7	7351e4ce-703f-4a0d-beac-b685d5456ce8	268f3322-1e48-4551-bbee-61ed8978f5a4	2026-06-22 22:39:35.79484+00	2026-06-22 22:44:20.58617+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["3c01d70b-8334-4bcd-912d-d2a874de3344"]}	2026-06-22 22:39:35.79484+00	"daemon_poll"		scanopy-daemon	0.17.0	2dab7160-a0b9-44f5-a65f-cdfcd6446406	\N	f	f	\N
002c66bd-230a-41a1-a174-8c8d1bfc91c0	7351e4ce-703f-4a0d-beac-b685d5456ce8	cf70c05a-1647-4958-806b-ff84119deb9a	2026-06-22 22:40:37.000611+00	2026-06-22 22:44:32.391207+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["87533c96-36e8-48d0-a633-ffcb7d69d45e", "c5d411f6-04a5-4757-a07c-03af6cff55df"]}	2026-06-22 22:40:37.000611+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.17.0	2dab7160-a0b9-44f5-a65f-cdfcd6446406	3824f5fe-8e89-4752-ba6c-aacb4af655a7	f	f	2026-06-22 22:44:32.391209+00
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, network_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
a52f1927-5c42-4bb7-a164-96a7c86b40f2	7351e4ce-703f-4a0d-beac-b685d5456ce8		\N	2026-06-22 22:44:04.769334+00	2026-06-22 22:44:04.769334+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath	Services	2026-06-22 22:44:04.769334+00	\N	\N	\N
\.


--
-- Data for Name: dependency_members; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependency_members (id, dependency_id, binding_id, "position", created_at, service_id, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, scan_count, force_full_scan, pending_credential_ids) FROM stdin;
6170c462-1272-4687-bf67-24b0a2cc6871	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-06-22T22:44:21.781338533Z", "finished_at": "2026-06-22T22:44:21.790664144Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:21.781338+00	2026-06-22 22:44:21.798865+00	0	f	{}
e2ef31f5-c382-41b3-a116-84ea540401a8	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Scheduled", "enabled": true, "last_run": "2026-06-22T22:39:35.805972588Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "268f3322-1e48-4551-bbee-61ed8978f5a4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:39:35.803828+00	2026-06-22 22:40:36.649538+00	1	f	{}
7f29604c-8019-45a7-9727-c60ad6f02e37	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "ac99a184-048f-4d27-8336-9e549b862089", "started_at": "2026-06-22T22:39:44.515645717Z", "finished_at": "2026-06-22T22:40:36.631605758Z", "discovery_id": "e2ef31f5-c382-41b3-a116-84ea540401a8", "discovery_type": {"type": "Unified", "host_id": "268f3322-1e48-4551-bbee-61ed8978f5a4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "268f3322-1e48-4551-bbee-61ed8978f5a4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:39:44.515645+00	2026-06-22 22:40:36.648754+00	0	f	{}
13e45b54-56a1-43d2-9e4e-388809b72da4	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "d2c685dd-56ad-4d49-9d36-c64b0483a15a", "started_at": "2026-06-22T22:44:22.054759734Z", "finished_at": "2026-06-22T22:44:22.064344209Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:22.054759+00	2026-06-22 22:44:22.072698+00	0	f	{}
12a2bbe0-a7da-4b68-acf1-2546c3153c3d	7351e4ce-703f-4a0d-beac-b685d5456ce8	002c66bd-230a-41a1-a174-8c8d1bfc91c0	{"type": "Scheduled", "enabled": true, "last_run": "2026-06-22T22:41:03.602622138Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:41:03.600876+00	2026-06-22 22:41:03.60263+00	0	f	{}
04cada81-391f-4199-b289-b734a5122804	7351e4ce-703f-4a0d-beac-b685d5456ce8	002c66bd-230a-41a1-a174-8c8d1bfc91c0	{"type": "AdHoc", "last_run": "2026-06-22T22:40:37.344938671Z"}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-06-22 22:40:37.334775+00	2026-06-22 22:44:04.727145+00	1	f	{}
5b088b04-c262-4376-be6e-140609912f2a	7351e4ce-703f-4a0d-beac-b685d5456ce8	002c66bd-230a-41a1-a174-8c8d1bfc91c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "002c66bd-230a-41a1-a174-8c8d1bfc91c0", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "bfc425bd-2840-4447-8d67-666671033cd6", "started_at": "2026-06-22T22:41:33.673899539Z", "finished_at": "2026-06-22T22:44:04.707351567Z", "discovery_id": "04cada81-391f-4199-b289-b734a5122804", "discovery_type": {"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:41:33.673899+00	2026-06-22 22:44:04.726184+00	0	f	{}
5fef00f2-debf-4636-ba0d-87f939a95021	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-06-22T22:44:22.225465297Z", "finished_at": "2026-06-22T22:44:22.235688939Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:22.225465+00	2026-06-22 22:44:22.246381+00	0	f	{}
bad9221b-6272-4724-9326-3103b3c1d7af	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-06-22T22:44:24.146210974Z", "finished_at": "2026-06-22T22:44:24.156281293Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:24.14621+00	2026-06-22 22:44:24.166974+00	0	f	{}
42a1fd69-94ae-4c55-9262-d26823be1d17	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-06-22T22:44:24.421891655Z", "finished_at": "2026-06-22T22:44:24.431112194Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:24.421891+00	2026-06-22 22:44:24.441531+00	0	f	{}
17dbd97f-1b67-4222-ab24-6e68d6044348	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-06-22T22:44:24.971447992Z", "finished_at": "2026-06-22T22:44:24.981198373Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:24.971447+00	2026-06-22 22:44:24.991941+00	0	f	{}
f0c06d97-1f21-44bd-b4f8-5eb90280ee40	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-06-22T22:44:26.607963423Z", "finished_at": "2026-06-22T22:44:26.617905040Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Network	2026-06-22 22:44:26.607963+00	2026-06-22 22:44:26.626236+00	0	f	{}
dfa8422a-2e77-47cc-b947-d87c9563c20a	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-06-22T22:44:29.053204438Z", "finished_at": "2026-06-22T22:44:29.063610189Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:29.053204+00	2026-06-22 22:44:29.07405+00	0	f	{}
e0530a60-b948-422f-8f2d-9786fb898ef7	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-06-22T22:44:22.501131295Z", "finished_at": "2026-06-22T22:44:22.511231532Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:22.501131+00	2026-06-22 22:44:22.521834+00	0	f	{}
bc9a02c6-b318-4d37-adf2-3c17575bba34	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-06-22T22:44:22.777403191Z", "finished_at": "2026-06-22T22:44:22.787416501Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:22.777403+00	2026-06-22 22:44:22.800086+00	0	f	{}
1cf9fdf5-c933-4229-a6c6-8f2a381f27f7	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-06-22T22:44:26.983317348Z", "finished_at": "2026-06-22T22:44:26.991884076Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:26.983317+00	2026-06-22 22:44:26.999811+00	0	f	{}
739e9d1e-c72c-400b-acee-eead6644fe2d	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-06-22T22:44:27.411773109Z", "finished_at": "2026-06-22T22:44:27.421901267Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Network	2026-06-22 22:44:27.411773+00	2026-06-22 22:44:27.430152+00	0	f	{}
1ed1fbd0-1f7c-4bcf-b3d6-68b01753b259	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-06-22T22:44:27.683394177Z", "finished_at": "2026-06-22T22:44:27.693310463Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:27.683394+00	2026-06-22 22:44:27.703736+00	0	f	{}
190e8d95-f2e2-44b6-ac7f-2702858a27e6	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-06-22T22:44:27.959016565Z", "finished_at": "2026-06-22T22:44:27.969438763Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:27.959016+00	2026-06-22 22:44:27.980371+00	0	f	{}
f4eac434-2d31-4c82-baf2-e713b8a0a910	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-06-22T22:44:28.238894856Z", "finished_at": "2026-06-22T22:44:28.249622652Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:28.238894+00	2026-06-22 22:44:28.260201+00	0	f	{}
62d0a92f-118c-4d08-8f8f-86a38996966a	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-06-22T22:44:23.043586063Z", "finished_at": "2026-06-22T22:44:23.053645143Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-06-22 22:44:23.043586+00	2026-06-22 22:44:23.062139+00	0	f	{}
8c78308a-e582-4281-91e7-1d608e341847	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-06-22T22:44:23.317415632Z", "finished_at": "2026-06-22T22:44:23.326766746Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:23.317415+00	2026-06-22 22:44:23.337253+00	0	f	{}
31545d72-799a-43fa-872d-067fb13770ef	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "34fe487d-22ef-418c-9099-42fa3376ded0", "started_at": "2026-06-22T22:44:24.798960590Z", "finished_at": "2026-06-22T22:44:24.808773321Z", "discovery_id": "6b552988-a1be-4f68-8187-f69624148974", "discovery_type": {"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:24.79896+00	2026-06-22 22:44:24.817262+00	0	f	{}
8a054ac0-786a-4fac-8a75-94d4626f5b1c	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-06-22T22:44:26.068646079Z", "finished_at": "2026-06-22T22:44:26.079598591Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Network	2026-06-22 22:44:26.068646+00	2026-06-22 22:44:26.091303+00	0	f	{}
59c943e0-86f0-4f6f-8767-bd9c1a73b7e6	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "73503118-cf7f-4008-881e-7f39b457bf47", "started_at": "2026-06-22T22:44:28.883783056Z", "finished_at": "2026-06-22T22:44:28.894133022Z", "discovery_id": "8ff643e5-aa2a-4d4f-adde-0218c0c8f22f", "discovery_type": {"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:28.883783+00	2026-06-22 22:44:28.902033+00	0	f	{}
e39c6eeb-fce9-4e2a-82c0-e2ee1862b7bd	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-06-22T22:44:29.601902868Z", "finished_at": "2026-06-22T22:44:29.612390860Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:29.601902+00	2026-06-22 22:44:29.623065+00	0	f	{}
8f543282-b7ee-4d26-8354-4c263d3aa35a	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-06-22T22:44:23.592892972Z", "finished_at": "2026-06-22T22:44:23.602899030Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:23.592892+00	2026-06-22 22:44:23.613976+00	0	f	{}
194bdc79-02cc-405f-8c93-a353ce182cf0	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-06-22T22:44:23.869874999Z", "finished_at": "2026-06-22T22:44:23.880258647Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:23.869874+00	2026-06-22 22:44:23.891497+00	0	f	{}
20685999-d7fa-4b66-94e6-1a743524c5cf	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "6465c546-3856-4e33-9cd0-5b8a8d99d55b", "started_at": "2026-06-22T22:44:25.907299660Z", "finished_at": "2026-06-22T22:44:25.917320290Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:25.907299+00	2026-06-22 22:44:25.925866+00	0	f	{}
7a67f4d2-12f4-468a-982e-54bdf23bc743	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-06-22T22:44:28.506098314Z", "finished_at": "2026-06-22T22:44:28.517321902Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Network	2026-06-22 22:44:28.506098+00	2026-06-22 22:44:28.52585+00	0	f	{}
1e31c402-78c8-42ab-83b0-304bc32b7bb8	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-06-22T22:44:25.247299272Z", "finished_at": "2026-06-22T22:44:25.256962364Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:25.247299+00	2026-06-22 22:44:25.2679+00	0	f	{}
2ce98ba0-4783-4382-b9be-e668c27d17f1	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-06-22T22:44:25.522355885Z", "finished_at": "2026-06-22T22:44:25.532025607Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Network	2026-06-22 22:44:25.522355+00	2026-06-22 22:44:25.542622+00	0	f	{}
b9d18ee5-8859-4a7c-8ab7-4905fcb1d81e	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "b049b806-80fc-4dc5-9400-f19f9a9857ff", "started_at": "2026-06-22T22:44:26.443965148Z", "finished_at": "2026-06-22T22:44:26.454219063Z", "discovery_id": "0f49ac2a-3bcc-4880-8623-073c6e609c41", "discovery_type": {"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:26.443965+00	2026-06-22 22:44:26.462795+00	0	f	{}
a3744ad1-ec66-420f-8bfe-cda1b0b5d3dc	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-06-22T22:44:27.146768846Z", "finished_at": "2026-06-22T22:44:27.157225731Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Network	2026-06-22 22:44:27.146768+00	2026-06-22 22:44:27.165459+00	0	f	{}
91e559a6-35a0-4dba-b8dd-3124bf4796d6	7351e4ce-703f-4a0d-beac-b685d5456ce8	d5482c01-a2a3-44cc-8d35-d114680dcfe7	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 5, "daemon_id": "d5482c01-a2a3-44cc-8d35-d114680dcfe7", "network_id": "7351e4ce-703f-4a0d-beac-b685d5456ce8", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-06-22T22:44:29.430645757Z", "finished_at": "2026-06-22T22:44:29.440613292Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-22 22:44:29.430645+00	2026-06-22 22:44:29.448483+00	0	f	{}
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
47ee5dc5-6e0b-4a06-8fd9-53109d3bf6e8	917e9ae7-5ed6-449b-85cc-eaf18f851c3c	"Service"	e0dce46e-c386-4c8c-b9a3-f3281d116cf0	2026-06-22 22:44:04.761807+00	2026-06-22 22:44:04.761807+00	\N	\N	\N
\.


--
-- Data for Name: host_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.host_credentials (host_id, credential_id, ip_address_ids) FROM stdin;
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, manufacturer, model, serial_number, sys_name, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
268f3322-1e48-4551-bbee-61ed8978f5a4	7351e4ce-703f-4a0d-beac-b685d5456ce8	9088dda02ad4	9088dda02ad4	Scanopy daemon	{"type": "Discovery"}	null	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	f	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	\N
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, host_id, network_id, created_at, updated_at, if_index, if_descr, if_alias, if_type, speed_bps, admin_status, oper_status, mac_address, ip_address_id, neighbor_interface_id, neighbor_host_id, lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc, lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id, cdp_platform, cdp_address, if_name, fdb_macs, native_vlan_id, vlan_ids, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: ip_addresses; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ip_addresses (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
14ca8455-226d-4741-b90f-706df52ca3b4	7351e4ce-703f-4a0d-beac-b685d5456ce8	268f3322-1e48-4551-bbee-61ed8978f5a4	0450b5c8-9b9a-4f45-ae3d-72ce3e48ae73	127.0.0.1	\N	lo	0	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	\N
16b57a3c-dcc6-40c8-8de5-dafe68965767	7351e4ce-703f-4a0d-beac-b685d5456ce8	268f3322-1e48-4551-bbee-61ed8978f5a4	616d6f07-084c-4564-9455-b3d4adc3d453	172.25.0.4	2e:f6:28:e2:c5:24	eth0	1	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	\N
\.


--
-- Data for Name: network_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.network_credentials (network_id, credential_id) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, organization_id) FROM stdin;
7351e4ce-703f-4a0d-beac-b685d5456ce8	My Network	2026-06-22 22:39:35.688173+00	2026-06-22 22:39:35.688173+00	21345308-85f8-42fc-8500-1965a280221b
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, brevo_company_id, has_payment_method, trial_end_date, plan_limit_notifications, use_case, last_paused_at, trial_extended_used, last_downgrade_at, last_downgrade_from_plan, last_discount_at, discount_save_offer_percent_off, discount_save_offer_active_until, next_renewal_at) FROM stdin;
21345308-85f8-42fc-8500-1965a280221b	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "trial_days": 0, "network_cents": null, "included_hosts": null, "included_seats": null, "included_networks": null}	active	2026-06-22 22:39:35.452851+00	2026-06-22 22:39:35.452851+00	["OnboardingModalCompleted", "OrgCreated", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstDependencyCreated", "FirstUserApiKeyCreated", "SecondNetworkCreated"]	\N	f	\N	{"hosts": "None", "seats": "None", "networks": "None"}	other	\N	f	\N	null	\N	\N	\N	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
ea73264e-cd98-465b-9536-9365f39e106c	7351e4ce-703f-4a0d-beac-b685d5456ce8	268f3322-1e48-4551-bbee-61ed8978f5a4	60073	Tcp	Custom	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	\N
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, "position", valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
556294d3-421e-4b45-8b34-a82624b6f937	7351e4ce-703f-4a0d-beac-b685d5456ce8	2026-06-22 22:44:20.621179+00	2026-06-22 22:44:20.621179+00	Scanopy Daemon	268f3322-1e48-4551-bbee-61ed8978f5a4	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-06-22 22:44:20.621179+00	\N	\N	2026-06-22 22:44:20.621179+00	\N	\N	\N
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, network_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at, enabled_views) FROM stdin;
\.


--
-- Data for Name: snapshots; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.snapshots (id, network_id, taken_at, created_by_user_id, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: subnet_vlans; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnet_vlans (id, subnet_id, vlan_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, network_id, created_at, updated_at, cidr, name, description, subnet_type, source, virtualization, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
0450b5c8-9b9a-4f45-ae3d-72ce3e48ae73	7351e4ce-703f-4a0d-beac-b685d5456ce8	2026-06-22 22:39:44.421677+00	2026-06-22 22:39:44.421677+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery"}	null	2026-06-22 22:39:44.421677+00	\N	\N	2026-06-22 22:39:44.421677+00	\N	\N	\N
616d6f07-084c-4564-9455-b3d4adc3d453	7351e4ce-703f-4a0d-beac-b685d5456ce8	2026-06-22 22:39:44.42172+00	2026-06-22 22:39:44.42172+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	null	2026-06-22 22:39:44.42172+00	\N	\N	2026-06-22 22:39:44.453693+00	\N	\N	\N
825199e9-5714-457b-b3f3-6f12cd222697	7351e4ce-703f-4a0d-beac-b685d5456ce8	2026-06-22 22:44:31.559483+00	2026-06-22 22:44:31.559483+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "System"}	null	2026-06-22 22:44:31.559483+00	\N	\N	2026-06-22 22:44:31.559483+00	\N	\N	\N
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, is_application, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
e0dce46e-c386-4c8c-b9a3-f3281d116cf0	21345308-85f8-42fc-8500-1965a280221b	Integration Test Tag	\N	2026-06-22 22:44:04.737309+00	2026-06-22 22:44:04.737309+00	Yellow	f	2026-06-22 22:44:04.737309+00	\N	\N	\N
515a4cd4-43dd-40f3-afe2-f61b2cf2e6bc	21345308-85f8-42fc-8500-1965a280221b	Test Tag	\N	2026-06-22 22:44:30.55472+00	2026-06-22 22:44:30.55472+00	Yellow	f	2026-06-22 22:44:30.55472+00	2026-06-22 22:44:30.577908+00	25684ff0-494e-44e9-83ca-55e0c5d0549e	\N
25684ff0-494e-44e9-83ca-55e0c5d0549e	21345308-85f8-42fc-8500-1965a280221b	Updated Tag	\N	2026-06-22 22:44:30.55472+00	2026-06-22 22:44:30.55472+00	Yellow	f	2026-06-22 22:44:30.577908+00	2026-06-22 22:44:30.594024+00	\N	\N
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, options, created_at, updated_at) FROM stdin;
5448f582-abcb-409e-9c04-b633df41e210	7351e4ce-703f-4a0d-beac-b685d5456ce8	{"local": {"tag_filter": {"hidden_host_tag_ids": [], "hidden_subnet_tag_ids": [], "hidden_service_tag_ids": []}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": ["Hypervisor"]}, "request": {"element_rules": [{"id": "28c04ebd-afc6-476a-858d-e3f7742eade1", "rule": "ByTrunkPort"}, {"id": "dcb0b13c-7011-4f9d-8b12-5e67923c45b3", "rule": "ByVLAN"}, {"id": "c309b857-8deb-4388-ae19-c53759dd492e", "rule": "ByPortOpStatus"}, {"id": "bd652237-e9e6-47d9-aa69-effc9984fa22", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "RemoteAccess", "Workstation", "Mobile", "Printer", "OpenPorts"], "is_infra_rule": true}}}, {"id": "f7f09af8-dae7-4dd1-a7a3-16bd35a585ae", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "35ae4da7-b2d3-4209-b9b4-25f723cb95a1", "rule": "ByHypervisor"}, {"id": "d94c666d-bb84-4769-9db6-9dcda1bf5b05", "rule": "ByContainerRuntime"}, {"id": "68fed867-042e-4e3e-8e2d-ae1673dc5c27", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "fb260759-a8eb-4a46-a9ec-5391df596a4d", "rule": "BySubnet"}, {"id": "8c65eeb2-e488-4581-be4d-26b80fe0d2de", "rule": "MergeDockerBridges"}], "Workloads": [{"id": "00e01d7c-7966-4ae6-9be9-623c965e2921", "rule": "ByHost"}], "L2Physical": [{"id": "00e01d7c-7966-4ae6-9be9-623c965e2921", "rule": "ByHost"}], "Application": [{"id": "a27d0dca-46da-4850-bd6f-d547c0da712b", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-06-22 22:39:35.756819+00	2026-06-22 22:39:35.756819+00
\.


--
-- Data for Name: user_api_key_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_key_network_access (id, api_key_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled) FROM stdin;
\.


--
-- Data for Name: user_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_network_access (id, user_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires, pending_email, email_settings) FROM stdin;
2dab7160-a0b9-44f5-a65f-cdfcd6446406	2026-06-22 22:39:35.54572+00	2026-06-22 22:39:35.54572+00	$argon2id$v=19$m=19456,t=2,p=1$mI3ysU5uRoa+4TW4TtjIMw$/z5bCw3DqqkAurfzpmRYUUgE+OYffh1ogmRuPmnxwWY	\N	\N	\N	user@gmail.com	21345308-85f8-42fc-8500-1965a280221b	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}
47a9f290-d376-49e6-8c13-7d292b414e68	2026-06-22 22:44:31.236656+00	2026-06-22 22:44:31.236656+00	\N	\N	\N	\N	user@example.com	21345308-85f8-42fc-8500-1965a280221b	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}
\.


--
-- Data for Name: vlans; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.vlans (id, vlan_number, name, description, network_id, organization_id, source, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
ES7RGtbrcJYu4bgUXZKtuw	\\x93c410bbad925d14b8e12e9670ebd61ad12e1181a7757365725f6964d92432646162373136302d613062392d343466352d613635662d63646663643634343634303699cd07eaccb4162723ce305cff40000000	2026-06-29 22:39:35.811401+00
zedT8x6vDgKaKYte8zI9BQ	\\x93c410053d32f35e8b299a020eaf1ef353e7cd82ad70656e64696e675f736574757083a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92432323332373464352d656236612d343234612d616132632d323866353832373138303863ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea87573655f63617365a56f74686572a7757365725f6964d92432646162373136302d613062392d343466352d613635662d63646663643634343634303699cd07eaccb4162c05ce1de593fc000000	2026-06-29 22:44:05.501584+00
boa6kcYwH4TRGvweqjpEtQ	\\x93c410b5443aaa1efc1ad1841f30c691ba866e82ad70656e64696e675f736574757083a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92435666636636333652d663935332d346665652d626365632d396539653331376465313430ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea87573655f63617365a56f74686572a7757365725f6964d92432646162373136302d613062392d343466352d613635662d63646663643634343634303699cd07eaccb4162c15ce044135bf000000	2026-06-29 22:44:21.071382+00
naMlNGXM9Y2fKasIIjJNOw	\\x93c4103b4d322208ab299f8df5cc653425a39d82a7757365725f6964d92432646162373136302d613062392d343466352d613635662d636466636436343436343036ad70656e64696e675f736574757083a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92463393933326164622d633434662d343131312d616135332d653264653861663764353332ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea87573655f63617365a56f7468657299cd07eaccb4162c1ece11f3ebe0000000	2026-06-29 22:44:30.301198+00
\.


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: api_keys api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_key_key UNIQUE (key);


--
-- Name: api_keys api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_pkey PRIMARY KEY (id);


--
-- Name: bindings bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_pkey PRIMARY KEY (id);


--
-- Name: credentials credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_pkey PRIMARY KEY (id);


--
-- Name: daemons daemons_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_pkey PRIMARY KEY (id);


--
-- Name: discovery discovery_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_pkey PRIMARY KEY (id);


--
-- Name: entity_tags entity_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_pkey PRIMARY KEY (id);


--
-- Name: dependency_members group_bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_pkey PRIMARY KEY (id);


--
-- Name: dependencies groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT groups_pkey PRIMARY KEY (id);


--
-- Name: host_credentials host_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_pkey PRIMARY KEY (host_id, credential_id);


--
-- Name: hosts hosts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_pkey PRIMARY KEY (id);


--
-- Name: interfaces if_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_pkey PRIMARY KEY (id);


--
-- Name: ip_addresses interfaces_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT interfaces_pkey PRIMARY KEY (id);


--
-- Name: invites invites_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_pkey PRIMARY KEY (id);


--
-- Name: network_credentials network_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_pkey PRIMARY KEY (network_id, credential_id);


--
-- Name: networks networks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT networks_pkey PRIMARY KEY (id);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: ports ports_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_pkey PRIMARY KEY (id);


--
-- Name: services services_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_pkey PRIMARY KEY (id);


--
-- Name: shares shares_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_pkey PRIMARY KEY (id);


--
-- Name: snapshots snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_pkey PRIMARY KEY (id);


--
-- Name: subnet_vlans subnet_vlans_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_pkey PRIMARY KEY (id);


--
-- Name: subnets subnets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_pkey PRIMARY KEY (id);


--
-- Name: tags tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_pkey PRIMARY KEY (id);


--
-- Name: topologies topologies_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_pkey PRIMARY KEY (id);


--
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_network_id_key UNIQUE (api_key_id, network_id);


--
-- Name: user_api_key_network_access user_api_key_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_pkey PRIMARY KEY (id);


--
-- Name: user_api_keys user_api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_key_key UNIQUE (key);


--
-- Name: user_api_keys user_api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_pkey PRIMARY KEY (id);


--
-- Name: user_network_access user_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_pkey PRIMARY KEY (id);


--
-- Name: user_network_access user_network_access_user_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_network_id_key UNIQUE (user_id, network_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: vlans vlans_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_pkey PRIMARY KEY (id);


--
-- Name: session session_pkey; Type: CONSTRAINT; Schema: tower_sessions; Owner: postgres
--

ALTER TABLE ONLY tower_sessions.session
    ADD CONSTRAINT session_pkey PRIMARY KEY (id);


--
-- Name: idx_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_key ON public.api_keys USING btree (key);


--
-- Name: idx_api_keys_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_network ON public.api_keys USING btree (network_id);


--
-- Name: idx_bindings_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_as_of ON public.bindings USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_bindings_ip_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_ip_address ON public.bindings USING btree (ip_address_id);


--
-- Name: idx_bindings_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_lineage ON public.bindings USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_bindings_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_live ON public.bindings USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_bindings_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_network ON public.bindings USING btree (network_id);


--
-- Name: idx_bindings_port; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_port ON public.bindings USING btree (port_id);


--
-- Name: idx_bindings_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_service ON public.bindings USING btree (service_id);


--
-- Name: idx_bindings_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_snapshot_id ON public.bindings USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_credentials_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_org ON public.credentials USING btree (organization_id);


--
-- Name: idx_credentials_type; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_type ON public.credentials USING btree (((credential_type ->> 'type'::text)));


--
-- Name: idx_daemon_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemon_host_id ON public.daemons USING btree (host_id);


--
-- Name: idx_daemons_api_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_api_key ON public.daemons USING btree (api_key_id) WHERE (api_key_id IS NOT NULL);


--
-- Name: idx_daemons_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_network ON public.daemons USING btree (network_id);


--
-- Name: idx_dependencies_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_as_of ON public.dependencies USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_dependencies_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_lineage ON public.dependencies USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_dependencies_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_live ON public.dependencies USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_dependencies_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_snapshot_id ON public.dependencies USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_dependency_members_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_as_of ON public.dependency_members USING btree (dependency_id, valid_from, valid_to);


--
-- Name: idx_dependency_members_binding; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_binding ON public.dependency_members USING btree (binding_id) WHERE (binding_id IS NOT NULL);


--
-- Name: idx_dependency_members_dependency; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_dependency ON public.dependency_members USING btree (dependency_id);


--
-- Name: idx_dependency_members_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_lineage ON public.dependency_members USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_dependency_members_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_live ON public.dependency_members USING btree (dependency_id) WHERE (valid_to IS NULL);


--
-- Name: idx_dependency_members_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_service ON public.dependency_members USING btree (service_id);


--
-- Name: idx_dependency_members_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_snapshot_id ON public.dependency_members USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_dependency_members_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_dependency_members_unique_live ON public.dependency_members USING btree (dependency_id, service_id) WHERE (valid_to IS NULL);


--
-- Name: idx_discovery_daemon; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_daemon ON public.discovery USING btree (daemon_id);


--
-- Name: idx_discovery_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_network ON public.discovery USING btree (network_id);


--
-- Name: idx_entity_tags_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_as_of ON public.entity_tags USING btree (entity_id, entity_type, valid_from, valid_to);


--
-- Name: idx_entity_tags_entity; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_entity ON public.entity_tags USING btree (entity_id, entity_type);


--
-- Name: idx_entity_tags_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_lineage ON public.entity_tags USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_entity_tags_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_live ON public.entity_tags USING btree (entity_id, entity_type) WHERE (valid_to IS NULL);


--
-- Name: idx_entity_tags_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_snapshot_id ON public.entity_tags USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_entity_tags_tag_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_tag_id ON public.entity_tags USING btree (tag_id);


--
-- Name: idx_entity_tags_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_entity_tags_unique_live ON public.entity_tags USING btree (entity_id, entity_type, tag_id) WHERE (valid_to IS NULL);


--
-- Name: idx_groups_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_groups_network ON public.dependencies USING btree (network_id);


--
-- Name: idx_hosts_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_as_of ON public.hosts USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_hosts_chassis_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_chassis_id ON public.hosts USING btree (chassis_id);


--
-- Name: idx_hosts_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_lineage ON public.hosts USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_hosts_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_live ON public.hosts USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


--
-- Name: idx_hosts_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_snapshot_id ON public.hosts USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_interfaces_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_as_of ON public.interfaces USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


--
-- Name: idx_interfaces_host_if_index; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host_if_index ON public.interfaces USING btree (host_id, if_index);


--
-- Name: idx_interfaces_host_name_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_interfaces_host_name_live ON public.interfaces USING btree (host_id, if_name) WHERE ((if_name IS NOT NULL) AND (valid_to IS NULL));


--
-- Name: idx_interfaces_ip_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_ip_address ON public.interfaces USING btree (ip_address_id);


--
-- Name: idx_interfaces_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_lineage ON public.interfaces USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_interfaces_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_live ON public.interfaces USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interfaces_mac_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_mac_address ON public.interfaces USING btree (mac_address);


--
-- Name: idx_interfaces_neighbor_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_neighbor_host ON public.interfaces USING btree (neighbor_host_id);


--
-- Name: idx_interfaces_neighbor_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_neighbor_interface ON public.interfaces USING btree (neighbor_interface_id);


--
-- Name: idx_interfaces_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_network ON public.interfaces USING btree (network_id);


--
-- Name: idx_interfaces_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_snapshot_id ON public.interfaces USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_invites_expires_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_expires_at ON public.invites USING btree (expires_at);


--
-- Name: idx_invites_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_organization ON public.invites USING btree (organization_id);


--
-- Name: idx_ip_addresses_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_as_of ON public.ip_addresses USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_ip_addresses_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_host ON public.ip_addresses USING btree (host_id);


--
-- Name: idx_ip_addresses_host_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_host_mac ON public.ip_addresses USING btree (host_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_ip_addresses_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_lineage ON public.ip_addresses USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_ip_addresses_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_live ON public.ip_addresses USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_ip_addresses_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_network ON public.ip_addresses USING btree (network_id);


--
-- Name: idx_ip_addresses_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_snapshot_id ON public.ip_addresses USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_ip_addresses_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_subnet ON public.ip_addresses USING btree (subnet_id);


--
-- Name: idx_ip_addresses_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_ip_addresses_unique_live ON public.ip_addresses USING btree (host_id, subnet_id, ip_address) WHERE (valid_to IS NULL);


--
-- Name: idx_networks_owner_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_networks_owner_organization ON public.networks USING btree (organization_id);


--
-- Name: idx_organizations_stripe_customer; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_organizations_stripe_customer ON public.organizations USING btree (stripe_customer_id);


--
-- Name: idx_ports_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_as_of ON public.ports USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_ports_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_host ON public.ports USING btree (host_id);


--
-- Name: idx_ports_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_lineage ON public.ports USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_ports_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_live ON public.ports USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_ports_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_network ON public.ports USING btree (network_id);


--
-- Name: idx_ports_number; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_number ON public.ports USING btree (port_number);


--
-- Name: idx_ports_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_snapshot_id ON public.ports USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_ports_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_ports_unique_live ON public.ports USING btree (host_id, port_number, protocol) WHERE (valid_to IS NULL);


--
-- Name: idx_services_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_as_of ON public.services USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_services_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_id ON public.services USING btree (host_id);


--
-- Name: idx_services_host_position; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_position ON public.services USING btree (host_id, "position");


--
-- Name: idx_services_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_lineage ON public.services USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_services_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_live ON public.services USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_services_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_network ON public.services USING btree (network_id);


--
-- Name: idx_services_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_snapshot_id ON public.services USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_shares_enabled; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_enabled ON public.shares USING btree (is_enabled) WHERE (is_enabled = true);


--
-- Name: idx_shares_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_network ON public.shares USING btree (network_id);


--
-- Name: idx_shares_topology; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_topology ON public.shares USING btree (topology_id);


--
-- Name: idx_snapshots_network_taken_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_snapshots_network_taken_at ON public.snapshots USING btree (network_id, taken_at DESC);


--
-- Name: idx_subnet_vlans_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_as_of ON public.subnet_vlans USING btree (subnet_id, valid_from, valid_to);


--
-- Name: idx_subnet_vlans_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_lineage ON public.subnet_vlans USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_subnet_vlans_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_live ON public.subnet_vlans USING btree (subnet_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnet_vlans_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_snapshot_id ON public.subnet_vlans USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_subnet_vlans_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_subnet ON public.subnet_vlans USING btree (subnet_id);


--
-- Name: idx_subnet_vlans_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_subnet_vlans_unique_live ON public.subnet_vlans USING btree (subnet_id, vlan_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnet_vlans_vlan; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_vlan ON public.subnet_vlans USING btree (vlan_id);


--
-- Name: idx_subnets_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_as_of ON public.subnets USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_subnets_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_lineage ON public.subnets USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_subnets_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_live ON public.subnets USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnets_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_network ON public.subnets USING btree (network_id);


--
-- Name: idx_subnets_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_snapshot_id ON public.subnets USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_tags_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_as_of ON public.tags USING btree (organization_id, valid_from, valid_to);


--
-- Name: idx_tags_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_lineage ON public.tags USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_tags_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_live ON public.tags USING btree (organization_id) WHERE (valid_to IS NULL);


--
-- Name: idx_tags_org_name_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tags_org_name_live ON public.tags USING btree (organization_id, name) WHERE (valid_to IS NULL);


--
-- Name: idx_tags_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_organization ON public.tags USING btree (organization_id);


--
-- Name: idx_tags_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_snapshot_id ON public.tags USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_topologies_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_topologies_network ON public.topologies USING btree (network_id);


--
-- Name: idx_user_api_key_network_access_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_key ON public.user_api_key_network_access USING btree (api_key_id);


--
-- Name: idx_user_api_key_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_network ON public.user_api_key_network_access USING btree (network_id);


--
-- Name: idx_user_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_key ON public.user_api_keys USING btree (key);


--
-- Name: idx_user_api_keys_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_org ON public.user_api_keys USING btree (organization_id);


--
-- Name: idx_user_api_keys_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_user ON public.user_api_keys USING btree (user_id);


--
-- Name: idx_user_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_network ON public.user_network_access USING btree (network_id);


--
-- Name: idx_user_network_access_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_user ON public.user_network_access USING btree (user_id);


--
-- Name: idx_users_email_lower; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_lower ON public.users USING btree (lower(email));


--
-- Name: idx_users_email_verification_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_email_verification_token ON public.users USING btree (email_verification_token) WHERE (email_verification_token IS NOT NULL);


--
-- Name: idx_users_oidc_provider_subject; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_oidc_provider_subject ON public.users USING btree (oidc_provider, oidc_subject) WHERE ((oidc_provider IS NOT NULL) AND (oidc_subject IS NOT NULL));


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: idx_users_password_reset_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_password_reset_token ON public.users USING btree (password_reset_token) WHERE (password_reset_token IS NOT NULL);


--
-- Name: idx_vlans_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_as_of ON public.vlans USING btree (network_id, valid_from, valid_to);


--
-- Name: idx_vlans_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_lineage ON public.vlans USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_vlans_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_live ON public.vlans USING btree (network_id) WHERE (valid_to IS NULL);


--
-- Name: idx_vlans_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_network ON public.vlans USING btree (network_id);


--
-- Name: idx_vlans_network_number_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_vlans_network_number_live ON public.vlans USING btree (network_id, vlan_number) WHERE (valid_to IS NULL);


--
-- Name: idx_vlans_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_organization ON public.vlans USING btree (organization_id);


--
-- Name: idx_vlans_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_snapshot_id ON public.vlans USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: users reassign_daemons_before_user_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER reassign_daemons_before_user_delete BEFORE DELETE ON public.users FOR EACH ROW EXECUTE FUNCTION public.reassign_daemons_on_user_delete();


--
-- Name: api_keys api_keys_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: bindings bindings_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_interface_id_fkey FOREIGN KEY (ip_address_id) REFERENCES public.ip_addresses(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: bindings bindings_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_port_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_port_id_fkey FOREIGN KEY (port_id) REFERENCES public.ports(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_service_id_fkey FOREIGN KEY (service_id) REFERENCES public.services(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: credentials credentials_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.api_keys(id) ON DELETE SET NULL;


--
-- Name: daemons daemons_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: dependencies dependencies_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT dependencies_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: dependency_members dependency_members_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT dependency_members_service_id_fkey FOREIGN KEY (service_id) REFERENCES public.services(id) ON DELETE CASCADE;


--
-- Name: dependency_members dependency_members_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT dependency_members_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: entity_tags entity_tags_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: entity_tags entity_tags_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: dependency_members group_bindings_binding_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_binding_id_fkey FOREIGN KEY (binding_id) REFERENCES public.bindings(id) ON DELETE CASCADE;


--
-- Name: dependency_members group_bindings_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_group_id_fkey FOREIGN KEY (dependency_id) REFERENCES public.dependencies(id) ON DELETE CASCADE;


--
-- Name: dependencies groups_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT groups_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: hosts hosts_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: hosts hosts_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: interfaces if_entries_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces if_entries_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_interface_id_fkey FOREIGN KEY (ip_address_id) REFERENCES public.ip_addresses(id) ON DELETE SET NULL;


--
-- Name: interfaces if_entries_native_vlan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_native_vlan_id_fkey FOREIGN KEY (native_vlan_id) REFERENCES public.vlans(id) ON DELETE SET NULL;


--
-- Name: interfaces if_entries_neighbor_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_neighbor_host_id_fkey FOREIGN KEY (neighbor_host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: interfaces if_entries_neighbor_if_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_neighbor_if_entry_id_fkey FOREIGN KEY (neighbor_interface_id) REFERENCES public.interfaces(id) ON DELETE SET NULL;


--
-- Name: interfaces if_entries_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT if_entries_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses interfaces_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT interfaces_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses interfaces_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT interfaces_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: ip_addresses interfaces_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT interfaces_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: invites invites_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: invites invites_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ip_addresses ip_addresses_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses ip_addresses_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses ip_addresses_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: network_credentials network_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: network_credentials network_credentials_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: networks organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ports ports_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ports ports_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ports ports_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ports ports_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: ports ports_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: services services_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: services services_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: services services_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: services services_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: services services_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: shares shares_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: shares shares_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: shares shares_topology_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_topology_id_fkey FOREIGN KEY (topology_id) REFERENCES public.topologies(id) ON DELETE CASCADE;


--
-- Name: snapshots snapshots_created_by_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_created_by_user_id_fkey FOREIGN KEY (created_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: snapshots snapshots_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_vlan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_vlan_id_fkey FOREIGN KEY (vlan_id) REFERENCES public.vlans(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: subnets subnets_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: subnets subnets_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: tags tags_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: tags tags_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: topologies topologies_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.user_api_keys(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_network_access user_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_network_access user_network_access_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: users users_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: vlans vlans_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: vlans vlans_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict hbNTv58f56aZ5lFHyvn9F1SZwldTyIikVblkNh76S4QYKdYflsVqgPWu4oI0TzQ

