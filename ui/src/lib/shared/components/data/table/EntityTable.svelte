<script lang="ts" generics="T">
	import { ArrowUpNarrowWide, ArrowDownWideNarrow } from 'lucide-svelte';
	import { getCoreRowModel, type ColumnDef } from '@tanstack/table-core';
	import { createSvelteTable } from './createSvelteTable.svelte';
	import type { EntityColumn } from './columns';
	import TableCell from './TableCell.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { getFieldValue } from '../controls/fieldValues';
	import type { SortState } from '../controls/sorting';
	import type { CardAction } from '../types';
	import {
		common_actions,
		common_selectRow,
		common_sortByColumn,
		common_deselectAll,
		common_selectAllOnPage
	} from '$lib/paraglide/messages';

	let {
		items,
		columns,
		sortState,
		selectable,
		selectedIds,
		allSelected,
		someSelected,
		getItemId,
		getActions,
		caption,
		onToggleSort,
		onToggleRow,
		onToggleAll
	}: {
		items: T[];
		columns: EntityColumn<T>[];
		sortState: SortState;
		selectable: boolean;
		selectedIds: ReadonlySet<string>;
		allSelected: boolean;
		someSelected: boolean;
		getItemId: (item: T) => string;
		getActions: ((item: T) => CardAction[]) | null;
		caption: string;
		onToggleSort: (fieldKey: string) => void;
		onToggleRow: (id: string, selected: boolean) => void;
		onToggleAll: () => void;
	} = $props();

	/**
	 * Column identity is keyed on the field keys, not the array.
	 *
	 * Tabs build `fields` inside a `$derived` over several queries, so every
	 * refetch yields a fresh array. Rebuilding the column defs on identity would
	 * throw away the row model on each one; the value functions read the current
	 * columns through a closure instead, so cells stay live regardless.
	 */
	let columnsKey = $derived(columns.map((c) => c.id).join('\0'));
	let columnDefs = $derived.by<ColumnDef<T>[]>(() => {
		void columnsKey;
		return columns.map((column) => ({
			id: column.id,
			accessorFn: (row: T) => getFieldValue(row, column.field),
			enableSorting: column.sortable
		}));
	});

	const view = createSvelteTable<T>(() => ({
		get data() {
			return items;
		},
		get columns() {
			return columnDefs;
		},
		getCoreRowModel: getCoreRowModel(),
		// Rows arrive already filtered, sorted and paged. table-core is given no
		// row model that could reorder or drop one, so its view cannot drift from
		// what the controls produced.
		manualSorting: true,
		manualFiltering: true,
		manualPagination: true,
		getRowId: (row: T) => getItemId(row),
		state: {
			get sorting() {
				return sortState.field
					? [{ id: sortState.field, desc: sortState.direction === 'desc' }]
					: [];
			}
		}
	}));

	let byId = $derived(new Map(columns.map((c) => [c.id, c])));
	let primaryColumn = $derived(columns.find((c) => c.primary) ?? columns[0]);

	function ariaSort(columnId: string): 'ascending' | 'descending' | 'none' {
		if (sortState.field !== columnId) return 'none';
		return sortState.direction === 'asc' ? 'ascending' : 'descending';
	}

	/** Names a row's checkbox after the row, so a column of them isn't all "Select". */
	function rowLabel(item: T): string {
		if (!primaryColumn) return '';
		const value = getFieldValue(item, primaryColumn.field);
		return value === null || value === undefined ? '' : String(value);
	}
</script>

<!--
	A plain table, deliberately: `role="grid"` promises two-dimensional arrow-key
	navigation with managed focus, and claiming it without implementing it takes
	away the table-reading commands screen reader users already have.

	The tabindex is deliberate too. The lint rule assumes one on a non-interactive
	element is a mistake, but a wide table overflows horizontally and a scroll
	container that only answers to the mouse is a WCAG 2.1.1 failure.

	Note this wrapper is the containing block for `position: sticky`, so a sticky
	header here would offset from the wrapper rather than the viewport — which is
	why the header scrolls with the rows instead of pinning.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="overflow-x-auto" tabindex="0" role="region" aria-label={caption}>
	<table class="w-full border-collapse text-sm">
		<caption class="sr-only">{caption}</caption>
		<thead>
			<tr>
				{#if selectable}
					<th scope="col" class="sticky left-0 z-10 w-10 bg-[var(--color-bg-body)] px-3 py-2">
						<input
							type="checkbox"
							checked={allSelected}
							indeterminate={someSelected}
							onchange={onToggleAll}
							aria-label={allSelected ? common_deselectAll() : common_selectAllOnPage()}
							class="checkbox-card h-4 w-4"
						/>
					</th>
				{/if}

				{#each view.headers as header (header.id)}
					{@const column = byId.get(header.column.id)}
					{#if column}
						<th
							scope="col"
							aria-sort={ariaSort(column.id)}
							style={column.width ? `width: ${column.width}px` : ''}
							class="text-secondary whitespace-nowrap px-3 py-2 text-xs font-medium {column.align ===
							'right'
								? 'text-right'
								: 'text-left'}"
						>
							{#if header.column.getCanSort()}
								<!--
									A real button: Enter and Space work natively, and the direction
									is announced through aria-sort rather than repeated in the name.
								-->
								<button
									type="button"
									onclick={() => onToggleSort(column.id)}
									aria-label={common_sortByColumn({ column: column.label })}
									class="hover:text-primary inline-flex items-center gap-1 transition-colors"
								>
									<span>{column.label}</span>
									{#if sortState.field === column.id}
										{#if sortState.direction === 'asc'}
											<ArrowUpNarrowWide class="h-3.5 w-3.5" aria-hidden="true" />
										{:else}
											<ArrowDownWideNarrow class="h-3.5 w-3.5" aria-hidden="true" />
										{/if}
									{/if}
								</button>
							{:else}
								<span>{column.label}</span>
							{/if}
						</th>
					{/if}
				{/each}

				{#if getActions}
					<th
						scope="col"
						class="text-secondary sticky right-0 z-10 bg-[var(--color-bg-body)] px-3 py-2 text-right text-xs font-medium"
					>
						{common_actions()}
					</th>
				{/if}
			</tr>
		</thead>

		<tbody>
			{#each view.rows as row (row.id)}
				{@const item = row.original}
				{@const itemId = getItemId(item)}
				{@const isSelected = selectedIds.has(itemId)}
				<tr
					class="border-t transition-colors {isSelected
						? 'bg-black/5 dark:bg-white/5'
						: 'hover:bg-black/[0.03] dark:hover:bg-white/[0.03]'}"
					style="border-color: var(--color-border)"
				>
					{#if selectable}
						<td class="w-10 px-3 py-2 align-middle">
							<input
								type="checkbox"
								checked={isSelected}
								onchange={(e) => onToggleRow(itemId, e.currentTarget.checked)}
								aria-label={common_selectRow({ name: rowLabel(item) })}
								class="checkbox-card h-4 w-4"
							/>
						</td>
					{/if}

					{#each view.headers as header (header.id)}
						{@const column = byId.get(header.column.id)}
						{#if column}
							{#if column.primary}
								<!-- Announces the row's identity before each cell when navigating across. -->
								<th
									scope="row"
									class="text-primary max-w-xs px-3 py-2 text-left align-middle font-medium"
								>
									<TableCell {item} {column} />
								</th>
							{:else}
								<td
									class="max-w-xs px-3 py-2 align-middle {column.align === 'right'
										? 'text-right'
										: ''}"
								>
									<TableCell {item} {column} />
								</td>
							{/if}
						{/if}
					{/each}

					{#if getActions}
						{@const actions = getActions(item)}
						<td class="px-3 py-2 text-right align-middle">
							<div class="flex items-center justify-end gap-1">
								{#each actions as action (action.label)}
									{@const tip =
										typeof action.tooltip === 'function'
											? action.tooltip(!!action.disabled)
											: (action.tooltip ?? action.label)}
									<!--
										The label floats in a tooltip rather than growing inside the
										button. An in-flow label has to span its neighbours to fit its
										text, which is what let one action cover the rest of the row.
									-->
									<button
										type="button"
										onclick={action.onClick}
										disabled={action.disabled}
										use:tooltip
										data-tooltip={tip}
										aria-label={action.label}
										class="{action.class ||
											'btn-icon'} disabled:cursor-not-allowed disabled:opacity-50"
									>
										<action.icon size={16} class={action.animation || ''} />
									</button>
								{/each}
							</div>
						</td>
					{/if}
				</tr>
			{/each}
		</tbody>
	</table>
</div>
