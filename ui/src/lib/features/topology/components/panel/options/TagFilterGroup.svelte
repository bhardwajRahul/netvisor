<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Color } from '$lib/shared/utils/styling';
	import type { components } from '$lib/api/schema';
	import { UNTAGGED_SENTINEL, hoveredTag, type HoveredTag } from '../../../interactions';

	type TagType = components['schemas']['Tag'];

	let {
		label,
		tags,
		hiddenTagIds,
		onToggle,
		entityType,
		hasUntagged = false
	}: {
		label: string;
		tags: TagType[];
		hiddenTagIds: string[];
		onToggle: (tagId: string) => void;
		entityType: HoveredTag['entityType'];
		hasUntagged?: boolean;
	} = $props();

	let isUntaggedHidden = $derived(hiddenTagIds.includes(UNTAGGED_SENTINEL));

	function handleMouseEnter(tagId: string, color: string) {
		hoveredTag.set({ tagId, color, entityType });
	}

	function handleMouseLeave() {
		hoveredTag.set(null);
	}
</script>

<div class="space-y-2">
	<div class="text-secondary text-sm font-medium">{label}</div>
	<div class="flex flex-wrap gap-1.5">
		{#if hasUntagged}
			<button
				onclick={() => onToggle(UNTAGGED_SENTINEL)}
				onmouseenter={() => handleMouseEnter(UNTAGGED_SENTINEL, 'Gray')}
				onmouseleave={handleMouseLeave}
				class="transition-opacity {isUntaggedHidden
					? 'opacity-50 hover:opacity-75'
					: 'opacity-100'}"
			>
				<Tag label="Untagged" color="Gray" />
			</button>
		{/if}
		{#each tags as tag (tag.id)}
			{@const isHidden = hiddenTagIds.includes(tag.id)}
			<button
				onclick={() => onToggle(tag.id)}
				onmouseenter={() => handleMouseEnter(tag.id, tag.color)}
				onmouseleave={handleMouseLeave}
				class="transition-opacity {isHidden ? 'opacity-50 hover:opacity-75' : 'opacity-100'}"
			>
				<Tag label={tag.name} color={tag.color as Color} />
			</button>
		{/each}
	</div>
</div>
