import {
	createTable,
	type RowData,
	type Table,
	type TableOptions,
	type TableOptionsResolved
} from '@tanstack/table-core';

/**
 * `@tanstack/table-core` as a derived view model.
 *
 * The adapter is deliberately thin, and the instance owns no state of its own:
 * `options.state` is complete and authoritative, supplied by whoever already
 * persists it. Rows arrive filtered, sorted and paginated, and the table is
 * given only the core row model — so there is no code path here that could
 * reorder or drop a row, and no second source of truth to drift from the first.
 *
 * `getOptions` is re-read inside a `$derived`, so every rune it touches
 * invalidates the snapshot. Re-applying options inside the derived rather than
 * an effect is what makes the first render correct: an effect runs after the
 * template has already read stale header groups.
 */
export function createSvelteTable<T extends RowData>(getOptions: () => TableOptions<T>) {
	const resolve = (options: TableOptions<T>): TableOptionsResolved<T> => ({
		...options,
		state: options.state ?? {},
		onStateChange: options.onStateChange ?? (() => {}),
		renderFallbackValue: null
	});

	const table: Table<T> = createTable(resolve(getOptions()));

	const snapshot = $derived.by(() => {
		table.setOptions(() => resolve(getOptions()));
		return {
			headers: table.getHeaderGroups()[0]?.headers ?? [],
			rows: table.getRowModel().rows
		};
	});

	return {
		get headers() {
			return snapshot.headers;
		},
		get rows() {
			return snapshot.rows;
		},
		get table() {
			return table;
		}
	};
}
