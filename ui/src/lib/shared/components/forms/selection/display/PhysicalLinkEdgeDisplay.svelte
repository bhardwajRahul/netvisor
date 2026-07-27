<script lang="ts" module>
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';

	// Serves both LLDP/CDP link types: `PhysicalLink` names the two ports and reaches their
	// hosts through them, `NeighborLink` carries the host ids directly because the ports are
	// exactly what could not be resolved. Both read as "host ↔ host".
	function hostNamesFor(edge: TopologyEdge, topology: RenderableTopology) {
		if ('source_host_id' in edge && 'target_host_id' in edge) {
			return [
				topology.hosts.find((h) => h.id === edge.source_host_id)?.name,
				topology.hosts.find((h) => h.id === edge.target_host_id)?.name
			];
		}
		if ('source_entity_id' in edge && 'target_entity_id' in edge) {
			const hostOfInterface = (interfaceId: string) => {
				const iface = topology.interfaces.find((e) => e.id === interfaceId);
				return iface ? topology.hosts.find((h) => h.id === iface.host_id)?.name : undefined;
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
			return `${sourceName ?? 'Unknown'} ↔ ${targetName ?? 'Unknown'}`;
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
