/**
 * Enriched topology helpers.
 *
 * The slim backend `Topology` row carries only graph layout
 * (`nodes`, `edges`, `options`, `snapshot_id`, …). Entity arrays
 * (`hosts`, `services`, `subnets`, …) used to be embedded in the
 * topology row but are now read live via per-entity queries. This
 * module wraps a `Topology` with those entity arrays so consumers can
 * keep reading `topology.hosts` etc. uniformly.
 *
 * Snapshot vs live: per project plan, entity reads are always live —
 * the snapshot row's `nodes`/`edges` already encode captured visual
 * state, and inspector-level entity details show the current live row
 * (matches the snapshot when nothing has changed since).
 */

import type { EnrichedTopology, Topology, Binding, Vlan } from './types/base';
import type { Host, IPAddress, Interface, Port } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Dependency } from '$lib/features/dependencies/types/base';
import type { Tag } from '$lib/features/tags/types/base';

export interface EntityBundle {
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
}

export const EMPTY_ENTITY_BUNDLE: EntityBundle = {
	hosts: [],
	services: [],
	subnets: [],
	ip_addresses: [],
	ports: [],
	bindings: [],
	interfaces: [],
	dependencies: [],
	vlans: [],
	entity_tags: []
};

/**
 * Combine a slim `Topology` with the entity arrays it used to embed.
 *
 * `name` is a UI-side display string supplied by the caller (network
 * name for live view, formatted `taken_at` for snapshots, share name
 * for read-only shared topologies).
 *
 * Filters entity arrays to the topology's network so a multi-network
 * cache doesn't leak into the inspector.
 */
export function enrichTopology(
	topology: Topology,
	bundle: EntityBundle,
	name: string
): EnrichedTopology {
	const networkId = topology.network_id;
	const hosts = bundle.hosts.filter((h) => h.network_id === networkId);
	const subnets = bundle.subnets.filter((s) => s.network_id === networkId);
	const dependencies = bundle.dependencies.filter((d) => d.network_id === networkId);
	const vlans = bundle.vlans.filter((v) => v.network_id === networkId);
	const hostIds = new Set(hosts.map((h) => h.id));
	const services = bundle.services.filter((s) => hostIds.has(s.host_id));
	const ipAddresses = bundle.ip_addresses.filter((i) => hostIds.has(i.host_id));
	const ports = bundle.ports.filter((p) => hostIds.has(p.host_id));
	const interfaces = bundle.interfaces.filter((i) => hostIds.has(i.host_id));
	const bindings = bundle.bindings.filter((b) => b.network_id === networkId);
	// Tags are org-scoped; filter to ids referenced by entities here.
	const referencedTagIds = new Set<string>();
	for (const h of hosts) for (const t of h.tags ?? []) referencedTagIds.add(t);
	for (const s of services) for (const t of s.tags ?? []) referencedTagIds.add(t);
	for (const s of subnets) for (const t of s.tags ?? []) referencedTagIds.add(t);
	const entityTags = bundle.entity_tags.filter((t) => referencedTagIds.has(t.id));

	return {
		...topology,
		hosts,
		services,
		subnets,
		ip_addresses: ipAddresses,
		ports,
		bindings,
		interfaces,
		dependencies,
		vlans,
		entity_tags: entityTags,
		name
	};
}
