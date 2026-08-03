<script lang="ts" generics="T">
	import {
		type FieldConfig,
		getFieldKey,
		groupPageSlice,
		type GroupPosition,
		type GroupSlice,
		type PageSizeOption,
		type CardAction
	} from './types';
	import { getFieldValue, getUniqueValues as uniqueValuesOf } from './controls/fieldValues';
	import {
		sortItems,
		nextSortState,
		sortableFields as sortableFieldsOf,
		groupableFields as groupableFieldsOf,
		type SortState
	} from './controls/sorting';
	import {
		matchesSearch,
		matchesFilters,
		hasActiveFilters as hasActiveFiltersOf
	} from './controls/filtering';
	import {
		visibleItems,
		isAllSelected,
		isPartiallySelected,
		visibleIds
	} from './controls/selection';
	import {
		parseStoredState,
		serializeState,
		DEFAULT_VIEW_MODE,
		type ViewMode,
		type StoredState
	} from './controls/dataControlsStorage';
	import ControlsBar from './controls/ControlsBar.svelte';
	import FilterPanel from './controls/FilterPanel.svelte';
	import BulkActionBar from './controls/BulkActionBar.svelte';
	import PaginationBar from './controls/PaginationBar.svelte';
	import EntityTable from './table/EntityTable.svelte';
	import ColumnVisibilityMenu from './table/ColumnVisibilityMenu.svelte';
	import { fieldsToColumns, reconcileColumnState, visibleColumns } from './table/columns';
	import { onMount, type Snippet } from 'svelte';
	import {
		common_all,
		common_groupTotalShowing,
		common_ungrouped,
		common_tableCaption,
		common_item,
		common_items
	} from '$lib/paraglide/messages';
	import {
		useTagsQuery,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { computeCommonTags } from '$lib/shared/utils/tags';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import throttle from 'just-throttle';
	import type { components } from '$lib/api/schema';

	type PaginationMeta = components['schemas']['PaginationMeta'];

	/** Debounce window for the search box, in ms. */
	const SEARCH_THROTTLE_MS = 300;

	/**
	 * Stands in for a null group key, which has no string form of its own.
	 * Prefixed with NUL because Postgres text values cannot contain one, so this
	 * can never collide with a real group value.
	 */
	const UNGROUPED_KEY = '\u0000ungrouped';

	let {
		items = $bindable([]),
		fields = $bindable([]),
		storageKey = null,
		onBulkDelete = null,
		allowBulkDelete = true,
		entityType = null,
		getItemTags = null,
		children,
		getItemId,
		// Server-side pagination (optional)
		serverPagination = null,
		onPageChange = null,
		// Server-side ordering callback (optional)
		// Called when grouping or sorting changes, allowing parent to update query params
		onOrderChange = null,
		// Server-side tag filtering callback (optional)
		// Called when tag filter selection changes, with array of selected tag IDs
		onTagFilterChange = null,
		// Server-side field filter callback (optional)
		// Called when a `serverFiltered` field's selection changes
		onFilterChange = null,
		// Server-side staleness filtering callback (optional)
		// Called when the "Stale only" toggle changes
		onStaleFilterChange = null,
		// Server-side search callback (optional)
		// Called (debounced) when the search box changes
		onSearchChange = null,
		// CSV export callback (optional, default behavior)
		// Called when user clicks export button; parent handles the actual export
		onCsvExport = null,
		// Export button click override (optional)
		// If provided, replaces onCsvExport entirely - use for custom export UI (e.g., modal with options)
		onExportClick = null,
		// Row actions for table mode. Cards build their own action list internally;
		// the table never renders a card, so it needs the same actions supplied here.
		getActions = null,
		// Names the table for screen readers, e.g. "Hosts".
		entityLabel = null
	}: {
		items: T[];
		fields: FieldConfig<T>[];
		storageKey?: string | null;
		onBulkDelete?: ((ids: string[]) => Promise<void>) | null;
		allowBulkDelete?: boolean;
		entityType?: EntityDiscriminants | null;
		getItemTags?: ((item: T) => string[]) | null;
		/**
		 * Renders one row as a card. Only invoked in card mode — table mode builds
		 * its cells from `fields`, so the snippet no longer receives a view mode.
		 */
		children: Snippet<[T, boolean, (selected: boolean) => void]>;
		getItemId: (item: T) => string;
		// Server-side pagination: when provided, pagination is server-controlled
		// Callback receives both page and pageSize so parent can use in query
		serverPagination?: PaginationMeta | null;
		onPageChange?: ((page: number, pageSize: number) => void) | null;
		// Server-side ordering: called when group/sort changes
		// Args: (groupBy field key, orderBy field key, direction)
		onOrderChange?:
			| ((groupBy: string | null, orderBy: string | null, direction: 'asc' | 'desc') => void)
			| null;
		// Server-side tag filtering: called when tag filter changes
		// Args: array of tag IDs to filter by
		onTagFilterChange?: ((tagIds: string[]) => void) | null;
		// Server-side field filter: called when a field marked `serverFiltered`
		// changes. Args: (fieldKey, selected values). The field opts in explicitly
		// rather than this applying to every filter, so a key the parent doesn't
		// handle keeps its client-side filtering instead of silently doing nothing.
		onFilterChange?: ((fieldKey: string, values: string[]) => void) | null;
		// Server-side staleness filtering: called when the "Stale only" toggle
		// changes. `true` = only stale, `null` = no staleness constraint.
		// Server-side because these lists are server-paginated — a client-side
		// filter would only ever filter the page currently loaded.
		onStaleFilterChange?: ((stale: boolean | null) => void) | null;
		// Server-side search: called with the current query, debounced.
		// Server-side for the same reason as the filters above — searching the
		// loaded page would silently miss every match on another page. Lists
		// that load everything omit this and keep the client-side search.
		onSearchChange?: ((query: string) => void) | null;
		// CSV export: default behavior when user clicks export button
		onCsvExport?: (() => void | Promise<void>) | null;
		// Export button click override: if provided, replaces onCsvExport entirely
		onExportClick?: (() => void | Promise<void>) | null;
		// Row actions for table mode, matching what the card builds for itself.
		getActions?: ((item: T) => CardAction[]) | null;
		// Accessible name for the table, e.g. "Hosts".
		entityLabel?: string | null;
	} = $props();

	// Tags query for filter display
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);

	// Bulk tag mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Search state
	let searchQuery = $state('');

	// Filter state
	interface FilterState {
		[key: string]: {
			type: 'string' | 'boolean' | 'array';
			values: SvelteSet<string>;
			showTrue?: boolean;
			showFalse?: boolean;
		};
	}

	let filterState = $state<FilterState>({});
	let showFilters = $state(false);
	// Staleness lives outside `filterState`: it's a server-side constraint
	// rather than a set of values matched against loaded rows.
	let staleOnly = $state(false);

	// Sort state
	let sortState = $state<SortState>({
		field: null,
		direction: 'asc'
	});

	// Grouping state
	let selectedGroupField = $state<string | null>(null);

	// View mode state
	let viewMode = $state<ViewMode>(DEFAULT_VIEW_MODE);

	// Column state — owned here so it persists alongside every other control,
	// and handed to table-core as controlled state rather than kept in parallel.
	let columnVisibility = $state<Record<string, boolean>>({});
	let columnOrder = $state<string[]>([]);
	let columnSizing = $state<Record<string, number>>({});

	/**
	 * Live height of the controls bar, so the table's sticky header sits directly
	 * under it. Measured rather than fixed because opening the filter panel
	 * changes that height.
	 */
	let controlsHeight = $state(0);

	// Pagination state
	let currentPage = $state(1);
	let pageSize = $state<PageSizeOption>(20);

	// Bulk selection state (always enabled when onBulkDelete is provided)
	let selectedIds = new SvelteSet<string>();

	// Load state from localStorage
	// Returns the restored pageSize if one was found, otherwise null
	function loadState(): PageSizeOption | null {
		if (!storageKey || typeof localStorage === 'undefined') return null;

		const state = parseStoredState(localStorage.getItem(storageKey));
		if (!state) return null;

		searchQuery = state.searchQuery;

		const restoredFilterState: FilterState = {};
		Object.entries(state.filterState).forEach(([key, saved]) => {
			restoredFilterState[key] = { ...saved, values: new SvelteSet(saved.values) };
		});
		filterState = restoredFilterState;

		sortState = state.sortState;
		if (state.selectedGroupField) selectedGroupField = state.selectedGroupField;
		showFilters = state.showFilters;
		// Already normalised, so a pre-table `list` lands on the table rather than
		// on a mode that matches neither branch.
		viewMode = state.viewMode;
		currentPage = state.currentPage;

		if (state.columnVisibility) columnVisibility = state.columnVisibility;
		if (state.columnOrder) columnOrder = state.columnOrder;
		if (state.columnSizing) columnSizing = state.columnSizing;

		if (state.pageSize) {
			pageSize = state.pageSize;
			return state.pageSize;
		}

		return null;
	}

	// Save state to localStorage
	function saveState() {
		if (!storageKey || typeof localStorage === 'undefined') return;

		try {
			const storedFilterState: StoredState['filterState'] = {};
			Object.entries(filterState).forEach(([key, filter]) => {
				storedFilterState[key] = { ...filter, values: Array.from(filter.values) };
			});

			localStorage.setItem(
				storageKey,
				serializeState({
					searchQuery,
					filterState: storedFilterState,
					sortState,
					selectedGroupField,
					showFilters,
					viewMode,
					currentPage,
					pageSize,
					columnVisibility,
					columnOrder,
					columnSizing
				})
			);
		} catch (e) {
			console.warn('Failed to save DataControls state to localStorage:', e);
		}
	}

	// Initialize filter state from fields
	$effect(() => {
		fields.forEach((field) => {
			const key = getFieldKey(field);
			if (field.filterable && !filterState[key]) {
				if (field.type === 'boolean') {
					filterState[key] = {
						type: 'boolean',
						values: new SvelteSet(),
						showTrue: true,
						showFalse: true
					};
				} else if (field.type === 'array') {
					filterState[key] = {
						type: 'array',
						values: new SvelteSet()
					};
				} else {
					filterState[key] = {
						type: 'string',
						values: new SvelteSet(field.filterDefaults)
					};
				}
			}
		});
	});

	// Load state on mount and set up auto-save
	onMount(() => {
		const restoredPageSize = loadState();

		// Notify parent of restored state for server-side pagination
		// This ensures the parent's query uses the restored pageSize
		if (restoredPageSize && onPageChange) {
			onPageChange(currentPage, restoredPageSize);
		}

		// Notify parent of restored ordering state
		if (onOrderChange && (selectedGroupField || sortState.field)) {
			onOrderChange(selectedGroupField, sortState.field, sortState.direction);
		}

		// Notify parent of restored search state
		if (onSearchChange && searchQuery.trim()) {
			onSearchChange(searchQuery);
		}

		// Notify parent of restored tag filter state
		const tagFilter = filterState['tags'];
		if (onTagFilterChange && tagFilter && tagFilter.values.size > 0) {
			onTagFilterChange(Array.from(tagFilter.values));
		}

		// Notify parent of restored server-side filter state
		if (onFilterChange) {
			for (const field of fields) {
				if (field.filterable && field.serverFiltered) {
					const key = getFieldKey(field);
					const filter = filterState[key];
					if (filter && filter.values.size > 0) {
						onFilterChange(key, Array.from(filter.values));
					}
				}
			}
		}

		// Set up reactive save (debounced)
		let saveTimeout: ReturnType<typeof setTimeout>;

		const unsubscribe = $effect.root(() => {
			$effect(() => {
				if (storageKey) {
					// Track all state that should trigger saves
					void searchQuery;
					void filterState;
					void sortState.field;
					void sortState.direction;
					void selectedGroupField;
					void showFilters;
					void viewMode;
					void currentPage;
					void pageSize;

					// Debounce saves
					clearTimeout(saveTimeout);
					saveTimeout = setTimeout(saveState, 100);
				}
			});
		});

		return () => {
			clearTimeout(saveTimeout);
			unsubscribe();
		};
	});

	// Get unique string values for a field (handles arrays by flattening)
	function getUniqueValues(field: FieldConfig<T>): string[] {
		return uniqueValuesOf(items, field);
	}

	let groupableFields = $derived(groupableFieldsOf(fields));
	let sortableFields = $derived(sortableFieldsOf(fields));

	// Apply all filters, sorting, and grouping
	let processedItems = $derived.by(() => {
		const serverMode = {
			tags: onTagFilterChange !== null,
			fields: onFilterChange !== null
		};

		const result = items.filter((item) => {
			// Search is skipped when the parent searches server-side — the rows that
			// arrived are already the matches.
			if (!onSearchChange && !matchesSearch(item, fields, searchQuery)) return false;
			return matchesFilters(item, fields, filterState, serverMode);
		});

		return sortItems(result, fields, sortState);
	});

	// Per-group totals across every page, when the server supplied them.
	let serverGroupCounts = $derived(serverPagination?.group_counts ?? null);

	// Group items by selected field
	let groupedItems = $derived.by(() => {
		if (!selectedGroupField) {
			return new SvelteMap([[common_all(), processedItems]]);
		}

		const field = fields.find((f) => getFieldKey(f) === selectedGroupField);
		if (!field) {
			return new SvelteMap([[common_all(), processedItems]]);
		}

		const groups = new SvelteMap<string, T[]>();

		processedItems.forEach((item) => {
			const value = getFieldValue(item, field);
			const groupKey = value !== null && value !== undefined ? String(value) : common_ungrouped();

			if (!groups.has(groupKey)) {
				groups.set(groupKey, []);
			}
			groups.get(groupKey)!.push(item);
		});

		// With server-side group totals the rows already arrive in the server's
		// group order; re-sorting here would desync the headers from the
		// cumulative offsets those totals are indexed by.
		if (serverGroupCounts) return groups;

		// Sort groups by key
		return new SvelteMap([...groups.entries()].sort((a, b) => a[0].localeCompare(b[0])));
	});

	// Where each group starts in the full ordered result set. The server
	// returns groups in the same order it orders rows, so a running sum of the
	// counts gives every group's global offset — which is what turns "this page
	// holds rows 100-199" into "this is rows 1-40 of that group".
	let groupOffsets = $derived.by(() => {
		const offsets = new SvelteMap<string, GroupPosition>();
		let cursor = 0;
		for (const group of serverGroupCounts ?? []) {
			offsets.set(group.value ?? UNGROUPED_KEY, { start: cursor, count: group.count });
			cursor += group.count;
		}
		return offsets;
	});

	// The value the server grouped these rows under, which is not always what
	// the header displays (a network group reads as a name, but groups by id).
	function serverGroupKey(groupItems: T[]): string {
		const field = fields.find((f) => getFieldKey(f) === selectedGroupField);
		if (!field || groupItems.length === 0) return UNGROUPED_KEY;

		const raw = field.getGroupValue
			? field.getGroupValue(groupItems[0])
			: getFieldValue(groupItems[0], field);

		return raw === null || raw === undefined ? UNGROUPED_KEY : String(raw);
	}

	/**
	 * How much of a group this page is showing, and how big the group really
	 * is. Null when the server didn't supply totals — on an unpaginated list
	 * the rendered count is already the whole group.
	 */
	function groupRange(groupItems: T[]): GroupSlice | null {
		if (!serverPagination) return null;

		const group = groupOffsets.get(serverGroupKey(groupItems));
		if (!group) return null;

		return groupPageSlice(group, serverPagination.offset, items.length);
	}

	// Toggle sort
	function toggleSort(fieldKey: string) {
		sortState = nextSortState(sortState, fieldKey);
	}

	// Toggle string/array filter value
	function toggleStringFilter(fieldKey: string, value: string) {
		const filter = filterState[fieldKey];
		if (!filter || (filter.type !== 'string' && filter.type !== 'array')) return;

		const newValues = new SvelteSet(filter.values);
		if (newValues.has(value)) {
			newValues.delete(value);
		} else {
			newValues.add(value);
		}

		filterState = {
			...filterState,
			[fieldKey]: {
				...filter,
				values: newValues
			}
		};

		// Notify parent of server-side filter changes
		if (onFilterChange) {
			const field = fields.find((f) => getFieldKey(f) === fieldKey);
			if (field?.serverFiltered) {
				onFilterChange(fieldKey, Array.from(newValues));
				// Reset pagination
				if (useServerPagination && onPageChange) {
					onPageChange(1, pageSize);
				} else {
					currentPage = 1;
				}
			}
		}
	}

	// Toggle boolean filter
	function toggleBooleanFilter(fieldKey: string, type: 'showTrue' | 'showFalse') {
		const filter = filterState[fieldKey];
		if (!filter || filter.type !== 'boolean') return;

		filterState = {
			...filterState,
			[fieldKey]: {
				...filter,
				[type]: !filter[type]
			}
		};
	}

	// Toggle tag filter (uses tag ID for server-side filtering)
	function toggleTagFilter(tagId: string) {
		const filter = filterState['tags'];
		if (!filter || filter.type !== 'array') return;

		const newValues = new SvelteSet(filter.values);
		if (newValues.has(tagId)) {
			newValues.delete(tagId);
		} else {
			newValues.add(tagId);
		}

		filterState = {
			...filterState,
			tags: {
				...filter,
				values: newValues
			}
		};
	}

	// Clear all filters (restores defaults for exclude filters)
	function clearFilters() {
		const newFilterState: FilterState = {};

		fields.forEach((field) => {
			if (field.filterable) {
				const key = getFieldKey(field);
				if (field.type === 'boolean') {
					newFilterState[key] = {
						type: 'boolean',
						values: new SvelteSet(),
						showTrue: true,
						showFalse: true
					};
				} else if (field.type === 'array') {
					newFilterState[key] = {
						type: 'array',
						values: new SvelteSet()
					};
				} else {
					newFilterState[key] = {
						type: 'string',
						values: new SvelteSet()
					};
				}
			}
		});

		filterState = newFilterState;

		if (staleOnly) {
			staleOnly = false;
			onStaleFilterChange?.(null);
		}

		// Notify parent that server-side filters were cleared
		if (onFilterChange) {
			fields.forEach((field) => {
				if (field.filterable && field.serverFiltered) {
					onFilterChange(getFieldKey(field), []);
				}
			});
		}
	}

	function toggleStaleFilter() {
		staleOnly = !staleOnly;
		// `null` rather than `false` — unchecking means "no staleness
		// constraint", not "show me only fresh entities".
		onStaleFilterChange?.(staleOnly ? true : null);
	}

	// Clear search
	function clearSearch() {
		searchQuery = '';
	}

	// Clear grouping
	function clearGrouping() {
		selectedGroupField = null;
	}

	// Select every rendered row — the same set `allSelected` reports on.
	function selectAll() {
		visibleIds(selectableItems, getItemId).forEach((id) => selectedIds.add(id));
	}

	// Deselect all items
	function selectNone() {
		selectedIds.clear();
	}

	// Handle bulk delete
	async function handleBulkDelete() {
		if (!allowBulkDelete) return;
		if (!onBulkDelete || selectedIds.size === 0) return;

		try {
			await onBulkDelete(Array.from(selectedIds));
			selectedIds.clear();
		} catch (error) {
			console.error('Bulk delete failed:', error);
		}
	}

	// Handle bulk tag add
	async function handleBulkTagAdd(tagId: string) {
		if (!entityType || selectedIds.size === 0) return;

		try {
			await bulkAddTagMutation.mutateAsync({
				entity_ids: Array.from(selectedIds),
				entity_type: entityType,
				tag_id: tagId
			});
		} catch (error) {
			console.error('Bulk tag add failed:', error);
		}
	}

	// Handle bulk tag remove
	async function handleBulkTagRemove(tagId: string) {
		if (!entityType || selectedIds.size === 0) return;

		try {
			await bulkRemoveTagMutation.mutateAsync({
				entity_ids: Array.from(selectedIds),
				entity_type: entityType,
				tag_id: tagId
			});
		} catch (error) {
			console.error('Bulk tag remove failed:', error);
		}
	}

	// Compute common tags across selected items (intersection)
	let commonTags = $derived.by(() => {
		if (!getItemTags || selectedIds.size === 0) return [];

		const selectedItems = items.filter((item) => selectedIds.has(getItemId(item)));
		if (selectedItems.length === 0) return [];

		return computeCommonTags(selectedItems.map((item) => ({ tags: getItemTags!(item) })));
	});

	// Check if bulk tagging is enabled
	let hasBulkTagging = $derived(entityType !== null && getItemTags !== null);

	// Check if any filters are active
	let hasActiveFilters = $derived(hasActiveFiltersOf(fields, filterState, staleOnly));

	let hasActiveSearch = $derived(searchQuery.trim().length > 0);
	let hasActiveGrouping = $derived(selectedGroupField !== null);

	// Check if using server-side pagination
	let useServerPagination = $derived(serverPagination !== null && onPageChange !== null);

	// Effective current page: derived from server offset when using server-side pagination
	let effectiveCurrentPage = $derived(
		useServerPagination && serverPagination
			? Math.floor(serverPagination.offset / pageSize) + 1
			: currentPage
	);

	// Pagination derived values (server-side or client-side)
	let totalPages = $derived(
		useServerPagination && serverPagination
			? Math.ceil(serverPagination.total_count / pageSize)
			: Math.ceil(processedItems.length / pageSize)
	);
	let canGoPrev = $derived(effectiveCurrentPage > 1);
	let canGoNext = $derived(
		useServerPagination && serverPagination
			? serverPagination.has_more
			: effectiveCurrentPage < totalPages
	);
	// The server's total already accounts for the search, so there is no longer
	// a filtered-vs-unfiltered discrepancy to paper over here.
	let showingStart = $derived(
		useServerPagination && serverPagination
			? Math.min(serverPagination.offset + 1, serverPagination.total_count)
			: Math.min((effectiveCurrentPage - 1) * pageSize + 1, processedItems.length)
	);
	let showingEnd = $derived(
		useServerPagination && serverPagination
			? Math.min(serverPagination.offset + processedItems.length, serverPagination.total_count)
			: Math.min(effectiveCurrentPage * pageSize, processedItems.length)
	);
	let totalCount = $derived(
		useServerPagination && serverPagination ? serverPagination.total_count : processedItems.length
	);

	// Paginated items for display
	// Server-side: items are already paginated, just apply client-side filtering
	// Client-side: slice the processed items
	let paginatedItems = $derived(
		useServerPagination
			? processedItems
			: processedItems.slice((effectiveCurrentPage - 1) * pageSize, effectiveCurrentPage * pageSize)
	);

	/**
	 * The rows select-all acts on: exactly what is rendered.
	 *
	 * Grouped mode renders every processed item; ungrouped renders the page
	 * slice. Deriving the action and its label from one set is what keeps the
	 * button's promise and a bulk operation's effect in agreement — comparing
	 * counts instead let any N carried-over selections read as "all".
	 */
	let selectableItems = $derived(visibleItems(hasActiveGrouping, processedItems, paginatedItems));
	let allSelected = $derived(isAllSelected(selectableItems, selectedIds, getItemId));

	function setRowSelected(itemId: string, selected: boolean) {
		if (selected) {
			selectedIds.add(itemId);
		} else {
			selectedIds.delete(itemId);
		}
	}

	/** Select-all scoped to one rendered block — a group's rows, or the page. */
	function toggleAllIn(rows: T[]) {
		if (isAllSelected(rows, selectedIds, getItemId)) {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.delete(id));
		} else {
			visibleIds(rows, getItemId).forEach((id) => selectedIds.add(id));
		}
	}

	// ---- Table columns -------------------------------------------------------

	let allColumns = $derived(fieldsToColumns(fields));
	let columnState = $derived(
		reconcileColumnState(allColumns, { visibility: columnVisibility, order: columnOrder })
	);
	let renderedColumns = $derived(visibleColumns(allColumns, columnState));
	let showSelection = $derived(Boolean(onBulkDelete) || hasBulkTagging);

	let tableCaptionText = $derived(
		common_tableCaption({
			entity: entityLabel ?? '',
			count: totalCount,
			itemLabel: totalCount === 1 ? common_item() : common_items()
		})
	);

	function toggleColumn(id: string) {
		columnVisibility = { ...columnState.visibility, [id]: columnState.visibility[id] === false };
	}

	function resetColumns() {
		columnVisibility = {};
		columnOrder = [];
	}

	// Reset to page 1 when filters/search change and current page would be out of bounds
	$effect(() => {
		if (effectiveCurrentPage > totalPages && totalPages > 0) {
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}
		}
	});

	// Track previous ordering state to detect changes and reset pagination
	let prevGroupBy: string | null = null;
	let prevOrderBy: string | null = null;
	let prevDirection: 'asc' | 'desc' = 'asc';
	let orderChangeInitialized = false;

	// Notify parent of ordering changes and reset pagination
	$effect(() => {
		const groupBy = selectedGroupField;
		const orderBy = sortState.field;
		const direction = sortState.direction;

		// Skip the initial run (state restoration)
		if (!orderChangeInitialized) {
			prevGroupBy = groupBy;
			prevOrderBy = orderBy;
			prevDirection = direction;
			orderChangeInitialized = true;
			return;
		}

		// Check if ordering actually changed
		const orderChanged =
			groupBy !== prevGroupBy || orderBy !== prevOrderBy || direction !== prevDirection;

		if (orderChanged) {
			prevGroupBy = groupBy;
			prevOrderBy = orderBy;
			prevDirection = direction;

			// Reset to page 1 when ordering changes
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}

			// Notify parent of the change
			if (onOrderChange) {
				onOrderChange(groupBy, orderBy, direction);
			}
		}
	});

	// Trailing throttle so a burst of keystrokes costs one request and the last
	// one is never dropped. Built once — rebuilding it per keystroke would
	// defeat the debounce entirely.
	const notifySearchChange = throttle(
		(query: string) => {
			onSearchChange?.(query);
			// Page 3 of the old result set is meaningless against the new one.
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}
		},
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	// Track previous search to detect changes
	let prevSearchQuery = '';
	let searchInitialized = false;

	// Notify parent of search changes
	$effect(() => {
		const query = searchQuery;

		// Skip the initial run (state restoration); onMount handles that, and
		// firing here would reset the restored page on every mount.
		if (!searchInitialized) {
			prevSearchQuery = query;
			searchInitialized = true;
			return;
		}

		if (query !== prevSearchQuery) {
			prevSearchQuery = query;
			notifySearchChange(query);
		}
	});

	// Track previous tag filter state to detect changes
	let prevTagFilterValues: string[] = [];
	let tagFilterInitialized = false;

	// Notify parent of tag filter changes
	$effect(() => {
		const tagFilter = filterState['tags'];
		const currentTagIds = tagFilter ? Array.from(tagFilter.values).sort() : [];

		// Skip the initial run (state restoration)
		if (!tagFilterInitialized) {
			prevTagFilterValues = currentTagIds;
			tagFilterInitialized = true;
			return;
		}

		// Check if tag filter actually changed
		const tagFilterChanged =
			currentTagIds.length !== prevTagFilterValues.length ||
			currentTagIds.some((id, i) => id !== prevTagFilterValues[i]);

		if (tagFilterChanged) {
			prevTagFilterValues = currentTagIds;

			// Reset to page 1 when tag filter changes
			if (useServerPagination && onPageChange) {
				onPageChange(1, pageSize);
			} else {
				currentPage = 1;
			}

			// Notify parent of the change
			if (onTagFilterChange) {
				onTagFilterChange(currentTagIds);
			}
		}
	});

	// Pagination handlers
	function goToPrevPage() {
		if (canGoPrev) {
			if (useServerPagination && onPageChange) {
				onPageChange(effectiveCurrentPage - 1, pageSize);
			} else {
				currentPage = currentPage - 1;
			}
		}
	}

	function goToNextPage() {
		if (canGoNext) {
			if (useServerPagination && onPageChange) {
				onPageChange(effectiveCurrentPage + 1, pageSize);
			} else {
				currentPage = currentPage + 1;
			}
		}
	}

	// Page size change handler
	function handlePageSizeChange(newSize: PageSizeOption) {
		pageSize = newSize;
		// Reset to page 1 when page size changes
		if (useServerPagination && onPageChange) {
			onPageChange(1, newSize);
		} else {
			currentPage = 1;
		}
	}

	// Export button state and handler
	let isExporting = $state(false);

	async function handleExportClick() {
		// Use onExportClick override if provided, otherwise fall back to onCsvExport
		const handler = onExportClick ?? onCsvExport;
		if (!handler || isExporting) return;

		isExporting = true;
		try {
			await handler();
		} finally {
			isExporting = false;
		}
	}

	// Show export button if either handler is provided
	let hasExportHandler = $derived(onExportClick !== null || onCsvExport !== null);

	// Sticky detection
	let isStuck = $state(false);
	let sentinelRef: HTMLDivElement | null = $state(null);

	$effect(() => {
		const sentinel = sentinelRef;
		if (!sentinel) return;

		// Find the scroll container (the main element with overflow-auto)
		const scrollContainer = sentinel.closest('main');

		const observer = new IntersectionObserver(
			([entry]) => {
				// Only set stuck if actually scrolled down (prevents flash on tab switch)
				const scrollTop = scrollContainer?.scrollTop ?? 0;
				isStuck = !entry.isIntersecting && scrollTop > 0;
			},
			{ threshold: 0, root: scrollContainer }
		);
		observer.observe(sentinel);

		return () => observer.disconnect();
	});
</script>

<div class="space-y-4">
	<!-- Sentinel for sticky detection -->
	<div bind:this={sentinelRef} class="h-0 w-full"></div>

	<!-- Sticky Controls Bar -->
	<div
		bind:clientHeight={controlsHeight}
		class="sticky top-0 z-20 -mx-8 border-b bg-[var(--color-bg-body)] px-8 pb-4 {isStuck
			? 'border-gray-700 pt-4 shadow-lg'
			: 'border-transparent'}"
	>
		<ControlsBar
			bind:searchQuery
			bind:selectedGroupField
			bind:sortState
			bind:viewMode
			bind:showFilters
			{fields}
			{groupableFields}
			{sortableFields}
			{hasActiveFilters}
			{hasActiveSearch}
			{hasActiveGrouping}
			showSelectAll={Boolean(onBulkDelete) || hasBulkTagging}
			{allSelected}
			{hasExportHandler}
			{isExporting}
			onToggleSort={toggleSort}
			onClearSearch={clearSearch}
			onClearGrouping={clearGrouping}
			onSelectAll={selectAll}
			onSelectNone={selectNone}
			onExport={handleExportClick}
		>
			{#snippet columnMenu()}
				<ColumnVisibilityMenu
					columns={allColumns}
					visibility={columnState.visibility}
					onToggle={toggleColumn}
					onReset={resetColumns}
				/>
			{/snippet}
		</ControlsBar>

		<!-- Filter Panel (inside sticky wrapper) -->
		{#if showFilters}
			<FilterPanel
				{fields}
				{filterState}
				{allTags}
				{staleOnly}
				{hasActiveFilters}
				showStaleFilter={onStaleFilterChange !== null}
				{getUniqueValues}
				onClearFilters={clearFilters}
				onToggleBoolean={toggleBooleanFilter}
				onToggleString={toggleStringFilter}
				onToggleTag={toggleTagFilter}
				onToggleStale={toggleStaleFilter}
			/>
		{/if}
	</div>

	<!-- Bulk Action Bar (shown when items are selected) -->
	{#if (onBulkDelete || hasBulkTagging) && selectedIds.size > 0}
		<BulkActionBar
			selectedCount={selectedIds.size}
			showDelete={Boolean(allowBulkDelete && onBulkDelete)}
			showTagging={hasBulkTagging}
			{commonTags}
			onClearSelection={selectNone}
			onBulkDelete={handleBulkDelete}
			onTagAdd={handleBulkTagAdd}
			onTagRemove={handleBulkTagRemove}
		/>
	{/if}

	<!-- Results Count and Pagination -->
	<PaginationBar
		{totalCount}
		{totalPages}
		currentPage={effectiveCurrentPage}
		{pageSize}
		{showingStart}
		{showingEnd}
		{canGoPrev}
		{canGoNext}
		groupCount={hasActiveGrouping ? groupedItems.size : null}
		{useServerPagination}
		processedCount={processedItems.length}
		itemCount={items.length}
		onPrevPage={goToPrevPage}
		onNextPage={goToNextPage}
		onPageSizeChange={handlePageSizeChange}
	/>

	<!-- Content -->
	{#if hasActiveGrouping}
		<!-- Grouped view -->
		<div class="space-y-6">
			{#each [...groupedItems.entries()] as [groupName, groupItems] (groupName)}
				{@const range = groupRange(groupItems)}
				<div class="space-y-3">
					<!-- Group Header -->
					<div class="flex items-center gap-3">
						<h3 class="text-primary text-lg font-semibold">{groupName}</h3>
						<span class="text-tertiary text-sm">
							{#if range}
								{common_groupTotalShowing({
									total: range.total,
									start: range.start,
									end: range.end
								})}
							{:else}
								({groupItems.length})
							{/if}
						</span>
					</div>

					<!-- Group Items -->
					{#if viewMode === 'table'}
						{@render tableFor(groupItems, groupName)}
					{:else}
						<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
							{#each groupItems as item (getItemId(item))}
								{@render cardFor(item)}
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{:else if viewMode === 'table'}
		{@render tableFor(paginatedItems, tableCaptionText)}
	{:else}
		<!-- Ungrouped view (paginated) -->
		<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
			{#each paginatedItems as item (getItemId(item))}
				{@render cardFor(item)}
			{/each}
		</div>
	{/if}
</div>

{#snippet cardFor(item: T)}
	{@const itemId = getItemId(item)}
	{@const isSelected = selectedIds.has(itemId)}
	{@render children(item, isSelected, (selected) => setRowSelected(itemId, selected))}
{/snippet}

{#snippet tableFor(rows: T[], caption: string)}
	<EntityTable
		items={rows}
		columns={renderedColumns}
		{sortState}
		selectable={showSelection}
		{selectedIds}
		allSelected={isAllSelected(rows, selectedIds, getItemId)}
		someSelected={isPartiallySelected(rows, selectedIds, getItemId)}
		{getItemId}
		{getActions}
		{caption}
		headerOffset={controlsHeight}
		onToggleSort={toggleSort}
		onToggleRow={setRowSelected}
		onToggleAll={() => toggleAllIn(rows)}
	/>
{/snippet}
