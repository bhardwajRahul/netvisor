<script lang="ts" module>
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { common_host, common_unknownEntity } from '$lib/paraglide/messages';

	// Serves both LLDP/CDP link types: `PhysicalLink` names the two ports and reaches their
	// hosts through them, `NeighborLink` carries the host ids directly because the ports are
	// exactly what could not be resolved. Both read as "host ↔ host".
	function hostNamesFor(edge: TopologyEdge, topology: RenderableTopology) {
		const titleOf = (hostId: string | undefined) => {
			const host = topology.hosts.find((h) => h.id === hostId);
			return host ? hostDisplayName(host) : undefined;
		};
		if ('source_host_id' in edge && 'target_host_id' in edge) {
			return [titleOf(edge.source_host_id), titleOf(edge.target_host_id)];
		}
		if ('source_entity_id' in edge && 'target_entity_id' in edge) {
			const hostOfInterface = (interfaceId: string) => {
				const iface = topology.interfaces.find((e) => e.id === interfaceId);
				return iface ? titleOf(iface.host_id) : undefined;
			};
			return [hostOfInterface(edge.source_entity_id), hostOfInterface(edge.target_entity_id)];
		}
		return [undefined, undefined];
	}

	export const PhysicalLinkEdgeDisplay: EntityDisplayComponent<TopologyEdge, EdgeDisplayContext> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology) return edgeTypes.getName(edge.edge_type);
			const [sourceName, targetName] = hostNamesFor(edge, context.topology);
			// The fallback here is a host the topology bundle didn't carry, not a host without a
			// name — `hostDisplayName` has already handled that one.
			const unknown = common_unknownEntity({ entity: common_host() });
			return `${sourceName ?? unknown} ↔ ${targetName ?? unknown}`;
		},
		getDescription: (edge) => {
			return 'protocol' in edge ? ((edge.protocol as string) ?? '') : '';
		},
		getIcon: (edge) => edgeTypes.getIconComponent(edge.edge_type),
		getIconColor: (edge) => edgeTypes.getColorHelper(edge.edge_type).icon
	};

	export interface EdgeDisplayContext {
		topology?: RenderableTopology;
	}
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: TopologyEdge;
		context: EdgeDisplayContext;
	}

	let { item, context }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={PhysicalLinkEdgeDisplay} />
