import {
	portRangeValidation,
	required,
	max,
	ipAddressFormat,
	min,
	url,
	type Validator
} from '$lib/shared/components/forms/validators';

interface FieldDef {
	id: string;
	label: string;
	type: 'string' | 'number' | 'boolean' | 'select';
	defaultValue?: string | number | boolean;
	cliFlag: string;
	envVar: string;
	helpText: string;
	section?: string; // undefined = basic field, string = advanced section name
	placeholder?: string | number;
	options?: { label: string; value: string }[];
	disabled?: (isNew: boolean) => boolean;
	validators?: Validator[];
	required?: boolean;
	showWhen?: (values: Record<string, string | number | boolean>) => boolean;
	docsOnly?: boolean;
}

export const fieldDefs: FieldDef[] = [
	// Docs-only fields (not shown in UI form, but needed for documentation)
	{
		id: 'serverUrl',
		label: 'Server URL',
		type: 'string',
		cliFlag: '--server-url',
		envVar: 'SCANOPY_SERVER_URL',
		helpText: 'URL where the daemon can reach the server',
		defaultValue: 'http://127.0.0.1:60072',
		docsOnly: true
	},
	{
		id: 'daemonApiKey',
		label: 'API Key',
		type: 'string',
		cliFlag: '--daemon-api-key',
		envVar: 'SCANOPY_DAEMON_API_KEY',
		helpText: 'Authentication key for daemon (generated via UI)',
		required: true,
		docsOnly: true
	},
	{
		id: 'networkId',
		label: 'Network ID',
		type: 'string',
		cliFlag: '--network-id',
		envVar: 'SCANOPY_NETWORK_ID',
		helpText: 'UUID of the network to scan',
		docsOnly: true
	},
	// UI form fields
	{
		id: 'name',
		label: 'Name',
		type: 'string',
		cliFlag: '--name',
		envVar: 'SCANOPY_NAME',
		helpText: 'Name for this daemon',
		placeholder: 'Enter a name for this daemon...',
		validators: [required, max(100)],
		required: true
	},
	{
		id: 'mode',
		label: 'Daemon Mode',
		type: 'select',
		defaultValue: 'Push',
		cliFlag: '--mode',
		envVar: 'SCANOPY_MODE',
		helpText:
			'Select whether the daemon will Pull work from the server or have work Pushed to it. If set to Push, you will need to ensure that network you are deploying the daemon on can be reached by the server by opening/forwarding the port to the daemon, and provide the Daemon URL where the server should try to reach the daemon. If set to Pull, no port opening/forwarding is needed',
		options: [
			{ label: 'Push', value: 'Push' },
			{ label: 'Pull', value: 'Pull' }
		],
		disabled: (isNew) => !isNew
	},
	{
		id: 'daemonUrl',
		label: 'Daemon URL',
		type: 'string',
		defaultValue: '',
		cliFlag: '--daemon-url',
		envVar: 'SCANOPY_DAEMON_URL',
		helpText:
			'Public URL where server can reach daemon in Push mode. Defaults to auto-detected IP + Daemon Port if not set',
		placeholder: 'https://daemon.example.com',
		validators: [url],
		showWhen: (values) => values.mode === 'Push'
	},
	// Network section
	{
		id: 'daemonPort',
		label: 'Port',
		type: 'number',
		placeholder: 60073,
		cliFlag: '--daemon-port',
		envVar: 'SCANOPY_DAEMON_PORT',
		helpText: 'Port for daemon to listen on',
		section: 'Server Connection',
		validators: [portRangeValidation]
	},
	{
		id: 'bindAddress',
		label: 'Bind Address',
		type: 'string',
		defaultValue: '',
		cliFlag: '--bind-address',
		envVar: 'SCANOPY_BIND_ADDRESS',
		helpText: 'IP address to bind daemon to',
		placeholder: '0.0.0.0',
		section: 'Server Connection',
		validators: [ipAddressFormat]
	},
	{
		id: 'allowSelfSignedCerts',
		label: 'Allow Self-Signed Certificates',
		type: 'boolean',
		defaultValue: false,
		cliFlag: '--allow-self-signed-certs',
		envVar: 'SCANOPY_ALLOW_SELF_SIGNED_CERTS',
		helpText: 'Allow self-signed certs for daemon -> server connections',
		section: 'Server Connection'
	},
	// Performance section
	{
		id: 'logLevel',
		label: 'Log Level',
		type: 'select',
		defaultValue: 'info',
		cliFlag: '--log-level',
		envVar: 'SCANOPY_LOG_LEVEL',
		helpText: 'Logging verbosity',
		section: 'Performance',
		options: [
			{ label: 'Trace', value: 'trace' },
			{ label: 'Debug', value: 'debug' },
			{ label: 'Info', value: 'info' },
			{ label: 'Warn', value: 'warn' },
			{ label: 'Error', value: 'error' }
		]
	},
	{
		id: 'heartbeatInterval',
		label: 'Heartbeat Interval',
		type: 'number',
		placeholder: 30,
		cliFlag: '--heartbeat-interval',
		envVar: 'SCANOPY_HEARTBEAT_INTERVAL',
		helpText:
			'Seconds between heartbeat updates / work requests (for daemons in pull mode) to server',
		section: 'Performance',
		validators: [min(0), max(300)]
	},
	{
		id: 'dockerProxy',
		label: 'Docker Proxy',
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy',
		envVar: 'SCANOPY_DOCKER_PROXY',
		helpText:
			'Optional proxy for Docker API. Can use both non-SSL and SSL proxy; SSL proxy requires additional SSL config vars',
		placeholder: 'http://localhost:80/',
		section: 'Docker Discovery',
		validators: [url]
	},
	{
		id: 'dockerProxySslCert',
		label: 'Docker Proxy SSL Cert',
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-cert',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_CERT',
		helpText: 'Path to SSL certificate if using a docker proxy with SSL',
		placeholder: '/certs/cert.pem',
		section: 'Docker Discovery',
		validators: []
	},
	{
		id: 'dockerProxySslKey',
		label: 'Docker Proxy SSL Key',
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-key',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_KEY',
		helpText: 'Path to SSL private key if using a docker proxy with SSL',
		placeholder: '/certs/key.pem',
		section: 'Docker Discovery',
		validators: []
	},
	{
		id: 'dockerProxySslChain',
		label: 'Docker Proxy SSL Chain',
		type: 'string',
		defaultValue: '',
		cliFlag: '--docker-proxy-ssl-chain',
		envVar: 'SCANOPY_DOCKER_PROXY_SSL_CHAIN',
		helpText: 'Path to SSL chain if using a docker proxy with SSL',
		placeholder: '/certs/ca.pem',
		section: 'Docker Discovery',
		validators: []
	},
	// Network Discovery
	{
		id: 'interfaces',
		label: 'Interfaces',
		type: 'string',
		defaultValue: '',
		cliFlag: '--interfaces',
		envVar: 'SCANOPY_INTERFACES',
		helpText:
			'Restrict daemon to specific network interface(s). Comma-separated for multiple (e.g., eth0,eth1). Leave empty for all interfaces. Only applies to network discovery',
		placeholder: 'eth0',
		section: 'Network Discovery'
	},
	{
		id: 'arp_retries',
		label: 'Arp Retries',
		type: 'number',
		cliFlag: '--arp-retries',
		envVar: 'SCANOPY_ARP_RETRIES',
		helpText:
			'Number of ARP retry rounds for non-responding hosts (default: 2, meaning 3 total attempts)',
		section: 'Network Discovery'
	},
	{
		id: 'arp_rate_pps',
		label: 'Arp Packets per Second',
		type: 'number',
		cliFlag: '--arp-rate-pps',
		envVar: 'SCANOPY_ARP_RATE_PPS',
		helpText:
			'Maximum ARP packets per second (default: 50, go more conservative for networks with enterprise switches)',
		section: 'Network Discovery'
	},
	{
		id: 'use_npcap_arp',
		label: 'Use Npcap for ARP on Windows',
		type: 'boolean',
		defaultValue: false,
		cliFlag: '--use-npcap-arp',
		envVar: 'SCANOPY_USE_NPCAP_ARP',
		helpText:
			"Enable faster ARP scanning on Windows by using broadcast ARP via Npcap instead of native SendARP, which doesn't support broadcast. **Requires Npcap installation**. Ignored on Linux/macOS",
		section: 'Network Discovery'
	},
	{
		id: 'concurrentScans',
		label: 'Concurrent Scans',
		type: 'number',
		cliFlag: '--concurrent-scans',
		envVar: 'SCANOPY_CONCURRENT_SCANS',
		helpText: 'Maximum parallel host scans',
		placeholder: 'Auto',
		section: 'Network Discovery'
	}
];
