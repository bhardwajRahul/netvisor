--
-- PostgreSQL database dump
--

\restrict kFpRM1MezonXcz3PhyuKraJZqwHFWuf1XsXKDVEuLEDERdcPHFW8JqxaBdEzCua

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
-- Name: groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.groups (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    group_type jsonb NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
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
    target jsonb NOT NULL,
    interfaces jsonb,
    services uuid[],
    ports jsonb,
    source jsonb NOT NULL,
    virtualization jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    hidden boolean DEFAULT false,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.hosts OWNER TO postgres;

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
    is_default boolean NOT NULL,
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
-- Name: services; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.services (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    name text NOT NULL,
    host_id uuid NOT NULL,
    bindings jsonb,
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
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.topologies OWNER TO postgres;

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
    network_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL,
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
20251006215000	users	2025-12-27 16:12:19.777543+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	3430792
20251006215100	networks	2025-12-27 16:12:19.78175+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	4399125
20251006215151	create hosts	2025-12-27 16:12:19.786578+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	2845708
20251006215155	create subnets	2025-12-27 16:12:19.789922+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	2973709
20251006215201	create groups	2025-12-27 16:12:19.793443+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	2554208
20251006215204	create daemons	2025-12-27 16:12:19.796321+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	2731417
20251006215212	create services	2025-12-27 16:12:19.799435+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	2777208
20251029193448	user-auth	2025-12-27 16:12:19.802521+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	3999458
20251030044828	daemon api	2025-12-27 16:12:19.806842+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1088750
20251030170438	host-hide	2025-12-27 16:12:19.808214+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	953333
20251102224919	create discovery	2025-12-27 16:12:19.80946+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	4241000
20251106235621	normalize-daemon-cols	2025-12-27 16:12:19.813984+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	1130625
20251107034459	api keys	2025-12-27 16:12:19.815427+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	4103708
20251107222650	oidc-auth	2025-12-27 16:12:19.819821+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	9707792
20251110181948	orgs-billing	2025-12-27 16:12:19.829796+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	3858416
20251113223656	group-enhancements	2025-12-27 16:12:19.833896+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	778083
20251117032720	daemon-mode	2025-12-27 16:12:19.834922+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	757000
20251118143058	set-default-plan	2025-12-27 16:12:19.836015+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	790209
20251118225043	save-topology	2025-12-27 16:12:19.837032+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	3355417
20251123232748	network-permissions	2025-12-27 16:12:19.840637+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	1671084
20251125001342	billing-updates	2025-12-27 16:12:19.842567+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	708375
20251128035448	org-onboarding-status	2025-12-27 16:12:19.843544+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	867375
20251129180942	nfs-consolidate	2025-12-27 16:12:19.844656+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	770875
20251206052641	discovery-progress	2025-12-27 16:12:19.845597+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	753417
20251206202200	plan-fix	2025-12-27 16:12:19.846546+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	623375
20251207061341	daemon-url	2025-12-27 16:12:19.847379+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	910666
20251210045929	tags	2025-12-27 16:12:19.848504+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	2671833
20251210175035	terms	2025-12-27 16:12:19.851411+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	597750
20251213025048	hash-keys	2025-12-27 16:12:19.852175+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	4637791
20251214050638	scanopy	2025-12-27 16:12:19.857028+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	775458
20251215215724	topo-scanopy-fix	2025-12-27 16:12:19.858005+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	600541
20251217153736	category rename	2025-12-27 16:12:19.858824+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	807750
20251218053111	invite-persistence	2025-12-27 16:12:19.859858+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	1712291
20251219211216	create shares	2025-12-27 16:12:19.861782+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	2561459
20251220170928	permissions-cleanup	2025-12-27 16:12:19.864521+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	736208
20251220180000	commercial-to-community	2025-12-27 16:12:19.865467+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	555750
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, tags) FROM stdin;
30e7135a-711f-487f-8e8c-97c40b148427	a8d2a26e90e98c2ed1bbe7b65699b4056327aab717ef8e47295e96499690ce22	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	daemon Api Key	2025-12-27 16:13:30.361764+00	2025-12-27 16:28:27.062937+00	2025-12-27 16:28:27.061248+00	\N	t	{}
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, tags) FROM stdin;
7dc85a03-c94f-42d9-b52f-13808fbe88d7	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	2025-12-27 16:13:56.976322+00	2025-12-27 16:28:27.063494+00	{"has_docker_socket": true, "interfaced_subnet_ids": ["e5b4263e-1964-439d-bcb8-2cbb11018742", "f1a114ad-b7c5-4146-8975-f772be3986d9"]}	2025-12-27 16:28:27.064623+00	"Push"	http://192.168.4.200:60073	test	{}
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, tags) FROM stdin;
b565398e-6f4b-4d87-93e1-0609ea93b818	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb"}	Self Report	2025-12-27 16:13:56.981339+00	2025-12-27 16:13:56.981339+00	{}
509c948a-de66-4b2a-ac48-64d151162406	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "host_naming_fallback": "BestService"}	Docker Discovery	2025-12-27 16:13:56.988231+00	2025-12-27 16:13:56.988231+00	{}
6cb1de33-7951-4146-a2ae-fc801f221508	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Scheduled", "enabled": true, "last_run": null, "cron_schedule": "0 0 0 * * *"}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2025-12-27 16:13:56.992212+00	2025-12-27 16:13:56.992212+00	{}
3f31db51-1bf7-4b3c-99e4-00783ff26a72	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "session_id": "0dd904f2-e5ad-467a-98db-19148cbc5b6b", "started_at": "2025-12-27T16:13:56.988074Z", "finished_at": "2025-12-27T16:13:57.018164Z", "discovery_type": {"type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb"}}}	{"type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb"}	Self Report	2025-12-27 16:13:56.988074+00	2025-12-27 16:13:57.020102+00	{}
9dc000ec-5b82-4c32-b93a-638f7baa9406	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "session_id": "15ebbff1-1a1a-48fd-8b7d-a6b86e3ab2eb", "started_at": "2025-12-27T16:13:57.057547Z", "finished_at": "2025-12-27T16:14:38.365761Z", "discovery_type": {"type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "host_naming_fallback": "BestService"}}}	{"type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "host_naming_fallback": "BestService"}	Docker Discovery	2025-12-27 16:13:57.057547+00	2025-12-27 16:14:38.368777+00	{}
8f634a5c-c192-4025-978e-6fb02fff99e0	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	7dc85a03-c94f-42d9-b52f-13808fbe88d7	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "session_id": "a1d125cd-77fc-4d64-aa1d-74516574ff36", "started_at": "2025-12-27T16:14:38.384145Z", "finished_at": "2025-12-27T16:28:16.352482Z", "discovery_type": {"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}}}	{"type": "Network", "subnet_ids": null, "host_naming_fallback": "BestService"}	Network Discovery	2025-12-27 16:14:38.384145+00	2025-12-27 16:28:16.35504+00	{}
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, network_id, name, description, group_type, created_at, updated_at, source, color, edge_style, tags) FROM stdin;
b6672bce-d110-4655-a025-fda8b5a9128a	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	test	\N	{"group_type": "RequestPath", "service_bindings": ["fe3f4f51-8635-4d04-8663-5fa20e96e577", "c58fee2e-df2d-4d04-b1b0-7dd9fc373dc1"]}	2025-12-27 16:21:26.476537+00	2025-12-27 16:21:26.476537+00	{"type": "Manual"}	rose	"Straight"	{}
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, target, interfaces, services, ports, source, virtualization, created_at, updated_at, hidden, tags) FROM stdin;
309cef94-06eb-4d7b-9b25-90a81789a0e1	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	ha.maya.cloud	ha.maya.cloud	\N	{"type": "Hostname"}	[{"id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.187", "mac_address": "BC:24:11:67:26:8E"}]	{69d48dbe-840f-40ce-93c9-3ef2b3bcf37d,6c5fc429-09fd-4841-b1f9-6b46597e8080,22e8c40e-f48b-47f9-9066-9c70bcfcd20e}	[{"id": "32b4b60c-6d8b-4306-af78-bb2dbd932047", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "66b4e5ef-a232-4842-83d5-544746185fc8", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "cf9afd3c-3d81-44a1-93dc-d01afacc8557", "type": "Custom", "number": 81, "protocol": "Tcp"}, {"id": "9e5d060f-6180-4019-8566-78bae229e8ad", "type": "Https", "number": 443, "protocol": "Tcp"}, {"id": "b1e709b9-2d28-41b6-ace0-7696f23806df", "type": "Http3000", "number": 3000, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:31.108404Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:31.108406+00	2025-12-27 16:26:37.953237+00	f	{}
560d8a7f-0c0a-478a-b16e-abe06611e797	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Mobile Device	\N	A mobile device connecting from a remote network	{"type": "ServiceBinding", "config": "994c1eec-0798-40cc-b1b3-fa3d1f88cc47"}	[{"id": "74464d04-e4d4-4cdc-8671-0e34423a7eec", "name": "Remote Network", "subnet_id": "ca1bf14c-9af9-4a9f-a9b3-c29f0e798066", "ip_address": "203.0.113.140", "mac_address": null}]	{586a6eda-e792-44f5-ad58-a96ad5b1ee06}	[{"id": "256ba520-397d-4837-85d0-8775b759944f", "type": "Custom", "number": 0, "protocol": "Tcp"}]	{"type": "System"}	null	2025-12-27 16:13:24.582412+00	2025-12-27 16:13:24.600389+00	f	{}
fd85da79-5f19-43ba-95bf-b3752c4ba9b5	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Cloudflare DNS		\N	{"type": "ServiceBinding", "config": "fe3f4f51-8635-4d04-8663-5fa20e96e577"}	[{"id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0", "name": "Internet", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "ip_address": "1.1.1.1", "mac_address": null}]	{dcf72989-2e9a-483a-82dd-54cf055fccc5}	[{"id": "1c115fc4-91b2-40c3-9a58-5ccbb70b44fe", "type": "DnsUdp", "number": 53, "protocol": "Udp"}]	{"type": "System"}	null	2025-12-27 16:13:24.5824+00	2025-12-27 16:21:40.454347+00	f	{00390626-53aa-4851-bde1-b7d5cbfdbf38}
6baa326f-7ab8-4d56-a689-9c51c6258b32	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Google.com		\N	{"type": "ServiceBinding", "config": "327558cf-6a01-46ac-85f7-40d84c8bf85f"}	[{"id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d", "name": "Internet", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "ip_address": "203.0.113.68", "mac_address": null}]	{9774e951-1df1-4abe-8094-c45095da8138}	[{"id": "7a76431c-b494-4ba4-93b9-6c8ffadfc053", "type": "Https", "number": 443, "protocol": "Tcp"}]	{"type": "System"}	null	2025-12-27 16:13:24.582406+00	2025-12-27 16:21:45.013347+00	f	{00390626-53aa-4851-bde1-b7d5cbfdbf38}
eb13ca22-e581-4a72-b188-fb80b1b4cac4	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	pi.hole	pi.hole	\N	{"type": "Hostname"}	[{"id": "894ae3a5-689f-4776-82d0-5661bd4074ad", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.188", "mac_address": "BC:24:11:72:2E:BC"}]	{a164ce69-5325-4bff-a24b-7b563005c37f,a1b0a520-e7a4-4baa-9614-5b837149c8f8,00b51667-6e22-42fc-a985-0cdd30567eea}	[{"id": "3570d7d6-bfc8-4cb7-bc41-9e55eecdf955", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "63c0a278-5537-4a71-80b2-3cdf716dad50", "type": "DnsTcp", "number": 53, "protocol": "Tcp"}, {"id": "6843343a-7702-4b6f-b079-6753e6e102de", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "51bc1341-266c-4225-9c35-0410d4bccb28", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "ef4e6114-60fa-43be-89d6-6ed213968f6d", "type": "Https", "number": 443, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:16.771837Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:16.771841+00	2025-12-27 16:26:23.519131+00	f	{}
4777c89f-fe55-4baf-9d17-2d6f70b696f0	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Eero Repeater	\N	\N	{"type": "None"}	[{"id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.214", "mac_address": "C4:F1:74:1E:56:32"}]	{fddb2f60-840b-44e5-a179-cb74633ebef3,bfbbb2bf-cf59-45b7-9339-582cec92860c,a3a68848-b0a5-4c8d-babc-8eae60c5db55,cd30c4e4-7163-4219-8a75-11127599fff3}	[{"id": "c613e0dc-e4f9-4bc7-a871-439070402d3f", "type": "Ntp", "number": 123, "protocol": "Udp"}, {"id": "2736a1b4-6d45-46a4-879f-46f735ad431b", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "902191b5-4c46-422e-b1f0-b21a9771c888", "type": "DnsTcp", "number": 53, "protocol": "Tcp"}, {"id": "5e5a370f-8ea8-462e-b0bf-3e8f03889578", "type": "Custom", "number": 3001, "protocol": "Tcp"}, {"id": "88f8765b-7701-4b2a-a904-5408d75df0e0", "type": "Custom", "number": 10101, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:23.541144Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:23.541145+00	2025-12-27 16:26:30.269159+00	f	{}
5c882c83-72f7-495e-8bdf-71413bd445c3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.4.196	\N	\N	{"type": "None"}	[{"id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.196", "mac_address": "BC:24:11:AD:B1:1E"}]	{aaaa46d2-f808-480d-a636-b1e03560c5b9,a6f5d5ef-f914-4d12-8da9-756edf4247a7}	[{"id": "2cd178ba-1657-49c7-b0a8-b236c0e40541", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "735e18b4-6780-4a74-abc7-ecc781fdbb11", "type": "Custom", "number": 8200, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:37.942294Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:37.942295+00	2025-12-27 16:26:51.945318+00	f	{}
69dca40d-98a8-424b-8469-2aa9f2ce71fe	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.7.76	\N	\N	{"type": "None"}	[{"id": "506701c8-52da-4fdf-b0e8-e657ed303188", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.7.76", "mac_address": "E6:B2:67:46:36:1D"}]	{}	[]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:17:30.300285Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:17:30.300287+00	2025-12-27 16:17:37.664438+00	f	{}
1effd89a-40ab-4a44-862b-c3a14db9074c	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Proxmox VE	\N	\N	{"type": "None"}	[{"id": "2089b1d5-759f-4fae-b47b-6707ddf3669e", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.135", "mac_address": "E8:FF:1E:D0:CE:A6"}]	{d311be56-1dd0-40c6-b7c7-0bee84cc1bde,e25cba30-c570-462e-a535-1281d3e7317f,50e66c44-301d-4331-9030-47ff6e746a00}	[{"id": "0a395d11-abb5-40e7-9940-e0df455bfa1b", "type": "Custom", "number": 8006, "protocol": "Tcp"}, {"id": "d7c195bf-5726-4423-9d3c-6bd45bf89ca7", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "87217851-d7f2-4ac8-aa19-ed2f081ff75e", "type": "Custom", "number": 3128, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:28:09.277990Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:28:09.277991+00	2025-12-27 16:28:16.348497+00	f	{}
94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	10.0.0.1	\N	\N	{"type": "None"}	[{"id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad", "name": null, "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "ip_address": "10.0.0.1", "mac_address": null}]	{b7a8bbea-2ca2-455b-aaf4-e15a394a8cdf,8d506cbb-44bf-4262-834f-dd3c55a8c29d}	[{"id": "7b5bded1-334c-4ce9-a218-f2d2b63fe5c7", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "71a0544f-1eb1-4ff5-aeb1-7ddf421457a7", "type": "Custom", "number": 10086, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:17:00.962110Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:17:00.962112+00	2025-12-27 16:17:00.979976+00	f	{}
cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	test	Mayas-MacBook-Pro.local	Scanopy daemon	{"type": "None"}	[{"id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83", "name": "en0", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.200", "mac_address": "34:66:91:EC:6C:2F"}, {"id": "952ba3b9-96fa-4d38-8866-887522dda68e", "name": "utun4", "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "ip_address": "10.0.0.3", "mac_address": null}, {"id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e", "name": "bridge", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "ip_address": "172.17.0.2", "mac_address": "EA:6C:7E:C5:B2:47"}, {"id": "4c298ae7-726e-4b7d-b514-1f26c513579e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.4", "mac_address": "F2:F6:AB:B3:8A:F6"}, {"id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.3", "mac_address": "F2:B4:BF:E9:C8:BF"}, {"id": "d8195316-15bc-4420-8d48-95d7a03f8d9e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.2", "mac_address": "A6:49:53:47:E7:4C"}]	{14c87e66-ce12-4aed-a634-b5d344d73f01,b219d4a5-bdfe-44b0-8852-443daff2e5dc,f9006309-f2f7-475b-aa03-aea6200af41a,564eb7bc-2f24-4c05-afc1-530a8a58f155,f953905a-26cd-4978-b271-49d584248ec3,51c913fb-31e0-4576-b06d-0a3e48967c9c,744bac31-2e5a-492b-abb1-c45dd70f5d02,6e50e416-eca4-4bd9-8289-7bfe23192563,c905eb81-ce8a-468a-8bb6-69a02c33be59,da2fc8f8-5351-4242-95ee-46bece8a0e71,c35744b5-8620-46da-87ef-617f309ac242}	[{"id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "type": "Custom", "number": 60073, "protocol": "Tcp"}, {"id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "type": "PostgreSQL", "number": 5432, "protocol": "Tcp"}, {"id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "type": "Http9000", "number": 9000, "protocol": "Tcp"}, {"id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "type": "Https9443", "number": 9443, "protocol": "Tcp"}, {"id": "67db1b3d-93ee-48cf-9e2a-a98c11f75243", "type": "Custom", "number": 60072, "protocol": "Tcp"}, {"id": "68ecbd7d-b49e-4483-bf59-b476420f54c9", "type": "Http3000", "number": 3000, "protocol": "Tcp"}, {"id": "f4c56874-7052-4df7-b99c-c9c260e6438e", "type": "Http5000", "number": 5000, "protocol": "Tcp"}, {"id": "a469779d-c6d7-4d37-85fc-8f5556b092aa", "type": "Custom", "number": 5173, "protocol": "Tcp"}, {"id": "62d74abe-bc84-46fd-8583-9ed50fbf2f92", "type": "Custom", "number": 7000, "protocol": "Tcp"}, {"id": "fa0529f1-1bb3-41eb-ab82-8316b794ead7", "type": "Http8080", "number": 8080, "protocol": "Tcp"}, {"id": "ca723695-40c3-4e8f-89c6-e3dc4d5e0d5c", "type": "Custom", "number": 56820, "protocol": "Tcp"}, {"id": "25da3a86-a1c7-48f4-8fb0-6817b55a95ed", "type": "Custom", "number": 59869, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:18:19.221518Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:31.382521Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:24.125843Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:06.649164Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:59.543099Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.074377Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.006894Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}	null	2025-12-27 16:13:56.969502+00	2025-12-27 16:18:19.328606+00	f	{}
66bb7da2-71db-441b-b8e6-ee60eabc4a59	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Portainer	\N	\N	{"type": "None"}	[{"id": "18dabc8d-7d81-432e-9489-06939088f005", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.126", "mac_address": "02:1C:65:32:E5:DD"}]	{0fab6211-dc92-40e2-b7a2-c26135feead3,591ac6bf-8b30-4d21-9546-5a7972201c2b,3f12d1f2-a049-438c-9fe3-1eb6116d68f2}	[{"id": "94015005-8c6e-454b-bcf4-f2b491163271", "type": "Http9000", "number": 9000, "protocol": "Tcp"}, {"id": "7110e884-7fe5-4fac-814a-c7172c0fd254", "type": "Custom", "number": 60073, "protocol": "Tcp"}, {"id": "961c3c56-ee02-460f-98c7-0637608c2f06", "type": "Custom", "number": 5355, "protocol": "Tcp"}, {"id": "59a3577d-1dbd-47ec-a46d-1e334bb1f6cf", "type": "Custom", "number": 8000, "protocol": "Tcp"}, {"id": "310ecfea-5467-4b09-8b32-6848a2315c4e", "type": "Https9443", "number": 9443, "protocol": "Tcp"}, {"id": "92706bde-ad98-46f2-ac52-9c0e8c597cf6", "type": "Custom", "number": 60072, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:44.857162Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:44.857162+00	2025-12-27 16:26:51.944832+00	f	{}
2493a39c-903a-47b1-8aff-3ae3ad9356fe	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	WGDashboard	\N	\N	{"type": "None"}	[{"id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.191", "mac_address": "BC:24:11:71:EF:5C"}]	{aebfef6c-eb50-40bb-aaf2-00236adefeba,4be6cb86-9b19-42dd-bbee-7980f2a526d6}	[{"id": "65b97cfd-fe2c-436b-ad2a-47e6af338fcc", "type": "Custom", "number": 10086, "protocol": "Tcp"}, {"id": "ed7e75e9-897d-495f-895e-e1f1f2571644", "type": "Ssh", "number": 22, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:05.945791Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:05.945792+00	2025-12-27 16:27:12.988225+00	f	{}
e35b7afb-7229-45b1-a3e0-f920406c57f5	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.4.146	\N	\N	{"type": "None"}	[{"id": "ded4ba49-f1b2-48ef-900e-989144101a6d", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.146", "mac_address": "DC:DA:0C:28:A8:FC"}]	{3089bd19-4bf9-4865-8cd9-60c97799c33b,efff2903-53f4-46cb-b46a-b93277685be8}	[{"id": "83d0dcde-fc79-49f8-b335-e45b21dcac16", "type": "MqttTls", "number": 8883, "protocol": "Tcp"}, {"id": "87f79850-4368-4cda-b498-25c9199b9db5", "type": "Custom", "number": 990, "protocol": "Tcp"}, {"id": "ae6ddc07-ea63-45b4-97fd-a740c448e388", "type": "Http3000", "number": 3000, "protocol": "Tcp"}, {"id": "9477e380-fcdc-44c3-a9b4-7abd8e3c870e", "type": "Custom", "number": 6000, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:28:02.234772Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:28:02.234773+00	2025-12-27 16:28:16.348285+00	f	{}
d166cc9b-c13d-44ec-a0fa-35381ccd961a	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Sonos Speaker	\N	\N	{"type": "None"}	[{"id": "4efab86e-7eed-479c-855b-cbb833c75e63", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.212", "mac_address": "38:42:0B:75:F4:86"}]	{71cd0689-f140-495c-b9b0-d8e0459ebe67,d2b2704a-1217-48b5-b260-693cdf428383}	[{"id": "6126e930-5ce2-45e6-ad87-22b305a9dddd", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "5e541ad5-dc6f-45ab-9bdc-bab792953000", "type": "Custom", "number": 1410, "protocol": "Tcp"}, {"id": "a8c35dfe-fc1b-4730-8906-33e52a091b2e", "type": "Custom", "number": 1843, "protocol": "Tcp"}, {"id": "599040c3-8e3b-42e2-9b51-29f58671a80d", "type": "Custom", "number": 1443, "protocol": "Tcp"}, {"id": "2e2391a7-2c83-401c-aa99-85db19da8a35", "type": "Custom", "number": 7000, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:51.934278Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:51.93428+00	2025-12-27 16:27:12.986316+00	f	{}
6a1525b2-7fdf-4c9a-937b-b868a853ae9a	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.4.213	\N	\N	{"type": "None"}	[{"id": "bfde5c48-01b3-428c-9dee-d3a9b30cd2f6", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.213", "mac_address": "D4:8C:49:F1:55:D0"}]	{}	[]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:48.091869Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:48.091871+00	2025-12-27 16:28:02.243017+00	f	{}
e4a0e35c-d3df-4629-9ce8-aa1e54a9749e	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.4.201	\N	\N	{"type": "None"}	[{"id": "f653c5f5-637d-455f-aea8-06bc34721af5", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.201", "mac_address": "BC:24:11:9E:49:A3"}]	{5c21b8c7-cd23-4345-865a-ba053c19cd29,6f21d4c7-e50e-46ad-a202-f8686a761884}	[{"id": "8460a345-d537-4379-af3b-2c222f2e6fae", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "1be344a9-e9fb-4eee-a69e-a41ee32b8fe4", "type": "Custom", "number": 60072, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:55.165013Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:55.165014+00	2025-12-27 16:28:09.286815+00	f	{}
896e4f49-f7d2-45b9-854f-c3e4f35997da	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Eero Gateway	\N	\N	{"type": "None"}	[{"id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.1", "mac_address": "C4:F1:74:2F:56:F2"}]	{15c9318d-f1b3-4562-885c-758765ebff5f,532f00a6-20b0-47bd-8f7d-9a378219ba53,6ab81963-9786-4a18-a962-92286bf0048e,00bb6c3d-6dcc-49db-93e5-a9dc419f832f,8b1d9b37-2d25-4bf7-af87-9e7a109d8074}	[{"id": "12444520-cd55-4dfd-9e33-49eccfc28025", "type": "Dhcp", "number": 67, "protocol": "Udp"}, {"id": "4d667268-c8b4-4ffb-a270-620aec096bf9", "type": "Ntp", "number": 123, "protocol": "Udp"}, {"id": "4ccf7ae9-b1d9-4c20-a02d-a923bef36957", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "6181df88-6660-456d-b173-ad44afe71272", "type": "Custom", "number": 3001, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:59.009128Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:26:59.009129+00	2025-12-27 16:27:12.98983+00	f	{}
5869cd7b-f4b3-4673-914a-e99f5c3c187b	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Home Assistant	\N	\N	{"type": "None"}	[{"id": "6fb5f173-74bc-4f65-a732-8872038550ab", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.181", "mac_address": "BC:24:11:70:13:4A"}]	{49574957-6e72-4a5a-a1e5-8b7831e2f5e4,a049934b-5721-46bc-893a-980e356a305e,9d7df32b-04f8-4134-aab4-705742ad90b3}	[{"id": "7b4593d3-1ff6-42ae-8b2d-73c1f26ba4fc", "type": "Custom", "number": 8123, "protocol": "Tcp"}, {"id": "c7fac658-12ee-4f11-ae4e-96d2cb0a3ac0", "type": "Mqtt", "number": 1883, "protocol": "Tcp"}, {"id": "42721fe8-6996-4676-a1fc-df82d0d69af2", "type": "MqttTls", "number": 8883, "protocol": "Tcp"}, {"id": "ccde549a-6327-4222-8a58-cb95895183db", "type": "Custom", "number": 111, "protocol": "Tcp"}, {"id": "abe76ea6-a278-4ae7-b142-1dbffaeebc97", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "1edde71c-3226-4943-8a12-aa7d706fdfe6", "type": "Custom", "number": 1884, "protocol": "Tcp"}, {"id": "c6f9bb53-aa4f-43a2-b0b3-5247aef7056d", "type": "Custom", "number": 4357, "protocol": "Tcp"}, {"id": "bbb29195-37b0-429f-80e2-e33c564e4436", "type": "Custom", "number": 5355, "protocol": "Tcp"}, {"id": "bc4f160e-0cb3-400f-980c-16e3e1cd82d8", "type": "Custom", "number": 8884, "protocol": "Tcp"}, {"id": "15d9eab1-a273-44b8-bb6c-7f980b83a570", "type": "Custom", "number": 18555, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:12.981236Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:12.981238+00	2025-12-27 16:27:19.996224+00	f	{}
26af0598-31ac-4c8a-80aa-f520cb1ae3a1	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Philips Hue Bridge	\N	\N	{"type": "None"}	[{"id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.20", "mac_address": "00:17:88:A4:BC:BA"}]	{cbd06526-ed50-4dbe-8334-c8bb96e80c89,26c18fb7-ec4d-4781-891e-16c7a9c203db}	[{"id": "4886a04c-d5f8-4002-bdeb-795254c2bff0", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "81de39d7-52bc-4061-a7a8-6f4e3541d14a", "type": "Https", "number": 443, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:19.982952Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:19.982953+00	2025-12-27 16:27:34.068082+00	f	{}
5f409d90-7208-4099-8c6f-77070a1298f6	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	Sonos Speaker	\N	\N	{"type": "None"}	[{"id": "dc97572b-4079-4e4e-a79d-9f15665796bb", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.29", "mac_address": "34:7E:5C:D3:84:3A"}]	{4cab006b-6e6e-4464-ade8-f328b3c797a6,f91d1417-a806-4459-95b7-c8b4fcea48ad}	[{"id": "c1f3c10f-bc86-4900-a32d-46e166f1a0ea", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "dd6e1edd-09d8-4ac6-ba1e-e0b884a66005", "type": "Custom", "number": 1410, "protocol": "Tcp"}, {"id": "c538bb6a-227a-4bc4-9214-775a3564c89f", "type": "Custom", "number": 1843, "protocol": "Tcp"}, {"id": "c30a7037-3489-4867-9aeb-df460b8c88e7", "type": "Custom", "number": 1443, "protocol": "Tcp"}, {"id": "0de6f147-5990-4547-abc9-82a4e77363a3", "type": "Custom", "number": 7000, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:34.052709Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:34.052709+00	2025-12-27 16:27:48.093791+00	f	{}
d0f8b80a-04f6-4c3a-9477-b25c12cdea16	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.7.73	\N	\N	{"type": "None"}	[{"id": "e118ed94-a256-4aae-99c2-657528afc582", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.7.73", "mac_address": "48:EA:62:0D:38:FA"}]	{6e917741-64ad-4671-9f8c-0e2aafe5aebb,4a53777c-82c8-460d-b2d3-04495db9b371,eabff2d3-f0b0-4ba0-857a-c99a9ae7c12a}	[{"id": "bbccde66-e0f0-486d-bca2-61430451f3a6", "type": "Snmp", "number": 161, "protocol": "Udp"}, {"id": "67c584f5-5923-4ed0-9f17-746504e20993", "type": "LdpTcp", "number": 515, "protocol": "Tcp"}, {"id": "35f25d13-ea1b-4e0b-837f-9fb1b06f163d", "type": "Https", "number": 443, "protocol": "Tcp"}, {"id": "3f2c8a7f-9f32-4056-8f78-722085d9630a", "type": "Http8080", "number": 8080, "protocol": "Tcp"}, {"id": "9f1b71e3-1eb3-4587-bd17-0ad5292b769d", "type": "Custom", "number": 9100, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:27.047969Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:27.04797+00	2025-12-27 16:27:34.068829+00	f	{}
45c0f069-fd0a-4ea2-b5c6-00e1f3500e52	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	192.168.4.30	\N	\N	{"type": "None"}	[{"id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.30", "mac_address": "C8:FF:77:17:D8:9D"}]	{fb4eebc6-dc39-4572-8f10-479d52b50684}	[{"id": "72e9da5d-dce0-4c42-ba0a-fc0c40693a38", "type": "Mqtt", "number": 1883, "protocol": "Tcp"}]	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:41.077575Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	null	2025-12-27 16:27:41.077576+00	2025-12-27 16:27:48.093696+00	f	{}
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, is_default, organization_id, tags) FROM stdin;
f30cdbff-4a20-4a9b-9699-220e8f1a0b02	test	2025-12-27 16:13:24.580067+00	2025-12-27 16:13:24.580067+00	f	880e6a2d-46c6-4afb-a39d-efb7f2368fc0	{}
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding) FROM stdin;
880e6a2d-46c6-4afb-a39d-efb7f2368fc0	test	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "seat_cents": null, "trial_days": 0, "network_cents": null, "included_seats": null, "included_networks": null}	\N	2025-12-27 16:13:24.570065+00	2025-12-27 16:13:57.107731+00	["OnboardingModalCompleted", "FirstApiKeyCreated", "FirstDaemonRegistered", "FirstTopologyRebuild"]
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, bindings, service_definition, virtualization, source, tags) FROM stdin;
586a6eda-e792-44f5-ad58-a96ad5b1ee06	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:24.582413+00	2025-12-27 16:13:24.582413+00	Mobile Device	560d8a7f-0c0a-478a-b16e-abe06611e797	[{"id": "994c1eec-0798-40cc-b1b3-fa3d1f88cc47", "type": "Port", "port_id": "256ba520-397d-4837-85d0-8775b759944f", "interface_id": "74464d04-e4d4-4cdc-8671-0e34423a7eec"}]	"Client"	null	{"type": "System"}	{}
b219d4a5-bdfe-44b0-8852-443daff2e5dc	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:57.074372+00	2025-12-27 16:13:57.074372+00	Docker	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[]	"Docker"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Docker daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-27T16:13:57.074372Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}	{}
564eb7bc-2f24-4c05-afc1-530a8a58f155	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:14:14.027365+00	2025-12-27 16:14:14.027365+00	authentik-postgresql-1	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "ec074984-3d3a-4bc5-8c51-99d482d6a7af", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "4c298ae7-726e-4b7d-b514-1f26c513579e"}]	"PostgreSQL"	{"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "07bde47b05aa71b7b7f4d01c3f872534e24775c5bc53a5d26bd5845c26017c22", "container_name": "authentik-postgresql-1"}}	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:14:14.027350Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
51c913fb-31e0-4576-b06d-0a3e48967c9c	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:14:38.348922+00	2025-12-27 16:14:38.348922+00	authentik-worker-1	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "66eb6941-2cf7-41e1-b904-e73a005de9ea", "type": "Interface", "interface_id": "d8195316-15bc-4420-8d48-95d7a03f8d9e"}]	"Docker Container"	{"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "76cb5435cb2178b9c137baf9dfee927fec2d44d7f867e2ee7607decf444417aa", "container_name": "authentik-worker-1"}}	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["All of", [{"data": "Service is running in docker container", "type": "reason"}, {"data": "No other services with this container's ID have been matched", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:14:38.348910Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
dcf72989-2e9a-483a-82dd-54cf055fccc5	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:24.582402+00	2025-12-27 16:21:40.452554+00	Cloudflare DNS	fd85da79-5f19-43ba-95bf-b3752c4ba9b5	[{"id": "fe3f4f51-8635-4d04-8663-5fa20e96e577", "type": "Port", "port_id": "1c115fc4-91b2-40c3-9a58-5ccbb70b44fe", "interface_id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0"}]	"Dns Server"	null	{"type": "System"}	{}
9774e951-1df1-4abe-8094-c45095da8138	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:24.582408+00	2025-12-27 16:21:45.010782+00	Google.com	6baa326f-7ab8-4d56-a689-9c51c6258b32	[{"id": "327558cf-6a01-46ac-85f7-40d84c8bf85f", "type": "Port", "port_id": "7a76431c-b494-4ba4-93b9-6c8ffadfc053", "interface_id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d"}]	"Web Service"	null	{"type": "System"}	{}
a3a68848-b0a5-4c8d-babc-8eae60c5db55	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:30.251834+00	2025-12-27 16:26:30.251834+00	Dns Server	4777c89f-fe55-4baf-9d17-2d6f70b696f0	[{"id": "7304480e-a2d9-4baf-b3b0-f268d749ebfb", "type": "Port", "port_id": "2736a1b4-6d45-46a4-879f-46f735ad431b", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}, {"id": "649e19fb-1532-4eb8-8c61-da747ef91e23", "type": "Port", "port_id": "902191b5-4c46-422e-b1f0-b21a9771c888", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}]	"Dns Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 53/tcp is open", "type": "reason"}, {"data": "Port 53/udp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251828Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
69d48dbe-840f-40ce-93c9-3ef2b3bcf37d	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:34.22294+00	2025-12-27 16:26:34.22294+00	Nginx Proxy Manager	309cef94-06eb-4d7b-9b25-90a81789a0e1	[{"id": "0859d540-90fe-4fcf-a81c-d0caa64f2a61", "type": "Port", "port_id": "32b4b60c-6d8b-4306-af78-bb2dbd932047", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}]	"Nginx Proxy Manager"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.187:80 contained \\"nginx proxy manager\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:34.222928Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
d2b2704a-1217-48b5-b260-693cdf428383	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:59.00885+00	2025-12-27 16:26:59.00885+00	Unclaimed Open Ports	d166cc9b-c13d-44ec-a0fa-35381ccd961a	[{"id": "44c2aea3-a689-44f9-8920-aee70bf52799", "type": "Port", "port_id": "599040c3-8e3b-42e2-9b51-29f58671a80d", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "05669fc0-fab0-4e64-a2fa-3861b8fde23f", "type": "Port", "port_id": "2e2391a7-2c83-401c-aa99-85db19da8a35", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:59.008837Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
8d506cbb-44bf-4262-834f-dd3c55a8c29d	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:17:00.96296+00	2025-12-27 16:17:00.96296+00	Unclaimed Open Ports	94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9	[{"id": "cef96d13-ef23-4f21-91ad-76976708efe6", "type": "Port", "port_id": "71a0544f-1eb1-4ff5-aeb1-7ddf421457a7", "interface_id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:17:00.962958Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
fddb2f60-840b-44e5-a179-cb74633ebef3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:26.896309+00	2025-12-27 16:26:26.896309+00	Eero Repeater	4777c89f-fe55-4baf-9d17-2d6f70b696f0	[{"id": "d532969f-6564-4388-a8ca-20d86d8997a9", "type": "Interface", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}]	"Eero Repeater"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor eero Inc", "type": "reason"}, {"data": "IP address is not in routing table, and does not end in 1 or 254 with no other gateways identified in subnet", "type": "reason"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:26:26.896299Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
a6f5d5ef-f914-4d12-8da9-756edf4247a7	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:44.856922+00	2025-12-27 16:26:44.856922+00	Unclaimed Open Ports	5c882c83-72f7-495e-8bdf-71413bd445c3	[{"id": "5f200e72-30ee-4341-8174-2257e4aafacf", "type": "Port", "port_id": "735e18b4-6780-4a74-abc7-ecc781fdbb11", "interface_id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:44.856920Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
f9006309-f2f7-475b-aa03-aea6200af41a	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:14:06.601579+00	2025-12-27 16:18:19.318809+00	scanopy-postgres	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "c58fee2e-df2d-4d04-b1b0-7dd9fc373dc1", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e"}, {"id": "d6be78e2-88fc-429f-ad0b-276f359d5b05", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "44d3781a-fc59-46ac-8287-6c6ebf37db4f", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}]	"PostgreSQL"	{"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "837f73c8d8e4545da7daa7968c022df9414eb032a35b065be2ba2aefcc565db1", "container_name": "scanopy-postgres"}}	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:18:19.292462Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:06.601567Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
14c87e66-ce12-4aed-a634-b5d344d73f01	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:57.006904+00	2025-12-27 16:18:19.320024+00	Scanopy Daemon	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "3c4b02f4-4fe1-480f-9706-de761c9861cb", "type": "Port", "port_id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "8f939956-de04-4eb0-ae96-ef1f2cb75911", "type": "Port", "port_id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}]	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-27T16:18:19.268486Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.006903Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}	{}
a164ce69-5325-4bff-a24b-7b563005c37f	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:18.806881+00	2025-12-27 16:26:18.806881+00	Pi-Hole	eb13ca22-e581-4a72-b188-fb80b1b4cac4	[{"id": "c4af949c-fce9-404f-9819-6608a02fe269", "type": "Port", "port_id": "3570d7d6-bfc8-4cb7-bc41-9e55eecdf955", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}, {"id": "12b49c36-0866-41b1-9fde-b051e72681a4", "type": "Port", "port_id": "63c0a278-5537-4a71-80b2-3cdf716dad50", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}, {"id": "61f5ebdc-8046-44a3-9b3c-bf5dc89837e4", "type": "Port", "port_id": "6843343a-7702-4b6f-b079-6753e6e102de", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}]	"Pi-Hole"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": ["Any of", [{"data": "Port 53/udp is open", "type": "reason"}, {"data": "Port 53/tcp is open", "type": "reason"}]], "type": "container"}, {"data": "Response for 192.168.4.188:80/admin contained \\"pi-hole\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:18.806867Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
cd30c4e4-7163-4219-8a75-11127599fff3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:30.251867+00	2025-12-27 16:26:30.251867+00	Unclaimed Open Ports	4777c89f-fe55-4baf-9d17-2d6f70b696f0	[{"id": "84c3d3a2-9120-463d-8694-55b8fc9607a4", "type": "Port", "port_id": "5e5a370f-8ea8-462e-b0bf-3e8f03889578", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}, {"id": "eaa95a2a-ed28-44a0-9c47-95a4304768bf", "type": "Port", "port_id": "88f8765b-7701-4b2a-a904-5408d75df0e0", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251862Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
22e8c40e-f48b-47f9-9066-9c70bcfcd20e	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:37.941595+00	2025-12-27 16:26:37.941595+00	Unclaimed Open Ports	309cef94-06eb-4d7b-9b25-90a81789a0e1	[{"id": "0d6a649d-9fb0-420e-b4df-445b9f92948d", "type": "Port", "port_id": "cf9afd3c-3d81-44a1-93dc-d01afacc8557", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}, {"id": "517354fd-954b-4560-9512-c9727a6e330f", "type": "Port", "port_id": "9e5d060f-6180-4019-8566-78bae229e8ad", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}, {"id": "52ca5b02-783f-4600-a344-85b78b6bccc8", "type": "Port", "port_id": "b1e709b9-2d28-41b6-ace0-7696f23806df", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:37.941587Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
0fab6211-dc92-40e2-b7a2-c26135feead3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:48.110255+00	2025-12-27 16:26:48.110255+00	Portainer	66bb7da2-71db-441b-b8e6-ee60eabc4a59	[{"id": "a105609c-426e-4bf7-ad47-1773715bbcb7", "type": "Port", "port_id": "94015005-8c6e-454b-bcf4-f2b491163271", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}]	"Portainer"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 192.168.4.126:9000/ contained \\"portainer.io\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:48.110242Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
532f00a6-20b0-47bd-8f7d-9a378219ba53	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:05.945151+00	2025-12-27 16:27:05.945151+00	Dhcp Server	896e4f49-f7d2-45b9-854f-c3e4f35997da	[{"id": "9039765a-fcdd-47cc-b0bd-a5ccad4b4570", "type": "Port", "port_id": "12444520-cd55-4dfd-9e33-49eccfc28025", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}]	"Dhcp Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 67/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945139Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
b7a8bbea-2ca2-455b-aaf4-e15a394a8cdf	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:17:00.962932+00	2025-12-27 16:17:00.962932+00	SSH	94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9	[{"id": "17925ad3-65c5-40b6-8033-b8cc768ba6ad", "type": "Port", "port_id": "7b5bded1-334c-4ce9-a218-f2d2b63fe5c7", "interface_id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:17:00.962925Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
a1b0a520-e7a4-4baa-9614-5b837149c8f8	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:23.506048+00	2025-12-27 16:26:23.506048+00	SSH	eb13ca22-e581-4a72-b188-fb80b1b4cac4	[{"id": "bcfbf890-052e-409f-9b2e-4a799336b661", "type": "Port", "port_id": "51bc1341-266c-4225-9c35-0410d4bccb28", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:23.506037Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
bfbbb2bf-cf59-45b7-9339-582cec92860c	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:30.251589+00	2025-12-27 16:26:30.251589+00	NTP Server	4777c89f-fe55-4baf-9d17-2d6f70b696f0	[{"id": "76cbf088-755b-4d16-beef-1e5c8386e4de", "type": "Port", "port_id": "c613e0dc-e4f9-4bc7-a871-439070402d3f", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}]	"NTP Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 123/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251578Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
aaaa46d2-f808-480d-a636-b1e03560c5b9	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:44.856897+00	2025-12-27 16:26:44.856897+00	SSH	5c882c83-72f7-495e-8bdf-71413bd445c3	[{"id": "c12300ab-2cd7-401e-9350-79f8f34986df", "type": "Port", "port_id": "2cd178ba-1657-49c7-b0a8-b236c0e40541", "interface_id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:44.856886Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
aebfef6c-eb50-40bb-aaf2-00236adefeba	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:09.121743+00	2025-12-27 16:27:09.121743+00	WGDashboard	2493a39c-903a-47b1-8aff-3ae3ad9356fe	[{"id": "c7b31f1e-4a43-455a-b70c-7235bbb86420", "type": "Port", "port_id": "65b97cfd-fe2c-436b-ad2a-47e6af338fcc", "interface_id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86"}]	"WGDashboard"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Port 10086/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Subnet 192.168.4.0/22 is not type VPN", "type": "reason"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:27:09.121730Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
9d7df32b-04f8-4134-aab4-705742ad90b3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:19.982534+00	2025-12-27 16:27:19.982534+00	Unclaimed Open Ports	5869cd7b-f4b3-4673-914a-e99f5c3c187b	[{"id": "60caa50b-16ac-4bc3-8571-7ccbf94244a1", "type": "Port", "port_id": "ccde549a-6327-4222-8a58-cb95895183db", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "1a00fc8f-2ef8-4ed2-aadc-5f6b9d5ed8d8", "type": "Port", "port_id": "abe76ea6-a278-4ae7-b142-1dbffaeebc97", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6f52c33c-0760-442c-b961-2b087cf01222", "type": "Port", "port_id": "1edde71c-3226-4943-8a12-aa7d706fdfe6", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6acb906e-2faa-490f-ae59-b07880cfd931", "type": "Port", "port_id": "c6f9bb53-aa4f-43a2-b0b3-5247aef7056d", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "aa05cb69-b1ab-4aa4-95d9-4ccd1d151f6c", "type": "Port", "port_id": "bbb29195-37b0-429f-80e2-e33c564e4436", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6833d4a8-3518-4088-83f3-08d5e14e5e30", "type": "Port", "port_id": "bc4f160e-0cb3-400f-980c-16e3e1cd82d8", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "28d14070-4394-4c65-a527-031b5e75b61a", "type": "Port", "port_id": "15d9eab1-a273-44b8-bb6c-7f980b83a570", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:19.982516Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
c905eb81-ce8a-468a-8bb6-69a02c33be59	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:18:19.28008+00	2025-12-27 16:18:19.28008+00	Scanopy Server	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "a52e96ba-c82e-4d59-8bf7-9cc42e63bd94", "type": "Port", "port_id": "67db1b3d-93ee-48cf-9e2a-a98c11f75243", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}]	"Scanopy Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 10.0.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:18:19.280072Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
c35744b5-8620-46da-87ef-617f309ac242	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:18:19.292508+00	2025-12-27 16:18:19.292508+00	Unclaimed Open Ports	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "be4cc988-f3c3-44c2-a080-e7461bb5e473", "type": "Port", "port_id": "68ecbd7d-b49e-4483-bf59-b476420f54c9", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "5a3da085-bd5f-4b40-afba-b6d2c996c77f", "type": "Port", "port_id": "f4c56874-7052-4df7-b99c-c9c260e6438e", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "5622395f-3bd0-4b2d-b456-5510b88ecb9c", "type": "Port", "port_id": "a469779d-c6d7-4d37-85fc-8f5556b092aa", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "a792c13c-95a4-4d9d-99cc-ddfe9bffe6a4", "type": "Port", "port_id": "62d74abe-bc84-46fd-8583-9ed50fbf2f92", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "eeab16a5-a85b-45d1-9a59-50ba6ad284b4", "type": "Port", "port_id": "fa0529f1-1bb3-41eb-ab82-8316b794ead7", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "478264e3-3783-4844-8085-b77a67d42524", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "9f27ce90-4bd6-4ba1-90a8-644d4b3b8a9b", "type": "Port", "port_id": "ca723695-40c3-4e8f-89c6-e3dc4d5e0d5c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "1661bc4d-ca02-4a34-a62c-1173b87d7547", "type": "Port", "port_id": "25da3a86-a1c7-48f4-8fb0-6817b55a95ed", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:18:19.292493Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
f953905a-26cd-4978-b271-49d584248ec3	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:14:27.361481+00	2025-12-27 16:18:19.31928+00	Authentik	cb0f322b-28a0-4dd9-9cc6-7570d79e23fb	[{"id": "5c04dbec-5cf4-4691-a316-da48c35eb5a3", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e"}, {"id": "23d35772-b12c-439f-a0dc-c955ed3a644c", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e"}, {"id": "7dabb720-4188-4a5b-9986-d437215bb15d", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "51c19ca1-7cd0-4657-b9b0-6913b22c6333", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "6e9028c1-5f56-45f4-8dc3-4f30f6217cba", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "c400df4b-8f1e-4d10-9da2-6a9d6cb775ad", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}]	"Authentik"	{"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "46960b2d2d4ef00adb65b188d7ff2648849293cda661943ea0e9719f532116f8", "container_name": "authentik-server-1"}}	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 172.18.0.3:9000/ contained \\"window.authentik\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:18:19.260768Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:27.361467Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
00b51667-6e22-42fc-a985-0cdd30567eea	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:23.506187+00	2025-12-27 16:26:23.506187+00	Unclaimed Open Ports	eb13ca22-e581-4a72-b188-fb80b1b4cac4	[{"id": "30fa1f62-5b4e-4a0d-9587-1484ed7308b5", "type": "Port", "port_id": "ef4e6114-60fa-43be-89d6-6ed213968f6d", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:23.506183Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
6c5fc429-09fd-4841-b1f9-6b46597e8080	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:37.941549+00	2025-12-27 16:26:37.941549+00	SSH	309cef94-06eb-4d7b-9b25-90a81789a0e1	[{"id": "f7a7ce0d-c3c3-4825-a400-9c66447fe5de", "type": "Port", "port_id": "66b4e5ef-a232-4842-83d5-544746185fc8", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:37.941538Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
3f12d1f2-a049-438c-9fe3-1eb6116d68f2	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:51.927125+00	2025-12-27 16:26:51.927125+00	Unclaimed Open Ports	66bb7da2-71db-441b-b8e6-ee60eabc4a59	[{"id": "5708f030-7364-47e5-aa8a-63d915554453", "type": "Port", "port_id": "961c3c56-ee02-460f-98c7-0637608c2f06", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "8522b641-f704-4e91-aec5-1ab75032c6fe", "type": "Port", "port_id": "59a3577d-1dbd-47ec-a46d-1e334bb1f6cf", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "ec1aa41c-edaa-4c40-80a8-5e8bbbf6754b", "type": "Port", "port_id": "310ecfea-5467-4b09-8b32-6848a2315c4e", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "fbdbdb0e-836b-473c-ae41-3f2cc702d052", "type": "Port", "port_id": "92706bde-ad98-46f2-ac52-9c0e8c597cf6", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:51.927109Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
591ac6bf-8b30-4d21-9546-5a7972201c2b	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:49.853142+00	2025-12-27 16:26:49.853142+00	Scanopy Daemon	66bb7da2-71db-441b-b8e6-ee60eabc4a59	[{"id": "c7b35cc7-e99f-493e-b869-e1b5c7763705", "type": "Port", "port_id": "7110e884-7fe5-4fac-814a-c7172c0fd254", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}]	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.126:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:49.853130Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
4be6cb86-9b19-42dd-bbee-7980f2a526d6	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:12.969048+00	2025-12-27 16:27:12.969048+00	SSH	2493a39c-903a-47b1-8aff-3ae3ad9356fe	[{"id": "e1853a3f-1bef-4674-8b94-6d1b62ddbf24", "type": "Port", "port_id": "ed7e75e9-897d-495f-895e-e1f1f2571644", "interface_id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:12.969035Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
71cd0689-f140-495c-b9b0-d8e0459ebe67	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:26:53.318827+00	2025-12-27 16:26:53.318827+00	Sonos Speaker	d166cc9b-c13d-44ec-a0fa-35381ccd961a	[{"id": "0407827a-769d-49b0-a5a5-23dac9065485", "type": "Port", "port_id": "6126e930-5ce2-45e6-ad87-22b305a9dddd", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "0986f130-f754-47ec-9a1e-c9f3af84a268", "type": "Port", "port_id": "5e541ad5-dc6f-45ab-9bdc-bab792953000", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "61a84d03-973b-46d3-b48f-29047072e976", "type": "Port", "port_id": "a8c35dfe-fc1b-4730-8906-33e52a091b2e", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}]	"Sonos Speaker"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Sonos, Inc", "type": "reason"}, {"data": ["Any of", [{"data": "Port 1400/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1410/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1843/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:26:53.318813Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
26c18fb7-ec4d-4781-891e-16c7a9c203db	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:27.047704+00	2025-12-27 16:27:27.047704+00	Unclaimed Open Ports	26af0598-31ac-4c8a-80aa-f520cb1ae3a1	[{"id": "fd296caf-5e0c-464b-bfa6-81ca6d7c1a58", "type": "Port", "port_id": "81de39d7-52bc-4061-a7a8-6f4e3541d14a", "interface_id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:27.047692Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
15c9318d-f1b3-4562-885c-758765ebff5f	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:00.737926+00	2025-12-27 16:27:00.737926+00	Eero Gateway	896e4f49-f7d2-45b9-854f-c3e4f35997da	[{"id": "80d41d8a-8ade-4886-b8ac-6391e7c9c1da", "type": "Interface", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}]	"Eero Gateway"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor eero Inc", "type": "reason"}, {"data": "Host IP address is in routing table of daemon 7dc85a03-c94f-42d9-b52f-13808fbe88d7", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:00.737916Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
fb4eebc6-dc39-4572-8f10-479d52b50684	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:48.077419+00	2025-12-27 16:27:48.077419+00	MQTT	45c0f069-fd0a-4ea2-b5c6-00e1f3500e52	[{"id": "dfeac1cb-f9a8-4359-a449-957d41412b86", "type": "Port", "port_id": "72e9da5d-dce0-4c42-ba0a-fc0c40693a38", "interface_id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd"}]	"MQTT"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 1883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:48.077405Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
6ab81963-9786-4a18-a962-92286bf0048e	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:05.945288+00	2025-12-27 16:27:05.945288+00	NTP Server	896e4f49-f7d2-45b9-854f-c3e4f35997da	[{"id": "2915315f-b73f-4d46-81df-d5d815e3be96", "type": "Port", "port_id": "4d667268-c8b4-4ffb-a270-620aec096bf9", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}]	"NTP Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 123/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945283Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
00bb6c3d-6dcc-49db-93e5-a9dc419f832f	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:05.945423+00	2025-12-27 16:27:05.945423+00	Dns Server	896e4f49-f7d2-45b9-854f-c3e4f35997da	[{"id": "4ea23655-81f1-4f04-a488-ca536a8713f8", "type": "Port", "port_id": "4ccf7ae9-b1d9-4c20-a02d-a923bef36957", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}]	"Dns Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 53/udp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945420Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
49574957-6e72-4a5a-a1e5-8b7831e2f5e4	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:17.886791+00	2025-12-27 16:27:17.886791+00	Home Assistant	5869cd7b-f4b3-4673-914a-e99f5c3c187b	[{"id": "f15de529-5514-421f-a415-d65146ac62a8", "type": "Port", "port_id": "7b4593d3-1ff6-42ae-8b2d-73c1f26ba4fc", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}]	"Home Assistant"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.181:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:17.886775Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
4a53777c-82c8-460d-b2d3-04495db9b371	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:34.04992+00	2025-12-27 16:27:34.04992+00	Print Server	d0f8b80a-04f6-4c3a-9477-b25c12cdea16	[{"id": "8a440fae-d24b-4900-9455-4653c36260c1", "type": "Port", "port_id": "67c584f5-5923-4ed0-9f17-746504e20993", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}]	"Print Server"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 515/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049917Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
8b1d9b37-2d25-4bf7-af87-9e7a109d8074	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:05.945452+00	2025-12-27 16:27:05.945452+00	Unclaimed Open Ports	896e4f49-f7d2-45b9-854f-c3e4f35997da	[{"id": "5228645c-2fd4-4e44-8c0d-d019ce050974", "type": "Port", "port_id": "6181df88-6660-456d-b173-ad44afe71272", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945449Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
6e917741-64ad-4671-9f8c-0e2aafe5aebb	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:34.04977+00	2025-12-27 16:27:34.04977+00	SNMP	d0f8b80a-04f6-4c3a-9477-b25c12cdea16	[{"id": "76d5a39c-324d-42de-9642-0749f22a4838", "type": "Port", "port_id": "bbccde66-e0f0-486d-bca2-61430451f3a6", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}]	"SNMP"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 161/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049759Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
f91d1417-a806-4459-95b7-c8b4fcea48ad	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:41.077307+00	2025-12-27 16:27:41.077307+00	Unclaimed Open Ports	5f409d90-7208-4099-8c6f-77070a1298f6	[{"id": "797fdd24-6a6b-4e0c-aec2-cc96c77a8ff5", "type": "Port", "port_id": "c30a7037-3489-4867-9aeb-df460b8c88e7", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "277eb415-943d-4f7c-9360-0337f9cb6156", "type": "Port", "port_id": "0de6f147-5990-4547-abc9-82a4e77363a3", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:41.077294Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
a049934b-5721-46bc-893a-980e356a305e	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:19.982478+00	2025-12-27 16:27:19.982478+00	MQTT	5869cd7b-f4b3-4673-914a-e99f5c3c187b	[{"id": "2f97b92c-8180-4a7e-bb9e-8c68e7ce094d", "type": "Port", "port_id": "c7fac658-12ee-4f11-ae4e-96d2cb0a3ac0", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6f407699-65dc-41db-b0d8-99ecb27e7c6a", "type": "Port", "port_id": "42721fe8-6996-4676-a1fc-df82d0d69af2", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}]	"MQTT"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 1883/tcp is open", "type": "reason"}, {"data": "Port 8883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:19.982461Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
cbd06526-ed50-4dbe-8334-c8bb96e80c89	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:24.270406+00	2025-12-27 16:27:24.270406+00	Philips Hue Bridge	26af0598-31ac-4c8a-80aa-f520cb1ae3a1	[{"id": "060ab0a7-4f85-4120-a817-47f2ca836776", "type": "Port", "port_id": "4886a04c-d5f8-4002-bdeb-795254c2bff0", "interface_id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330"}]	"Philips Hue Bridge"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Philips Lighting BV", "type": "reason"}, {"data": "Response for 192.168.4.20:80/ contained \\"hue\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:24.270393Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
eabff2d3-f0b0-4ba0-857a-c99a9ae7c12a	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:34.049962+00	2025-12-27 16:27:34.049962+00	Unclaimed Open Ports	d0f8b80a-04f6-4c3a-9477-b25c12cdea16	[{"id": "dbddfa4e-0473-456c-9c19-417a5fea9e42", "type": "Port", "port_id": "35f25d13-ea1b-4e0b-837f-9fb1b06f163d", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}, {"id": "6722a116-9452-4087-ba8c-6b584ac80594", "type": "Port", "port_id": "3f2c8a7f-9f32-4056-8f78-722085d9630a", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}, {"id": "391c3046-9c39-48b3-9840-394ae8847cd2", "type": "Port", "port_id": "9f1b71e3-1eb3-4587-bd17-0ad5292b769d", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049956Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
4cab006b-6e6e-4464-ade8-f328b3c797a6	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:27:35.4718+00	2025-12-27 16:27:35.4718+00	Sonos Speaker	5f409d90-7208-4099-8c6f-77070a1298f6	[{"id": "08d37d76-5500-4708-be58-7e15cc4292a4", "type": "Port", "port_id": "c1f3c10f-bc86-4900-a32d-46e166f1a0ea", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "3b6be90c-47cb-4630-b831-12de30ce1f49", "type": "Port", "port_id": "dd6e1edd-09d8-4ac6-ba1e-e0b884a66005", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "30c763dd-8d9b-4266-bcc8-913e5b35209b", "type": "Port", "port_id": "c538bb6a-227a-4bc4-9214-775a3564c89f", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}]	"Sonos Speaker"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Sonos, Inc", "type": "reason"}, {"data": ["Any of", [{"data": "Port 1400/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1410/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1843/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:27:35.471785Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
6f21d4c7-e50e-46ad-a202-f8686a761884	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:02.234517+00	2025-12-27 16:28:02.234517+00	Unclaimed Open Ports	e4a0e35c-d3df-4629-9ce8-aa1e54a9749e	[{"id": "63f083ca-b983-4909-88a0-4cae20ba8049", "type": "Port", "port_id": "1be344a9-e9fb-4eee-a69e-a41ee32b8fe4", "interface_id": "f653c5f5-637d-455f-aea8-06bc34721af5"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:02.234514Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
5c21b8c7-cd23-4345-865a-ba053c19cd29	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:02.234485+00	2025-12-27 16:28:02.234485+00	SSH	e4a0e35c-d3df-4629-9ce8-aa1e54a9749e	[{"id": "c2cf3fd2-4d4b-4001-8ba8-0847d739c706", "type": "Port", "port_id": "8460a345-d537-4379-af3b-2c222f2e6fae", "interface_id": "f653c5f5-637d-455f-aea8-06bc34721af5"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:02.234468Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
3089bd19-4bf9-4865-8cd9-60c97799c33b	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:09.27766+00	2025-12-27 16:28:09.27766+00	MQTT	e35b7afb-7229-45b1-a3e0-f920406c57f5	[{"id": "34466b44-b12e-428a-8f44-9670b6dad48d", "type": "Port", "port_id": "83d0dcde-fc79-49f8-b335-e45b21dcac16", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}]	"MQTT"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 8883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:09.277648Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
d311be56-1dd0-40c6-b7c7-0bee84cc1bde	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:11.7455+00	2025-12-27 16:28:11.7455+00	Proxmox VE	1effd89a-40ab-4a44-862b-c3a14db9074c	[{"id": "d4012352-ee83-485e-9da9-0f289e787236", "type": "Port", "port_id": "0a395d11-abb5-40e7-9940-e0df455bfa1b", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}]	"Proxmox VE"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 192.168.4.135:8006/ contained \\"proxmox\\" in body", "type": "reason"}, {"data": "Port 8006/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:28:11.745488Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
efff2903-53f4-46cb-b46a-b93277685be8	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:09.27772+00	2025-12-27 16:28:09.27772+00	Unclaimed Open Ports	e35b7afb-7229-45b1-a3e0-f920406c57f5	[{"id": "4de39328-83b4-49c1-b016-d299dabad939", "type": "Port", "port_id": "87f79850-4368-4cda-b498-25c9199b9db5", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}, {"id": "4cab2190-e99a-4c73-ab15-cf58d1c1ea11", "type": "Port", "port_id": "ae6ddc07-ea63-45b4-97fd-a740c448e388", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}, {"id": "7bb21d78-ede8-4b83-ac86-2f90931702a1", "type": "Port", "port_id": "9477e380-fcdc-44c3-a9b4-7abd8e3c870e", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:09.277689Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
50e66c44-301d-4331-9030-47ff6e746a00	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:16.336908+00	2025-12-27 16:28:16.336908+00	Unclaimed Open Ports	1effd89a-40ab-4a44-862b-c3a14db9074c	[{"id": "6d6c34b4-9c6e-42cd-aace-237b187e1fe1", "type": "Port", "port_id": "87217851-d7f2-4ac8-aa19-ed2f081ff75e", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}]	"Unclaimed Open Ports"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:16.336906Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
e25cba30-c570-462e-a535-1281d3e7317f	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:28:16.336877+00	2025-12-27 16:28:16.336877+00	SSH	1effd89a-40ab-4a44-862b-c3a14db9074c	[{"id": "6607bc12-cb15-490a-b77c-38e2cfdeff6f", "type": "Port", "port_id": "d7c195bf-5726-4423-9d3c-6bd45bf89ca7", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}]	"SSH"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:16.336867Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}	{}
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
8579ce39-369f-44b1-84ac-7cd21dee9bdd	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:24.582359+00	2025-12-27 16:13:24.582359+00	"0.0.0.0/0"	Internet	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).	"Internet"	{"type": "System"}	{}
ca1bf14c-9af9-4a9f-a9b3-c29f0e798066	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:24.58236+00	2025-12-27 16:13:24.58236+00	"0.0.0.0/0"	Remote Network	This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).	"Remote"	{"type": "System"}	{}
f1a114ad-b7c5-4146-8975-f772be3986d9	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:56.988698+00	2025-12-27 16:13:56.988698+00	"10.0.0.0/24"	10.0.0.0/24	\N	"VpnTunnel"	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:56.988698Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}	{}
e5b4263e-1964-439d-bcb8-2cbb11018742	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:56.988656+00	2025-12-27 16:13:56.988656+00	"192.168.4.0/22"	192.168.4.0/22	\N	"Lan"	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:56.988655Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}	{}
8f8917b5-02b6-4998-8efe-42c6dedf0029	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:57.065315+00	2025-12-27 16:13:57.065315+00	"172.18.0.0/16"	authentik_default	\N	"DockerBridge"	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:57.065315Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
fa8903e6-fe1f-4e65-b169-75df44e72c5e	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	2025-12-27 16:13:57.065307+00	2025-12-27 16:13:57.065307+00	"172.17.0.0/16"	bridge	\N	"DockerBridge"	{"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:57.065306Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}	{}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
00390626-53aa-4851-bde1-b7d5cbfdbf38	880e6a2d-46c6-4afb-a39d-efb7f2368fc0	test	\N	2025-12-27 16:21:33.729197+00	2025-12-27 16:21:33.729197+00	yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags) FROM stdin;
8376fd8d-f4fc-4bbb-ac17-41530350a80c	f30cdbff-4a20-4a9b-9699-220e8f1a0b02	My Topology	[{"id": "86cc672d-fabc-41d3-893d-54f497a352ad", "label": "test", "source": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83", "target": "952ba3b9-96fa-4d38-8866-887522dda68e", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "edge_type": "Interface", "is_multi_hop": false, "source_handle": "Top", "target_handle": "Bottom"}, {"id": "795d8958-7a67-4b49-9051-5d702a506c43", "label": "test", "source": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0", "target": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e", "group_id": "b6672bce-d110-4655-a025-fda8b5a9128a", "edge_type": "RequestPath", "is_multi_hop": true, "source_handle": "Bottom", "target_handle": "Left", "source_binding_id": "fe3f4f51-8635-4d04-8663-5fa20e96e577", "target_binding_id": "c58fee2e-df2d-4d04-b1b0-7dd9fc373dc1"}, {"id": "493a1022-8b45-4c36-bd48-1eb1f1e73c70", "label": "Docker @ test", "source": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83", "target": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "edge_type": "ServiceVirtualization", "is_multi_hop": false, "source_handle": "Bottom", "target_handle": "Top", "containerizing_service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc"}]	[{"id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "size": {"x": 650, "y": 500}, "header": null, "position": {"x": 125, "y": 450}, "node_type": "SubnetNode", "infra_width": 0}, {"id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "size": {"x": 1900, "y": 1200}, "header": null, "position": {"x": 125, "y": 1075}, "node_type": "SubnetNode", "infra_width": 650}, {"id": "ca1bf14c-9af9-4a9f-a9b3-c29f0e798066", "size": {"x": 350, "y": 200}, "header": null, "position": {"x": 950, "y": 125}, "node_type": "SubnetNode", "infra_width": 0}, {"id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "size": {"x": 700, "y": 200}, "header": null, "position": {"x": 125, "y": 125}, "node_type": "SubnetNode", "infra_width": 350}, {"id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "size": {"x": 650, "y": 350}, "header": "Docker Bridge: (172.17.0.0/16, 172.18.0.0/16)", "position": {"x": 125, "y": 2400}, "node_type": "SubnetNode", "infra_width": 0}, {"id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e", "size": {"x": 250, "y": 100}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 50, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "interface_id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e"}, {"id": "4c298ae7-726e-4b7d-b514-1f26c513579e", "size": {"x": 250, "y": 100}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 350, "y": 200}, "node_type": "InterfaceNode", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "interface_id": "4c298ae7-726e-4b7d-b514-1f26c513579e"}, {"id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e", "size": {"x": 250, "y": 100}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 350, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "interface_id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e"}, {"id": "d8195316-15bc-4420-8d48-95d7a03f8d9e", "size": {"x": 250, "y": 100}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 50, "y": 200}, "node_type": "InterfaceNode", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "interface_id": "d8195316-15bc-4420-8d48-95d7a03f8d9e"}, {"id": "74464d04-e4d4-4cdc-8671-0e34423a7eec", "size": {"x": 250, "y": 100}, "header": null, "host_id": "560d8a7f-0c0a-478a-b16e-abe06611e797", "is_infra": false, "position": {"x": 50, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "ca1bf14c-9af9-4a9f-a9b3-c29f0e798066", "interface_id": "74464d04-e4d4-4cdc-8671-0e34423a7eec"}, {"id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad", "size": {"x": 250, "y": 175}, "header": null, "host_id": "94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9", "is_infra": false, "position": {"x": 350, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "interface_id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad"}, {"id": "952ba3b9-96fa-4d38-8866-887522dda68e", "size": {"x": 250, "y": 400}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 50, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0", "size": {"x": 250, "y": 100}, "header": null, "host_id": "fd85da79-5f19-43ba-95bf-b3752c4ba9b5", "is_infra": true, "position": {"x": 50, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "interface_id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0"}, {"id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d", "size": {"x": 250, "y": 100}, "header": null, "host_id": "6baa326f-7ab8-4d56-a689-9c51c6258b32", "is_infra": false, "position": {"x": 400, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "interface_id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d"}, {"id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5", "size": {"x": 250, "y": 275}, "header": "ha.maya.cloud", "host_id": "309cef94-06eb-4d7b-9b25-90a81789a0e1", "is_infra": true, "position": {"x": 350, "y": 500}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}, {"id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3", "size": {"x": 250, "y": 325}, "header": null, "host_id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "is_infra": true, "position": {"x": 50, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}, {"id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d", "size": {"x": 250, "y": 400}, "header": null, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "is_infra": true, "position": {"x": 350, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}, {"id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd", "size": {"x": 250, "y": 100}, "header": null, "host_id": "45c0f069-fd0a-4ea2-b5c6-00e1f3500e52", "is_infra": false, "position": {"x": 700, "y": 800}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd"}, {"id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897", "size": {"x": 250, "y": 175}, "header": null, "host_id": "5c882c83-72f7-495e-8bdf-71413bd445c3", "is_infra": false, "position": {"x": 700, "y": 575}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897"}, {"id": "4efab86e-7eed-479c-855b-cbb833c75e63", "size": {"x": 250, "y": 175}, "header": null, "host_id": "d166cc9b-c13d-44ec-a0fa-35381ccd961a", "is_infra": false, "position": {"x": 1000, "y": 575}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86", "size": {"x": 250, "y": 175}, "header": null, "host_id": "2493a39c-903a-47b1-8aff-3ae3ad9356fe", "is_infra": false, "position": {"x": 1000, "y": 800}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86"}, {"id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330", "size": {"x": 250, "y": 175}, "header": null, "host_id": "26af0598-31ac-4c8a-80aa-f520cb1ae3a1", "is_infra": false, "position": {"x": 700, "y": 350}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330"}, {"id": "dc97572b-4079-4e4e-a79d-9f15665796bb", "size": {"x": 250, "y": 175}, "header": null, "host_id": "5f409d90-7208-4099-8c6f-77070a1298f6", "is_infra": false, "position": {"x": 1000, "y": 350}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "f653c5f5-637d-455f-aea8-06bc34721af5", "size": {"x": 250, "y": 175}, "header": null, "host_id": "e4a0e35c-d3df-4629-9ce8-aa1e54a9749e", "is_infra": false, "position": {"x": 1300, "y": 350}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "f653c5f5-637d-455f-aea8-06bc34721af5"}, {"id": "ded4ba49-f1b2-48ef-900e-989144101a6d", "size": {"x": 250, "y": 175}, "header": null, "host_id": "e35b7afb-7229-45b1-a3e0-f920406c57f5", "is_infra": false, "position": {"x": 1300, "y": 575}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}, {"id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83", "size": {"x": 250, "y": 250}, "header": null, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "is_infra": false, "position": {"x": 1600, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "18dabc8d-7d81-432e-9489-06939088f005", "size": {"x": 250, "y": 250}, "header": null, "host_id": "66bb7da2-71db-441b-b8e6-ee60eabc4a59", "is_infra": false, "position": {"x": 1300, "y": 800}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "6fb5f173-74bc-4f65-a732-8872038550ab", "size": {"x": 250, "y": 250}, "header": null, "host_id": "5869cd7b-f4b3-4673-914a-e99f5c3c187b", "is_infra": false, "position": {"x": 700, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "e118ed94-a256-4aae-99c2-657528afc582", "size": {"x": 250, "y": 250}, "header": null, "host_id": "d0f8b80a-04f6-4c3a-9477-b25c12cdea16", "is_infra": false, "position": {"x": 1000, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}, {"id": "2089b1d5-759f-4fae-b47b-6707ddf3669e", "size": {"x": 250, "y": 250}, "header": null, "host_id": "1effd89a-40ab-4a44-862b-c3a14db9074c", "is_infra": false, "position": {"x": 1300, "y": 50}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}, {"id": "894ae3a5-689f-4776-82d0-5661bd4074ad", "size": {"x": 250, "y": 275}, "header": "pi.hole", "host_id": "eb13ca22-e581-4a72-b188-fb80b1b4cac4", "is_infra": false, "position": {"x": 1600, "y": 350}, "node_type": "InterfaceNode", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}]	{"local": {"no_fade_edges": false, "hide_edge_types": [], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": [], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[{"id": "fd85da79-5f19-43ba-95bf-b3752c4ba9b5", "name": "Cloudflare DNS", "tags": ["00390626-53aa-4851-bde1-b7d5cbfdbf38"], "ports": [{"id": "1c115fc4-91b2-40c3-9a58-5ccbb70b44fe", "type": "DnsUdp", "number": 53, "protocol": "Udp"}], "hidden": false, "source": {"type": "System"}, "target": {"type": "ServiceBinding", "config": "fe3f4f51-8635-4d04-8663-5fa20e96e577"}, "hostname": "", "services": ["dcf72989-2e9a-483a-82dd-54cf055fccc5"], "created_at": "2025-12-27T16:13:24.582400Z", "interfaces": [{"id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0", "name": "Internet", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "ip_address": "1.1.1.1", "mac_address": null}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:21:40.454347Z", "description": null, "virtualization": null}, {"id": "6baa326f-7ab8-4d56-a689-9c51c6258b32", "name": "Google.com", "tags": ["00390626-53aa-4851-bde1-b7d5cbfdbf38"], "ports": [{"id": "7a76431c-b494-4ba4-93b9-6c8ffadfc053", "type": "Https", "number": 443, "protocol": "Tcp"}], "hidden": false, "source": {"type": "System"}, "target": {"type": "ServiceBinding", "config": "327558cf-6a01-46ac-85f7-40d84c8bf85f"}, "hostname": "", "services": ["9774e951-1df1-4abe-8094-c45095da8138"], "created_at": "2025-12-27T16:13:24.582406Z", "interfaces": [{"id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d", "name": "Internet", "subnet_id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "ip_address": "203.0.113.68", "mac_address": null}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:21:45.013347Z", "description": null, "virtualization": null}, {"id": "560d8a7f-0c0a-478a-b16e-abe06611e797", "name": "Mobile Device", "tags": [], "ports": [{"id": "256ba520-397d-4837-85d0-8775b759944f", "type": "Custom", "number": 0, "protocol": "Tcp"}], "hidden": false, "source": {"type": "System"}, "target": {"type": "ServiceBinding", "config": "994c1eec-0798-40cc-b1b3-fa3d1f88cc47"}, "hostname": null, "services": ["586a6eda-e792-44f5-ad58-a96ad5b1ee06"], "created_at": "2025-12-27T16:13:24.582412Z", "interfaces": [{"id": "74464d04-e4d4-4cdc-8671-0e34423a7eec", "name": "Remote Network", "subnet_id": "ca1bf14c-9af9-4a9f-a9b3-c29f0e798066", "ip_address": "203.0.113.140", "mac_address": null}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:24.600389Z", "description": "A mobile device connecting from a remote network", "virtualization": null}, {"id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "name": "test", "tags": [], "ports": [{"id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "type": "Custom", "number": 60073, "protocol": "Tcp"}, {"id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "type": "PostgreSQL", "number": 5432, "protocol": "Tcp"}, {"id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "type": "Http9000", "number": 9000, "protocol": "Tcp"}, {"id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "type": "Https9443", "number": 9443, "protocol": "Tcp"}, {"id": "67db1b3d-93ee-48cf-9e2a-a98c11f75243", "type": "Custom", "number": 60072, "protocol": "Tcp"}, {"id": "68ecbd7d-b49e-4483-bf59-b476420f54c9", "type": "Http3000", "number": 3000, "protocol": "Tcp"}, {"id": "f4c56874-7052-4df7-b99c-c9c260e6438e", "type": "Http5000", "number": 5000, "protocol": "Tcp"}, {"id": "a469779d-c6d7-4d37-85fc-8f5556b092aa", "type": "Custom", "number": 5173, "protocol": "Tcp"}, {"id": "62d74abe-bc84-46fd-8583-9ed50fbf2f92", "type": "Custom", "number": 7000, "protocol": "Tcp"}, {"id": "fa0529f1-1bb3-41eb-ab82-8316b794ead7", "type": "Http8080", "number": 8080, "protocol": "Tcp"}, {"id": "ca723695-40c3-4e8f-89c6-e3dc4d5e0d5c", "type": "Custom", "number": 56820, "protocol": "Tcp"}, {"id": "25da3a86-a1c7-48f4-8fb0-6817b55a95ed", "type": "Custom", "number": 59869, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:18:19.221518Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:31.382521Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:24.125843Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:06.649164Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:59.543099Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.074377Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.006894Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}, "target": {"type": "None"}, "hostname": "Mayas-MacBook-Pro.local", "services": ["14c87e66-ce12-4aed-a634-b5d344d73f01", "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "f9006309-f2f7-475b-aa03-aea6200af41a", "564eb7bc-2f24-4c05-afc1-530a8a58f155", "f953905a-26cd-4978-b271-49d584248ec3", "51c913fb-31e0-4576-b06d-0a3e48967c9c", "744bac31-2e5a-492b-abb1-c45dd70f5d02", "6e50e416-eca4-4bd9-8289-7bfe23192563", "c905eb81-ce8a-468a-8bb6-69a02c33be59", "da2fc8f8-5351-4242-95ee-46bece8a0e71", "c35744b5-8620-46da-87ef-617f309ac242"], "created_at": "2025-12-27T16:13:56.969502Z", "interfaces": [{"id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83", "name": "en0", "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.200", "mac_address": "34:66:91:EC:6C:2F"}, {"id": "952ba3b9-96fa-4d38-8866-887522dda68e", "name": "utun4", "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "ip_address": "10.0.0.3", "mac_address": null}, {"id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e", "name": "bridge", "subnet_id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "ip_address": "172.17.0.2", "mac_address": "EA:6C:7E:C5:B2:47"}, {"id": "4c298ae7-726e-4b7d-b514-1f26c513579e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.4", "mac_address": "F2:F6:AB:B3:8A:F6"}, {"id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.3", "mac_address": "F2:B4:BF:E9:C8:BF"}, {"id": "d8195316-15bc-4420-8d48-95d7a03f8d9e", "name": "authentik_default", "subnet_id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "ip_address": "172.18.0.2", "mac_address": "A6:49:53:47:E7:4C"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.328606Z", "description": "Scanopy daemon", "virtualization": null}, {"id": "94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9", "name": "10.0.0.1", "tags": [], "ports": [{"id": "7b5bded1-334c-4ce9-a218-f2d2b63fe5c7", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "71a0544f-1eb1-4ff5-aeb1-7ddf421457a7", "type": "Custom", "number": 10086, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:17:00.962110Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["b7a8bbea-2ca2-455b-aaf4-e15a394a8cdf", "8d506cbb-44bf-4262-834f-dd3c55a8c29d"], "created_at": "2025-12-27T16:17:00.962112Z", "interfaces": [{"id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad", "name": null, "subnet_id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "ip_address": "10.0.0.1", "mac_address": null}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:17:00.979976Z", "description": null, "virtualization": null}, {"id": "69dca40d-98a8-424b-8469-2aa9f2ce71fe", "name": "192.168.7.76", "tags": [], "ports": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:17:30.300285Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": [], "created_at": "2025-12-27T16:17:30.300287Z", "interfaces": [{"id": "506701c8-52da-4fdf-b0e8-e657ed303188", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.7.76", "mac_address": "E6:B2:67:46:36:1D"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:17:37.664438Z", "description": null, "virtualization": null}, {"id": "eb13ca22-e581-4a72-b188-fb80b1b4cac4", "name": "pi.hole", "tags": [], "ports": [{"id": "3570d7d6-bfc8-4cb7-bc41-9e55eecdf955", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "63c0a278-5537-4a71-80b2-3cdf716dad50", "type": "DnsTcp", "number": 53, "protocol": "Tcp"}, {"id": "6843343a-7702-4b6f-b079-6753e6e102de", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "51bc1341-266c-4225-9c35-0410d4bccb28", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "ef4e6114-60fa-43be-89d6-6ed213968f6d", "type": "Https", "number": 443, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:16.771837Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "Hostname"}, "hostname": "pi.hole", "services": ["a164ce69-5325-4bff-a24b-7b563005c37f", "a1b0a520-e7a4-4baa-9614-5b837149c8f8", "00b51667-6e22-42fc-a985-0cdd30567eea"], "created_at": "2025-12-27T16:26:16.771841Z", "interfaces": [{"id": "894ae3a5-689f-4776-82d0-5661bd4074ad", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.188", "mac_address": "BC:24:11:72:2E:BC"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:23.519131Z", "description": null, "virtualization": null}, {"id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "name": "Eero Repeater", "tags": [], "ports": [{"id": "c613e0dc-e4f9-4bc7-a871-439070402d3f", "type": "Ntp", "number": 123, "protocol": "Udp"}, {"id": "2736a1b4-6d45-46a4-879f-46f735ad431b", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "902191b5-4c46-422e-b1f0-b21a9771c888", "type": "DnsTcp", "number": 53, "protocol": "Tcp"}, {"id": "5e5a370f-8ea8-462e-b0bf-3e8f03889578", "type": "Custom", "number": 3001, "protocol": "Tcp"}, {"id": "88f8765b-7701-4b2a-a904-5408d75df0e0", "type": "Custom", "number": 10101, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:23.541144Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["fddb2f60-840b-44e5-a179-cb74633ebef3", "bfbbb2bf-cf59-45b7-9339-582cec92860c", "a3a68848-b0a5-4c8d-babc-8eae60c5db55", "cd30c4e4-7163-4219-8a75-11127599fff3"], "created_at": "2025-12-27T16:26:23.541145Z", "interfaces": [{"id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.214", "mac_address": "C4:F1:74:1E:56:32"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:30.269159Z", "description": null, "virtualization": null}, {"id": "309cef94-06eb-4d7b-9b25-90a81789a0e1", "name": "ha.maya.cloud", "tags": [], "ports": [{"id": "32b4b60c-6d8b-4306-af78-bb2dbd932047", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "66b4e5ef-a232-4842-83d5-544746185fc8", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "cf9afd3c-3d81-44a1-93dc-d01afacc8557", "type": "Custom", "number": 81, "protocol": "Tcp"}, {"id": "9e5d060f-6180-4019-8566-78bae229e8ad", "type": "Https", "number": 443, "protocol": "Tcp"}, {"id": "b1e709b9-2d28-41b6-ace0-7696f23806df", "type": "Http3000", "number": 3000, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:31.108404Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "Hostname"}, "hostname": "ha.maya.cloud", "services": ["69d48dbe-840f-40ce-93c9-3ef2b3bcf37d", "6c5fc429-09fd-4841-b1f9-6b46597e8080", "22e8c40e-f48b-47f9-9066-9c70bcfcd20e"], "created_at": "2025-12-27T16:26:31.108406Z", "interfaces": [{"id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.187", "mac_address": "BC:24:11:67:26:8E"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:37.953237Z", "description": null, "virtualization": null}, {"id": "5c882c83-72f7-495e-8bdf-71413bd445c3", "name": "192.168.4.196", "tags": [], "ports": [{"id": "2cd178ba-1657-49c7-b0a8-b236c0e40541", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "735e18b4-6780-4a74-abc7-ecc781fdbb11", "type": "Custom", "number": 8200, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:37.942294Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["aaaa46d2-f808-480d-a636-b1e03560c5b9", "a6f5d5ef-f914-4d12-8da9-756edf4247a7"], "created_at": "2025-12-27T16:26:37.942295Z", "interfaces": [{"id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.196", "mac_address": "BC:24:11:AD:B1:1E"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:51.945318Z", "description": null, "virtualization": null}, {"id": "66bb7da2-71db-441b-b8e6-ee60eabc4a59", "name": "Portainer", "tags": [], "ports": [{"id": "94015005-8c6e-454b-bcf4-f2b491163271", "type": "Http9000", "number": 9000, "protocol": "Tcp"}, {"id": "7110e884-7fe5-4fac-814a-c7172c0fd254", "type": "Custom", "number": 60073, "protocol": "Tcp"}, {"id": "961c3c56-ee02-460f-98c7-0637608c2f06", "type": "Custom", "number": 5355, "protocol": "Tcp"}, {"id": "59a3577d-1dbd-47ec-a46d-1e334bb1f6cf", "type": "Custom", "number": 8000, "protocol": "Tcp"}, {"id": "310ecfea-5467-4b09-8b32-6848a2315c4e", "type": "Https9443", "number": 9443, "protocol": "Tcp"}, {"id": "92706bde-ad98-46f2-ac52-9c0e8c597cf6", "type": "Custom", "number": 60072, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:44.857162Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["0fab6211-dc92-40e2-b7a2-c26135feead3", "591ac6bf-8b30-4d21-9546-5a7972201c2b", "3f12d1f2-a049-438c-9fe3-1eb6116d68f2"], "created_at": "2025-12-27T16:26:44.857162Z", "interfaces": [{"id": "18dabc8d-7d81-432e-9489-06939088f005", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.126", "mac_address": "02:1C:65:32:E5:DD"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:51.944832Z", "description": null, "virtualization": null}, {"id": "d166cc9b-c13d-44ec-a0fa-35381ccd961a", "name": "Sonos Speaker", "tags": [], "ports": [{"id": "6126e930-5ce2-45e6-ad87-22b305a9dddd", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "5e541ad5-dc6f-45ab-9bdc-bab792953000", "type": "Custom", "number": 1410, "protocol": "Tcp"}, {"id": "a8c35dfe-fc1b-4730-8906-33e52a091b2e", "type": "Custom", "number": 1843, "protocol": "Tcp"}, {"id": "599040c3-8e3b-42e2-9b51-29f58671a80d", "type": "Custom", "number": 1443, "protocol": "Tcp"}, {"id": "2e2391a7-2c83-401c-aa99-85db19da8a35", "type": "Custom", "number": 7000, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:51.934278Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["71cd0689-f140-495c-b9b0-d8e0459ebe67", "d2b2704a-1217-48b5-b260-693cdf428383"], "created_at": "2025-12-27T16:26:51.934280Z", "interfaces": [{"id": "4efab86e-7eed-479c-855b-cbb833c75e63", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.212", "mac_address": "38:42:0B:75:F4:86"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:12.986316Z", "description": null, "virtualization": null}, {"id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "name": "Eero Gateway", "tags": [], "ports": [{"id": "12444520-cd55-4dfd-9e33-49eccfc28025", "type": "Dhcp", "number": 67, "protocol": "Udp"}, {"id": "4d667268-c8b4-4ffb-a270-620aec096bf9", "type": "Ntp", "number": 123, "protocol": "Udp"}, {"id": "4ccf7ae9-b1d9-4c20-a02d-a923bef36957", "type": "DnsUdp", "number": 53, "protocol": "Udp"}, {"id": "6181df88-6660-456d-b173-ad44afe71272", "type": "Custom", "number": 3001, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:26:59.009128Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["15c9318d-f1b3-4562-885c-758765ebff5f", "532f00a6-20b0-47bd-8f7d-9a378219ba53", "6ab81963-9786-4a18-a962-92286bf0048e", "00bb6c3d-6dcc-49db-93e5-a9dc419f832f", "8b1d9b37-2d25-4bf7-af87-9e7a109d8074"], "created_at": "2025-12-27T16:26:59.009129Z", "interfaces": [{"id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.1", "mac_address": "C4:F1:74:2F:56:F2"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:12.989830Z", "description": null, "virtualization": null}, {"id": "2493a39c-903a-47b1-8aff-3ae3ad9356fe", "name": "WGDashboard", "tags": [], "ports": [{"id": "65b97cfd-fe2c-436b-ad2a-47e6af338fcc", "type": "Custom", "number": 10086, "protocol": "Tcp"}, {"id": "ed7e75e9-897d-495f-895e-e1f1f2571644", "type": "Ssh", "number": 22, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:05.945791Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["aebfef6c-eb50-40bb-aaf2-00236adefeba", "4be6cb86-9b19-42dd-bbee-7980f2a526d6"], "created_at": "2025-12-27T16:27:05.945792Z", "interfaces": [{"id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.191", "mac_address": "BC:24:11:71:EF:5C"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:12.988225Z", "description": null, "virtualization": null}, {"id": "5869cd7b-f4b3-4673-914a-e99f5c3c187b", "name": "Home Assistant", "tags": [], "ports": [{"id": "7b4593d3-1ff6-42ae-8b2d-73c1f26ba4fc", "type": "Custom", "number": 8123, "protocol": "Tcp"}, {"id": "c7fac658-12ee-4f11-ae4e-96d2cb0a3ac0", "type": "Mqtt", "number": 1883, "protocol": "Tcp"}, {"id": "42721fe8-6996-4676-a1fc-df82d0d69af2", "type": "MqttTls", "number": 8883, "protocol": "Tcp"}, {"id": "ccde549a-6327-4222-8a58-cb95895183db", "type": "Custom", "number": 111, "protocol": "Tcp"}, {"id": "abe76ea6-a278-4ae7-b142-1dbffaeebc97", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "1edde71c-3226-4943-8a12-aa7d706fdfe6", "type": "Custom", "number": 1884, "protocol": "Tcp"}, {"id": "c6f9bb53-aa4f-43a2-b0b3-5247aef7056d", "type": "Custom", "number": 4357, "protocol": "Tcp"}, {"id": "bbb29195-37b0-429f-80e2-e33c564e4436", "type": "Custom", "number": 5355, "protocol": "Tcp"}, {"id": "bc4f160e-0cb3-400f-980c-16e3e1cd82d8", "type": "Custom", "number": 8884, "protocol": "Tcp"}, {"id": "15d9eab1-a273-44b8-bb6c-7f980b83a570", "type": "Custom", "number": 18555, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:12.981236Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["49574957-6e72-4a5a-a1e5-8b7831e2f5e4", "a049934b-5721-46bc-893a-980e356a305e", "9d7df32b-04f8-4134-aab4-705742ad90b3"], "created_at": "2025-12-27T16:27:12.981238Z", "interfaces": [{"id": "6fb5f173-74bc-4f65-a732-8872038550ab", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.181", "mac_address": "BC:24:11:70:13:4A"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:19.996224Z", "description": null, "virtualization": null}, {"id": "26af0598-31ac-4c8a-80aa-f520cb1ae3a1", "name": "Philips Hue Bridge", "tags": [], "ports": [{"id": "4886a04c-d5f8-4002-bdeb-795254c2bff0", "type": "Http", "number": 80, "protocol": "Tcp"}, {"id": "81de39d7-52bc-4061-a7a8-6f4e3541d14a", "type": "Https", "number": 443, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:19.982952Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["cbd06526-ed50-4dbe-8334-c8bb96e80c89", "26c18fb7-ec4d-4781-891e-16c7a9c203db"], "created_at": "2025-12-27T16:27:19.982953Z", "interfaces": [{"id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.20", "mac_address": "00:17:88:A4:BC:BA"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:34.068082Z", "description": null, "virtualization": null}, {"id": "d0f8b80a-04f6-4c3a-9477-b25c12cdea16", "name": "192.168.7.73", "tags": [], "ports": [{"id": "bbccde66-e0f0-486d-bca2-61430451f3a6", "type": "Snmp", "number": 161, "protocol": "Udp"}, {"id": "67c584f5-5923-4ed0-9f17-746504e20993", "type": "LdpTcp", "number": 515, "protocol": "Tcp"}, {"id": "35f25d13-ea1b-4e0b-837f-9fb1b06f163d", "type": "Https", "number": 443, "protocol": "Tcp"}, {"id": "3f2c8a7f-9f32-4056-8f78-722085d9630a", "type": "Http8080", "number": 8080, "protocol": "Tcp"}, {"id": "9f1b71e3-1eb3-4587-bd17-0ad5292b769d", "type": "Custom", "number": 9100, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:27.047969Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["6e917741-64ad-4671-9f8c-0e2aafe5aebb", "4a53777c-82c8-460d-b2d3-04495db9b371", "eabff2d3-f0b0-4ba0-857a-c99a9ae7c12a"], "created_at": "2025-12-27T16:27:27.047970Z", "interfaces": [{"id": "e118ed94-a256-4aae-99c2-657528afc582", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.7.73", "mac_address": "48:EA:62:0D:38:FA"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:34.068829Z", "description": null, "virtualization": null}, {"id": "5f409d90-7208-4099-8c6f-77070a1298f6", "name": "Sonos Speaker", "tags": [], "ports": [{"id": "c1f3c10f-bc86-4900-a32d-46e166f1a0ea", "type": "Custom", "number": 1400, "protocol": "Tcp"}, {"id": "dd6e1edd-09d8-4ac6-ba1e-e0b884a66005", "type": "Custom", "number": 1410, "protocol": "Tcp"}, {"id": "c538bb6a-227a-4bc4-9214-775a3564c89f", "type": "Custom", "number": 1843, "protocol": "Tcp"}, {"id": "c30a7037-3489-4867-9aeb-df460b8c88e7", "type": "Custom", "number": 1443, "protocol": "Tcp"}, {"id": "0de6f147-5990-4547-abc9-82a4e77363a3", "type": "Custom", "number": 7000, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:34.052709Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["4cab006b-6e6e-4464-ade8-f328b3c797a6", "f91d1417-a806-4459-95b7-c8b4fcea48ad"], "created_at": "2025-12-27T16:27:34.052709Z", "interfaces": [{"id": "dc97572b-4079-4e4e-a79d-9f15665796bb", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.29", "mac_address": "34:7E:5C:D3:84:3A"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:48.093791Z", "description": null, "virtualization": null}, {"id": "45c0f069-fd0a-4ea2-b5c6-00e1f3500e52", "name": "192.168.4.30", "tags": [], "ports": [{"id": "72e9da5d-dce0-4c42-ba0a-fc0c40693a38", "type": "Mqtt", "number": 1883, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:41.077575Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["fb4eebc6-dc39-4572-8f10-479d52b50684"], "created_at": "2025-12-27T16:27:41.077576Z", "interfaces": [{"id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.30", "mac_address": "C8:FF:77:17:D8:9D"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:48.093696Z", "description": null, "virtualization": null}, {"id": "6a1525b2-7fdf-4c9a-937b-b868a853ae9a", "name": "192.168.4.213", "tags": [], "ports": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:48.091869Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": [], "created_at": "2025-12-27T16:27:48.091871Z", "interfaces": [{"id": "bfde5c48-01b3-428c-9dee-d3a9b30cd2f6", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.213", "mac_address": "D4:8C:49:F1:55:D0"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:02.243017Z", "description": null, "virtualization": null}, {"id": "e4a0e35c-d3df-4629-9ce8-aa1e54a9749e", "name": "192.168.4.201", "tags": [], "ports": [{"id": "8460a345-d537-4379-af3b-2c222f2e6fae", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "1be344a9-e9fb-4eee-a69e-a41ee32b8fe4", "type": "Custom", "number": 60072, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:27:55.165013Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["5c21b8c7-cd23-4345-865a-ba053c19cd29", "6f21d4c7-e50e-46ad-a202-f8686a761884"], "created_at": "2025-12-27T16:27:55.165014Z", "interfaces": [{"id": "f653c5f5-637d-455f-aea8-06bc34721af5", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.201", "mac_address": "BC:24:11:9E:49:A3"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:09.286815Z", "description": null, "virtualization": null}, {"id": "e35b7afb-7229-45b1-a3e0-f920406c57f5", "name": "192.168.4.146", "tags": [], "ports": [{"id": "83d0dcde-fc79-49f8-b335-e45b21dcac16", "type": "MqttTls", "number": 8883, "protocol": "Tcp"}, {"id": "87f79850-4368-4cda-b498-25c9199b9db5", "type": "Custom", "number": 990, "protocol": "Tcp"}, {"id": "ae6ddc07-ea63-45b4-97fd-a740c448e388", "type": "Http3000", "number": 3000, "protocol": "Tcp"}, {"id": "9477e380-fcdc-44c3-a9b4-7abd8e3c870e", "type": "Custom", "number": 6000, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:28:02.234772Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["3089bd19-4bf9-4865-8cd9-60c97799c33b", "efff2903-53f4-46cb-b46a-b93277685be8"], "created_at": "2025-12-27T16:28:02.234773Z", "interfaces": [{"id": "ded4ba49-f1b2-48ef-900e-989144101a6d", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.146", "mac_address": "DC:DA:0C:28:A8:FC"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:16.348285Z", "description": null, "virtualization": null}, {"id": "1effd89a-40ab-4a44-862b-c3a14db9074c", "name": "Proxmox VE", "tags": [], "ports": [{"id": "0a395d11-abb5-40e7-9940-e0df455bfa1b", "type": "Custom", "number": 8006, "protocol": "Tcp"}, {"id": "d7c195bf-5726-4423-9d3c-6bd45bf89ca7", "type": "Ssh", "number": 22, "protocol": "Tcp"}, {"id": "87217851-d7f2-4ac8-aa19-ed2f081ff75e", "type": "Custom", "number": 3128, "protocol": "Tcp"}], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:28:09.277990Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "target": {"type": "None"}, "hostname": null, "services": ["d311be56-1dd0-40c6-b7c7-0bee84cc1bde", "e25cba30-c570-462e-a535-1281d3e7317f", "50e66c44-301d-4331-9030-47ff6e746a00"], "created_at": "2025-12-27T16:28:09.277991Z", "interfaces": [{"id": "2089b1d5-759f-4fae-b47b-6707ddf3669e", "name": null, "subnet_id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "ip_address": "192.168.4.135", "mac_address": "E8:FF:1E:D0:CE:A6"}], "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:16.348497Z", "description": null, "virtualization": null}]	[{"id": "8579ce39-369f-44b1-84ac-7cd21dee9bdd", "cidr": "0.0.0.0/0", "name": "Internet", "tags": [], "source": {"type": "System"}, "created_at": "2025-12-27T16:13:24.582359Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:24.582359Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for services running on the internet (e.g., public DNS servers, cloud services, etc.).", "subnet_type": "Internet"}, {"id": "ca1bf14c-9af9-4a9f-a9b3-c29f0e798066", "cidr": "0.0.0.0/0", "name": "Remote Network", "tags": [], "source": {"type": "System"}, "created_at": "2025-12-27T16:13:24.582360Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:24.582360Z", "description": "This subnet uses the 0.0.0.0/0 CIDR as an organizational container for hosts on remote networks (e.g., mobile connections, friend's networks, public WiFi, etc.).", "subnet_type": "Remote"}, {"id": "e5b4263e-1964-439d-bcb8-2cbb11018742", "cidr": "192.168.4.0/22", "name": "192.168.4.0/22", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:56.988655Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}, "created_at": "2025-12-27T16:13:56.988656Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:56.988656Z", "description": null, "subnet_type": "Lan"}, {"id": "f1a114ad-b7c5-4146-8975-f772be3986d9", "cidr": "10.0.0.0/24", "name": "10.0.0.0/24", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:56.988698Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}, "created_at": "2025-12-27T16:13:56.988698Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:56.988698Z", "description": null, "subnet_type": "VpnTunnel"}, {"id": "fa8903e6-fe1f-4e65-b169-75df44e72c5e", "cidr": "172.17.0.0/16", "name": "bridge", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:57.065306Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "created_at": "2025-12-27T16:13:57.065307Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:57.065307Z", "description": null, "subnet_type": "DockerBridge"}, {"id": "8f8917b5-02b6-4998-8efe-42c6dedf0029", "cidr": "172.18.0.0/16", "name": "authentik_default", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2025-12-27T16:13:57.065315Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "created_at": "2025-12-27T16:13:57.065315Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:57.065315Z", "description": null, "subnet_type": "DockerBridge"}]	[{"id": "dcf72989-2e9a-483a-82dd-54cf055fccc5", "name": "Cloudflare DNS", "tags": [], "source": {"type": "System"}, "host_id": "fd85da79-5f19-43ba-95bf-b3752c4ba9b5", "bindings": [{"id": "fe3f4f51-8635-4d04-8663-5fa20e96e577", "type": "Port", "port_id": "1c115fc4-91b2-40c3-9a58-5ccbb70b44fe", "interface_id": "98d6b2a6-b0e2-4ae7-bbb8-851324181da0"}], "created_at": "2025-12-27T16:13:24.582402Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:21:40.452554Z", "virtualization": null, "service_definition": "Dns Server"}, {"id": "9774e951-1df1-4abe-8094-c45095da8138", "name": "Google.com", "tags": [], "source": {"type": "System"}, "host_id": "6baa326f-7ab8-4d56-a689-9c51c6258b32", "bindings": [{"id": "327558cf-6a01-46ac-85f7-40d84c8bf85f", "type": "Port", "port_id": "7a76431c-b494-4ba4-93b9-6c8ffadfc053", "interface_id": "24a85229-bc3f-44a5-8cbe-5c9af6c3aa6d"}], "created_at": "2025-12-27T16:13:24.582408Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:21:45.010782Z", "virtualization": null, "service_definition": "Web Service"}, {"id": "586a6eda-e792-44f5-ad58-a96ad5b1ee06", "name": "Mobile Device", "tags": [], "source": {"type": "System"}, "host_id": "560d8a7f-0c0a-478a-b16e-abe06611e797", "bindings": [{"id": "994c1eec-0798-40cc-b1b3-fa3d1f88cc47", "type": "Port", "port_id": "256ba520-397d-4837-85d0-8775b759944f", "interface_id": "74464d04-e4d4-4cdc-8671-0e34423a7eec"}], "created_at": "2025-12-27T16:13:24.582413Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:24.582413Z", "virtualization": null, "service_definition": "Client"}, {"id": "14c87e66-ce12-4aed-a634-b5d344d73f01", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-27T16:18:19.268486Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:13:57.006903Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "3c4b02f4-4fe1-480f-9706-de761c9861cb", "type": "Port", "port_id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "8f939956-de04-4eb0-ae96-ef1f2cb75911", "type": "Port", "port_id": "faed2fab-2502-40d4-85e0-368a7e4592b6", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}], "created_at": "2025-12-27T16:13:57.006904Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.320024Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "name": "Docker", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Docker daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2025-12-27T16:13:57.074372Z", "type": "SelfReport", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [], "created_at": "2025-12-27T16:13:57.074372Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:13:57.074372Z", "virtualization": null, "service_definition": "Docker"}, {"id": "f9006309-f2f7-475b-aa03-aea6200af41a", "name": "scanopy-postgres", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:18:19.292462Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:06.601567Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "c58fee2e-df2d-4d04-b1b0-7dd9fc373dc1", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "cfd549dc-f79e-4ac5-8c88-9010a48f7f4e"}, {"id": "d6be78e2-88fc-429f-ad0b-276f359d5b05", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "44d3781a-fc59-46ac-8287-6c6ebf37db4f", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}], "created_at": "2025-12-27T16:14:06.601579Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.318809Z", "virtualization": {"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "837f73c8d8e4545da7daa7968c022df9414eb032a35b065be2ba2aefcc565db1", "container_name": "scanopy-postgres"}}, "service_definition": "PostgreSQL"}, {"id": "564eb7bc-2f24-4c05-afc1-530a8a58f155", "name": "authentik-postgresql-1", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:14:14.027350Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "ec074984-3d3a-4bc5-8c51-99d482d6a7af", "type": "Port", "port_id": "7ca99435-9f91-4a63-9e81-f9a5f73cf3b4", "interface_id": "4c298ae7-726e-4b7d-b514-1f26c513579e"}], "created_at": "2025-12-27T16:14:14.027365Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:14:14.027365Z", "virtualization": {"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "07bde47b05aa71b7b7f4d01c3f872534e24775c5bc53a5d26bd5845c26017c22", "container_name": "authentik-postgresql-1"}}, "service_definition": "PostgreSQL"}, {"id": "f953905a-26cd-4978-b271-49d584248ec3", "name": "Authentik", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 172.18.0.3:9000/ contained \\"window.authentik\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:18:19.260768Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}, {"date": "2025-12-27T16:14:27.361467Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "5c04dbec-5cf4-4691-a316-da48c35eb5a3", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e"}, {"id": "23d35772-b12c-439f-a0dc-c955ed3a644c", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "bace3ac0-527f-4bed-82e6-cc627bbf8e3e"}, {"id": "7dabb720-4188-4a5b-9986-d437215bb15d", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "51c19ca1-7cd0-4657-b9b0-6913b22c6333", "type": "Port", "port_id": "f9336fe5-ae3e-4df3-82bf-ae417c057c6d", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "6e9028c1-5f56-45f4-8dc3-4f30f6217cba", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "df4edee9-c5fd-4da8-bfc5-7a2138f48c83"}, {"id": "c400df4b-8f1e-4d10-9da2-6a9d6cb775ad", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}], "created_at": "2025-12-27T16:14:27.361481Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.319280Z", "virtualization": {"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "46960b2d2d4ef00adb65b188d7ff2648849293cda661943ea0e9719f532116f8", "container_name": "authentik-server-1"}}, "service_definition": "Authentik"}, {"id": "51c913fb-31e0-4576-b06d-0a3e48967c9c", "name": "authentik-worker-1", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["All of", [{"data": "Service is running in docker container", "type": "reason"}, {"data": "No other services with this container's ID have been matched", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:14:38.348910Z", "type": "Docker", "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "66eb6941-2cf7-41e1-b904-e73a005de9ea", "type": "Interface", "interface_id": "d8195316-15bc-4420-8d48-95d7a03f8d9e"}], "created_at": "2025-12-27T16:14:38.348922Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:14:38.348922Z", "virtualization": {"type": "Docker", "details": {"service_id": "b219d4a5-bdfe-44b0-8852-443daff2e5dc", "container_id": "76cb5435cb2178b9c137baf9dfee927fec2d44d7f867e2ee7607decf444417aa", "container_name": "authentik-worker-1"}}, "service_definition": "Docker Container"}, {"id": "b7a8bbea-2ca2-455b-aaf4-e15a394a8cdf", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:17:00.962925Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9", "bindings": [{"id": "17925ad3-65c5-40b6-8033-b8cc768ba6ad", "type": "Port", "port_id": "7b5bded1-334c-4ce9-a218-f2d2b63fe5c7", "interface_id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad"}], "created_at": "2025-12-27T16:17:00.962932Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:17:00.962932Z", "virtualization": null, "service_definition": "SSH"}, {"id": "8d506cbb-44bf-4262-834f-dd3c55a8c29d", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:17:00.962958Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "94685d04-6a4c-4b7a-9ba2-bfbb9b8d9fb9", "bindings": [{"id": "cef96d13-ef23-4f21-91ad-76976708efe6", "type": "Port", "port_id": "71a0544f-1eb1-4ff5-aeb1-7ddf421457a7", "interface_id": "cb4acefb-9624-4d8d-bcd8-f4f4f4c8cbad"}], "created_at": "2025-12-27T16:17:00.962960Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:17:00.962960Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "c905eb81-ce8a-468a-8bb6-69a02c33be59", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 10.0.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:18:19.280072Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "a52e96ba-c82e-4d59-8bf7-9cc42e63bd94", "type": "Port", "port_id": "67db1b3d-93ee-48cf-9e2a-a98c11f75243", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}], "created_at": "2025-12-27T16:18:19.280080Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.280080Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "c35744b5-8620-46da-87ef-617f309ac242", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:18:19.292493Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "cb0f322b-28a0-4dd9-9cc6-7570d79e23fb", "bindings": [{"id": "be4cc988-f3c3-44c2-a080-e7461bb5e473", "type": "Port", "port_id": "68ecbd7d-b49e-4483-bf59-b476420f54c9", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "5a3da085-bd5f-4b40-afba-b6d2c996c77f", "type": "Port", "port_id": "f4c56874-7052-4df7-b99c-c9c260e6438e", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "5622395f-3bd0-4b2d-b456-5510b88ecb9c", "type": "Port", "port_id": "a469779d-c6d7-4d37-85fc-8f5556b092aa", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "a792c13c-95a4-4d9d-99cc-ddfe9bffe6a4", "type": "Port", "port_id": "62d74abe-bc84-46fd-8583-9ed50fbf2f92", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "eeab16a5-a85b-45d1-9a59-50ba6ad284b4", "type": "Port", "port_id": "fa0529f1-1bb3-41eb-ab82-8316b794ead7", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "478264e3-3783-4844-8085-b77a67d42524", "type": "Port", "port_id": "9da0b4b1-c614-43f2-8579-4216f4edcc6c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "9f27ce90-4bd6-4ba1-90a8-644d4b3b8a9b", "type": "Port", "port_id": "ca723695-40c3-4e8f-89c6-e3dc4d5e0d5c", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}, {"id": "1661bc4d-ca02-4a34-a62c-1173b87d7547", "type": "Port", "port_id": "25da3a86-a1c7-48f4-8fb0-6817b55a95ed", "interface_id": "952ba3b9-96fa-4d38-8866-887522dda68e"}], "created_at": "2025-12-27T16:18:19.292508Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:18:19.292508Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "a164ce69-5325-4bff-a24b-7b563005c37f", "name": "Pi-Hole", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": ["Any of", [{"data": "Port 53/udp is open", "type": "reason"}, {"data": "Port 53/tcp is open", "type": "reason"}]], "type": "container"}, {"data": "Response for 192.168.4.188:80/admin contained \\"pi-hole\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:18.806867Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "eb13ca22-e581-4a72-b188-fb80b1b4cac4", "bindings": [{"id": "c4af949c-fce9-404f-9819-6608a02fe269", "type": "Port", "port_id": "3570d7d6-bfc8-4cb7-bc41-9e55eecdf955", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}, {"id": "12b49c36-0866-41b1-9fde-b051e72681a4", "type": "Port", "port_id": "63c0a278-5537-4a71-80b2-3cdf716dad50", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}, {"id": "61f5ebdc-8046-44a3-9b3c-bf5dc89837e4", "type": "Port", "port_id": "6843343a-7702-4b6f-b079-6753e6e102de", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}], "created_at": "2025-12-27T16:26:18.806881Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:18.806881Z", "virtualization": null, "service_definition": "Pi-Hole"}, {"id": "a1b0a520-e7a4-4baa-9614-5b837149c8f8", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:23.506037Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "eb13ca22-e581-4a72-b188-fb80b1b4cac4", "bindings": [{"id": "bcfbf890-052e-409f-9b2e-4a799336b661", "type": "Port", "port_id": "51bc1341-266c-4225-9c35-0410d4bccb28", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}], "created_at": "2025-12-27T16:26:23.506048Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:23.506048Z", "virtualization": null, "service_definition": "SSH"}, {"id": "00b51667-6e22-42fc-a985-0cdd30567eea", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:23.506183Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "eb13ca22-e581-4a72-b188-fb80b1b4cac4", "bindings": [{"id": "30fa1f62-5b4e-4a0d-9587-1484ed7308b5", "type": "Port", "port_id": "ef4e6114-60fa-43be-89d6-6ed213968f6d", "interface_id": "894ae3a5-689f-4776-82d0-5661bd4074ad"}], "created_at": "2025-12-27T16:26:23.506187Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:23.506187Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "fddb2f60-840b-44e5-a179-cb74633ebef3", "name": "Eero Repeater", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor eero Inc", "type": "reason"}, {"data": "IP address is not in routing table, and does not end in 1 or 254 with no other gateways identified in subnet", "type": "reason"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:26:26.896299Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "bindings": [{"id": "d532969f-6564-4388-a8ca-20d86d8997a9", "type": "Interface", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}], "created_at": "2025-12-27T16:26:26.896309Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:26.896309Z", "virtualization": null, "service_definition": "Eero Repeater"}, {"id": "bfbbb2bf-cf59-45b7-9339-582cec92860c", "name": "NTP Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 123/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251578Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "bindings": [{"id": "76cbf088-755b-4d16-beef-1e5c8386e4de", "type": "Port", "port_id": "c613e0dc-e4f9-4bc7-a871-439070402d3f", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}], "created_at": "2025-12-27T16:26:30.251589Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:30.251589Z", "virtualization": null, "service_definition": "NTP Server"}, {"id": "a3a68848-b0a5-4c8d-babc-8eae60c5db55", "name": "Dns Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 53/tcp is open", "type": "reason"}, {"data": "Port 53/udp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251828Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "bindings": [{"id": "7304480e-a2d9-4baf-b3b0-f268d749ebfb", "type": "Port", "port_id": "2736a1b4-6d45-46a4-879f-46f735ad431b", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}, {"id": "649e19fb-1532-4eb8-8c61-da747ef91e23", "type": "Port", "port_id": "902191b5-4c46-422e-b1f0-b21a9771c888", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}], "created_at": "2025-12-27T16:26:30.251834Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:30.251834Z", "virtualization": null, "service_definition": "Dns Server"}, {"id": "cd30c4e4-7163-4219-8a75-11127599fff3", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:30.251862Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "4777c89f-fe55-4baf-9d17-2d6f70b696f0", "bindings": [{"id": "84c3d3a2-9120-463d-8694-55b8fc9607a4", "type": "Port", "port_id": "5e5a370f-8ea8-462e-b0bf-3e8f03889578", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}, {"id": "eaa95a2a-ed28-44a0-9c47-95a4304768bf", "type": "Port", "port_id": "88f8765b-7701-4b2a-a904-5408d75df0e0", "interface_id": "ea6f344d-39ec-46b5-92d8-7d1c39ddacf3"}], "created_at": "2025-12-27T16:26:30.251867Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:30.251867Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "69d48dbe-840f-40ce-93c9-3ef2b3bcf37d", "name": "Nginx Proxy Manager", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.187:80 contained \\"nginx proxy manager\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:34.222928Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "309cef94-06eb-4d7b-9b25-90a81789a0e1", "bindings": [{"id": "0859d540-90fe-4fcf-a81c-d0caa64f2a61", "type": "Port", "port_id": "32b4b60c-6d8b-4306-af78-bb2dbd932047", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}], "created_at": "2025-12-27T16:26:34.222940Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:34.222940Z", "virtualization": null, "service_definition": "Nginx Proxy Manager"}, {"id": "6c5fc429-09fd-4841-b1f9-6b46597e8080", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:37.941538Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "309cef94-06eb-4d7b-9b25-90a81789a0e1", "bindings": [{"id": "f7a7ce0d-c3c3-4825-a400-9c66447fe5de", "type": "Port", "port_id": "66b4e5ef-a232-4842-83d5-544746185fc8", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}], "created_at": "2025-12-27T16:26:37.941549Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:37.941549Z", "virtualization": null, "service_definition": "SSH"}, {"id": "22e8c40e-f48b-47f9-9066-9c70bcfcd20e", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:37.941587Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "309cef94-06eb-4d7b-9b25-90a81789a0e1", "bindings": [{"id": "0d6a649d-9fb0-420e-b4df-445b9f92948d", "type": "Port", "port_id": "cf9afd3c-3d81-44a1-93dc-d01afacc8557", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}, {"id": "517354fd-954b-4560-9512-c9727a6e330f", "type": "Port", "port_id": "9e5d060f-6180-4019-8566-78bae229e8ad", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}, {"id": "52ca5b02-783f-4600-a344-85b78b6bccc8", "type": "Port", "port_id": "b1e709b9-2d28-41b6-ace0-7696f23806df", "interface_id": "bde344cb-28ab-4b30-9ad6-5b54dea762d5"}], "created_at": "2025-12-27T16:26:37.941595Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:37.941595Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "aaaa46d2-f808-480d-a636-b1e03560c5b9", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:44.856886Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5c882c83-72f7-495e-8bdf-71413bd445c3", "bindings": [{"id": "c12300ab-2cd7-401e-9350-79f8f34986df", "type": "Port", "port_id": "2cd178ba-1657-49c7-b0a8-b236c0e40541", "interface_id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897"}], "created_at": "2025-12-27T16:26:44.856897Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:44.856897Z", "virtualization": null, "service_definition": "SSH"}, {"id": "a6f5d5ef-f914-4d12-8da9-756edf4247a7", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:44.856920Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5c882c83-72f7-495e-8bdf-71413bd445c3", "bindings": [{"id": "5f200e72-30ee-4341-8174-2257e4aafacf", "type": "Port", "port_id": "735e18b4-6780-4a74-abc7-ecc781fdbb11", "interface_id": "b7b4ae30-6378-4604-9dc2-ec633bfb9897"}], "created_at": "2025-12-27T16:26:44.856922Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:44.856922Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "0fab6211-dc92-40e2-b7a2-c26135feead3", "name": "Portainer", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 192.168.4.126:9000/ contained \\"portainer.io\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:48.110242Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "66bb7da2-71db-441b-b8e6-ee60eabc4a59", "bindings": [{"id": "a105609c-426e-4bf7-ad47-1773715bbcb7", "type": "Port", "port_id": "94015005-8c6e-454b-bcf4-f2b491163271", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}], "created_at": "2025-12-27T16:26:48.110255Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:48.110255Z", "virtualization": null, "service_definition": "Portainer"}, {"id": "591ac6bf-8b30-4d21-9546-5a7972201c2b", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.126:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:26:49.853130Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "66bb7da2-71db-441b-b8e6-ee60eabc4a59", "bindings": [{"id": "c7b35cc7-e99f-493e-b869-e1b5c7763705", "type": "Port", "port_id": "7110e884-7fe5-4fac-814a-c7172c0fd254", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}], "created_at": "2025-12-27T16:26:49.853142Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:49.853142Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "3f12d1f2-a049-438c-9fe3-1eb6116d68f2", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:51.927109Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "66bb7da2-71db-441b-b8e6-ee60eabc4a59", "bindings": [{"id": "5708f030-7364-47e5-aa8a-63d915554453", "type": "Port", "port_id": "961c3c56-ee02-460f-98c7-0637608c2f06", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "8522b641-f704-4e91-aec5-1ab75032c6fe", "type": "Port", "port_id": "59a3577d-1dbd-47ec-a46d-1e334bb1f6cf", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "ec1aa41c-edaa-4c40-80a8-5e8bbbf6754b", "type": "Port", "port_id": "310ecfea-5467-4b09-8b32-6848a2315c4e", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}, {"id": "fbdbdb0e-836b-473c-ae41-3f2cc702d052", "type": "Port", "port_id": "92706bde-ad98-46f2-ac52-9c0e8c597cf6", "interface_id": "18dabc8d-7d81-432e-9489-06939088f005"}], "created_at": "2025-12-27T16:26:51.927125Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:51.927125Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "71cd0689-f140-495c-b9b0-d8e0459ebe67", "name": "Sonos Speaker", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Sonos, Inc", "type": "reason"}, {"data": ["Any of", [{"data": "Port 1400/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1410/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1843/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:26:53.318813Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "d166cc9b-c13d-44ec-a0fa-35381ccd961a", "bindings": [{"id": "0407827a-769d-49b0-a5a5-23dac9065485", "type": "Port", "port_id": "6126e930-5ce2-45e6-ad87-22b305a9dddd", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "0986f130-f754-47ec-9a1e-c9f3af84a268", "type": "Port", "port_id": "5e541ad5-dc6f-45ab-9bdc-bab792953000", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "61a84d03-973b-46d3-b48f-29047072e976", "type": "Port", "port_id": "a8c35dfe-fc1b-4730-8906-33e52a091b2e", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}], "created_at": "2025-12-27T16:26:53.318827Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:53.318827Z", "virtualization": null, "service_definition": "Sonos Speaker"}, {"id": "d2b2704a-1217-48b5-b260-693cdf428383", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:26:59.008837Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "d166cc9b-c13d-44ec-a0fa-35381ccd961a", "bindings": [{"id": "44c2aea3-a689-44f9-8920-aee70bf52799", "type": "Port", "port_id": "599040c3-8e3b-42e2-9b51-29f58671a80d", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}, {"id": "05669fc0-fab0-4e64-a2fa-3861b8fde23f", "type": "Port", "port_id": "2e2391a7-2c83-401c-aa99-85db19da8a35", "interface_id": "4efab86e-7eed-479c-855b-cbb833c75e63"}], "created_at": "2025-12-27T16:26:59.008850Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:26:59.008850Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "15c9318d-f1b3-4562-885c-758765ebff5f", "name": "Eero Gateway", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor eero Inc", "type": "reason"}, {"data": "Host IP address is in routing table of daemon 7dc85a03-c94f-42d9-b52f-13808fbe88d7", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:00.737916Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "bindings": [{"id": "80d41d8a-8ade-4886-b8ac-6391e7c9c1da", "type": "Interface", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}], "created_at": "2025-12-27T16:27:00.737926Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:00.737926Z", "virtualization": null, "service_definition": "Eero Gateway"}, {"id": "532f00a6-20b0-47bd-8f7d-9a378219ba53", "name": "Dhcp Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 67/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945139Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "bindings": [{"id": "9039765a-fcdd-47cc-b0bd-a5ccad4b4570", "type": "Port", "port_id": "12444520-cd55-4dfd-9e33-49eccfc28025", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}], "created_at": "2025-12-27T16:27:05.945151Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:05.945151Z", "virtualization": null, "service_definition": "Dhcp Server"}, {"id": "6ab81963-9786-4a18-a962-92286bf0048e", "name": "NTP Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 123/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945283Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "bindings": [{"id": "2915315f-b73f-4d46-81df-d5d815e3be96", "type": "Port", "port_id": "4d667268-c8b4-4ffb-a270-620aec096bf9", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}], "created_at": "2025-12-27T16:27:05.945288Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:05.945288Z", "virtualization": null, "service_definition": "NTP Server"}, {"id": "00bb6c3d-6dcc-49db-93e5-a9dc419f832f", "name": "Dns Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 53/udp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945420Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "bindings": [{"id": "4ea23655-81f1-4f04-a488-ca536a8713f8", "type": "Port", "port_id": "4ccf7ae9-b1d9-4c20-a02d-a923bef36957", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}], "created_at": "2025-12-27T16:27:05.945423Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:05.945423Z", "virtualization": null, "service_definition": "Dns Server"}, {"id": "8b1d9b37-2d25-4bf7-af87-9e7a109d8074", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:05.945449Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "896e4f49-f7d2-45b9-854f-c3e4f35997da", "bindings": [{"id": "5228645c-2fd4-4e44-8c0d-d019ce050974", "type": "Port", "port_id": "6181df88-6660-456d-b173-ad44afe71272", "interface_id": "b47b4eb1-1e7b-4c4e-a952-43242b7af17d"}], "created_at": "2025-12-27T16:27:05.945452Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:05.945452Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "aebfef6c-eb50-40bb-aaf2-00236adefeba", "name": "WGDashboard", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Port 10086/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Subnet 192.168.4.0/22 is not type VPN", "type": "reason"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:27:09.121730Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "2493a39c-903a-47b1-8aff-3ae3ad9356fe", "bindings": [{"id": "c7b31f1e-4a43-455a-b70c-7235bbb86420", "type": "Port", "port_id": "65b97cfd-fe2c-436b-ad2a-47e6af338fcc", "interface_id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86"}], "created_at": "2025-12-27T16:27:09.121743Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:09.121743Z", "virtualization": null, "service_definition": "WGDashboard"}, {"id": "4be6cb86-9b19-42dd-bbee-7980f2a526d6", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:12.969035Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "2493a39c-903a-47b1-8aff-3ae3ad9356fe", "bindings": [{"id": "e1853a3f-1bef-4674-8b94-6d1b62ddbf24", "type": "Port", "port_id": "ed7e75e9-897d-495f-895e-e1f1f2571644", "interface_id": "97e0d45a-29cb-4e7f-8083-4b54b90f2d86"}], "created_at": "2025-12-27T16:27:12.969048Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:12.969048Z", "virtualization": null, "service_definition": "SSH"}, {"id": "49574957-6e72-4a5a-a1e5-8b7831e2f5e4", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 192.168.4.181:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:17.886775Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5869cd7b-f4b3-4673-914a-e99f5c3c187b", "bindings": [{"id": "f15de529-5514-421f-a415-d65146ac62a8", "type": "Port", "port_id": "7b4593d3-1ff6-42ae-8b2d-73c1f26ba4fc", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}], "created_at": "2025-12-27T16:27:17.886791Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:17.886791Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "a049934b-5721-46bc-893a-980e356a305e", "name": "MQTT", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 1883/tcp is open", "type": "reason"}, {"data": "Port 8883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:19.982461Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5869cd7b-f4b3-4673-914a-e99f5c3c187b", "bindings": [{"id": "2f97b92c-8180-4a7e-bb9e-8c68e7ce094d", "type": "Port", "port_id": "c7fac658-12ee-4f11-ae4e-96d2cb0a3ac0", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6f407699-65dc-41db-b0d8-99ecb27e7c6a", "type": "Port", "port_id": "42721fe8-6996-4676-a1fc-df82d0d69af2", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}], "created_at": "2025-12-27T16:27:19.982478Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:19.982478Z", "virtualization": null, "service_definition": "MQTT"}, {"id": "9d7df32b-04f8-4134-aab4-705742ad90b3", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:19.982516Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5869cd7b-f4b3-4673-914a-e99f5c3c187b", "bindings": [{"id": "60caa50b-16ac-4bc3-8571-7ccbf94244a1", "type": "Port", "port_id": "ccde549a-6327-4222-8a58-cb95895183db", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "1a00fc8f-2ef8-4ed2-aadc-5f6b9d5ed8d8", "type": "Port", "port_id": "abe76ea6-a278-4ae7-b142-1dbffaeebc97", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6f52c33c-0760-442c-b961-2b087cf01222", "type": "Port", "port_id": "1edde71c-3226-4943-8a12-aa7d706fdfe6", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6acb906e-2faa-490f-ae59-b07880cfd931", "type": "Port", "port_id": "c6f9bb53-aa4f-43a2-b0b3-5247aef7056d", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "aa05cb69-b1ab-4aa4-95d9-4ccd1d151f6c", "type": "Port", "port_id": "bbb29195-37b0-429f-80e2-e33c564e4436", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "6833d4a8-3518-4088-83f3-08d5e14e5e30", "type": "Port", "port_id": "bc4f160e-0cb3-400f-980c-16e3e1cd82d8", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}, {"id": "28d14070-4394-4c65-a527-031b5e75b61a", "type": "Port", "port_id": "15d9eab1-a273-44b8-bb6c-7f980b83a570", "interface_id": "6fb5f173-74bc-4f65-a732-8872038550ab"}], "created_at": "2025-12-27T16:27:19.982534Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:19.982534Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "cbd06526-ed50-4dbe-8334-c8bb96e80c89", "name": "Philips Hue Bridge", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Philips Lighting BV", "type": "reason"}, {"data": "Response for 192.168.4.20:80/ contained \\"hue\\" in body", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:27:24.270393Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "26af0598-31ac-4c8a-80aa-f520cb1ae3a1", "bindings": [{"id": "060ab0a7-4f85-4120-a817-47f2ca836776", "type": "Port", "port_id": "4886a04c-d5f8-4002-bdeb-795254c2bff0", "interface_id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330"}], "created_at": "2025-12-27T16:27:24.270406Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:24.270406Z", "virtualization": null, "service_definition": "Philips Hue Bridge"}, {"id": "26c18fb7-ec4d-4781-891e-16c7a9c203db", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:27.047692Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "26af0598-31ac-4c8a-80aa-f520cb1ae3a1", "bindings": [{"id": "fd296caf-5e0c-464b-bfa6-81ca6d7c1a58", "type": "Port", "port_id": "81de39d7-52bc-4061-a7a8-6f4e3541d14a", "interface_id": "3b01aceb-3d7c-4d06-ab3d-fc136363f330"}], "created_at": "2025-12-27T16:27:27.047704Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:27.047704Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "6e917741-64ad-4671-9f8c-0e2aafe5aebb", "name": "SNMP", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 161/udp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049759Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "d0f8b80a-04f6-4c3a-9477-b25c12cdea16", "bindings": [{"id": "76d5a39c-324d-42de-9642-0749f22a4838", "type": "Port", "port_id": "bbccde66-e0f0-486d-bca2-61430451f3a6", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}], "created_at": "2025-12-27T16:27:34.049770Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:34.049770Z", "virtualization": null, "service_definition": "SNMP"}, {"id": "4a53777c-82c8-460d-b2d3-04495db9b371", "name": "Print Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 515/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049917Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "d0f8b80a-04f6-4c3a-9477-b25c12cdea16", "bindings": [{"id": "8a440fae-d24b-4900-9455-4653c36260c1", "type": "Port", "port_id": "67c584f5-5923-4ed0-9f17-746504e20993", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}], "created_at": "2025-12-27T16:27:34.049920Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:34.049920Z", "virtualization": null, "service_definition": "Print Server"}, {"id": "eabff2d3-f0b0-4ba0-857a-c99a9ae7c12a", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:34.049956Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "d0f8b80a-04f6-4c3a-9477-b25c12cdea16", "bindings": [{"id": "dbddfa4e-0473-456c-9c19-417a5fea9e42", "type": "Port", "port_id": "35f25d13-ea1b-4e0b-837f-9fb1b06f163d", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}, {"id": "6722a116-9452-4087-ba8c-6b584ac80594", "type": "Port", "port_id": "3f2c8a7f-9f32-4056-8f78-722085d9630a", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}, {"id": "391c3046-9c39-48b3-9840-394ae8847cd2", "type": "Port", "port_id": "9f1b71e3-1eb3-4587-bd17-0ad5292b769d", "interface_id": "e118ed94-a256-4aae-99c2-657528afc582"}], "created_at": "2025-12-27T16:27:34.049962Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:34.049962Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "4cab006b-6e6e-4464-ade8-f328b3c797a6", "name": "Sonos Speaker", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["All of", [{"data": "Mac address is from vendor Sonos, Inc", "type": "reason"}, {"data": ["Any of", [{"data": "Port 1400/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1410/tcp is open and is not used in other service match patterns", "type": "reason"}, {"data": "Port 1843/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "Medium"}, "metadata": [{"date": "2025-12-27T16:27:35.471785Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5f409d90-7208-4099-8c6f-77070a1298f6", "bindings": [{"id": "08d37d76-5500-4708-be58-7e15cc4292a4", "type": "Port", "port_id": "c1f3c10f-bc86-4900-a32d-46e166f1a0ea", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "3b6be90c-47cb-4630-b831-12de30ce1f49", "type": "Port", "port_id": "dd6e1edd-09d8-4ac6-ba1e-e0b884a66005", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "30c763dd-8d9b-4266-bcc8-913e5b35209b", "type": "Port", "port_id": "c538bb6a-227a-4bc4-9214-775a3564c89f", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}], "created_at": "2025-12-27T16:27:35.471800Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:35.471800Z", "virtualization": null, "service_definition": "Sonos Speaker"}, {"id": "f91d1417-a806-4459-95b7-c8b4fcea48ad", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:41.077294Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "5f409d90-7208-4099-8c6f-77070a1298f6", "bindings": [{"id": "797fdd24-6a6b-4e0c-aec2-cc96c77a8ff5", "type": "Port", "port_id": "c30a7037-3489-4867-9aeb-df460b8c88e7", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}, {"id": "277eb415-943d-4f7c-9360-0337f9cb6156", "type": "Port", "port_id": "0de6f147-5990-4547-abc9-82a4e77363a3", "interface_id": "dc97572b-4079-4e4e-a79d-9f15665796bb"}], "created_at": "2025-12-27T16:27:41.077307Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:41.077307Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "fb4eebc6-dc39-4572-8f10-479d52b50684", "name": "MQTT", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 1883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:27:48.077405Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "45c0f069-fd0a-4ea2-b5c6-00e1f3500e52", "bindings": [{"id": "dfeac1cb-f9a8-4359-a449-957d41412b86", "type": "Port", "port_id": "72e9da5d-dce0-4c42-ba0a-fc0c40693a38", "interface_id": "a6e0a1b6-1aff-4229-95f4-502a2f1e57bd"}], "created_at": "2025-12-27T16:27:48.077419Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:27:48.077419Z", "virtualization": null, "service_definition": "MQTT"}, {"id": "5c21b8c7-cd23-4345-865a-ba053c19cd29", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:02.234468Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "e4a0e35c-d3df-4629-9ce8-aa1e54a9749e", "bindings": [{"id": "c2cf3fd2-4d4b-4001-8ba8-0847d739c706", "type": "Port", "port_id": "8460a345-d537-4379-af3b-2c222f2e6fae", "interface_id": "f653c5f5-637d-455f-aea8-06bc34721af5"}], "created_at": "2025-12-27T16:28:02.234485Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:02.234485Z", "virtualization": null, "service_definition": "SSH"}, {"id": "6f21d4c7-e50e-46ad-a202-f8686a761884", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:02.234514Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "e4a0e35c-d3df-4629-9ce8-aa1e54a9749e", "bindings": [{"id": "63f083ca-b983-4909-88a0-4cae20ba8049", "type": "Port", "port_id": "1be344a9-e9fb-4eee-a69e-a41ee32b8fe4", "interface_id": "f653c5f5-637d-455f-aea8-06bc34721af5"}], "created_at": "2025-12-27T16:28:02.234517Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:02.234517Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "3089bd19-4bf9-4865-8cd9-60c97799c33b", "name": "MQTT", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": ["Any of", [{"data": "Port 8883/tcp is open", "type": "reason"}]], "type": "container"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:09.277648Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "e35b7afb-7229-45b1-a3e0-f920406c57f5", "bindings": [{"id": "34466b44-b12e-428a-8f44-9670b6dad48d", "type": "Port", "port_id": "83d0dcde-fc79-49f8-b335-e45b21dcac16", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}], "created_at": "2025-12-27T16:28:09.277660Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:09.277660Z", "virtualization": null, "service_definition": "MQTT"}, {"id": "efff2903-53f4-46cb-b46a-b93277685be8", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:09.277689Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "e35b7afb-7229-45b1-a3e0-f920406c57f5", "bindings": [{"id": "4de39328-83b4-49c1-b016-d299dabad939", "type": "Port", "port_id": "87f79850-4368-4cda-b498-25c9199b9db5", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}, {"id": "4cab2190-e99a-4c73-ab15-cf58d1c1ea11", "type": "Port", "port_id": "ae6ddc07-ea63-45b4-97fd-a740c448e388", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}, {"id": "7bb21d78-ede8-4b83-ac86-2f90931702a1", "type": "Port", "port_id": "9477e380-fcdc-44c3-a9b4-7abd8e3c870e", "interface_id": "ded4ba49-f1b2-48ef-900e-989144101a6d"}], "created_at": "2025-12-27T16:28:09.277720Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:09.277720Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}, {"id": "d311be56-1dd0-40c6-b7c7-0bee84cc1bde", "name": "Proxmox VE", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Any of", [{"data": "Response for 192.168.4.135:8006/ contained \\"proxmox\\" in body", "type": "reason"}, {"data": "Port 8006/tcp is open and is not used in other service match patterns", "type": "reason"}]], "type": "container"}, "confidence": "High"}, "metadata": [{"date": "2025-12-27T16:28:11.745488Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "1effd89a-40ab-4a44-862b-c3a14db9074c", "bindings": [{"id": "d4012352-ee83-485e-9da9-0f289e787236", "type": "Port", "port_id": "0a395d11-abb5-40e7-9940-e0df455bfa1b", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}], "created_at": "2025-12-27T16:28:11.745500Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:11.745500Z", "virtualization": null, "service_definition": "Proxmox VE"}, {"id": "e25cba30-c570-462e-a535-1281d3e7317f", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:16.336867Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "1effd89a-40ab-4a44-862b-c3a14db9074c", "bindings": [{"id": "6607bc12-cb15-490a-b77c-38e2cfdeff6f", "type": "Port", "port_id": "d7c195bf-5726-4423-9d3c-6bd45bf89ca7", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}], "created_at": "2025-12-27T16:28:16.336877Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:16.336877Z", "virtualization": null, "service_definition": "SSH"}, {"id": "50e66c44-301d-4331-9030-47ff6e746a00", "name": "Unclaimed Open Ports", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Has unbound open ports", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2025-12-27T16:28:16.336906Z", "type": "Network", "daemon_id": "7dc85a03-c94f-42d9-b52f-13808fbe88d7", "subnet_ids": null, "host_naming_fallback": "BestService"}]}, "host_id": "1effd89a-40ab-4a44-862b-c3a14db9074c", "bindings": [{"id": "6d6c34b4-9c6e-42cd-aace-237b187e1fe1", "type": "Port", "port_id": "87217851-d7f2-4ac8-aa19-ed2f081ff75e", "interface_id": "2089b1d5-759f-4fae-b47b-6707ddf3669e"}], "created_at": "2025-12-27T16:28:16.336908Z", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:28:16.336908Z", "virtualization": null, "service_definition": "Unclaimed Open Ports"}]	[{"id": "b6672bce-d110-4655-a025-fda8b5a9128a", "name": "test", "tags": [], "color": "rose", "source": {"type": "Manual"}, "created_at": "2025-12-27T16:21:26.476537Z", "edge_style": "Straight", "group_type": "RequestPath", "network_id": "f30cdbff-4a20-4a9b-9699-220e8f1a0b02", "updated_at": "2025-12-27T16:21:26.476537Z", "description": null, "service_bindings": ["fe3f4f51-8635-4d04-8663-5fa20e96e577", "c58fee2e-df2d-4d04-b1b0-7dd9fc373dc1"]}]	f	2025-12-27 16:28:16.720025+00	f	\N	\N	{}	{}	{}	{}	\N	2025-12-27 16:13:24.600969+00	2025-12-27 16:28:16.72746+00	{}
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, network_ids, tags, terms_accepted_at) FROM stdin;
ae488859-a60f-40d0-8d95-6bbe85386ad9	2025-12-27 16:13:24.57497+00	2025-12-27 16:13:24.57497+00	$argon2id$v=19$m=19456,t=2,p=1$08r//rHHlLKeEOIusrE50g$28R6ZSk9tGZAV0iLP1jPCP7HDyHSb6X2jVrDUsF6ql0	\N	\N	\N	maya@maya.cloud	880e6a2d-46c6-4afb-a39d-efb7f2368fc0	Owner	{}	{}	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
xGj-FRhSgO6-INYBvJ_xiQ	\\x93c41089f19fbc01d620beee80521815fe68c481a7757365725f6964d92461653438383835392d613630662d343064302d386439352d36626265383533383661643999cd07ea1a100d18ce2427a550000000	2026-01-26 16:13:24.606578+00
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
-- Name: idx_groups_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_groups_network ON public.groups USING btree (network_id);


--
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


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
-- Name: idx_users_email_lower; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_lower ON public.users USING btree (lower(email));


--
-- Name: idx_users_network_ids; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_network_ids ON public.users USING gin (network_ids);


--
-- Name: idx_users_oidc_provider_subject; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_oidc_provider_subject ON public.users USING btree (oidc_provider, oidc_subject) WHERE ((oidc_provider IS NOT NULL) AND (oidc_subject IS NOT NULL));


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: api_keys api_keys_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


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
-- Name: users users_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict kFpRM1MezonXcz3PhyuKraJZqwHFWuf1XsXKDVEuLEDERdcPHFW8JqxaBdEzCua

