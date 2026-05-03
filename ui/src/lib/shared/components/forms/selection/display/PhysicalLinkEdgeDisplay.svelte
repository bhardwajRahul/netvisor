<script lang="ts" module>
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import type { EnrichedTopology, TopologyEdge } from '$lib/features/topology/types/base';

	export const PhysicalLinkEdgeDisplay: EntityDisplayComponent<TopologyEdge, EdgeDisplayContext> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology || !('source_entity_id' in edge)) return 'Physical Link';
			const sourceInterface = context.topology.interfaces.find(
				(e) => e.id === edge.source_entity_id
			);
			const targetInterface =
				'target_entity_id' in edge
					? context.topology.interfaces.find((e) => e.id === edge.target_entity_id)
					: null;
			const sourceHost = sourceInterface
				? context.topology.hosts.find((h) => h.id === sourceInterface.host_id)
				: null;
			const targetHost = targetInterface
				? context.topology.hosts.find((h) => h.id === targetInterface.host_id)
				: null;
			const sourceName = sourceHost?.name ?? 'Unknown';
			const targetName = targetHost?.name ?? 'Unknown';
			return `${sourceName} ↔ ${targetName}`;
		},
		getDescription: (edge) => {
			return 'protocol' in edge ? ((edge.protocol as string) ?? '') : '';
		},
		getIcon: () => edgeTypes.getIconComponent('PhysicalLink'),
		getIconColor: () => edgeTypes.getColorHelper('PhysicalLink').icon
	};

	export interface EdgeDisplayContext {
		topology?: EnrichedTopology;
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
