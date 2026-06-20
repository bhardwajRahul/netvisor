import type { components } from '$lib/api/schema';
import type { Service } from '$lib/features/services/types/base';
import type { Host, IPAddress, Interface, Port } from '$lib/features/hosts/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Dependency } from '$lib/features/dependencies/types/base';
import type { Tag } from '$lib/features/tags/types/base';

// Re-export generated types
export type Topology = components['schemas']['Topology'];
export type TopologyBase = components['schemas']['TopologyBase'];
export type TopologyOptions = components['schemas']['TopologyOptions'];
export type TopologyLocalOptions = components['schemas']['TopologyLocalOptions'];
export type TopologyRequestOptions = components['schemas']['TopologyRequestOptions'];
export type TopologyEdge = components['schemas']['Edge'];
export type TopologyNode = components['schemas']['Node'];
export type EdgeHandle = components['schemas']['EdgeHandle'];
export type Binding = components['schemas']['Binding'];
export type Vlan = components['schemas']['Vlan'];

/**
 * Topology graph row plus the live entity arrays needed for inspectors,
 * resolvers, and rendering. The slim backend `Topology` only carries
 * `{ id, network_id, snapshot_id, nodes, edges, options, ... }`. Entity
 * data (hosts, services, subnets, etc.) is loaded separately via per-entity
 * queries and bundled here so consumers can read everything off a single
 * object — preserving the pre-refactor field shape used across many
 * components.
 *
 * Always loaded live: snapshot rows still receive live entity data because
 * the snapshot's nodes/edges already encode the captured visual state, and
 * inspectors are expected to show current entity details.
 *
 * `name` is a UI-side display string (network name for live, formatted
 * `taken_at` for snapshots, share name for shared topologies).
 *
 * The backend `Topology` row stores `nodes`/`edges` keyed per view
 * (`Record<TopologyView, …[]>`). `toRenderableTopology` selects the active view's
 * slice, so `RenderableTopology` flattens those back to plain arrays — the shape
 * every pipeline/layout/renderer consumer already expects.
 */
export interface RenderableTopology extends Omit<Topology, 'nodes' | 'edges'> {
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	hosts: Host[];
	services: Service[];
	subnets: Subnet[];
	ip_addresses: IPAddress[];
	ports: Port[];
	bindings: Binding[];
	interfaces: Interface[];
	dependencies: Dependency[];
	vlans: Vlan[];
	entity_tags: Tag[];
	name: string;
}

// Variant types from Node union
export type ElementNode = Extract<TopologyNode, { node_type: 'Element' }>;
export type ContainerNode = Extract<TopologyNode, { node_type: 'Container' }>;

// Frontend-specific render types (not from backend)
export interface PortStatus {
	operStatus: 'Up' | 'Down' | string;
	speed: string | null;
	macAddress: string | null;
}

type ElementEntityTypeDiscriminant = components['schemas']['ElementEntityType']['element_type'];

export interface ElementRenderData {
	elementType: ElementEntityTypeDiscriminant;
	headerText: string | null;
	subtitleText?: string | null;
	footerText: string | null;
	bodyText: string | null;
	showServices: boolean;
	isVirtualized: boolean;
	isContainerized: boolean;
	services: Service[];
	hiddenOpenPorts: Service[];
	ip_address_id: string;
	isCategoryHidden?: boolean;
	portStatus?: PortStatus;
}

// ContainerRenderData removed — ContainerNode now reads icon/color directly
// from node data (set by backend graph builder) and metadata.
