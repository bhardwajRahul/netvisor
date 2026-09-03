import { getFieldKey, type FieldConfig } from '../types';
import { getFieldValue } from './fieldValues';

/** One field's filter selection. Readonly set so both `Set` and `SvelteSet` fit. */
export interface FieldFilter {
	type: 'string' | 'boolean' | 'array';
	values: ReadonlySet<string>;
	showTrue?: boolean;
	showFalse?: boolean;
}

export type FilterState = Record<string, FieldFilter>;

/**
 * Which filters the parent applies server-side, and so must not be re-applied
 * against the loaded rows.
 *
 * Filtering a server-paginated list on the client would only ever narrow the
 * page in hand, silently hiding every match on other pages — so these flags
 * gate the client pass rather than the server one.
 */
export interface ServerFilterMode {
	/** `onTagFilterChange` is wired, so tag filtering happens server-side. */
	tags: boolean;
	/** `onFilterChange` is wired, so `serverFiltered` fields are handled server-side. */
	fields: boolean;
}

/** Whether an item survives the search box. Opt-in per field via `searchable`. */
export function matchesSearch<T>(item: T, fields: FieldConfig<T>[], query: string): boolean {
	const q = query.trim().toLowerCase();
	if (!q) return true;

	// An unmarked field is not searched: matching everything by default meant a
	// date field turned "2026" into a match on every row.
	return fields
		.filter((f) => f.searchable === true)
		.some((field) => {
			const value = getFieldValue(item, field);
			if (value === null || value === undefined) return false;

			if (field.type === 'array' && Array.isArray(value)) {
				return value.some((v) => String(v).toLowerCase().includes(q));
			}

			return String(value).toLowerCase().includes(q);
		});
}

/**
 * Whether the parent, rather than the client pass, applies this field's filter.
 *
 * The single definition of the handover, read both by `matchesFilters` — which
 * skips what the parent owns — and by `serverFilterViolations`, which reports
 * what nobody owns. Keeping them on one predicate is what stops the guard and
 * the filter drifting apart.
 */
export function isHandledServerSide<T>(field: FieldConfig<T>, server: ServerFilterMode): boolean {
	if (getFieldKey(field) === 'tags' && server.tags) return true;
	return field.serverFiltered === true && server.fields;
}

/**
 * Filterable fields a server-paginated list would wrongly filter client-side.
 *
 * Under server pagination the client holds one page while the count describes
 * the whole match set, so a filter applied here narrows the page and leaves the
 * count describing different rows — the list says "62 of 1550" and pages
 * through the wrong hosts. Every filter therefore has to be handled by the
 * server that produced the count, or not offered at all.
 *
 * Returns the offending field keys so a caller can name them; empty when the
 * list is not server-paginated, where client-side filtering is correct.
 */
export function serverFilterViolations<T>(
	fields: FieldConfig<T>[],
	serverPaginated: boolean,
	server: ServerFilterMode
): string[] {
	if (!serverPaginated) return [];

	return fields
		.filter((field) => field.filterable === true && !isHandledServerSide(field, server))
		.map(getFieldKey);
}

/** Whether an item survives every active field filter. */
export function matchesFilters<T>(
	item: T,
	fields: FieldConfig<T>[],
	filterState: FilterState,
	server: ServerFilterMode
): boolean {
	return fields.every((field) => {
		if (!field.filterable) return true;

		const fieldKey = getFieldKey(field);
		const filterConfig = filterState[fieldKey];
		if (!filterConfig) return true;

		if (isHandledServerSide(field, server)) return true;

		const value = getFieldValue(item, field);

		if (field.type === 'boolean') {
			// A null boolean is "unknown" rather than false, so neither box excludes it.
			if (value === null || value === undefined) return true;
			const boolValue = Boolean(value);
			if (boolValue && !filterConfig.showTrue) return false;
			if (!boolValue && !filterConfig.showFalse) return false;
			return true;
		}

		if (field.type === 'array') {
			// An empty selection matches everything — otherwise opening the filter
			// panel would blank the list before the user has chosen anything.
			if (filterConfig.values.size === 0) return true;
			if (!Array.isArray(value) || value.length === 0) return false;
			return value.some((v) => filterConfig.values.has(String(v)));
		}

		if (field.type === 'string') {
			if (filterConfig.values.size === 0) return true;
			if (field.filterMode === 'exclude') {
				// Exclude hides the checked values. A null has no value to exclude,
				// so it survives — where include-mode drops it for not matching.
				return value == null || !filterConfig.values.has(String(value));
			}
			if (value === null || value === undefined) return false;
			return filterConfig.values.has(String(value));
		}

		return true;
	});
}

/** Whether any filter would currently narrow the list. */
export function hasActiveFilters<T>(
	fields: FieldConfig<T>[],
	filterState: FilterState,
	staleOnly: boolean
): boolean {
	if (staleOnly) return true;

	return fields.some((field) => {
		if (!field.filterable) return false;
		const filter = filterState[getFieldKey(field)];
		if (!filter) return false;

		if (field.type === 'boolean') {
			return !filter.showTrue || !filter.showFalse;
		}
		return filter.values.size > 0;
	});
}
