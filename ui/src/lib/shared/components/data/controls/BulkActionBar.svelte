<script lang="ts">
	import { Trash2 } from 'lucide-svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		common_itemsSelected,
		common_item,
		common_items,
		common_clearSelection,
		common_deleteSelected,
		common_tags,
		common_noCommonTags
	} from '$lib/paraglide/messages';

	let {
		selectedCount,
		showDelete,
		showTagging,
		/** Tags every selected row already carries, so removing one is unambiguous. */
		commonTags,
		onClearSelection,
		onBulkDelete,
		onTagAdd,
		onTagRemove
	}: {
		selectedCount: number;
		showDelete: boolean;
		showTagging: boolean;
		commonTags: string[];
		onClearSelection: () => void;
		onBulkDelete: () => void;
		onTagAdd: (tagId: string) => void;
		onTagRemove: (tagId: string) => void;
	} = $props();
</script>

<div class="card space-y-3 p-4">
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			<span class="text-primary text-sm font-medium">
				{common_itemsSelected({
					count: selectedCount,
					itemLabel: selectedCount === 1 ? common_item() : common_items()
				})}
			</span>
			<button
				onclick={onClearSelection}
				class="text-tertiary hover:text-secondary text-sm transition-colors"
			>
				{common_clearSelection()}
			</button>
		</div>
		{#if showDelete}
			<button onclick={onBulkDelete} class="btn-danger flex items-center gap-2">
				<Trash2 class="h-4 w-4" />
				{common_deleteSelected()}
			</button>
		{/if}
	</div>

	<!-- Bulk Tagging -->
	{#if showTagging}
		<div class="flex items-center gap-3 border-t border-gray-700 pt-3">
			<span class="text-secondary text-sm">{common_tags()}:</span>
			<TagPickerInline selectedTagIds={commonTags} onAdd={onTagAdd} onRemove={onTagRemove} />
			{#if commonTags.length === 0 && selectedCount > 1}
				<span class="text-tertiary text-xs">{common_noCommonTags()}</span>
			{/if}
		</div>
	{/if}
</div>
