<script lang="ts" module>
	import { edgeTypes, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	export const ContainerRuntimeEdgeDisplay: EntityDisplayComponent<
		TopologyEdge,
		EdgeDisplayContext
	> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology || !('service_id' in edge)) return 'Container Runtime';
			// Find containerized services (services whose virtualization points to this containerizer)
			const containerizingId = edge.service_id;
			// Any container runtime, not just Docker — the runtime is whichever service this
			// points at, so Podman containers are counted here too.
			const containerized = context.topology.services.filter(
				(s) => s.virtualization_service_id === containerizingId
			);
			if (containerized.length === 0) return 'Container Runtime';
			if (containerized.length === 1) return containerized[0].name;
			return `${containerized.length} containerized services`;
		},
		getDescription: (edge, context) => {
			if (!context?.topology || !('service_id' in edge)) return '';
			const containerizer = context.topology.services.find((s) => s.id === edge.service_id);
			const host =
				'host_id' in edge ? context.topology.hosts.find((h) => h.id === edge.host_id) : null;
			const parts: string[] = [];
			if (containerizer) parts.push(containerizer.name);
			if (host) parts.push(hostDisplayName(host));
			return parts.join(' · ');
		},
		getIcon: () => edgeTypes.getIconComponent('ContainerRuntime'),
		getIconColor: () => edgeTypes.getColorHelper('ContainerRuntime').icon,
		getTags: (edge, context) => {
			if (!context?.topology || !('service_id' in edge)) return [];
			const containerizer = context.topology.services.find((s) => s.id === edge.service_id);
			if (!containerizer) return [];
			const defName = serviceDefinitions.getName(containerizer.service_definition);
			return defName
				? [{ label: defName, color: edgeTypes.getColorHelper('ContainerRuntime').color }]
				: [];
		}
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

<ListSelectItem {item} {context} displayComponent={ContainerRuntimeEdgeDisplay} />
