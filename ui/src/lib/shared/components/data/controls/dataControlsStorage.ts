import { SvelteSet } from 'svelte/reactivity';
import { PAGE_SIZE_OPTIONS, type PageSizeOption } from '../types';
import type { FilterState } from './filtering';
import type { SortState } from './sorting';

export type ViewMode = 'card' | 'table';

export const DEFAULT_VIEW_MODE: ViewMode = 'table';

/** Filter selections as they sit in localStorage — sets marshalled to arrays. */
export interface StoredFieldFilter {
	type: 'string' | 'boolean' | 'array';
	values: string[];
	showTrue?: boolean;
	showFalse?: boolean;
}

export interface StoredState {
	searchQuery: string;
	filterState: Record<string, StoredFieldFilter>;
	sortState: SortState;
	selectedGroupField: string | null;
	showFilters: boolean;
	viewMode: ViewMode;
	currentPage: number;
	pageSize?: PageSizeOption;
	columnVisibility?: Record<string, boolean>;
	columnOrder?: string[];
	columnSizing?: Record<string, number>;
}

/**
 * Normalise a persisted view mode.
 *
 * List view was folded into the table, so a stored `'list'` means the user chose
 * the dense view and maps to `'table'` rather than resetting them to cards.
 *
 * `'card'` is matched explicitly rather than left to fall through. It used to
 * reach the same answer by accident, because the default was `'card'` — but the
 * default is now `'table'`, and a stored value is a choice the user made. Falling
 * through would silently flip every existing card user to the table.
 *
 * Everything else — a future mode, a hand-edited value, `undefined` — falls back
 * to the default, so no stored string can produce an out-of-union view mode.
 */
export function migrateViewMode(raw: unknown): ViewMode {
	if (raw === 'list' || raw === 'table') return 'table';
	if (raw === 'card') return 'card';
	return DEFAULT_VIEW_MODE;
}

function isPageSize(raw: unknown): raw is PageSizeOption {
	return PAGE_SIZE_OPTIONS.includes(raw as PageSizeOption);
}

function parseSortState(raw: unknown): SortState {
	const value = raw as Partial<SortState> | undefined;
	const direction = value?.direction === 'desc' ? 'desc' : 'asc';
	return { field: typeof value?.field === 'string' ? value.field : null, direction };
}

function parseFilterState(raw: unknown): Record<string, StoredFieldFilter> {
	if (!raw || typeof raw !== 'object') return {};

	const out: Record<string, StoredFieldFilter> = {};
	for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
		const filter = value as Partial<StoredFieldFilter> | undefined;
		if (!filter || typeof filter !== 'object') continue;
		out[key] = {
			type: filter.type === 'boolean' || filter.type === 'array' ? filter.type : 'string',
			values: Array.isArray(filter.values) ? filter.values.map(String) : [],
			showTrue: filter.showTrue,
			showFalse: filter.showFalse
		};
	}
	return out;
}

/**
 * Parse a stored blob into a fully-typed state.
 *
 * Every field is treated as optional so a blob written by an older build still
 * loads: a corrupt or partial key must never blank a tab. Returns `null` when
 * there is nothing usable, which the caller reads as "use defaults".
 */
export function parseStoredState(raw: string | null): StoredState | null {
	if (!raw) return null;

	let parsed: unknown;
	try {
		parsed = JSON.parse(raw);
	} catch {
		return null;
	}

	if (!parsed || typeof parsed !== 'object') return null;
	const state = parsed as Record<string, unknown>;

	return {
		searchQuery: typeof state.searchQuery === 'string' ? state.searchQuery : '',
		filterState: parseFilterState(state.filterState),
		sortState: parseSortState(state.sortState),
		selectedGroupField:
			typeof state.selectedGroupField === 'string' ? state.selectedGroupField : null,
		showFilters: state.showFilters === true,
		viewMode: migrateViewMode(state.viewMode),
		currentPage:
			typeof state.currentPage === 'number' && state.currentPage > 0 ? state.currentPage : 1,
		pageSize: isPageSize(state.pageSize) ? state.pageSize : undefined,
		columnVisibility: isRecordOf(state.columnVisibility, 'boolean')
			? (state.columnVisibility as Record<string, boolean>)
			: undefined,
		columnOrder: Array.isArray(state.columnOrder) ? state.columnOrder.map(String) : undefined,
		columnSizing: isRecordOf(state.columnSizing, 'number')
			? (state.columnSizing as Record<string, number>)
			: undefined
	};
}

function isRecordOf(raw: unknown, type: 'boolean' | 'number'): boolean {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return false;
	return Object.values(raw as Record<string, unknown>).every((v) => typeof v === type);
}

export function serializeState(state: StoredState): string {
	return JSON.stringify(state);
}

/**
 * Rehydrate saved filter selections into the reactive sets the panel mutates.
 *
 * Stored as plain arrays because a Set has no JSON form; they have to come back
 * as `SvelteSet` or a restored filter renders but never reacts.
 */
export function reviveFilterState(stored: StoredState['filterState']): FilterState {
	const revived: FilterState = {};

	for (const [key, saved] of Object.entries(stored)) {
		revived[key] = { ...saved, values: new SvelteSet(saved.values) };
	}

	return revived;
}

/** The JSON-safe form of the live filter state. */
export function toStoredFilterState(live: FilterState): StoredState['filterState'] {
	const stored: StoredState['filterState'] = {};

	for (const [key, filter] of Object.entries(live)) {
		stored[key] = { ...filter, values: Array.from(filter.values) };
	}

	return stored;
}
