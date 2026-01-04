--
-- PostgreSQL database dump
--

\restrict LkFuFmhspVOcbW2PhUcYnskZxOo4o5iJw1GRHNg6723lQZxG3Kbll4psRsZVbzl

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
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_network_id_fkey;
DROP TRIGGER IF EXISTS trigger_remove_deleted_tag_from_entities ON public.tags;
DROP INDEX IF EXISTS public.idx_users_organization;
DROP INDEX IF EXISTS public.idx_users_oidc_provider_subject;
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
DROP INDEX IF EXISTS public.idx_networks_owner_organization;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_subnet;
DROP INDEX IF EXISTS public.idx_interfaces_network;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_hosts_network;
DROP INDEX IF EXISTS public.idx_groups_network;
DROP INDEX IF EXISTS public.idx_group_bindings_group;
DROP INDEX IF EXISTS public.idx_group_bindings_binding;
DROP INDEX IF EXISTS public.idx_discovery_network;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_daemons_network;
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
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_port_number_protocol_key;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_subnet_id_ip_address_key;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_binding_id_key;
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
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.networks;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.groups;
DROP TABLE IF EXISTS public.group_bindings;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.daemons;
DROP TABLE IF EXISTS public.bindings;
DROP TABLE IF EXISTS public.api_keys;
DROP TABLE IF EXISTS public._sqlx_migrations;
DROP FUNCTION IF EXISTS public.remove_deleted_tag_from_entities();
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
-- Name: remove_deleted_tag_from_entities(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.remove_deleted_tag_from_entities() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- Remove the deleted tag ID from all entity tags arrays
    UPDATE users SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE discovery SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE hosts SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE networks SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE subnets SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE groups SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE daemons SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE services SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE api_keys SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);
    UPDATE topologies SET tags = array_remove(tags, OLD.id), updated_at = NOW() WHERE OLD.id = ANY(tags);

    RETURN OLD;
END;
$$;


ALTER FUNCTION public.remove_deleted_tag_from_entities() OWNER TO postgres;

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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    last_seen timestamp with time zone NOT NULL,
    capabilities jsonb DEFAULT '{}'::jsonb,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    mode text DEFAULT '"Push"'::text,
    url text NOT NULL,
    name text,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    version text,
    user_id uuid NOT NULL
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
    updated_at timestamp with time zone NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.discovery OWNER TO postgres;

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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.hosts OWNER TO postgres;

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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    onboarding jsonb DEFAULT '[]'::jsonb
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    removed_bindings uuid[] DEFAULT '{}'::uuid[]
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
    is_enabled boolean DEFAULT true NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    terms_accepted_at timestamp with time zone
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
20251006215000	users	2026-01-04 19:54:30.040822+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	3440812
20251006215100	networks	2026-01-04 19:54:30.045405+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	5150523
20251006215151	create hosts	2026-01-04 19:54:30.050882+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	3803359
20251006215155	create subnets	2026-01-04 19:54:30.055373+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	3775206
20251006215201	create groups	2026-01-04 19:54:30.059834+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	3995837
20251006215204	create daemons	2026-01-04 19:54:30.064152+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	4841927
20251006215212	create services	2026-01-04 19:54:30.069378+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	4925751
20251029193448	user-auth	2026-01-04 19:54:30.074732+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	7141509
20251030044828	daemon api	2026-01-04 19:54:30.082171+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1644447
20251030170438	host-hide	2026-01-04 19:54:30.084122+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1139536
20251102224919	create discovery	2026-01-04 19:54:30.085561+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	11924707
20251106235621	normalize-daemon-cols	2026-01-04 19:54:30.097893+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	1939139
20251107034459	api keys	2026-01-04 19:54:30.100519+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	9663996
20251107222650	oidc-auth	2026-01-04 19:54:30.110224+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	30501211
20251110181948	orgs-billing	2026-01-04 19:54:30.141501+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	16956090
20251113223656	group-enhancements	2026-01-04 19:54:30.158818+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	1133132
20251117032720	daemon-mode	2026-01-04 19:54:30.160306+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	1305256
20251118143058	set-default-plan	2026-01-04 19:54:30.161932+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1218364
20251118225043	save-topology	2026-01-04 19:54:30.163477+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	9152211
20251123232748	network-permissions	2026-01-04 19:54:30.172997+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	2798593
20251125001342	billing-updates	2026-01-04 19:54:30.176143+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	1032307
20251128035448	org-onboarding-status	2026-01-04 19:54:30.17771+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1492395
20251129180942	nfs-consolidate	2026-01-04 19:54:30.179575+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1330413
20251206052641	discovery-progress	2026-01-04 19:54:30.181204+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	1746529
20251206202200	plan-fix	2026-01-04 19:54:30.183239+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	938562
20251207061341	daemon-url	2026-01-04 19:54:30.184862+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	2868623
20251210045929	tags	2026-01-04 19:54:30.188224+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	9566339
20251210175035	terms	2026-01-04 19:54:30.198142+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	940676
20251213025048	hash-keys	2026-01-04 19:54:30.199397+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	11477605
20251214050638	scanopy	2026-01-04 19:54:30.211243+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	1544038
20251215215724	topo-scanopy-fix	2026-01-04 19:54:30.213091+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	852664
20251217153736	category rename	2026-01-04 19:54:30.214274+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	1776893
20251218053111	invite-persistence	2026-01-04 19:54:30.216356+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	5616392
20251219211216	create shares	2026-01-04 19:54:30.222315+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	7032215
20251220170928	permissions-cleanup	2026-01-04 19:54:30.22971+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1589390
20251220180000	commercial-to-community	2026-01-04 19:54:30.231626+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	960863
20251221010000	cleanup subnet type	2026-01-04 19:54:30.232894+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	1009624
20251221020000	remove host target	2026-01-04 19:54:30.234213+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	1060425
20251221030000	user network access	2026-01-04 19:54:30.235569+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	7875238
20251221040000	interfaces table	2026-01-04 19:54:30.243818+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	10043014
20251221050000	ports table	2026-01-04 19:54:30.254271+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	10373765
20251221060000	bindings table	2026-01-04 19:54:30.265347+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	12190402
20251221070000	group bindings	2026-01-04 19:54:30.27789+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	7201400
20251222020000	tag cascade delete	2026-01-04 19:54:30.285631+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1452822
20251223232524	network remove default	2026-01-04 19:54:30.287408+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	1303472
20251225100000	color enum	2026-01-04 19:54:30.289011+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1475103
20251227010000	topology snapshot migration	2026-01-04 19:54:30.290775+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	4937586
20251228010000	user api keys	2026-01-04 19:54:30.296006+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	13053693
20251230160000	daemon version and maintainer	2026-01-04 19:54:30.309851+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	3122337
20260103000000	service position	2026-01-04 19:54:30.313321+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	2115849
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, tags) FROM stdin;
8df69360-074d-473b-b944-1779ab16d697	44e92efc87ec7827ec844b42c177134977d4d9e58857f668385ddee21746292b	a051243b-9aca-436f-920e-1758547254b5	Integrated Daemon API Key	2026-01-04 19:54:32.457745+00	2026-01-04 19:55:55.645914+00	2026-01-04 19:55:55.645025+00	\N	t	{}
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
da8faef3-b5d5-44f1-ba53-e5cd3fdf8431	a051243b-9aca-436f-920e-1758547254b5	28238f9c-dc79-4133-83c3-bc8c9708b8c3	Port	ff452e13-d569-4b6f-9755-f6d6bb8d904d	e324179c-9c3e-4d88-8ae6-3f0e101beaf3	2026-01-04 19:54:32.627229+00	2026-01-04 19:54:32.627229+00
c596397d-e891-46fa-8675-7e5c15a611d3	a051243b-9aca-436f-920e-1758547254b5	ab0fa104-6658-4b3d-a961-1146f8b80846	Port	fde2270f-a12a-45c7-b54f-e879363c7161	6faf2397-9705-4097-8f14-e567e55a3372	2026-01-04 19:54:53.439595+00	2026-01-04 19:54:53.439595+00
93e926c5-cbe6-426a-9f6e-cfa151292e06	a051243b-9aca-436f-920e-1758547254b5	b4432cd4-9049-427b-aeb6-6973c6e55483	Port	88caf9e3-e04f-42e1-830e-770b71bcb7b1	916f5915-7c87-4571-a606-5ded65b300a8	2026-01-04 19:55:07.121357+00	2026-01-04 19:55:07.121357+00
bec6439c-f6f6-40f0-aafb-8b2185c103c0	a051243b-9aca-436f-920e-1758547254b5	fdc711dd-a1bb-4960-af5c-84ff3af4d74c	Port	88caf9e3-e04f-42e1-830e-770b71bcb7b1	fe72d9d0-518d-4733-a683-ed6a936d14db	2026-01-04 19:55:21.92495+00	2026-01-04 19:55:21.92495+00
f1711409-e3ed-4a0f-859d-ea02211bfb64	a051243b-9aca-436f-920e-1758547254b5	8c235806-1c56-42d7-8870-01233158b968	Port	86be787e-ee48-4b2b-bfb0-0f83ae04a318	af91f605-6a02-4ced-a018-e3448f75ee77	2026-01-04 19:55:36.719585+00	2026-01-04 19:55:36.719585+00
de82ec81-3e6e-4d78-a6cd-c67fd8357904	a051243b-9aca-436f-920e-1758547254b5	5bd1f1e9-419f-427f-8400-9ae429cab67e	Port	293118d3-1f77-41d2-a481-c8236ef115db	a0e6418e-2336-4cd1-87ac-a1ec70725cd7	2026-01-04 19:55:40.783836+00	2026-01-04 19:55:40.783836+00
9c10826c-b956-4b2b-901e-b22b65a49b2e	a051243b-9aca-436f-920e-1758547254b5	19e9fa6f-0d14-484e-b1d2-b42e1911d980	Port	293118d3-1f77-41d2-a481-c8236ef115db	9b3dfe0d-53ab-46aa-943a-85e989302904	2026-01-04 19:55:42.263183+00	2026-01-04 19:55:42.263183+00
36ab3b9b-0625-43a4-9ba0-49ea49d777cd	a051243b-9aca-436f-920e-1758547254b5	f346aa97-5ffd-43ca-8111-7cf73ecf19e4	Port	293118d3-1f77-41d2-a481-c8236ef115db	0e334292-a49a-4e06-9474-591bc839d863	2026-01-04 19:55:55.58483+00	2026-01-04 19:55:55.58483+00
b9b882f0-e1d5-4aef-8bfd-f6b63debc4bb	a051243b-9aca-436f-920e-1758547254b5	5ba36995-7a64-42a7-a23e-eff40f690d52	Port	293118d3-1f77-41d2-a481-c8236ef115db	d014527f-a9aa-4308-a9fa-fcafcab068a4	2026-01-04 19:55:55.585011+00	2026-01-04 19:55:55.585011+00
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, tags, version, user_id) FROM stdin;
b8790ab5-3919-42be-a93d-aa897e9b71d4	a051243b-9aca-436f-920e-1758547254b5	c00a6cf6-6937-4204-96de-ba16434b2852	2026-01-04 19:54:32.598041+00	2026-01-04 19:55:46.62303+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["c2949a43-46c9-4376-8091-a3a7901663ff"]}	2026-01-04 19:55:46.623716+00	"Push"	http://172.25.0.4:60073	scanopy-daemon	{}	0.13.0	b829ba99-881f-4898-9954-3529b9d06a61
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, tags) FROM stdin;
6d4358f2-e7d8-4fea-a43a-f51e1152df1d	a051243b-9aca-436f-920e-1758547254b5	b8790ab5-3919-42be-a93d-aa897e9b71d4	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852"}	Self Report	2026-01-04 19:54:32.606431+00	2026-01-04 19:54:32.606431+00	{}
2cdaaa7d-acce-4023-9e59-6a568d71cde8	a051243b-9aca-436f-920e-1758547254b5	b8790ab5-3919-42be-a93d-aa897e9b71d4	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2026-01-04 19:54:32.612138+00	2026-01-04 19:54:32.612138+00	{}
3bb79028-f3d5-4c4e-9e08-507a53665c10	a051243b-9aca-436f-920e-1758547254b5	b8790ab5-3919-42be-a93d-aa897e9b71d4	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "session_id": "24427ef0-81c7-4855-9d07-60c5d4d6ff10", "started_at": "2026-01-04T19:54:32.611715800Z", "finished_at": "2026-01-04T19:54:32.813934797Z", "discovery_type": {"type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852"}}}	{"type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852"}	Self Report	2026-01-04 19:54:32.611715+00	2026-01-04 19:54:32.819545+00	{}
d97bb82b-62e3-4f64-b6f6-39d21cf5afb3	a051243b-9aca-436f-920e-1758547254b5	b8790ab5-3919-42be-a93d-aa897e9b71d4	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "session_id": "80be9fed-3981-490a-a963-fa16a368427f", "started_at": "2026-01-04T19:54:33.179340836Z", "finished_at": "2026-01-04T19:55:55.643872411Z", "discovery_type": {"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2026-01-04 19:54:33.17934+00	2026-01-04 19:55:55.646564+00	{}
\.


--
-- Data for Name: group_bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.group_bindings (id, group_id, binding_id, "position", created_at) FROM stdin;
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, network_id, name, description, created_at, updated_at, source, color, edge_style, tags, group_type) FROM stdin;
a5596caf-96fe-4160-aad2-2798b97901c1	a051243b-9aca-436f-920e-1758547254b5		\N	2026-01-04 19:55:55.657911+00	2026-01-04 19:55:55.657911+00	{"type": "Manual"}	Yellow	"SmoothStep"	{}	RequestPath
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, tags) FROM stdin;
c00a6cf6-6937-4204-96de-ba16434b2852	a051243b-9aca-436f-920e-1758547254b5	scanopy-daemon	328ec9cadeb7	\N	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:32.627183176Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}	null	2026-01-04 19:54:32.591353+00	2026-01-04 19:54:32.638523+00	f	{}
b65d2f56-3021-4dec-bd51-c4966620d087	a051243b-9aca-436f-920e-1758547254b5	scanopy-server-1.scanopy_scanopy-dev	scanopy-server-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:51.890746226Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2026-01-04 19:54:51.890747+00	2026-01-04 19:54:51.890747+00	f	{}
160908a3-90ed-4191-96fc-0b6cd5cf6941	a051243b-9aca-436f-920e-1758547254b5	homeassistant-discovery.scanopy_scanopy-dev	homeassistant-discovery.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:07.120874334Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2026-01-04 19:55:07.120875+00	2026-01-04 19:55:07.120875+00	f	{}
2f566fa6-d669-462d-9cf9-df928022f3e9	a051243b-9aca-436f-920e-1758547254b5	scanopy-postgres-dev-1.scanopy_scanopy-dev	scanopy-postgres-dev-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:21.940425908Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2026-01-04 19:55:21.940426+00	2026-01-04 19:55:21.940426+00	f	{}
18d03e66-4f30-4874-8171-7b78b6eb2648	a051243b-9aca-436f-920e-1758547254b5	runnervmh13bl	runnervmh13bl	\N	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:40.783365733Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2026-01-04 19:55:40.783366+00	2026-01-04 19:55:40.783366+00	f	{}
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at) FROM stdin;
ff452e13-d569-4b6f-9755-f6d6bb8d904d	a051243b-9aca-436f-920e-1758547254b5	c00a6cf6-6937-4204-96de-ba16434b2852	c2949a43-46c9-4376-8091-a3a7901663ff	172.25.0.4	86:80:26:30:ec:dc	eth0	0	2026-01-04 19:54:32.611906+00	2026-01-04 19:54:32.611906+00
fde2270f-a12a-45c7-b54f-e879363c7161	a051243b-9aca-436f-920e-1758547254b5	b65d2f56-3021-4dec-bd51-c4966620d087	c2949a43-46c9-4376-8091-a3a7901663ff	172.25.0.3	d2:d8:09:d0:7b:45	\N	0	2026-01-04 19:54:51.890724+00	2026-01-04 19:54:51.890724+00
88caf9e3-e04f-42e1-830e-770b71bcb7b1	a051243b-9aca-436f-920e-1758547254b5	160908a3-90ed-4191-96fc-0b6cd5cf6941	c2949a43-46c9-4376-8091-a3a7901663ff	172.25.0.5	fe:ca:7a:7e:55:37	\N	0	2026-01-04 19:55:07.120846+00	2026-01-04 19:55:07.120846+00
86be787e-ee48-4b2b-bfb0-0f83ae04a318	a051243b-9aca-436f-920e-1758547254b5	2f566fa6-d669-462d-9cf9-df928022f3e9	c2949a43-46c9-4376-8091-a3a7901663ff	172.25.0.6	aa:76:11:ee:0b:80	\N	0	2026-01-04 19:55:21.940405+00	2026-01-04 19:55:21.940405+00
293118d3-1f77-41d2-a481-c8236ef115db	a051243b-9aca-436f-920e-1758547254b5	18d03e66-4f30-4874-8171-7b78b6eb2648	c2949a43-46c9-4376-8091-a3a7901663ff	172.25.0.1	b2:0a:66:e9:4a:24	\N	0	2026-01-04 19:55:40.783339+00	2026-01-04 19:55:40.783339+00
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, organization_id, tags) FROM stdin;
a051243b-9aca-436f-920e-1758547254b5	My Network	2026-01-04 19:54:32.441419+00	2026-01-04 19:54:32.441419+00	0ab14575-7a61-4f1d-a286-c72a2ab199ab	{}
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding) FROM stdin;
0ab14575-7a61-4f1d-a286-c72a2ab199ab	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "trial_days": 0}	active	2026-01-04 19:54:32.434959+00	2026-01-04 19:55:56.474518+00	["OnboardingModalCompleted", "FirstDaemonRegistered", "FirstApiKeyCreated"]
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
e324179c-9c3e-4d88-8ae6-3f0e101beaf3	a051243b-9aca-436f-920e-1758547254b5	c00a6cf6-6937-4204-96de-ba16434b2852	60073	Tcp	Custom	2026-01-04 19:54:32.627012+00	2026-01-04 19:54:32.627012+00
6faf2397-9705-4097-8f14-e567e55a3372	a051243b-9aca-436f-920e-1758547254b5	b65d2f56-3021-4dec-bd51-c4966620d087	60072	Tcp	Custom	2026-01-04 19:54:53.439583+00	2026-01-04 19:54:53.439583+00
916f5915-7c87-4571-a606-5ded65b300a8	a051243b-9aca-436f-920e-1758547254b5	160908a3-90ed-4191-96fc-0b6cd5cf6941	8123	Tcp	Custom	2026-01-04 19:55:07.12135+00	2026-01-04 19:55:07.12135+00
fe72d9d0-518d-4733-a683-ed6a936d14db	a051243b-9aca-436f-920e-1758547254b5	160908a3-90ed-4191-96fc-0b6cd5cf6941	18555	Tcp	Custom	2026-01-04 19:55:21.92494+00	2026-01-04 19:55:21.92494+00
af91f605-6a02-4ced-a018-e3448f75ee77	a051243b-9aca-436f-920e-1758547254b5	2f566fa6-d669-462d-9cf9-df928022f3e9	5432	Tcp	PostgreSQL	2026-01-04 19:55:36.719576+00	2026-01-04 19:55:36.719576+00
a0e6418e-2336-4cd1-87ac-a1ec70725cd7	a051243b-9aca-436f-920e-1758547254b5	18d03e66-4f30-4874-8171-7b78b6eb2648	8123	Tcp	Custom	2026-01-04 19:55:40.783828+00	2026-01-04 19:55:40.783828+00
9b3dfe0d-53ab-46aa-943a-85e989302904	a051243b-9aca-436f-920e-1758547254b5	18d03e66-4f30-4874-8171-7b78b6eb2648	60072	Tcp	Custom	2026-01-04 19:55:42.263172+00	2026-01-04 19:55:42.263172+00
0e334292-a49a-4e06-9474-591bc839d863	a051243b-9aca-436f-920e-1758547254b5	18d03e66-4f30-4874-8171-7b78b6eb2648	22	Tcp	Ssh	2026-01-04 19:55:55.584821+00	2026-01-04 19:55:55.584821+00
d014527f-a9aa-4308-a9fa-fcafcab068a4	a051243b-9aca-436f-920e-1758547254b5	18d03e66-4f30-4874-8171-7b78b6eb2648	5435	Tcp	Custom	2026-01-04 19:55:55.585007+00	2026-01-04 19:55:55.585007+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, tags, "position") FROM stdin;
28238f9c-dc79-4133-83c3-bc8c9708b8c3	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:54:32.627234+00	2026-01-04 19:54:32.627234+00	Scanopy Daemon	c00a6cf6-6937-4204-96de-ba16434b2852	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-01-04T19:54:32.627233370Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}	{}	0
ab0fa104-6658-4b3d-a961-1146f8b80846	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:54:53.439598+00	2026-01-04 19:54:53.439598+00	Scanopy Server	b65d2f56-3021-4dec-bd51-c4966620d087	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:54:53.439577336Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	0
b4432cd4-9049-427b-aeb6-6973c6e55483	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:07.121361+00	2026-01-04 19:55:07.121361+00	Home Assistant	160908a3-90ed-4191-96fc-0b6cd5cf6941	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:07.121345313Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	0
fdc711dd-a1bb-4960-af5c-84ff3af4d74c	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:21.924956+00	2026-01-04 19:55:21.924956+00	Unclaimed Open Ports	160908a3-90ed-4191-96fc-0b6cd5cf6941	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:21.924935688Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	1
8c235806-1c56-42d7-8870-01233158b968	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:36.719589+00	2026-01-04 19:55:36.719589+00	PostgreSQL	2f566fa6-d669-462d-9cf9-df928022f3e9	"PostgreSQL"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:36.719573124Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	0
5bd1f1e9-419f-427f-8400-9ae429cab67e	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:40.783838+00	2026-01-04 19:55:40.783838+00	Home Assistant	18d03e66-4f30-4874-8171-7b78b6eb2648	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:40.783823969Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	0
19e9fa6f-0d14-484e-b1d2-b42e1911d980	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:42.263201+00	2026-01-04 19:55:42.263201+00	Scanopy Server	18d03e66-4f30-4874-8171-7b78b6eb2648	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:42.263166499Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	1
f346aa97-5ffd-43ca-8111-7cf73ecf19e4	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:55.584834+00	2026-01-04 19:55:55.584834+00	SSH	18d03e66-4f30-4874-8171-7b78b6eb2648	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:55.584816857Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	2
5ba36995-7a64-42a7-a23e-eff40f690d52	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:55:55.585014+00	2026-01-04 19:55:55.585014+00	Unclaimed Open Ports	18d03e66-4f30-4874-8171-7b78b6eb2648	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:55.585005549Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}	3
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, network_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, network_id, created_at, updated_at, cidr, name, description, subnet_type, source, tags) FROM stdin;
1fc10ddb-4137-4b78-b153-c635e265e81a	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:54:32.442928+00	2026-01-04 19:54:32.442928+00	"0.0.0.0/0"	Internet	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).	Internet	{"type": "System"}	{}
adbc61ec-fb82-492c-8bf1-54de20c2bacf	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:54:32.442932+00	2026-01-04 19:54:32.442932+00	"0.0.0.0/0"	Remote Network	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).	Remote	{"type": "System"}	{}
c2949a43-46c9-4376-8091-a3a7901663ff	a051243b-9aca-436f-920e-1758547254b5	2026-01-04 19:54:32.611883+00	2026-01-04 19:54:32.611883+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:32.611881750Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}	{}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
5ddba8f5-6c4f-421e-b8fb-e5e275bef00c	0ab14575-7a61-4f1d-a286-c72a2ab199ab	New Tag	\N	2026-01-04 19:55:55.665106+00	2026-01-04 19:55:55.665106+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings) FROM stdin;
6633f648-02cb-4417-bd64-ea4f78c80e4f	a051243b-9aca-436f-920e-1758547254b5	My Topology	[]	[]	{"local": {"no_fade_edges": false, "hide_edge_types": [], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": [], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[{"id": "c00a6cf6-6937-4204-96de-ba16434b2852", "name": "scanopy-daemon", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:32.627183176Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}, "hostname": "328ec9cadeb7", "created_at": "2026-01-04T19:54:32.591353Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:32.638523Z", "description": null, "virtualization": null}, {"id": "b65d2f56-3021-4dec-bd51-c4966620d087", "name": "scanopy-server-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:51.890746226Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-server-1.scanopy_scanopy-dev", "created_at": "2026-01-04T19:54:51.890747Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:51.890747Z", "description": null, "virtualization": null}, {"id": "160908a3-90ed-4191-96fc-0b6cd5cf6941", "name": "homeassistant-discovery.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:07.120874334Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "homeassistant-discovery.scanopy_scanopy-dev", "created_at": "2026-01-04T19:55:07.120875Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:07.120875Z", "description": null, "virtualization": null}, {"id": "2f566fa6-d669-462d-9cf9-df928022f3e9", "name": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:21.940425908Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "created_at": "2026-01-04T19:55:21.940426Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:21.940426Z", "description": null, "virtualization": null}, {"id": "18d03e66-4f30-4874-8171-7b78b6eb2648", "name": "runnervmh13bl", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:55:40.783365733Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "runnervmh13bl", "created_at": "2026-01-04T19:55:40.783366Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:40.783366Z", "description": null, "virtualization": null}, {"id": "7853a020-4b16-4bef-a862-a354a9ee98ea", "name": "Service Test Host", "tags": [], "hidden": false, "source": {"type": "Manual"}, "hostname": "service-test.local", "created_at": "2026-01-04T19:55:56.349347Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:56.349347Z", "description": null, "virtualization": null}]	[{"id": "1fc10ddb-4137-4b78-b153-c635e265e81a", "cidr": "0.0.0.0/0", "name": "Internet", "tags": [], "source": {"type": "System"}, "created_at": "2026-01-04T19:54:32.442928Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:32.442928Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).", "subnet_type": "Internet"}, {"id": "adbc61ec-fb82-492c-8bf1-54de20c2bacf", "cidr": "0.0.0.0/0", "name": "Remote Network", "tags": [], "source": {"type": "System"}, "created_at": "2026-01-04T19:54:32.442932Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:32.442932Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).", "subnet_type": "Remote"}, {"id": "c2949a43-46c9-4376-8091-a3a7901663ff", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-01-04T19:54:32.611881750Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}, "created_at": "2026-01-04T19:54:32.611883Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:32.611883Z", "description": null, "subnet_type": "Lan"}]	[{"id": "28238f9c-dc79-4133-83c3-bc8c9708b8c3", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-01-04T19:54:32.627233370Z", "type": "SelfReport", "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4"}]}, "host_id": "c00a6cf6-6937-4204-96de-ba16434b2852", "bindings": [{"id": "da8faef3-b5d5-44f1-ba53-e5cd3fdf8431", "type": "Port", "port_id": "e324179c-9c3e-4d88-8ae6-3f0e101beaf3", "created_at": "2026-01-04T19:54:32.627229Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "28238f9c-dc79-4133-83c3-bc8c9708b8c3", "updated_at": "2026-01-04T19:54:32.627229Z", "interface_id": "ff452e13-d569-4b6f-9755-f6d6bb8d904d"}], "position": 0, "created_at": "2026-01-04T19:54:32.627234Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:32.627234Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "ab0fa104-6658-4b3d-a961-1146f8b80846", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:54:53.439577336Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "b65d2f56-3021-4dec-bd51-c4966620d087", "bindings": [{"id": "c596397d-e891-46fa-8675-7e5c15a611d3", "type": "Port", "port_id": "6faf2397-9705-4097-8f14-e567e55a3372", "created_at": "2026-01-04T19:54:53.439595Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "ab0fa104-6658-4b3d-a961-1146f8b80846", "updated_at": "2026-01-04T19:54:53.439595Z", "interface_id": "fde2270f-a12a-45c7-b54f-e879363c7161"}], "position": 0, "created_at": "2026-01-04T19:54:53.439598Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:54:53.439598Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "b4432cd4-9049-427b-aeb6-6973c6e55483", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:07.121345313Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "160908a3-90ed-4191-96fc-0b6cd5cf6941", "bindings": [{"id": "93e926c5-cbe6-426a-9f6e-cfa151292e06", "type": "Port", "port_id": "916f5915-7c87-4571-a606-5ded65b300a8", "created_at": "2026-01-04T19:55:07.121357Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "b4432cd4-9049-427b-aeb6-6973c6e55483", "updated_at": "2026-01-04T19:55:07.121357Z", "interface_id": "88caf9e3-e04f-42e1-830e-770b71bcb7b1"}], "position": 0, "created_at": "2026-01-04T19:55:07.121361Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:07.121361Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "fdc711dd-a1bb-4960-af5c-84ff3af4d74c", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:21.924935688Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "160908a3-90ed-4191-96fc-0b6cd5cf6941", "bindings": [{"id": "bec6439c-f6f6-40f0-aafb-8b2185c103c0", "type": "Port", "port_id": "fe72d9d0-518d-4733-a683-ed6a936d14db", "created_at": "2026-01-04T19:55:21.924950Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "fdc711dd-a1bb-4960-af5c-84ff3af4d74c", "updated_at": "2026-01-04T19:55:21.924950Z", "interface_id": "88caf9e3-e04f-42e1-830e-770b71bcb7b1"}], "position": 1, "created_at": "2026-01-04T19:55:21.924956Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:21.924956Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "8c235806-1c56-42d7-8870-01233158b968", "name": "PostgreSQL", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:36.719573124Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "2f566fa6-d669-462d-9cf9-df928022f3e9", "bindings": [{"id": "f1711409-e3ed-4a0f-859d-ea02211bfb64", "type": "Port", "port_id": "af91f605-6a02-4ced-a018-e3448f75ee77", "created_at": "2026-01-04T19:55:36.719585Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "8c235806-1c56-42d7-8870-01233158b968", "updated_at": "2026-01-04T19:55:36.719585Z", "interface_id": "86be787e-ee48-4b2b-bfb0-0f83ae04a318"}], "position": 0, "created_at": "2026-01-04T19:55:36.719589Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:36.719589Z", "virtualization": null, "service_definition": "PostgreSQL"}, {"id": "5bd1f1e9-419f-427f-8400-9ae429cab67e", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:40.783823969Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "18d03e66-4f30-4874-8171-7b78b6eb2648", "bindings": [{"id": "de82ec81-3e6e-4d78-a6cd-c67fd8357904", "type": "Port", "port_id": "a0e6418e-2336-4cd1-87ac-a1ec70725cd7", "created_at": "2026-01-04T19:55:40.783836Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "5bd1f1e9-419f-427f-8400-9ae429cab67e", "updated_at": "2026-01-04T19:55:40.783836Z", "interface_id": "293118d3-1f77-41d2-a481-c8236ef115db"}], "position": 0, "created_at": "2026-01-04T19:55:40.783838Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:40.783838Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "19e9fa6f-0d14-484e-b1d2-b42e1911d980", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-01-04T19:55:42.263166499Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "18d03e66-4f30-4874-8171-7b78b6eb2648", "bindings": [{"id": "9c10826c-b956-4b2b-901e-b22b65a49b2e", "type": "Port", "port_id": "9b3dfe0d-53ab-46aa-943a-85e989302904", "created_at": "2026-01-04T19:55:42.263183Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "19e9fa6f-0d14-484e-b1d2-b42e1911d980", "updated_at": "2026-01-04T19:55:42.263183Z", "interface_id": "293118d3-1f77-41d2-a481-c8236ef115db"}], "position": 1, "created_at": "2026-01-04T19:55:42.263201Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:42.263201Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "f346aa97-5ffd-43ca-8111-7cf73ecf19e4", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:55.584816857Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "18d03e66-4f30-4874-8171-7b78b6eb2648", "bindings": [{"id": "36ab3b9b-0625-43a4-9ba0-49ea49d777cd", "type": "Port", "port_id": "0e334292-a49a-4e06-9474-591bc839d863", "created_at": "2026-01-04T19:55:55.584830Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "f346aa97-5ffd-43ca-8111-7cf73ecf19e4", "updated_at": "2026-01-04T19:55:55.584830Z", "interface_id": "293118d3-1f77-41d2-a481-c8236ef115db"}], "position": 2, "created_at": "2026-01-04T19:55:55.584834Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:55.584834Z", "virtualization": null, "service_definition": "SSH"}, {"id": "5ba36995-7a64-42a7-a23e-eff40f690d52", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-01-04T19:55:55.585005549Z", "type": "Network", "daemon_id": "b8790ab5-3919-42be-a93d-aa897e9b71d4", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "18d03e66-4f30-4874-8171-7b78b6eb2648", "bindings": [{"id": "b9b882f0-e1d5-4aef-8bfd-f6b63debc4bb", "type": "Port", "port_id": "d014527f-a9aa-4308-a9fa-fcafcab068a4", "created_at": "2026-01-04T19:55:55.585011Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "service_id": "5ba36995-7a64-42a7-a23e-eff40f690d52", "updated_at": "2026-01-04T19:55:55.585011Z", "interface_id": "293118d3-1f77-41d2-a481-c8236ef115db"}], "position": 3, "created_at": "2026-01-04T19:55:55.585014Z", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:55.585014Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}]	[{"id": "a5596caf-96fe-4160-aad2-2798b97901c1", "name": "", "tags": [], "color": "Yellow", "source": {"type": "Manual"}, "created_at": "2026-01-04T19:55:55.657911Z", "edge_style": "SmoothStep", "group_type": "RequestPath", "network_id": "a051243b-9aca-436f-920e-1758547254b5", "updated_at": "2026-01-04T19:55:55.657911Z", "binding_ids": [], "description": null}]	t	2026-01-04 19:54:32.455838+00	f	\N	\N	{e210f788-a65a-4681-90bd-2f2e16c3ad30,7853a020-4b16-4bef-a862-a354a9ee98ea,5e12e549-d3de-4c0b-bc9f-47daf8fefaf9}	{99762f0a-1e7e-44e4-9a84-79384a08d742}	{4c993bfd-f652-4286-aa83-b8aa9d414b4a}	{798126dd-82bc-40e0-b8d7-7fbfaefb5aa5}	\N	2026-01-04 19:54:32.447446+00	2026-01-04 19:55:57.805849+00	{}	[]	{}	[]	{}	[]	{}
\.


--
-- Data for Name: user_api_key_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_key_network_access (id, api_key_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled, tags) FROM stdin;
\.


--
-- Data for Name: user_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_network_access (id, user_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at) FROM stdin;
b829ba99-881f-4898-9954-3529b9d06a61	2026-01-04 19:54:32.438142+00	2026-01-04 19:54:32.438142+00	$argon2id$v=19$m=19456,t=2,p=1$XhCES22+41o75n3lDUccXg$7Q24XsgXc2SSo0aQxcfqLxk5W6mDIebQctSivdofh4M	\N	\N	\N	user@gmail.com	0ab14575-7a61-4f1d-a286-c72a2ab199ab	Owner	{}	\N
ffe054b8-c14d-4e22-b9c3-9fb8c6c920af	2026-01-04 19:55:56.965678+00	2026-01-04 19:55:56.965678+00	\N	\N	\N	\N	user@example.com	0ab14575-7a61-4f1d-a286-c72a2ab199ab	Owner	{}	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
exwgwpayuCTVLkS7OqEiuA	\\x93c410b822a13abb442ed524b8b296c2201c7b81a7757365725f6964d92462383239626139392d383831662d343839382d393935342d33353239623964303661363199cd07ea0b133620ce251c8899000000	2026-01-11 19:54:32.622626+00
eiE5FMtoOmDRxtxP9AOwiQ	\\x93c41089b003f44fdcc6d1603a68cb1439217a82ad70656e64696e675f736574757082a86e6574776f726b739182a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92434346465623235662d646336392d343065332d393230652d666338643735343237666365a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92462383239626139392d383831662d343839382d393935342d33353239623964303661363199cd07ea0b133738ce0eeef05e000000	2026-01-11 19:55:56.25054+00
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
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


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
-- Name: idx_users_oidc_provider_subject; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_oidc_provider_subject ON public.users USING btree (oidc_provider, oidc_subject) WHERE ((oidc_provider IS NOT NULL) AND (oidc_subject IS NOT NULL));


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: tags trigger_remove_deleted_tag_from_entities; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER trigger_remove_deleted_tag_from_entities BEFORE DELETE ON public.tags FOR EACH ROW EXECUTE FUNCTION public.remove_deleted_tag_from_entities();


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

\unrestrict LkFuFmhspVOcbW2PhUcYnskZxOo4o5iJw1GRHNg6723lQZxG3Kbll4psRsZVbzl

