--
-- PostgreSQL database dump
--

\restrict NVYVTmkRh89cFPDq4SdxDhSe8GvsEiXQZHAgCHWbCPMCM6YW1iazTeGoNTbeyQi

-- Dumped from database version 17.7
-- Dumped by pg_dump version 17.7

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

ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.snmp_credentials DROP CONSTRAINT IF EXISTS snmp_credentials_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_snmp_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_if_entry_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_snmp_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_tag_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_network_id_fkey;
DROP TRIGGER IF EXISTS reassign_daemons_before_user_delete ON public.users;
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
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name;
DROP INDEX IF EXISTS public.idx_subnets_network;
DROP INDEX IF EXISTS public.idx_snmp_credentials_org;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_network;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_network;
DROP INDEX IF EXISTS public.idx_services_host_position;
DROP INDEX IF EXISTS public.idx_services_host_id;
DROP INDEX IF EXISTS public.idx_ports_number;
DROP INDEX IF EXISTS public.idx_ports_network;
DROP INDEX IF EXISTS public.idx_ports_host;
DROP INDEX IF EXISTS public.idx_organizations_stripe_customer;
DROP INDEX IF EXISTS public.idx_networks_snmp_credential;
DROP INDEX IF EXISTS public.idx_networks_owner_organization;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_subnet;
DROP INDEX IF EXISTS public.idx_interfaces_network;
DROP INDEX IF EXISTS public.idx_interfaces_host_mac;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_if_entries_network;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_if_entry;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_host;
DROP INDEX IF EXISTS public.idx_if_entries_mac_address;
DROP INDEX IF EXISTS public.idx_if_entries_interface;
DROP INDEX IF EXISTS public.idx_if_entries_host;
DROP INDEX IF EXISTS public.idx_hosts_snmp_credential;
DROP INDEX IF EXISTS public.idx_hosts_network;
DROP INDEX IF EXISTS public.idx_hosts_chassis_id;
DROP INDEX IF EXISTS public.idx_groups_network;
DROP INDEX IF EXISTS public.idx_group_bindings_group;
DROP INDEX IF EXISTS public.idx_group_bindings_binding;
DROP INDEX IF EXISTS public.idx_entity_tags_tag_id;
DROP INDEX IF EXISTS public.idx_entity_tags_entity;
DROP INDEX IF EXISTS public.idx_discovery_network;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_daemons_network;
DROP INDEX IF EXISTS public.idx_daemons_api_key;
DROP INDEX IF EXISTS public.idx_daemon_host_id;
DROP INDEX IF EXISTS public.idx_bindings_service;
DROP INDEX IF EXISTS public.idx_bindings_port;
DROP INDEX IF EXISTS public.idx_bindings_network;
DROP INDEX IF EXISTS public.idx_bindings_interface;
DROP INDEX IF EXISTS public.idx_api_keys_network;
DROP INDEX IF EXISTS public.idx_api_keys_key;
ALTER TABLE IF EXISTS ONLY tower_sessions.session DROP CONSTRAINT IF EXISTS session_pkey;
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
ALTER TABLE IF EXISTS ONLY public.snmp_credentials DROP CONSTRAINT IF EXISTS snmp_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.snmp_credentials DROP CONSTRAINT IF EXISTS snmp_credentials_organization_id_name_key;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_port_number_protocol_key;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_subnet_id_ip_address_key;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_pkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_if_index_key;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_binding_id_key;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_pkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_entity_id_entity_type_tag_id_key;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_pkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_pkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public._sqlx_migrations DROP CONSTRAINT IF EXISTS _sqlx_migrations_pkey;
DROP TABLE IF EXISTS tower_sessions.session;
DROP TABLE IF EXISTS public.users;
DROP TABLE IF EXISTS public.user_network_access;
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.user_api_key_network_access;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.snmp_credentials;
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.networks;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.if_entries;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.groups;
DROP TABLE IF EXISTS public.group_bindings;
DROP TABLE IF EXISTS public.entity_tags;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.daemons;
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
    interface_id uuid,
    port_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT bindings_binding_type_check CHECK ((binding_type = ANY (ARRAY['Interface'::text, 'Port'::text]))),
    CONSTRAINT valid_binding CHECK ((((binding_type = 'Interface'::text) AND (interface_id IS NOT NULL) AND (port_id IS NULL)) OR ((binding_type = 'Port'::text) AND (port_id IS NOT NULL))))
);


ALTER TABLE public.bindings OWNER TO postgres;

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
    is_unreachable boolean DEFAULT false NOT NULL
);


ALTER TABLE public.daemons OWNER TO postgres;

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
    updated_at timestamp with time zone NOT NULL
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
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.entity_tags OWNER TO postgres;

--
-- Name: group_bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.group_bindings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    group_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    "position" integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.group_bindings OWNER TO postgres;

--
-- Name: groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.groups (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    group_type text NOT NULL
);


ALTER TABLE public.groups OWNER TO postgres;

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
    snmp_credential_id uuid
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
-- Name: COLUMN hosts.snmp_credential_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.snmp_credential_id IS 'Per-host SNMP credential override (null = use network default)';


--
-- Name: if_entries; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.if_entries (
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
    interface_id uuid,
    neighbor_if_entry_id uuid,
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
    CONSTRAINT chk_neighbor_exclusive CHECK (((neighbor_if_entry_id IS NULL) OR (neighbor_host_id IS NULL)))
);


ALTER TABLE public.if_entries OWNER TO postgres;

--
-- Name: TABLE if_entries; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.if_entries IS 'SNMP ifTable entries - physical/logical interfaces on network devices';


--
-- Name: COLUMN if_entries.if_index; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_index IS 'SNMP ifIndex - stable identifier within device';


--
-- Name: COLUMN if_entries.if_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_descr IS 'SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)';


--
-- Name: COLUMN if_entries.if_alias; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_alias IS 'SNMP ifAlias - user-configured description';


--
-- Name: COLUMN if_entries.if_type; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_type IS 'SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)';


--
-- Name: COLUMN if_entries.speed_bps; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.speed_bps IS 'Interface speed from ifSpeed/ifHighSpeed in bits per second';


--
-- Name: COLUMN if_entries.admin_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.admin_status IS 'SNMP ifAdminStatus: 1=up, 2=down, 3=testing';


--
-- Name: COLUMN if_entries.oper_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.oper_status IS 'SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown';


--
-- Name: COLUMN if_entries.interface_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.interface_id IS 'FK to Interface entity when this ifEntry has an IP address (must be on same host)';


--
-- Name: COLUMN if_entries.neighbor_if_entry_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_if_entry_id IS 'Full neighbor resolution: FK to remote IfEntry discovered via LLDP/CDP';


--
-- Name: COLUMN if_entries.neighbor_host_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_host_id IS 'Partial neighbor resolution: FK to remote Host when specific port is unknown';


--
-- Name: COLUMN if_entries.lldp_mgmt_addr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_mgmt_addr IS 'LLDP remote management address (lldpRemManAddr)';


--
-- Name: COLUMN if_entries.lldp_sys_desc; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_sys_desc IS 'LLDP remote system description (lldpRemSysDesc)';


--
-- Name: COLUMN if_entries.cdp_device_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_device_id IS 'CDP cache remote device ID (typically hostname)';


--
-- Name: COLUMN if_entries.cdp_port_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_port_id IS 'CDP cache remote port ID string';


--
-- Name: COLUMN if_entries.cdp_platform; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_platform IS 'CDP cache remote device platform (e.g., Cisco IOS)';


--
-- Name: COLUMN if_entries.cdp_address; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_address IS 'CDP cache remote device management IP address';


--
-- Name: interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interfaces (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    ip_address inet NOT NULL,
    mac_address macaddr,
    name text,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.interfaces OWNER TO postgres;

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
-- Name: networks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.networks (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    organization_id uuid NOT NULL,
    snmp_credential_id uuid
);


ALTER TABLE public.networks OWNER TO postgres;

--
-- Name: COLUMN networks.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.networks.organization_id IS 'The organization that owns and pays for this network';


--
-- Name: COLUMN networks.snmp_credential_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.networks.snmp_credential_id IS 'Default SNMP credential for this network (presence enables SNMP discovery)';


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
    hubspot_company_id text
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
    "position" integer DEFAULT 0 NOT NULL
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
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.shares OWNER TO postgres;

--
-- Name: snmp_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.snmp_credentials (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    name text NOT NULL,
    version text DEFAULT 'V2c'::text NOT NULL,
    community text NOT NULL
);


ALTER TABLE public.snmp_credentials OWNER TO postgres;

--
-- Name: TABLE snmp_credentials; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.snmp_credentials IS 'SNMP credentials scoped to organization, reusable across networks';


--
-- Name: COLUMN snmp_credentials.version; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.snmp_credentials.version IS 'SNMP version: V2c (MVP), V3 (future)';


--
-- Name: COLUMN snmp_credentials.community; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.snmp_credentials.community IS 'SNMPv2c community string (encrypted)';


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
    source jsonb NOT NULL
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
    color text NOT NULL
);


ALTER TABLE public.tags OWNER TO postgres;

--
-- Name: topologies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.topologies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    edges jsonb NOT NULL,
    nodes jsonb NOT NULL,
    options jsonb NOT NULL,
    hosts jsonb NOT NULL,
    subnets jsonb NOT NULL,
    services jsonb NOT NULL,
    groups jsonb NOT NULL,
    is_stale boolean,
    last_refreshed timestamp with time zone DEFAULT now() NOT NULL,
    is_locked boolean,
    locked_at timestamp with time zone,
    locked_by uuid,
    removed_hosts uuid[],
    removed_services uuid[],
    removed_subnets uuid[],
    removed_groups uuid[],
    parent_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    interfaces jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_interfaces uuid[] DEFAULT '{}'::uuid[],
    ports jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_ports uuid[] DEFAULT '{}'::uuid[],
    bindings jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_bindings uuid[] DEFAULT '{}'::uuid[],
    if_entries jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_if_entries uuid[] DEFAULT '{}'::uuid[]
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
    password_reset_expires timestamp with time zone
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
20251006215000	users	2026-02-02 14:20:33.53543+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	3629701
20251006215100	networks	2026-02-02 14:20:33.540077+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	4718655
20251006215151	create hosts	2026-02-02 14:20:33.545143+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	3892996
20251006215155	create subnets	2026-02-02 14:20:33.549396+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	4092670
20251006215201	create groups	2026-02-02 14:20:33.553831+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	4388105
20251006215204	create daemons	2026-02-02 14:20:33.55856+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	4533167
20251006215212	create services	2026-02-02 14:20:33.563444+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	5331936
20251029193448	user-auth	2026-02-02 14:20:33.569119+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	6598493
20251030044828	daemon api	2026-02-02 14:20:33.576042+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1616183
20251030170438	host-hide	2026-02-02 14:20:33.577971+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1173562
20251102224919	create discovery	2026-02-02 14:20:33.579444+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	11493419
20251106235621	normalize-daemon-cols	2026-02-02 14:20:33.591245+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	1772095
20251107034459	api keys	2026-02-02 14:20:33.593318+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	9364353
20251107222650	oidc-auth	2026-02-02 14:20:33.603048+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	27133459
20251110181948	orgs-billing	2026-02-02 14:20:33.630487+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	26576151
20251113223656	group-enhancements	2026-02-02 14:20:33.658303+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	2827667
20251117032720	daemon-mode	2026-02-02 14:20:33.669459+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	6726353
20251118143058	set-default-plan	2026-02-02 14:20:33.676556+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1632344
20251118225043	save-topology	2026-02-02 14:20:33.67898+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	178104602
20251123232748	network-permissions	2026-02-02 14:20:33.863743+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	3055994
20251125001342	billing-updates	2026-02-02 14:20:33.867136+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	1016648
20251128035448	org-onboarding-status	2026-02-02 14:20:33.868465+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1522788
20251129180942	nfs-consolidate	2026-02-02 14:20:33.870317+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1460701
20251206052641	discovery-progress	2026-02-02 14:20:33.872122+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	1699500
20251206202200	plan-fix	2026-02-02 14:20:33.874134+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	1004445
20251207061341	daemon-url	2026-02-02 14:20:33.875448+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	2410584
20251210045929	tags	2026-02-02 14:20:33.878181+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	8915731
20251210175035	terms	2026-02-02 14:20:33.887455+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	993464
20251213025048	hash-keys	2026-02-02 14:20:33.888846+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	10625277
20251214050638	scanopy	2026-02-02 14:20:33.899824+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	1498282
20251215215724	topo-scanopy-fix	2026-02-02 14:20:33.901649+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	825399
20251217153736	category rename	2026-02-02 14:20:33.90281+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	2133241
20251218053111	invite-persistence	2026-02-02 14:20:33.905928+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	5549144
20251219211216	create shares	2026-02-02 14:20:33.911819+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	6809168
20251220170928	permissions-cleanup	2026-02-02 14:20:33.919+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1580916
20251220180000	commercial-to-community	2026-02-02 14:20:33.920941+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	892907
20251221010000	cleanup subnet type	2026-02-02 14:20:33.922145+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	896555
20251221020000	remove host target	2026-02-02 14:20:33.923337+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	972876
20251221030000	user network access	2026-02-02 14:20:33.924602+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	6938270
20251221040000	interfaces table	2026-02-02 14:20:33.931879+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	9701897
20251221050000	ports table	2026-02-02 14:20:33.941935+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	8935128
20251221060000	bindings table	2026-02-02 14:20:33.951195+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	10930913
20251221070000	group bindings	2026-02-02 14:20:33.962473+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	6517411
20251222020000	tag cascade delete	2026-02-02 14:20:33.969325+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1225652
20251223232524	network remove default	2026-02-02 14:20:33.970828+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	986381
20251225100000	color enum	2026-02-02 14:20:33.972109+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1268561
20251227010000	topology snapshot migration	2026-02-02 14:20:33.973683+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	4420412
20251228010000	user api keys	2026-02-02 14:20:33.978421+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	11614056
20251230160000	daemon version and maintainer	2026-02-02 14:20:33.99037+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	2857092
20260103000000	service position	2026-02-02 14:20:33.993541+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	2075785
20260106000000	interface mac index	2026-02-02 14:20:33.995931+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	1801340
20260106204402	entity tags junction	2026-02-02 14:20:33.998055+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	12763833
20260108033856	fix entity tags json format	2026-02-02 14:20:34.011145+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	1422740
20260110000000	email verification	2026-02-02 14:20:34.012869+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	3240471
20260114145808	daemon user fk set null	2026-02-02 14:20:34.016438+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	993785
20260116010000	snmp credentials	2026-02-02 14:20:34.017737+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	6743866
20260116020000	host snmp fields	2026-02-02 14:20:34.024798+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	4203959
20260116030000	if entries	2026-02-02 14:20:34.029296+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	13317882
20260116100000	daemon api key link	2026-02-02 14:20:34.04303+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	3181381
20260131190000	add hubspot company id	2026-02-02 14:20:34.046523+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	882025
20260201021238	fix service acronym capitalization	2026-02-02 14:20:34.047875+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	1802943
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext) FROM stdin;
ac199f17-1bf3-4d40-b8f8-b704e531e940	0a06593fec72521c13c242b202cfa7cded2585ce680b0a390f4c7a2ac122b26e	9ca6ee23-2d86-4463-8e3a-275f914a88b2	Integrated Daemon API Key	2026-02-02 14:20:38.865122+00	2026-02-02 14:20:38.865122+00	2026-02-02 14:35:21.209787+00	\N	t	\N
6e332afd-8197-4a05-94fb-b8757b62c2d8	897d53b4c5e1c5084fa694e4bea508fc6d1d7816e1c42534ea8afeaff7110036	9ca6ee23-2d86-4463-8e3a-275f914a88b2	Compat Test API Key	2026-02-02 14:26:57.530836+00	2026-02-02 14:26:57.530836+00	2026-02-02 14:27:00.86696+00	\N	t	\N
99b698af-ebad-41e7-99b5-6161326c61d7	2f346ca08e798bc1e817f7865d2459c5a4a1bccfb35657bd0a929ae8c2bf26b1	9ca6ee23-2d86-4463-8e3a-275f914a88b2	scanopy-daemon-serverpoll API Key	2026-02-02 14:26:56.653312+00	2026-02-02 14:26:56.653312+00	2026-02-02 14:35:35.180541+00	\N	t	scp_d_mQ4NIHvOHRpwl3OgC2EGrjzU6AoM3fkW
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
3bf5df1f-f062-49b4-b02a-c976357d2ec3	9ca6ee23-2d86-4463-8e3a-275f914a88b2	3451ebf3-abcd-49bf-bfe9-670e80df58cd	Port	8ea80825-f774-4fe7-8981-0ef6d5179ee8	51928506-84b0-4cb6-b896-7adb21adb124	2026-02-02 14:27:04.230142+00	2026-02-02 14:27:04.230142+00
cdc705ab-d382-4246-97aa-f97e51d0e699	9ca6ee23-2d86-4463-8e3a-275f914a88b2	b515e6e4-f12d-4b05-b3f4-98e695095516	Port	8dbf816d-ec1d-43dd-a126-c98fe550f54a	cf1ca8a0-dec0-4fde-8bb9-3582fef408cc	2026-02-02 14:33:14.117139+00	2026-02-02 14:33:14.117139+00
aa11d1b2-7c57-4af5-bcf5-7a6edf62b752	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2dd59191-4668-42e3-b3a8-27e1109fecff	Port	0e4d8148-4485-4855-bfbb-7dceeae451c2	7390c302-6f12-42ca-b5eb-688b3e359c66	2026-02-02 14:33:59.809885+00	2026-02-02 14:33:59.809885+00
40dbd4c3-06d4-4302-b737-d8ce011a4875	9ca6ee23-2d86-4463-8e3a-275f914a88b2	f9a5578f-7de6-4386-8ecd-243f36e787f8	Port	2148cd1e-3972-4c3a-9bed-d23802868da6	103b7bab-0890-4e38-8c69-a54c0029fb0d	2026-02-02 14:33:45.214513+00	2026-02-02 14:33:45.214513+00
0a40020e-5d63-4f71-bf35-6986c8231485	9ca6ee23-2d86-4463-8e3a-275f914a88b2	d326fc68-d04d-4127-aa91-caa515119ca2	Port	b4e72ade-fe84-491c-8c96-eb345c19af59	ecb6b2fb-acdb-4464-9d90-0fbb2663612c	2026-02-02 14:34:18.593161+00	2026-02-02 14:34:18.593161+00
e2ba7aad-706d-410a-b869-a4855889f8b1	9ca6ee23-2d86-4463-8e3a-275f914a88b2	d326fc68-d04d-4127-aa91-caa515119ca2	Port	b4e72ade-fe84-491c-8c96-eb345c19af59	bb78b421-c9e9-4b5a-b29c-8908dccf8103	2026-02-02 14:34:18.593163+00	2026-02-02 14:34:18.593163+00
6d73a3ae-2ce3-413b-9099-3280aaf3b0d7	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0d168686-156e-4d36-bfa9-f0bfd4a5971f	Port	f26cfee1-96a3-44f1-abf4-a54b1fb25703	2bd74709-8289-4621-88a2-6a23a9411bec	2026-02-02 14:34:33.938235+00	2026-02-02 14:34:33.938235+00
80bdf007-9c3f-4ce9-9080-ba876174e6d9	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8b9a212e-edb4-4c10-887d-f452a994e3ea	Port	f26cfee1-96a3-44f1-abf4-a54b1fb25703	44264368-2d26-4958-b0f7-a9b611a97bad	2026-02-02 14:34:36.971542+00	2026-02-02 14:34:36.971542+00
feeb7640-943b-4f53-9d96-b047dd631878	9ca6ee23-2d86-4463-8e3a-275f914a88b2	ee0cfd70-7814-44fa-a84b-440f61b19bca	Port	f26cfee1-96a3-44f1-abf4-a54b1fb25703	25e38624-e3f2-47f4-ba16-c42e4c600e05	2026-02-02 14:34:39.228201+00	2026-02-02 14:34:39.228201+00
905ddf73-0473-4358-8f04-3e3b888f1bf1	9ca6ee23-2d86-4463-8e3a-275f914a88b2	eb42c5ac-45aa-4781-8559-e084f35165c1	Port	f26cfee1-96a3-44f1-abf4-a54b1fb25703	fd3ac533-5cbb-4f7f-89c3-38495a3132ab	2026-02-02 14:34:39.22846+00	2026-02-02 14:34:39.22846+00
bf6e6318-1e08-4330-915d-1ce587408dc7	9ca6ee23-2d86-4463-8e3a-275f914a88b2	eb42c5ac-45aa-4781-8559-e084f35165c1	Port	f26cfee1-96a3-44f1-abf4-a54b1fb25703	a49bf5db-b6b1-4db3-8cca-1ef2dfa28b8a	2026-02-02 14:34:39.228461+00	2026-02-02 14:34:39.228461+00
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable) FROM stdin;
e253fb29-5cd5-48a9-a9a9-3a56f78b5989	9ca6ee23-2d86-4463-8e3a-275f914a88b2	53cf4d7a-0ed6-452f-8bc6-4e4a3d611182	2026-02-02 14:20:39.014176+00	2026-02-02 14:35:21.213756+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["1d02490c-9921-4823-bf70-1838de3c4f1c"]}	2026-02-02 14:20:39.014176+00	"daemon_poll"		scanopy-daemon	0.14.1	3d92a70e-c526-4e92-bcfd-1976c030b0b5	\N	f
8934f1fb-af23-4c39-ada2-2414397efb1e	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e4c75d9f-96a4-4780-978e-b6dd0fbcba48	2026-02-02 14:26:56.656929+00	2026-02-02 14:35:34.104308+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["30c59d3a-2913-4b6b-8fff-d0f739437431"]}	2026-02-02 14:26:56.656929+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.14.1	3d92a70e-c526-4e92-bcfd-1976c030b0b5	99b698af-ebad-41e7-99b5-6161326c61d7	f
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at) FROM stdin;
4801a6f5-3d15-4b22-ada2-3bd56cbcf948	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report	2026-02-02 14:20:39.02027+00	2026-02-02 14:20:39.02027+00
6fe9b6da-a330-4c04-ab5f-f6fee18df3c2	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery	2026-02-02 14:20:39.022565+00	2026-02-02 14:20:39.022565+00
c72b1b46-e08a-4523-b2ca-cdc31fd4104a	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-02-02T14:20:51.271811919Z", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report	2026-02-02 14:20:51.227652+00	2026-02-02 14:20:51.276744+00
e50580ba-c469-43dc-a064-b290f0d332db	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-02-02T14:26:56.263973705Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery	2026-02-02 14:21:21.225991+00	2026-02-02 14:26:56.269104+00
7b308850-e2ea-4dc4-bc4d-12f13373ffe0	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "AdHoc", "last_run": "2026-02-02T14:27:03.377228398Z"}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-02-02 14:27:03.370773+00	2026-02-02 14:27:03.370773+00
bbd6528e-cdac-40a5-9471-a99312201353	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "SelfReport", "host_id": "e4c75d9f-96a4-4780-978e-b6dd0fbcba48"}	Self Report	2026-02-02 14:27:04.10863+00	2026-02-02 14:27:04.10863+00
833d1bad-7e9a-4137-8fdd-9a98ca459622	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery	2026-02-02 14:27:04.109983+00	2026-02-02 14:27:04.109983+00
596859e8-49e3-4d1b-8343-e9f608479f7f	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Discovery Run (Stalled)	2026-02-02 14:21:21.225991+00	2026-02-02 14:32:34.098782+00
6231b945-34b3-4566-a5f8-6d0f4e5dc3d0	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "7c20860a-b07c-4c4f-81bc-c7f44bee7a3d", "started_at": "2026-02-02T14:29:34.148055772Z", "finished_at": "2026-02-02T14:35:35.179318587Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Network Discovery	2026-02-02 14:29:34.148055+00	2026-02-02 14:35:35.18476+00
ad255020-3bf5-435c-8648-99ee34e20fc4	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-02-02T14:27:04.215944568Z", "finished_at": "2026-02-02T14:28:39.236359731Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report	2026-02-02 14:27:04.215944+00	2026-02-02 14:28:39.244203+00
b35ac98d-c0b7-4922-9e1a-93901941a475	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "f5a833cc-002f-41e1-b540-c0a623d6775c", "started_at": "2026-01-25T23:12:10.196614920Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}}}	{"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}	Discovery Run (Stalled)	2026-01-25 23:12:10.196614+00	2026-02-02 14:32:34.098782+00
009eec0d-6891-4065-a5b4-4fff4532eae4	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-02-02T14:27:04.215944568Z", "finished_at": "2026-02-02T14:28:39.236359731Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report	2026-02-02 14:27:04.215944+00	2026-02-02 14:29:04.109451+00
41510588-98ba-462e-92d6-29793415ad87	9ca6ee23-2d86-4463-8e3a-275f914a88b2	8934f1fb-af23-4c39-ada2-2414397efb1e	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-02-02T14:27:04.215944568Z", "finished_at": "2026-02-02T14:28:39.236359731Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report	2026-02-02 14:27:04.215944+00	2026-02-02 14:29:34.108914+00
89b02930-09ea-48e5-b5e2-26a62b48a759	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "f7531810-14a6-48d2-aa0f-8b4e9e97d176", "started_at": "2026-01-26T13:52:00.181047960Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Discovery Run (Stalled)	2026-01-26 13:52:00.181047+00	2026-02-02 14:32:34.098782+00
381cf61e-75c7-4592-a976-4d8700ad4a36	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "adfad9b0-e25d-4bd4-88bc-e0ecc0f3f4a4", "started_at": "2026-01-26T13:51:30.181086877Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}}}	{"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}	Discovery Run (Stalled)	2026-01-26 13:51:30.181086+00	2026-02-02 14:32:34.098782+00
12771d0f-9b25-4851-b709-2826ca3343c8	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "1a182cd9-9710-482e-8645-d95c40db018a", "started_at": "2026-01-26T14:03:24.338877430Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}}}	{"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}	Discovery Run (Stalled)	2026-01-26 14:03:24.338877+00	2026-02-02 14:32:34.098782+00
da8e5876-d1b9-4fce-9820-e87d6f21543b	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "7c314669-ec4f-4719-9e7e-2c579c7d9cd6", "started_at": "2026-01-25T23:12:40.158142587Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Discovery Run (Stalled)	2026-01-25 23:12:40.158142+00	2026-02-02 14:32:34.098782+00
74f94d6b-4081-4209-ab0d-06a1ee0d12fe	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "580379b9-0101-428f-baef-843afbc5dfee", "started_at": "2026-01-26T14:03:54.326189222Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}	Discovery Run (Stalled)	2026-01-26 14:03:54.326189+00	2026-02-02 14:32:34.098782+00
0372c341-bf79-4ba6-8afc-4050e780b919	9ca6ee23-2d86-4463-8e3a-275f914a88b2	e253fb29-5cd5-48a9-a9a9-3a56f78b5989	{"type": "Historical", "results": {"error": "Session stalled - no updates received from daemon for more than 5 minutes", "phase": "Failed", "progress": 0, "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-02-02T14:32:34.098782329Z", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Discovery Run (Stalled)	2026-02-02 14:20:51.227652+00	2026-02-02 14:32:34.098782+00
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at) FROM stdin;
5be64391-3070-4a3f-bc6a-f8d6a883e723	0d168686-156e-4d36-bfa9-f0bfd4a5971f	"Service"	621d9c15-32ad-4322-9bf5-a6beac86cfc7	2026-02-02 14:35:35.207209+00
\.


--
-- Data for Name: group_bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.group_bindings (id, group_id, binding_id, "position", created_at) FROM stdin;
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, network_id, name, description, created_at, updated_at, source, color, edge_style, group_type) FROM stdin;
728ee63a-0768-4d76-9e5b-a9b8feaff38f	9ca6ee23-2d86-4463-8e3a-275f914a88b2		\N	2026-02-02 14:35:35.20992+00	2026-02-02 14:35:35.20992+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, snmp_credential_id) FROM stdin;
9944ef1b-0351-49ae-9e1f-184b79190ca2	9ca6ee23-2d86-4463-8e3a-275f914a88b2	scanopy-server-1.scanopy_scanopy-dev	scanopy-server-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:33:45.224112706Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	null	2026-02-02 14:33:45.224113+00	2026-02-02 14:33:45.224113+00	f	\N	\N	\N	\N	\N	\N	\N
5c8f15f0-e636-4e1f-aef7-01498f8d094e	9ca6ee23-2d86-4463-8e3a-275f914a88b2	scanopy-postgres-dev-1.scanopy_scanopy-dev	scanopy-postgres-dev-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:33:28.508588891Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	null	2026-02-02 14:33:28.508592+00	2026-02-02 14:33:28.508592+00	f	\N	\N	\N	\N	\N	\N	\N
a9590643-88e0-45c1-8420-738ed98070ba	9ca6ee23-2d86-4463-8e3a-275f914a88b2	83afdc1d5412	83afdc1d5412	Scanopy daemon	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:27:04.230126090Z", "type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e"}]}	null	2026-02-02 14:27:04.230128+00	2026-02-02 14:27:04.230128+00	f	\N	\N	\N	\N	\N	\N	\N
466374ae-501d-4900-b866-a4f66ab15c38	9ca6ee23-2d86-4463-8e3a-275f914a88b2	scanopy-daemon-1.scanopy_scanopy-dev	scanopy-daemon-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:33:11.793585410Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	null	2026-02-02 14:33:11.793588+00	2026-02-02 14:33:11.793588+00	f	\N	\N	\N	\N	\N	\N	\N
1daef5d1-b5e0-4dbe-b710-83aed1d495f0	9ca6ee23-2d86-4463-8e3a-275f914a88b2	homeassistant-discovery.scanopy_scanopy-dev	homeassistant-discovery.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:34:02.127496836Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	null	2026-02-02 14:34:02.127498+00	2026-02-02 14:34:02.127498+00	f	\N	\N	\N	\N	\N	\N	\N
0a94360d-830c-4ce5-b048-834588c775dc	9ca6ee23-2d86-4463-8e3a-275f914a88b2	runnervmkj6or	runnervmkj6or	\N	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:34:22.654010553Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	null	2026-02-02 14:34:22.654014+00	2026-02-02 14:34:22.654014+00	f	\N	\N	\N	\N	\N	\N	\N
\.


--
-- Data for Name: if_entries; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.if_entries (id, host_id, network_id, created_at, updated_at, if_index, if_descr, if_alias, if_type, speed_bps, admin_status, oper_status, mac_address, interface_id, neighbor_if_entry_id, neighbor_host_id, lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc, lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id, cdp_platform, cdp_address) FROM stdin;
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at) FROM stdin;
8ea80825-f774-4fe7-8981-0ef6d5179ee8	9ca6ee23-2d86-4463-8e3a-275f914a88b2	a9590643-88e0-45c1-8420-738ed98070ba	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.7	72:20:85:e5:c7:6a	eth0	0	2026-02-02 14:27:04.221816+00	2026-02-02 14:27:04.221816+00
8dbf816d-ec1d-43dd-a126-c98fe550f54a	9ca6ee23-2d86-4463-8e3a-275f914a88b2	466374ae-501d-4900-b866-a4f66ab15c38	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.4	1a:65:26:47:b1:16	\N	0	2026-02-02 14:33:11.793548+00	2026-02-02 14:33:11.793548+00
0e4d8148-4485-4855-bfbb-7dceeae451c2	9ca6ee23-2d86-4463-8e3a-275f914a88b2	9944ef1b-0351-49ae-9e1f-184b79190ca2	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.3	1a:ad:10:30:40:05	\N	0	2026-02-02 14:33:45.224083+00	2026-02-02 14:33:45.224083+00
2148cd1e-3972-4c3a-9bed-d23802868da6	9ca6ee23-2d86-4463-8e3a-275f914a88b2	5c8f15f0-e636-4e1f-aef7-01498f8d094e	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.6	f2:81:f7:4e:a6:4c	\N	0	2026-02-02 14:33:28.508534+00	2026-02-02 14:33:28.508534+00
b4e72ade-fe84-491c-8c96-eb345c19af59	9ca6ee23-2d86-4463-8e3a-275f914a88b2	1daef5d1-b5e0-4dbe-b710-83aed1d495f0	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.5	3a:2b:51:0d:7f:e0	\N	0	2026-02-02 14:34:02.127463+00	2026-02-02 14:34:02.127463+00
f26cfee1-96a3-44f1-abf4-a54b1fb25703	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	30c59d3a-2913-4b6b-8fff-d0f739437431	172.25.0.1	9a:3e:2b:7f:b9:cd	\N	0	2026-02-02 14:34:22.653968+00	2026-02-02 14:34:22.653968+00
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, organization_id, snmp_credential_id) FROM stdin;
9ca6ee23-2d86-4463-8e3a-275f914a88b2	My Network	2026-02-02 14:20:38.84596+00	2026-02-02 14:20:38.84596+00	5a5a66f6-df1d-46f8-b434-769f5837ee9f	\N
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, hubspot_company_id) FROM stdin;
5a5a66f6-df1d-46f8-b434-769f5837ee9f	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "trial_days": 0}	active	2026-02-02 14:20:38.83781+00	2026-02-02 14:20:38.83781+00	["OnboardingModalCompleted", "FirstDaemonRegistered", "FirstApiKeyCreated"]	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
51928506-84b0-4cb6-b896-7adb21adb124	9ca6ee23-2d86-4463-8e3a-275f914a88b2	a9590643-88e0-45c1-8420-738ed98070ba	60074	Tcp	Custom	2026-02-02 14:27:04.229962+00	2026-02-02 14:27:04.229962+00
cf1ca8a0-dec0-4fde-8bb9-3582fef408cc	9ca6ee23-2d86-4463-8e3a-275f914a88b2	466374ae-501d-4900-b866-a4f66ab15c38	60073	Tcp	Custom	2026-02-02 14:33:14.117129+00	2026-02-02 14:33:14.117129+00
7390c302-6f12-42ca-b5eb-688b3e359c66	9ca6ee23-2d86-4463-8e3a-275f914a88b2	9944ef1b-0351-49ae-9e1f-184b79190ca2	60072	Tcp	Custom	2026-02-02 14:33:59.809873+00	2026-02-02 14:33:59.809873+00
103b7bab-0890-4e38-8c69-a54c0029fb0d	9ca6ee23-2d86-4463-8e3a-275f914a88b2	5c8f15f0-e636-4e1f-aef7-01498f8d094e	5432	Tcp	PostgreSQL	2026-02-02 14:33:45.214502+00	2026-02-02 14:33:45.214502+00
ecb6b2fb-acdb-4464-9d90-0fbb2663612c	9ca6ee23-2d86-4463-8e3a-275f914a88b2	1daef5d1-b5e0-4dbe-b710-83aed1d495f0	8123	Tcp	Custom	2026-02-02 14:34:18.593149+00	2026-02-02 14:34:18.593149+00
bb78b421-c9e9-4b5a-b29c-8908dccf8103	9ca6ee23-2d86-4463-8e3a-275f914a88b2	1daef5d1-b5e0-4dbe-b710-83aed1d495f0	18555	Tcp	Custom	2026-02-02 14:34:18.593156+00	2026-02-02 14:34:18.593156+00
2bd74709-8289-4621-88a2-6a23a9411bec	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	8123	Tcp	Custom	2026-02-02 14:34:33.938224+00	2026-02-02 14:34:33.938224+00
44264368-2d26-4958-b0f7-a9b611a97bad	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	60072	Tcp	Custom	2026-02-02 14:34:36.971531+00	2026-02-02 14:34:36.971531+00
25e38624-e3f2-47f4-ba16-c42e4c600e05	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	22	Tcp	Ssh	2026-02-02 14:34:39.228187+00	2026-02-02 14:34:39.228187+00
fd3ac533-5cbb-4f7f-89c3-38495a3132ab	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	5435	Tcp	Custom	2026-02-02 14:34:39.228454+00	2026-02-02 14:34:39.228454+00
a49bf5db-b6b1-4db3-8cca-1ef2dfa28b8a	9ca6ee23-2d86-4463-8e3a-275f914a88b2	0a94360d-830c-4ce5-b048-834588c775dc	60074	Tcp	Custom	2026-02-02 14:34:39.228457+00	2026-02-02 14:34:39.228457+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, "position") FROM stdin;
3451ebf3-abcd-49bf-bfe9-670e80df58cd	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:27:04.230145+00	2026-02-02 14:27:04.230145+00	Scanopy Daemon	a9590643-88e0-45c1-8420-738ed98070ba	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-02-02T14:27:04.230144895Z", "type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e"}]}	0
b515e6e4-f12d-4b05-b3f4-98e695095516	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:33:14.117141+00	2026-02-02 14:33:14.117141+00	Scanopy Daemon	466374ae-501d-4900-b866-a4f66ab15c38	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.4:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-02-02T14:33:14.117120395Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	0
2dd59191-4668-42e3-b3a8-27e1109fecff	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:33:59.809889+00	2026-02-02 14:33:59.809889+00	Scanopy Server	9944ef1b-0351-49ae-9e1f-184b79190ca2	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-02-02T14:33:59.809861860Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	0
f9a5578f-7de6-4386-8ecd-243f36e787f8	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:33:45.214517+00	2026-02-02 14:33:45.214517+00	PostgreSQL	5c8f15f0-e636-4e1f-aef7-01498f8d094e	"PostgreSQL"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-02-02T14:33:45.214493261Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	0
d326fc68-d04d-4127-aa91-caa515119ca2	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:34:18.593167+00	2026-02-02 14:34:18.593167+00	Unclaimed Open Ports	1daef5d1-b5e0-4dbe-b710-83aed1d495f0	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-02-02T14:34:18.593140828Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	0
0d168686-156e-4d36-bfa9-f0bfd4a5971f	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:34:33.938238+00	2026-02-02 14:34:33.938238+00	Home Assistant	0a94360d-830c-4ce5-b048-834588c775dc	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-02-02T14:34:33.938212664Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	0
8b9a212e-edb4-4c10-887d-f452a994e3ea	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:34:36.971545+00	2026-02-02 14:34:36.971545+00	Scanopy Server	0a94360d-830c-4ce5-b048-834588c775dc	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-02-02T14:34:36.971519744Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	1
ee0cfd70-7814-44fa-a84b-440f61b19bca	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:34:39.228205+00	2026-02-02 14:34:39.228205+00	SSH	0a94360d-830c-4ce5-b048-834588c775dc	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-02-02T14:34:39.228172904Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	2
eb42c5ac-45aa-4781-8559-e084f35165c1	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:34:39.228462+00	2026-02-02 14:34:39.228462+00	Unclaimed Open Ports	0a94360d-830c-4ce5-b048-834588c775dc	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-02-02T14:34:39.228451948Z", "type": "Network", "daemon_id": "8934f1fb-af23-4c39-ada2-2414397efb1e", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}	3
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, network_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: snmp_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.snmp_credentials (id, organization_id, created_at, updated_at, name, version, community) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, network_id, created_at, updated_at, cidr, name, description, subnet_type, source) FROM stdin;
30c59d3a-2913-4b6b-8fff-d0f739437431	9ca6ee23-2d86-4463-8e3a-275f914a88b2	2026-02-02 14:27:03.002238+00	2026-02-02 14:27:03.002238+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2026-02-02T14:27:03.002233832Z", "type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba", "daemon_id": "6229832f-7a1c-45c2-8c9c-4050db38933f"}]}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
621d9c15-32ad-4322-9bf5-a6beac86cfc7	5a5a66f6-df1d-46f8-b434-769f5837ee9f	Integration Test Tag	\N	2026-02-02 14:35:35.195212+00	2026-02-02 14:35:35.195212+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings, if_entries, removed_if_entries) FROM stdin;
16904ce2-6b85-4076-ae3f-277e1d77da96	9ca6ee23-2d86-4463-8e3a-275f914a88b2	My Topology	[]	[]	{"local": {"no_fade_edges": false, "hide_edge_types": [], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": [], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[{"id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182", "name": "scanopy-daemon", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:20:51.242334391Z", "type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989"}]}, "hostname": "8570f4f91779", "created_at": "2026-02-02T14:20:39.008379Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:20:39.008379Z", "description": null, "virtualization": null}, {"id": "29b82cf3-9c7d-4bf5-b3cf-bad543a1e335", "name": "scanopy-server-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:24:59.379840916Z", "type": "Network", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-server-1.scanopy_scanopy-dev", "created_at": "2026-02-02T14:24:59.379842Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:24:59.379842Z", "description": null, "virtualization": null}, {"id": "ecb8e46c-a56f-416a-8618-a2fcfa4c5c29", "name": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:25:15.750068913Z", "type": "Network", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, {"date": "2026-02-02T14:25:15.750068913Z", "type": "Network", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "created_at": "2026-02-02T14:25:15.750071Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:25:15.750071Z", "description": null, "virtualization": null}, {"id": "73c48cb8-ac29-460c-aadf-74257d8f5a82", "name": "homeassistant-discovery.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:25:31.996693904Z", "type": "Network", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "homeassistant-discovery.scanopy_scanopy-dev", "created_at": "2026-02-02T14:25:31.996695Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:25:31.996695Z", "description": null, "virtualization": null}, {"id": "8b61b042-cc29-48b2-8a41-e967ab561387", "name": "scanopy-daemon-serverpoll-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:25:47.950827434Z", "type": "Network", "daemon_id": "e253fb29-5cd5-48a9-a9a9-3a56f78b5989", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-daemon-serverpoll-1.scanopy_scanopy-dev", "created_at": "2026-02-02T14:25:47.950829Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:25:47.950829Z", "description": null, "virtualization": null}]	[{"id": "30c59d3a-2913-4b6b-8fff-d0f739437431", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-02-02T14:27:03.002233832Z", "type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba", "daemon_id": "6229832f-7a1c-45c2-8c9c-4050db38933f"}]}, "created_at": "2026-02-02T14:27:03.002238Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "updated_at": "2026-02-02T14:27:03.002238Z", "description": null, "subnet_type": "Lan"}]	[]	[]	t	2026-02-02 14:20:38.863474+00	f	\N	\N	{3f41d1c3-657c-4a9b-87e5-f5bb07732d06,f2f34819-53a1-46bf-b9ce-dc9568fd4d9a,e2fab8c5-8937-4e57-ad68-75dd1441bab3}	{494c2aa7-208a-494b-af74-75ca6365f173}	{77c027d1-e218-421b-9d47-81407d6ee962}	{d645465c-3214-4694-a520-3de51b14a305}	\N	2026-02-02 14:20:38.851322+00	2026-02-02 14:20:38.851322+00	{}	[]	{}	[]	{}	[{"id": "676df18c-e5eb-4ee1-8646-c55e9faa1cad", "type": "Port", "port_id": "62c2afd7-7659-43a8-9606-9afc72cefc4c", "created_at": "2026-02-02T14:20:51.242351Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "8a897619-83a3-4426-8407-140c07eaa6de", "updated_at": "2026-02-02T14:20:51.242351Z", "interface_id": "2cb3fe46-06dd-4d71-b186-e7e3cdf3fe1b"}, {"id": "90611413-ce2e-4e8d-9d74-cf57409d1e70", "type": "Port", "port_id": "797e53b5-daef-4c6c-b9af-cb316610d1c6", "created_at": "2026-02-02T14:25:13.372021Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "2ef6f1b4-22de-4bab-a0b0-1e3b51ee3ea4", "updated_at": "2026-02-02T14:25:13.372021Z", "interface_id": "4c7f846f-207d-45ae-ae67-10c61cde67c3"}, {"id": "bebe8261-7dcc-4964-8ef0-b6cdcf456078", "type": "Port", "port_id": "ce8f5f6e-2f59-43a9-bba4-db76cca671d1", "created_at": "2026-02-02T14:25:31.995949Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "52ef160a-9796-4397-ac04-8dfeabd20709", "updated_at": "2026-02-02T14:25:31.995949Z", "interface_id": "414d6f99-f4b8-4f88-8426-eccc28f3455e"}, {"id": "79d4f228-01ea-4264-8634-1c28e4707d72", "type": "Port", "port_id": "66795aff-70ee-4131-a32b-20eae4a572aa", "created_at": "2026-02-02T14:25:47.931586Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "addf5a0a-3fde-4934-8b57-75f36ae7a53a", "updated_at": "2026-02-02T14:25:47.931586Z", "interface_id": "b2fb4780-e43b-415b-92ff-05fe33da98bd"}, {"id": "889d13bf-5c16-431e-9c69-700cefcd050d", "type": "Port", "port_id": "6bd53c2e-ce62-4e07-8f13-6f549ad6ecd8", "created_at": "2026-02-02T14:25:47.931588Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "addf5a0a-3fde-4934-8b57-75f36ae7a53a", "updated_at": "2026-02-02T14:25:47.931588Z", "interface_id": "b2fb4780-e43b-415b-92ff-05fe33da98bd"}, {"id": "01a3b94c-a068-4ebd-b072-7af2da88ffa7", "type": "Port", "port_id": "f8c31f13-a90a-4886-8d34-986cc3f8aec7", "created_at": "2026-02-02T14:26:03.746226Z", "network_id": "9ca6ee23-2d86-4463-8e3a-275f914a88b2", "service_id": "0e7a8560-f752-4c15-8afa-d214374c4d1e", "updated_at": "2026-02-02T14:26:03.746226Z", "interface_id": "7c592a0d-ffd9-4c57-9854-25a506cc0187"}]	{}	[]	{}
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

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires) FROM stdin;
3d92a70e-c526-4e92-bcfd-1976c030b0b5	2026-02-02 14:20:38.84074+00	2026-02-02 14:20:38.84074+00	$argon2id$v=19$m=19456,t=2,p=1$HoIQwSZuCZt3QdMzDOr5mg$u5Nd19FbZZu7ibGUnNRD//83xJRkjP5P/uj/++0npPk	\N	\N	\N	user@gmail.com	5a5a66f6-df1d-46f8-b434-769f5837ee9f	Owner	{}	\N	t	\N	\N	\N	\N
6cd3d31a-74d4-48cf-99e4-a9c63c75616e	2026-02-02 14:35:36.64024+00	2026-02-02 14:35:36.64024+00	\N	\N	\N	\N	user@example.com	5a5a66f6-df1d-46f8-b434-769f5837ee9f	Owner	{}	\N	f	\N	\N	\N	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
WnWsQyYL5EQaNK-WufntIQ	\\x93c41021edf9b996af341a44e40b2643ac755a81a7757365725f6964d92433643932613730652d633532362d346539322d626366642d31393736633033306230623599cd07ea280e1427ce01ea43aa000000	2026-02-09 14:20:39.032129+00
dMt6Y8YQcUyipN-f22Ax-Q	\\x93c410f93160db9fdfa4a24c7110c6637acb7482a7757365725f6964d92433643932613730652d633532362d346539322d626366642d313937366330333062306235ad70656e64696e675f736574757082a86e6574776f726b739183a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92461306638343039622d333965622d343433642d613831612d383536373834383938666439ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea280e1a39ce1f848790000000	2026-02-09 14:26:57.528779+00
6uI4VEcmqenp0bvAQM22mQ	\\x93c41099b6cd40c0bbd1e9e9a926475438e2ea82a7757365725f6964d92433643932613730652d633532362d346539322d626366642d313937366330333062306235ad70656e64696e675f736574757082a86e6574776f726b739183a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92462376165363765332d366662642d346666632d383337662d303532323238613831343731ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea280e1b01ce1799e2ef000000	2026-02-09 14:27:01.395961+00
b1YEqV6Ae0vDwHUq6cFrvw	\\x93c410bf6bc1e92a75c0c34b7b805ea904566f82ad70656e64696e675f736574757082a86e6574776f726b739183a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92462383939623839302d343631352d346136642d393936302d313534666161653038666564ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92433643932613730652d633532362d346539322d626366642d31393736633033306230623599cd07ea280e2323ce2efc9ba5000000	2026-02-09 14:35:35.788306+00
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
-- Name: entity_tags entity_tags_entity_id_entity_type_tag_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_entity_id_entity_type_tag_id_key UNIQUE (entity_id, entity_type, tag_id);


--
-- Name: entity_tags entity_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_pkey PRIMARY KEY (id);


--
-- Name: group_bindings group_bindings_group_id_binding_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_binding_id_key UNIQUE (group_id, binding_id);


--
-- Name: group_bindings group_bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_pkey PRIMARY KEY (id);


--
-- Name: groups groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_pkey PRIMARY KEY (id);


--
-- Name: hosts hosts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_pkey PRIMARY KEY (id);


--
-- Name: if_entries if_entries_host_id_if_index_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_if_index_key UNIQUE (host_id, if_index);


--
-- Name: if_entries if_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_pkey PRIMARY KEY (id);


--
-- Name: interfaces interfaces_host_id_subnet_id_ip_address_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_subnet_id_ip_address_key UNIQUE (host_id, subnet_id, ip_address);


--
-- Name: interfaces interfaces_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_pkey PRIMARY KEY (id);


--
-- Name: invites invites_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_pkey PRIMARY KEY (id);


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
-- Name: ports ports_host_id_port_number_protocol_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_port_number_protocol_key UNIQUE (host_id, port_number, protocol);


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
-- Name: snmp_credentials snmp_credentials_organization_id_name_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snmp_credentials
    ADD CONSTRAINT snmp_credentials_organization_id_name_key UNIQUE (organization_id, name);


--
-- Name: snmp_credentials snmp_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snmp_credentials
    ADD CONSTRAINT snmp_credentials_pkey PRIMARY KEY (id);


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
-- Name: idx_bindings_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_interface ON public.bindings USING btree (interface_id);


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
-- Name: idx_discovery_daemon; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_daemon ON public.discovery USING btree (daemon_id);


--
-- Name: idx_discovery_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_network ON public.discovery USING btree (network_id);


--
-- Name: idx_entity_tags_entity; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_entity ON public.entity_tags USING btree (entity_id, entity_type);


--
-- Name: idx_entity_tags_tag_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_tag_id ON public.entity_tags USING btree (tag_id);


--
-- Name: idx_group_bindings_binding; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_binding ON public.group_bindings USING btree (binding_id);


--
-- Name: idx_group_bindings_group; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_group ON public.group_bindings USING btree (group_id);


--
-- Name: idx_groups_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_groups_network ON public.groups USING btree (network_id);


--
-- Name: idx_hosts_chassis_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_chassis_id ON public.hosts USING btree (chassis_id);


--
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


--
-- Name: idx_hosts_snmp_credential; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_snmp_credential ON public.hosts USING btree (snmp_credential_id);


--
-- Name: idx_if_entries_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_host ON public.if_entries USING btree (host_id);


--
-- Name: idx_if_entries_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_interface ON public.if_entries USING btree (interface_id);


--
-- Name: idx_if_entries_mac_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_mac_address ON public.if_entries USING btree (mac_address);


--
-- Name: idx_if_entries_neighbor_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_host ON public.if_entries USING btree (neighbor_host_id);


--
-- Name: idx_if_entries_neighbor_if_entry; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_if_entry ON public.if_entries USING btree (neighbor_if_entry_id);


--
-- Name: idx_if_entries_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_network ON public.if_entries USING btree (network_id);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


--
-- Name: idx_interfaces_host_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host_mac ON public.interfaces USING btree (host_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_interfaces_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_network ON public.interfaces USING btree (network_id);


--
-- Name: idx_interfaces_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_subnet ON public.interfaces USING btree (subnet_id);


--
-- Name: idx_invites_expires_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_expires_at ON public.invites USING btree (expires_at);


--
-- Name: idx_invites_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_organization ON public.invites USING btree (organization_id);


--
-- Name: idx_networks_owner_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_networks_owner_organization ON public.networks USING btree (organization_id);


--
-- Name: idx_networks_snmp_credential; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_networks_snmp_credential ON public.networks USING btree (snmp_credential_id);


--
-- Name: idx_organizations_stripe_customer; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_organizations_stripe_customer ON public.organizations USING btree (stripe_customer_id);


--
-- Name: idx_ports_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_host ON public.ports USING btree (host_id);


--
-- Name: idx_ports_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_network ON public.ports USING btree (network_id);


--
-- Name: idx_ports_number; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_number ON public.ports USING btree (port_number);


--
-- Name: idx_services_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_id ON public.services USING btree (host_id);


--
-- Name: idx_services_host_position; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_position ON public.services USING btree (host_id, "position");


--
-- Name: idx_services_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_network ON public.services USING btree (network_id);


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
-- Name: idx_snmp_credentials_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_snmp_credentials_org ON public.snmp_credentials USING btree (organization_id);


--
-- Name: idx_subnets_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_network ON public.subnets USING btree (network_id);


--
-- Name: idx_tags_org_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tags_org_name ON public.tags USING btree (organization_id, name);


--
-- Name: idx_tags_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_organization ON public.tags USING btree (organization_id);


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
-- Name: users reassign_daemons_before_user_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER reassign_daemons_before_user_delete BEFORE DELETE ON public.users FOR EACH ROW EXECUTE FUNCTION public.reassign_daemons_on_user_delete();


--
-- Name: api_keys api_keys_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


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
-- Name: entity_tags entity_tags_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_binding_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_binding_id_fkey FOREIGN KEY (binding_id) REFERENCES public.bindings(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_fkey FOREIGN KEY (group_id) REFERENCES public.groups(id) ON DELETE CASCADE;


--
-- Name: groups groups_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_snmp_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_snmp_credential_id_fkey FOREIGN KEY (snmp_credential_id) REFERENCES public.snmp_credentials(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: if_entries if_entries_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_host_id_fkey FOREIGN KEY (neighbor_host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_if_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_if_entry_id_fkey FOREIGN KEY (neighbor_if_entry_id) REFERENCES public.if_entries(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
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
-- Name: networks networks_snmp_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT networks_snmp_credential_id_fkey FOREIGN KEY (snmp_credential_id) REFERENCES public.snmp_credentials(id) ON DELETE SET NULL;


--
-- Name: networks organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ports ports_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ports ports_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: services services_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: services services_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


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
-- Name: snmp_credentials snmp_credentials_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snmp_credentials
    ADD CONSTRAINT snmp_credentials_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: tags tags_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


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
-- PostgreSQL database dump complete
--

\unrestrict NVYVTmkRh89cFPDq4SdxDhSe8GvsEiXQZHAgCHWbCPMCM6YW1iazTeGoNTbeyQi

