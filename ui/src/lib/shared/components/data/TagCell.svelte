<script lang="ts">
	import { Plus } from 'lucide-svelte';
	import Tag from './Tag.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { EntityDiscriminants } from '$lib/features/tags/queries';
	import type { CardFieldItem } from './types';
	import { MAX_ITEMS_IN_CELL } from './types';
	import { common_moreItems, tags_addTag } from '$lib/paraglide/messages';

	/**
	 * A row's tags: read-only chips until the user chooses to edit.
	 *
	 * `TagPickerInline` is not cheap per instance — it appends a portal container
	 * to the document and opens six query and mutation observers — so mounting one
	 * per row costs all of that times the page size. Only the cell being edited
	 * mounts one, which holds that cost at one instance per page however many rows
	 * are on screen.
	 *
	 * Chips are supplied already resolved so this component never opens a tags
	 * query of its own, for the same reason.
	 */
	let {
		items,
		tagIds,
		entityId,
		entityType,
		editable = true,
		/** Show every chip instead of collapsing past the cap — card layouts have the room. */
		expanded = false
	}: {
		items: CardFieldItem[];
		tagIds: string[];
		entityId: string;
		entityType: EntityDiscriminants;
		editable?: boolean;
		expanded?: boolean;
	} = $props();

	let editing = $state(false);
	let showAll = $state(false);

	let visible = $derived(expanded || showAll ? items : items.slice(0, MAX_ITEMS_IN_CELL));
	let overflow = $derived(items.length - visible.length);
</script>

{#if editing}
	<TagPickerInline selectedTagIds={tagIds} {entityId} {entityType} bind:open={editing} />
{:else}
	<div class="flex flex-wrap items-center gap-1">
		{#each visible as item (item.id)}
			<Tag
				label={item.label}
				color={item.color}
				icon={item.icon}
				badge={item.badge}
				title={item.title}
			/>
		{/each}

		{#if overflow > 0}
			<button
				type="button"
				onclick={() => (showAll = true)}
				class="text-tertiary hover:text-secondary text-xs transition-colors"
			>
				{common_moreItems({ count: overflow })}
			</button>
		{/if}

		{#if editable}
			<button
				type="button"
				onclick={() => (editing = true)}
				aria-label={tags_addTag()}
				class="text-tertiary hover:text-secondary inline-flex h-5 w-5 items-center justify-center rounded-full border border-dashed border-gray-400 transition-colors dark:border-gray-500"
			>
				<Plus class="h-3 w-3" />
			</button>
		{/if}
	</div>
{/if}
