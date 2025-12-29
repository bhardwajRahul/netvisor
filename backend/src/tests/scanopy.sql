--
-- PostgreSQL database dump
--

\restrict tucYbLsBawnolD7dcrncudhWsVdrWOWUK8NhJY6ttSxIQ54IulfWRu4ly6y1hzm

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
DROP INDEX IF EXISTS public.idx_topologies_network;
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name;
DROP INDEX IF EXISTS public.idx_subnets_network;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_network;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_network;
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
20251006215000	users	2025-12-29 02:22:53.817054+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	3487427
20251006215100	networks	2025-12-29 02:22:53.821489+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	5057937
20251006215151	create hosts	2025-12-29 02:22:53.826875+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	3954098
20251006215155	create subnets	2025-12-29 02:22:53.831217+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	3765426
20251006215201	create groups	2025-12-29 02:22:53.835561+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	4276710
20251006215204	create daemons	2025-12-29 02:22:53.840153+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	4305183
20251006215212	create services	2025-12-29 02:22:53.845529+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	5152493
20251029193448	user-auth	2025-12-29 02:22:53.851066+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	9126959
20251030044828	daemon api	2025-12-29 02:22:53.860543+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1917548
20251030170438	host-hide	2025-12-29 02:22:53.862748+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1483581
20251102224919	create discovery	2025-12-29 02:22:53.864548+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	11839438
20251106235621	normalize-daemon-cols	2025-12-29 02:22:53.876809+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	1976520
20251107034459	api keys	2025-12-29 02:22:53.879218+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	10088510
20251107222650	oidc-auth	2025-12-29 02:22:53.889638+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	34018439
20251110181948	orgs-billing	2025-12-29 02:22:53.924232+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	12446412
20251113223656	group-enhancements	2025-12-29 02:22:53.93711+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	1162359
20251117032720	daemon-mode	2025-12-29 02:22:53.938596+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	1240294
20251118143058	set-default-plan	2025-12-29 02:22:53.940151+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1360719
20251118225043	save-topology	2025-12-29 02:22:53.941907+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	9839087
20251123232748	network-permissions	2025-12-29 02:22:53.952081+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	2883840
20251125001342	billing-updates	2025-12-29 02:22:53.955313+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	905389
20251128035448	org-onboarding-status	2025-12-29 02:22:53.95659+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1553038
20251129180942	nfs-consolidate	2025-12-29 02:22:53.95851+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1233943
20251206052641	discovery-progress	2025-12-29 02:22:53.962036+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	1958144
20251206202200	plan-fix	2025-12-29 02:22:53.964371+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	962386
20251207061341	daemon-url	2025-12-29 02:22:53.96571+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	2372006
20251210045929	tags	2025-12-29 02:22:53.968419+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	9003676
20251210175035	terms	2025-12-29 02:22:53.977874+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	872157
20251213025048	hash-keys	2025-12-29 02:22:53.979436+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	12128160
20251214050638	scanopy	2025-12-29 02:22:53.992208+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	1390223
20251215215724	topo-scanopy-fix	2025-12-29 02:22:53.994202+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	963247
20251217153736	category rename	2025-12-29 02:22:53.995553+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	1649538
20251218053111	invite-persistence	2025-12-29 02:22:53.997779+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	5122077
20251219211216	create shares	2025-12-29 02:22:54.003346+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	7153916
20251220170928	permissions-cleanup	2025-12-29 02:22:54.010905+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1583825
20251220180000	commercial-to-community	2025-12-29 02:22:54.012861+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	897365
20251221010000	cleanup subnet type	2025-12-29 02:22:54.014216+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	1004394
20251221020000	remove host target	2025-12-29 02:22:54.015584+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	935062
20251221030000	user network access	2025-12-29 02:22:54.016918+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	7333532
20251221040000	interfaces table	2025-12-29 02:22:54.024626+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	9391733
20251221050000	ports table	2025-12-29 02:22:54.034396+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	8768971
20251221060000	bindings table	2025-12-29 02:22:54.043584+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	10402939
20251221070000	group bindings	2025-12-29 02:22:54.054399+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	6428574
20251222020000	tag cascade delete	2025-12-29 02:22:54.061334+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1189299
20251223232524	network remove default	2025-12-29 02:22:54.0629+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	1055529
20251225100000	color enum	2025-12-29 02:22:54.064287+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1353535
20251227010000	topology snapshot migration	2025-12-29 02:22:54.065939+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	4682326
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, tags) FROM stdin;
44a7c442-c684-4ae0-ad4a-214f018fd135	2f49102deeffd3433df66e3e8ac17a46941d3a9271a59ad170632051dda08f9c	22796e5c-793a-4ef0-a263-df4386a6a5ad	Integrated Daemon API Key	2025-12-29 02:22:56.751777+00	2025-12-29 02:24:20.403622+00	2025-12-29 02:24:20.402621+00	\N	t	{}
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
d0dd8326-daed-42e6-8535-20d8867d339a	22796e5c-793a-4ef0-a263-df4386a6a5ad	cd4344ba-2556-48b8-917e-d32dcfc7f50e	Port	1fd819f2-7b71-4649-98bb-5b18e2729a01	8a57d47d-446c-4184-865d-4304cdf71862	2025-12-29 02:22:56.951714+00	2025-12-29 02:22:56.951714+00
aa001b63-b418-456e-8089-0be39898e426	22796e5c-793a-4ef0-a263-df4386a6a5ad	a61b9c3e-d1c2-4cdb-8ff2-f6c08490e228	Port	f771a90f-71dd-43ab-acef-d6eafa2cc63d	b7258fd5-bf55-44bd-9e6c-0d9239a8b485	2025-12-29 02:23:28.651456+00	2025-12-29 02:23:28.651456+00
c73a085b-ee77-4dd4-81f4-4c621d86f089	22796e5c-793a-4ef0-a263-df4386a6a5ad	0bc840de-2b91-48b3-930b-11c870cedbda	Port	15449c7e-d0fc-4b88-a77c-55e17a2ce423	eff6c400-488d-43fb-bc64-29d22eab59de	2025-12-29 02:23:45.012027+00	2025-12-29 02:23:45.012027+00
e58fefd8-95bb-4caf-a01e-037b866062ce	22796e5c-793a-4ef0-a263-df4386a6a5ad	1dfd2be6-d13e-46b0-bd02-f95c1ac6be5b	Port	540a52bb-80c2-4107-9cc9-facecac5dc28	9a94d981-6ef8-41c4-b8af-4be88c049cb7	2025-12-29 02:23:57.600507+00	2025-12-29 02:23:57.600507+00
17b95657-b791-449d-b79c-76bef8e2553a	22796e5c-793a-4ef0-a263-df4386a6a5ad	6604a25b-e412-4c34-ab45-65689266467d	Port	540a52bb-80c2-4107-9cc9-facecac5dc28	71f2ebc4-878f-4348-9570-1ad118f60a6b	2025-12-29 02:23:59.761593+00	2025-12-29 02:23:59.761593+00
5bb4195e-0e9e-4797-9e45-a4aeb44a24c6	22796e5c-793a-4ef0-a263-df4386a6a5ad	54341761-b932-4b30-9ef3-bec61e4ce65d	Port	2e0da3d5-7d88-4bcc-a968-fece98e488f0	7550e06b-8121-47ec-9d84-5160a98ec773	2025-12-29 02:24:18.173184+00	2025-12-29 02:24:18.173184+00
c63de402-6154-4c77-b7b1-16bff5c249f4	22796e5c-793a-4ef0-a263-df4386a6a5ad	48e75be3-4135-4b76-8994-7f6b61d16428	Port	2e0da3d5-7d88-4bcc-a968-fece98e488f0	e9000b03-dda1-4b14-a008-eec8bb237617	2025-12-29 02:24:18.889561+00	2025-12-29 02:24:18.889561+00
516a37b7-6316-4321-9227-6b905e6058f6	22796e5c-793a-4ef0-a263-df4386a6a5ad	220fdc4f-a063-45b6-ba84-fb054755fe0e	Port	2e0da3d5-7d88-4bcc-a968-fece98e488f0	e3b59a52-6d94-4d03-827c-3219afc3895f	2025-12-29 02:24:20.338754+00	2025-12-29 02:24:20.338754+00
fa640584-f9e3-4b63-8ce7-fd3da9f92f13	22796e5c-793a-4ef0-a263-df4386a6a5ad	d54abc41-c1cf-418f-a720-86f0c7d6466b	Port	2e0da3d5-7d88-4bcc-a968-fece98e488f0	5958ca39-be37-4ac6-972e-e1fa02d7aa26	2025-12-29 02:24:20.339145+00	2025-12-29 02:24:20.339145+00
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, tags) FROM stdin;
5b304c70-3863-4fc6-bb61-17ed83bb5465	22796e5c-793a-4ef0-a263-df4386a6a5ad	a21a23f8-faa7-4ab5-a3a3-75803aec24d3	2025-12-29 02:22:56.879457+00	2025-12-29 02:24:13.1377+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["5e1f25c0-9558-4960-ad83-146e599f29ec"]}	2025-12-29 02:24:13.138758+00	"Push"	http://172.25.0.4:60073	scanopy-daemon	{}
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, tags) FROM stdin;
67d94d3b-4c9c-488c-bd0f-2cb9e1e01ca2	22796e5c-793a-4ef0-a263-df4386a6a5ad	5b304c70-3863-4fc6-bb61-17ed83bb5465	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3"}	Self Report	2025-12-29 02:22:56.887296+00	2025-12-29 02:22:56.887296+00	{}
70895787-7a78-40a9-b101-28e22b7f0736	22796e5c-793a-4ef0-a263-df4386a6a5ad	5b304c70-3863-4fc6-bb61-17ed83bb5465	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2025-12-29 02:22:56.894102+00	2025-12-29 02:22:56.894102+00	{}
c7efe3a6-e3cb-4800-a086-306a1735ca01	22796e5c-793a-4ef0-a263-df4386a6a5ad	5b304c70-3863-4fc6-bb61-17ed83bb5465	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "session_id": "b81ecbda-5222-412d-b856-97a517fc370a", "started_at": "2025-12-29T02:22:56.893728733Z", "finished_at": "2025-12-29T02:22:57.031085503Z", "discovery_type": {"type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3"}}}	{"type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3"}	Self Report	2025-12-29 02:22:56.893728+00	2025-12-29 02:22:57.034023+00	{}
c6392535-713a-4c6e-99c2-e9f3f5c11950	22796e5c-793a-4ef0-a263-df4386a6a5ad	5b304c70-3863-4fc6-bb61-17ed83bb5465	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "session_id": "da9cd0b8-a3ca-4790-b26f-2610208a8130", "started_at": "2025-12-29T02:22:57.043737496Z", "finished_at": "2025-12-29T02:24:20.400559781Z", "discovery_type": {"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2025-12-29 02:22:57.043737+00	2025-12-29 02:24:20.40295+00	{}
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
ccab338d-55b0-46ea-ba41-a46065a204d4	22796e5c-793a-4ef0-a263-df4386a6a5ad		\N	2025-12-29 02:24:20.419726+00	2025-12-29 02:24:20.419726+00	{"type": "Manual"}	Yellow	"SmoothStep"	{}	RequestPath
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, tags) FROM stdin;
a21a23f8-faa7-4ab5-a3a3-75803aec24d3	22796e5c-793a-4ef0-a263-df4386a6a5ad	scanopy-daemon	7efd33b8bbcc	\N	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:22:56.951687727Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}	null	2025-12-29 02:22:56.839513+00	2025-12-29 02:22:57.008612+00	f	{}
3a6e6f9c-a655-45b4-8c5b-2b1623a19814	22796e5c-793a-4ef0-a263-df4386a6a5ad	scanopy-server-1.scanopy_scanopy-dev	scanopy-server-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:15.331493974Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-29 02:23:15.331495+00	2025-12-29 02:23:15.331495+00	f	{}
63b6d3b1-92b7-4804-9c9c-30d496761ee5	22796e5c-793a-4ef0-a263-df4386a6a5ad	scanopy-postgres-dev-1.scanopy_scanopy-dev	scanopy-postgres-dev-1.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:30.240950802Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-29 02:23:30.240951+00	2025-12-29 02:23:30.240951+00	f	{}
fa720e2b-68bc-4c98-8f86-0fb4534b9fed	22796e5c-793a-4ef0-a263-df4386a6a5ad	homeassistant-discovery.scanopy_scanopy-dev	homeassistant-discovery.scanopy_scanopy-dev	\N	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:45.069328982Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-29 02:23:45.06933+00	2025-12-29 02:23:45.06933+00	f	{}
b991cb46-f547-4281-b08d-3cf2da1632b8	22796e5c-793a-4ef0-a263-df4386a6a5ad	runnervmh13bl	runnervmh13bl	\N	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:24:05.835975236Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-29 02:24:05.835976+00	2025-12-29 02:24:05.835976+00	f	{}
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at) FROM stdin;
1fd819f2-7b71-4649-98bb-5b18e2729a01	22796e5c-793a-4ef0-a263-df4386a6a5ad	a21a23f8-faa7-4ab5-a3a3-75803aec24d3	5e1f25c0-9558-4960-ad83-146e599f29ec	172.25.0.4	6e:8f:1f:2e:8c:40	eth0	0	2025-12-29 02:22:56.893896+00	2025-12-29 02:22:56.893896+00
f771a90f-71dd-43ab-acef-d6eafa2cc63d	22796e5c-793a-4ef0-a263-df4386a6a5ad	3a6e6f9c-a655-45b4-8c5b-2b1623a19814	5e1f25c0-9558-4960-ad83-146e599f29ec	172.25.0.3	2e:0e:e1:5f:36:00	\N	0	2025-12-29 02:23:15.331472+00	2025-12-29 02:23:15.331472+00
15449c7e-d0fc-4b88-a77c-55e17a2ce423	22796e5c-793a-4ef0-a263-df4386a6a5ad	63b6d3b1-92b7-4804-9c9c-30d496761ee5	5e1f25c0-9558-4960-ad83-146e599f29ec	172.25.0.6	fe:f7:a3:7b:72:12	\N	0	2025-12-29 02:23:30.240929+00	2025-12-29 02:23:30.240929+00
540a52bb-80c2-4107-9cc9-facecac5dc28	22796e5c-793a-4ef0-a263-df4386a6a5ad	fa720e2b-68bc-4c98-8f86-0fb4534b9fed	5e1f25c0-9558-4960-ad83-146e599f29ec	172.25.0.5	7a:18:e6:9e:9d:24	\N	0	2025-12-29 02:23:45.069307+00	2025-12-29 02:23:45.069307+00
2e0da3d5-7d88-4bcc-a968-fece98e488f0	22796e5c-793a-4ef0-a263-df4386a6a5ad	b991cb46-f547-4281-b08d-3cf2da1632b8	5e1f25c0-9558-4960-ad83-146e599f29ec	172.25.0.1	66:19:75:9e:f0:3d	\N	0	2025-12-29 02:24:05.83592+00	2025-12-29 02:24:05.83592+00
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
22796e5c-793a-4ef0-a263-df4386a6a5ad	My Network	2025-12-29 02:22:56.735649+00	2025-12-29 02:22:56.735649+00	58f593f6-4f62-4169-9374-131dbb7662e4	{}
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding) FROM stdin;
58f593f6-4f62-4169-9374-131dbb7662e4	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "trial_days": 0}	active	2025-12-29 02:22:56.729046+00	2025-12-29 02:24:21.300802+00	["OnboardingModalCompleted", "FirstDaemonRegistered", "FirstApiKeyCreated"]
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
8a57d47d-446c-4184-865d-4304cdf71862	22796e5c-793a-4ef0-a263-df4386a6a5ad	a21a23f8-faa7-4ab5-a3a3-75803aec24d3	60073	Tcp	Custom	2025-12-29 02:22:56.951491+00	2025-12-29 02:22:56.951491+00
b7258fd5-bf55-44bd-9e6c-0d9239a8b485	22796e5c-793a-4ef0-a263-df4386a6a5ad	3a6e6f9c-a655-45b4-8c5b-2b1623a19814	60072	Tcp	Custom	2025-12-29 02:23:28.651445+00	2025-12-29 02:23:28.651445+00
eff6c400-488d-43fb-bc64-29d22eab59de	22796e5c-793a-4ef0-a263-df4386a6a5ad	63b6d3b1-92b7-4804-9c9c-30d496761ee5	5432	Tcp	PostgreSQL	2025-12-29 02:23:45.012017+00	2025-12-29 02:23:45.012017+00
9a94d981-6ef8-41c4-b8af-4be88c049cb7	22796e5c-793a-4ef0-a263-df4386a6a5ad	fa720e2b-68bc-4c98-8f86-0fb4534b9fed	8123	Tcp	Custom	2025-12-29 02:23:57.600497+00	2025-12-29 02:23:57.600497+00
71f2ebc4-878f-4348-9570-1ad118f60a6b	22796e5c-793a-4ef0-a263-df4386a6a5ad	fa720e2b-68bc-4c98-8f86-0fb4534b9fed	18555	Tcp	Custom	2025-12-29 02:23:59.761583+00	2025-12-29 02:23:59.761583+00
7550e06b-8121-47ec-9d84-5160a98ec773	22796e5c-793a-4ef0-a263-df4386a6a5ad	b991cb46-f547-4281-b08d-3cf2da1632b8	8123	Tcp	Custom	2025-12-29 02:24:18.173174+00	2025-12-29 02:24:18.173174+00
e9000b03-dda1-4b14-a008-eec8bb237617	22796e5c-793a-4ef0-a263-df4386a6a5ad	b991cb46-f547-4281-b08d-3cf2da1632b8	60072	Tcp	Custom	2025-12-29 02:24:18.889551+00	2025-12-29 02:24:18.889551+00
e3b59a52-6d94-4d03-827c-3219afc3895f	22796e5c-793a-4ef0-a263-df4386a6a5ad	b991cb46-f547-4281-b08d-3cf2da1632b8	22	Tcp	Ssh	2025-12-29 02:24:20.338744+00	2025-12-29 02:24:20.338744+00
5958ca39-be37-4ac6-972e-e1fa02d7aa26	22796e5c-793a-4ef0-a263-df4386a6a5ad	b991cb46-f547-4281-b08d-3cf2da1632b8	5435	Tcp	Custom	2025-12-29 02:24:20.339141+00	2025-12-29 02:24:20.339141+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, tags) FROM stdin;
cd4344ba-2556-48b8-917e-d32dcfc7f50e	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:22:56.951718+00	2025-12-29 02:22:56.951718+00	Scanopy Daemon	a21a23f8-faa7-4ab5-a3a3-75803aec24d3	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-29T02:22:56.951717233Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}	{}
a61b9c3e-d1c2-4cdb-8ff2-f6c08490e228	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:23:28.65146+00	2025-12-29 02:23:28.65146+00	Scanopy Server	3a6e6f9c-a655-45b4-8c5b-2b1623a19814	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:23:28.651439726Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
0bc840de-2b91-48b3-930b-11c870cedbda	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:23:45.012031+00	2025-12-29 02:23:45.012031+00	PostgreSQL	63b6d3b1-92b7-4804-9c9c-30d496761ee5	"PostgreSQL"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:23:45.012011386Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
1dfd2be6-d13e-46b0-bd02-f95c1ac6be5b	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:23:57.600511+00	2025-12-29 02:23:57.600511+00	Home Assistant	fa720e2b-68bc-4c98-8f86-0fb4534b9fed	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:23:57.600490758Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
6604a25b-e412-4c34-ab45-65689266467d	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:23:59.761597+00	2025-12-29 02:23:59.761597+00	Unclaimed Open Ports	fa720e2b-68bc-4c98-8f86-0fb4534b9fed	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:23:59.761578740Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
54341761-b932-4b30-9ef3-bec61e4ce65d	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:24:18.173188+00	2025-12-29 02:24:18.173188+00	Home Assistant	b991cb46-f547-4281-b08d-3cf2da1632b8	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:24:18.173168054Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
48e75be3-4135-4b76-8994-7f6b61d16428	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:24:18.889564+00	2025-12-29 02:24:18.889564+00	Scanopy Server	b991cb46-f547-4281-b08d-3cf2da1632b8	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:24:18.889545384Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
220fdc4f-a063-45b6-ba84-fb054755fe0e	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:24:20.338758+00	2025-12-29 02:24:20.338758+00	SSH	b991cb46-f547-4281-b08d-3cf2da1632b8	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:24:20.338739584Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
d54abc41-c1cf-418f-a720-86f0c7d6466b	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:24:20.339148+00	2025-12-29 02:24:20.339148+00	Unclaimed Open Ports	b991cb46-f547-4281-b08d-3cf2da1632b8	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:24:20.339138738Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
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
1690f4e9-d414-4685-88e8-ece6dc257c73	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:22:56.737197+00	2025-12-29 02:22:56.737197+00	"0.0.0.0/0"	Internet	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).	Internet	{"type": "System"}	{}
e8b1a257-25bf-4a2b-a173-4f4a5634a02f	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:22:56.7372+00	2025-12-29 02:22:56.7372+00	"0.0.0.0/0"	Remote Network	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).	Remote	{"type": "System"}	{}
5e1f25c0-9558-4960-ad83-146e599f29ec	22796e5c-793a-4ef0-a263-df4386a6a5ad	2025-12-29 02:22:56.893875+00	2025-12-29 02:22:56.893875+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2025-12-29T02:22:56.893873763Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}	{}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
4e77d76c-fcf2-4986-ac4f-5d2e8cb4ec6c	58f593f6-4f62-4169-9374-131dbb7662e4	New Tag	\N	2025-12-29 02:24:20.43071+00	2025-12-29 02:24:20.43071+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings) FROM stdin;
223678f9-9790-4b06-9117-15bf3edf3e33	22796e5c-793a-4ef0-a263-df4386a6a5ad	My Topology	[]	[]	{"local": {"no_fade_edges": false, "hide_edge_types": [], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": [], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[{"id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "name": "scanopy-daemon", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:22:56.951687727Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}, "hostname": "7efd33b8bbcc", "created_at": "2025-12-29T02:22:56.839513Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:22:57.008612Z", "description": null, "virtualization": null}, {"id": "3a6e6f9c-a655-45b4-8c5b-2b1623a19814", "name": "scanopy-server-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:15.331493974Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-server-1.scanopy_scanopy-dev", "created_at": "2025-12-29T02:23:15.331495Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:15.331495Z", "description": null, "virtualization": null}, {"id": "63b6d3b1-92b7-4804-9c9c-30d496761ee5", "name": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:30.240950802Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "created_at": "2025-12-29T02:23:30.240951Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:30.240951Z", "description": null, "virtualization": null}, {"id": "fa720e2b-68bc-4c98-8f86-0fb4534b9fed", "name": "homeassistant-discovery.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:23:45.069328982Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "homeassistant-discovery.scanopy_scanopy-dev", "created_at": "2025-12-29T02:23:45.069330Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:45.069330Z", "description": null, "virtualization": null}, {"id": "b991cb46-f547-4281-b08d-3cf2da1632b8", "name": "runnervmh13bl", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:24:05.835975236Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "hostname": "runnervmh13bl", "created_at": "2025-12-29T02:24:05.835976Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:05.835976Z", "description": null, "virtualization": null}, {"id": "b7f39a78-8e57-47d7-9f55-ea401bcee22e", "name": "Updated Host", "tags": [], "hidden": false, "source": {"type": "Manual"}, "hostname": "test.local", "created_at": "2025-12-29T02:24:21.095864Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:21.113361Z", "description": null, "virtualization": null}]	[{"id": "1690f4e9-d414-4685-88e8-ece6dc257c73", "cidr": "0.0.0.0/0", "name": "Internet", "tags": [], "source": {"type": "System"}, "created_at": "2025-12-29T02:22:56.737197Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:22:56.737197Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).", "subnet_type": "Internet"}, {"id": "e8b1a257-25bf-4a2b-a173-4f4a5634a02f", "cidr": "0.0.0.0/0", "name": "Remote Network", "tags": [], "source": {"type": "System"}, "created_at": "2025-12-29T02:22:56.737200Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:22:56.737200Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).", "subnet_type": "Remote"}, {"id": "5e1f25c0-9558-4960-ad83-146e599f29ec", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2025-12-29T02:22:56.893873763Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}, "created_at": "2025-12-29T02:22:56.893875Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:22:56.893875Z", "description": null, "subnet_type": "Lan"}]	[{"id": "cd4344ba-2556-48b8-917e-d32dcfc7f50e", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-29T02:22:56.951717233Z", "type": "SelfReport", "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465"}]}, "host_id": "a21a23f8-faa7-4ab5-a3a3-75803aec24d3", "bindings": [{"id": "d0dd8326-daed-42e6-8535-20d8867d339a", "type": "Port", "port_id": "8a57d47d-446c-4184-865d-4304cdf71862", "created_at": "2025-12-29T02:22:56.951714Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "cd4344ba-2556-48b8-917e-d32dcfc7f50e", "updated_at": "2025-12-29T02:22:56.951714Z", "interface_id": "1fd819f2-7b71-4649-98bb-5b18e2729a01"}], "created_at": "2025-12-29T02:22:56.951718Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:22:56.951718Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "a61b9c3e-d1c2-4cdb-8ff2-f6c08490e228", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:23:28.651439726Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "3a6e6f9c-a655-45b4-8c5b-2b1623a19814", "bindings": [{"id": "aa001b63-b418-456e-8089-0be39898e426", "type": "Port", "port_id": "b7258fd5-bf55-44bd-9e6c-0d9239a8b485", "created_at": "2025-12-29T02:23:28.651456Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "a61b9c3e-d1c2-4cdb-8ff2-f6c08490e228", "updated_at": "2025-12-29T02:23:28.651456Z", "interface_id": "f771a90f-71dd-43ab-acef-d6eafa2cc63d"}], "created_at": "2025-12-29T02:23:28.651460Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:28.651460Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "0bc840de-2b91-48b3-930b-11c870cedbda", "name": "PostgreSQL", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:23:45.012011386Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "63b6d3b1-92b7-4804-9c9c-30d496761ee5", "bindings": [{"id": "c73a085b-ee77-4dd4-81f4-4c621d86f089", "type": "Port", "port_id": "eff6c400-488d-43fb-bc64-29d22eab59de", "created_at": "2025-12-29T02:23:45.012027Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "0bc840de-2b91-48b3-930b-11c870cedbda", "updated_at": "2025-12-29T02:23:45.012027Z", "interface_id": "15449c7e-d0fc-4b88-a77c-55e17a2ce423"}], "created_at": "2025-12-29T02:23:45.012031Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:45.012031Z", "virtualization": null, "service_definition": "PostgreSQL"}, {"id": "1dfd2be6-d13e-46b0-bd02-f95c1ac6be5b", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:23:57.600490758Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "fa720e2b-68bc-4c98-8f86-0fb4534b9fed", "bindings": [{"id": "e58fefd8-95bb-4caf-a01e-037b866062ce", "type": "Port", "port_id": "9a94d981-6ef8-41c4-b8af-4be88c049cb7", "created_at": "2025-12-29T02:23:57.600507Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "1dfd2be6-d13e-46b0-bd02-f95c1ac6be5b", "updated_at": "2025-12-29T02:23:57.600507Z", "interface_id": "540a52bb-80c2-4107-9cc9-facecac5dc28"}], "created_at": "2025-12-29T02:23:57.600511Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:57.600511Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "6604a25b-e412-4c34-ab45-65689266467d", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:23:59.761578740Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "fa720e2b-68bc-4c98-8f86-0fb4534b9fed", "bindings": [{"id": "17b95657-b791-449d-b79c-76bef8e2553a", "type": "Port", "port_id": "71f2ebc4-878f-4348-9570-1ad118f60a6b", "created_at": "2025-12-29T02:23:59.761593Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "6604a25b-e412-4c34-ab45-65689266467d", "updated_at": "2025-12-29T02:23:59.761593Z", "interface_id": "540a52bb-80c2-4107-9cc9-facecac5dc28"}], "created_at": "2025-12-29T02:23:59.761597Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:23:59.761597Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "54341761-b932-4b30-9ef3-bec61e4ce65d", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:24:18.173168054Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "b991cb46-f547-4281-b08d-3cf2da1632b8", "bindings": [{"id": "5bb4195e-0e9e-4797-9e45-a4aeb44a24c6", "type": "Port", "port_id": "7550e06b-8121-47ec-9d84-5160a98ec773", "created_at": "2025-12-29T02:24:18.173184Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "54341761-b932-4b30-9ef3-bec61e4ce65d", "updated_at": "2025-12-29T02:24:18.173184Z", "interface_id": "2e0da3d5-7d88-4bcc-a968-fece98e488f0"}], "created_at": "2025-12-29T02:24:18.173188Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:18.173188Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "48e75be3-4135-4b76-8994-7f6b61d16428", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-29T02:24:18.889545384Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "b991cb46-f547-4281-b08d-3cf2da1632b8", "bindings": [{"id": "c63de402-6154-4c77-b7b1-16bff5c249f4", "type": "Port", "port_id": "e9000b03-dda1-4b14-a008-eec8bb237617", "created_at": "2025-12-29T02:24:18.889561Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "48e75be3-4135-4b76-8994-7f6b61d16428", "updated_at": "2025-12-29T02:24:18.889561Z", "interface_id": "2e0da3d5-7d88-4bcc-a968-fece98e488f0"}], "created_at": "2025-12-29T02:24:18.889564Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:18.889564Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "220fdc4f-a063-45b6-ba84-fb054755fe0e", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:24:20.338739584Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "b991cb46-f547-4281-b08d-3cf2da1632b8", "bindings": [{"id": "516a37b7-6316-4321-9227-6b905e6058f6", "type": "Port", "port_id": "e3b59a52-6d94-4d03-827c-3219afc3895f", "created_at": "2025-12-29T02:24:20.338754Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "220fdc4f-a063-45b6-ba84-fb054755fe0e", "updated_at": "2025-12-29T02:24:20.338754Z", "interface_id": "2e0da3d5-7d88-4bcc-a968-fece98e488f0"}], "created_at": "2025-12-29T02:24:20.338758Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:20.338758Z", "virtualization": null, "service_definition": "SSH"}, {"id": "d54abc41-c1cf-418f-a720-86f0c7d6466b", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-29T02:24:20.339138738Z", "type": "Network", "daemon_id": "5b304c70-3863-4fc6-bb61-17ed83bb5465", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "b991cb46-f547-4281-b08d-3cf2da1632b8", "bindings": [{"id": "fa640584-f9e3-4b63-8ce7-fd3da9f92f13", "type": "Port", "port_id": "5958ca39-be37-4ac6-972e-e1fa02d7aa26", "created_at": "2025-12-29T02:24:20.339145Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "service_id": "d54abc41-c1cf-418f-a720-86f0c7d6466b", "updated_at": "2025-12-29T02:24:20.339145Z", "interface_id": "2e0da3d5-7d88-4bcc-a968-fece98e488f0"}], "created_at": "2025-12-29T02:24:20.339148Z", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:20.339148Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}]	[{"id": "ccab338d-55b0-46ea-ba41-a46065a204d4", "name": "", "tags": [], "color": "Yellow", "source": {"type": "Manual"}, "created_at": "2025-12-29T02:24:20.419726Z", "edge_style": "SmoothStep", "group_type": "RequestPath", "network_id": "22796e5c-793a-4ef0-a263-df4386a6a5ad", "updated_at": "2025-12-29T02:24:20.419726Z", "binding_ids": [], "description": null}]	t	2025-12-29 02:22:56.749688+00	f	\N	\N	{b7f39a78-8e57-47d7-9f55-ea401bcee22e,34dcbda1-a558-4b1d-9a9c-b299729e329e,561703eb-f60b-4b04-ad29-b6fd48c3fc4e}	{e5258e25-32a7-41de-8f10-a9a379ea1cd2}	{a0e70fcd-178d-486f-899d-544324f6b32e}	{2d76c75d-0b9e-4bdd-868a-cc30d90850d8}	\N	2025-12-29 02:22:56.741449+00	2025-12-29 02:24:22.16284+00	{}	[]	{}	[]	{}	[]	{}
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
488e0467-b8a7-42da-a29d-84a31f4a8533	2025-12-29 02:22:56.732252+00	2025-12-29 02:22:56.732252+00	$argon2id$v=19$m=19456,t=2,p=1$ICUwJGPr8kWUUgOcoM495Q$1XbUE51apQbQU6eton06znFFCC2dpILqTxAcKuhDOFs	\N	\N	\N	user@gmail.com	58f593f6-4f62-4169-9374-131dbb7662e4	Owner	{}	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
deM6SZ0XnpQzztZ0iJjBsw	\\x93c410b3c1988874d6ce33949e179d493ae37581a7757365725f6964d92434383865303436372d623861372d343264612d613239642d38346133316634613835333399cd07ea1c021638ce35c46963000000	2026-01-28 02:22:56.902064+00
-YmkHwjv6jbfiJze8nUw2Q	\\x93c410d93075f2de9c88df36eaef081fa489f982ad70656e64696e675f736574757082a86e6574776f726b739182a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92461393337646338622d636466622d343662312d623531302d306561383861343330623366a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92434383865303436372d623861372d343264612d613239642d38346133316634613835333399cd07ea1c021815ce026f8408000000	2026-01-28 02:24:21.040862+00
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

\unrestrict tucYbLsBawnolD7dcrncudhWsVdrWOWUK8NhJY6ttSxIQ54IulfWRu4ly6y1hzm

