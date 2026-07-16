import type { components } from '$lib/api/schema';

// Re-export generated types
export type DiscoveryType = components['schemas']['DiscoveryType'];
export type DiscoveryPhase = components['schemas']['DiscoveryPhase'];
export type HostNamingFallback = components['schemas']['HostNamingFallback'];

// Variant types from DiscoveryType union for type guards
export type SelfReportDiscovery = Extract<DiscoveryType, { type: 'SelfReport' }>;
export type NetworkDiscovery = Extract<DiscoveryType, { type: 'Network' }>;
export type DockerDiscovery = Extract<DiscoveryType, { type: 'Docker' }>;

// Frontend-specific types for WebSocket updates (not from backend API schema)
export interface DiscoveryUpdatePayload {
	session_id: string;
	daemon_id: string;
	network_id: string;
	discovery_type: DiscoveryType;
	phase: DiscoveryPhase;
	progress: number;
	error?: string | null;
	warnings?: string[];
	started_at?: string | null;
	finished_at?: string | null;
	hosts_discovered?: number | null;
	estimated_remaining_secs?: number | null;
	discovery_id?: string | null;
}
