<script lang="ts" generics="T">
	import Tag from '../Tag.svelte';
	import EntityTag from '../EntityTag.svelte';
	import { MAX_ITEMS_IN_CELL, type CardFieldItem } from '../types';
	import { getFieldValue } from '../controls/fieldValues';
	import type { EntityColumn } from './columns';
	import { common_moreItems, common_none } from '$lib/paraglide/messages';

	let { item, column }: { item: T; column: EntityColumn<T> } = $props();

	let showAll = $state(false);

	let items = $derived<CardFieldItem[] | null>(column.column.getItems?.(item) ?? null);
	let value = $derived(items ? null : getFieldValue(item, column.field));

	let visible = $derived(items === null ? [] : showAll ? items : items.slice(0, MAX_ITEMS_IN_CELL));
	let overflow = $derived(items === null ? 0 : items.length - visible.length);

	/** Dates arrive as ISO strings; render them in the viewer's locale. */
	function formatValue(raw: string | boolean | Date | string[] | null): string {
		if (raw === null || raw === undefined || raw === '') return '';
		if (raw instanceof Date) return raw.toLocaleString();
		if (Array.isArray(raw)) return raw.join(', ');
		if (typeof raw === 'boolean') return String(raw);
		if (column.field.type === 'date') {
			const parsed = new Date(raw);
			return Number.isNaN(parsed.getTime()) ? raw : parsed.toLocaleString();
		}
		return raw;
	}

	let text = $derived(items === null ? formatValue(value) : '');
</script>

{#if column.column.cell}
	{@render column.column.cell(item)}
{:else if items !== null}
	{#if items.length === 0}
		<!-- An em dash on its own is read as "dash", or skipped entirely. -->
		<span class="text-muted" aria-hidden="true">—</span>
		<span class="sr-only">{common_none()}</span>
	{:else}
		<div class="flex flex-wrap items-center gap-1">
			{#each visible as entry (entry.id)}
				{#if entry.entityRef}
					<EntityTag
						entityRef={entry.entityRef}
						icon={entry.icon}
						disabled={entry.disabled}
						color={entry.color}
						badge={entry.badge}
						label={entry.label}
					/>
				{:else}
					<Tag
						icon={entry.icon}
						disabled={entry.disabled}
						color={entry.color}
						badge={entry.badge}
						label={entry.label}
						title={entry.title}
					/>
				{/if}
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
		</div>
	{/if}
{:else if text === ''}
	<span class="text-muted" aria-hidden="true">—</span>
	<span class="sr-only">{common_none()}</span>
{:else}
	<span class="text-tertiary block truncate" title={text}>{text}</span>
{/if}
