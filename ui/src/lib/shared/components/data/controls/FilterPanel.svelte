<script lang="ts" generics="T">
	import { getFieldKey, type FieldConfig } from '../types';
	import type { FilterState } from './filtering';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import Tag from '../Tag.svelte';
	import { scrollFade } from '$lib/shared/utils/scrollFade';
	import type { Color } from '$lib/shared/utils/styling';
	import {
		common_filters,
		common_clearAll,
		common_lastSeen,
		common_staleOnly,
		common_showTrue,
		common_showFalse,
		common_noTagsAvailable,
		common_noValuesAvailable
	} from '$lib/paraglide/messages';

	let {
		fields,
		filterState,
		allTags,
		staleOnly,
		hasActiveFilters,
		/** Staleness is server-only, so the control appears only when the parent handles it. */
		showStaleFilter,
		getUniqueValues,
		onClearFilters,
		onToggleBoolean,
		onToggleString,
		onToggleTag,
		onToggleStale
	}: {
		fields: FieldConfig<T>[];
		filterState: FilterState;
		allTags: TagType[];
		staleOnly: boolean;
		hasActiveFilters: boolean;
		showStaleFilter: boolean;
		getUniqueValues: (field: FieldConfig<T>) => string[];
		onClearFilters: () => void;
		onToggleBoolean: (fieldKey: string, which: 'showTrue' | 'showFalse') => void;
		onToggleString: (fieldKey: string, value: string) => void;
		onToggleTag: (tagId: string) => void;
		onToggleStale: () => void;
	} = $props();
</script>

<div class="card mt-4 !rounded-lg !p-5">
	<div class="flex items-center justify-between">
		<h3 class="text-primary text-sm font-semibold">{common_filters()}</h3>
		{#if hasActiveFilters}
			<button
				onclick={onClearFilters}
				class="text-tertiary hover:text-secondary text-xs transition-colors"
			>
				{common_clearAll()}
			</button>
		{/if}
	</div>

	<div class="mt-4 grid grid-cols-1 gap-x-8 gap-y-5 md:grid-cols-2 lg:grid-cols-3">
		{#if showStaleFilter}
			<!-- Server-side: the list is server-paginated, so filtering
			     client-side would only filter the loaded page. -->
			<div class="space-y-2">
				<div class="text-secondary text-sm font-medium">{common_lastSeen()}</div>
				<div class="space-y-1.5">
					<label class="flex cursor-pointer items-center gap-2">
						<input
							type="checkbox"
							checked={staleOnly}
							onchange={onToggleStale}
							class="checkbox-card h-4 w-4 rounded"
						/>
						<span class="text-secondary text-sm">{common_staleOnly()}</span>
					</label>
				</div>
			</div>
		{/if}
		{#each fields.filter((f) => f.filterable) as field (getFieldKey(field))}
			{@const fieldKey = getFieldKey(field)}
			<div class="space-y-2">
				<div class="text-secondary text-sm font-medium">{field.label}</div>

				{#if field.type === 'boolean'}
					{@const filter = filterState[fieldKey]}
					<div class="space-y-1.5">
						<label class="flex cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								checked={filter?.showTrue}
								onchange={() => onToggleBoolean(fieldKey, 'showTrue')}
								class="checkbox-card h-4 w-4 rounded"
							/>
							<span class="text-secondary text-sm">{common_showTrue()}</span>
						</label>
						<label class="flex cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								checked={filter?.showFalse}
								onchange={() => onToggleBoolean(fieldKey, 'showFalse')}
								class="checkbox-card h-4 w-4 rounded"
							/>
							<span class="text-secondary text-sm">{common_showFalse()}</span>
						</label>
					</div>
				{:else if fieldKey === 'tags'}
					<!-- Special tag filter with colored tags (stores tag IDs for server-side filtering) -->
					{@const filter = filterState[fieldKey]}
					<div
						use:scrollFade
						class="flex max-h-32 flex-wrap gap-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
					>
						{#if allTags.length === 0}
							<p class="text-tertiary text-xs">{common_noTagsAvailable()}</p>
						{:else}
							{#each allTags as tag (tag.id)}
								{@const isSelected = filter?.values.has(tag.id)}
								<button
									onclick={() => onToggleTag(tag.id)}
									class="transition-opacity {isSelected
										? 'opacity-100'
										: 'opacity-50 hover:opacity-75'}"
								>
									<Tag label={tag.name} color={tag.color as Color} />
								</button>
							{/each}
						{/if}
					</div>
				{:else}
					{@const uniqueValues = field.filterOptions ?? getUniqueValues(field)}
					{@const filter = filterState[fieldKey]}
					<div
						use:scrollFade
						class="max-h-32 space-y-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
					>
						{#if uniqueValues.length === 0}
							<p class="text-tertiary text-xs">{common_noValuesAvailable()}</p>
						{:else}
							{#each uniqueValues as value (value)}
								<label class="flex cursor-pointer items-center gap-2">
									<input
										type="checkbox"
										checked={filter?.values.has(value)}
										onchange={() => onToggleString(fieldKey, value)}
										class="checkbox-card h-4 w-4 rounded"
									/>
									<span class="text-secondary truncate text-sm" title={value}>{value}</span>
								</label>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
</div>
