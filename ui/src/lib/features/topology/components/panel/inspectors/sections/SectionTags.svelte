<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { EnrichedTopology, TopologyNode } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import { resolveTagTarget } from '$lib/features/topology/resolvers';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { common_tags } from '$lib/paraglide/messages';

	let {
		node,
		topology,
		editState
	}: {
		node: Node;
		topology: EnrichedTopology;
		editState: TopologyEditState;
	} = $props();

	let target = $derived(resolveTagTarget(node.id, node.data as TopologyNode));

	let selectedTagIds = $derived.by((): string[] => {
		if (!target) return [];
		const source = target.entityType === 'Host' ? topology.hosts : topology.services;
		return source.find((e) => e.id === target!.entityId)?.tags ?? [];
	});

	const tagsQuery = useTagsQuery();
	let entityTags = $derived.by(() => {
		const topoTags = topology?.entity_tags ?? [];
		const cachedTags = tagsQuery.data ?? [];
		const topoIds = new Set(topoTags.map((t) => t.id));
		return [...topoTags, ...cachedTags.filter((t) => !topoIds.has(t.id))];
	});
</script>

{#if target}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">{common_tags()}</span>
		<TagPickerInline
			{selectedTagIds}
			entityId={target.entityId}
			entityType={target.entityType}
			disabled={!editState.isEditable}
			availableTags={entityTags}
		/>
	</div>
{/if}
