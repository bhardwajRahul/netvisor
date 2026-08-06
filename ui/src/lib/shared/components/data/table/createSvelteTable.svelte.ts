import {
	createTable,
	type RowData,
	type Table,
	type TableOptions,
	type TableOptionsResolved,
	type TableState
} from '@tanstack/table-core';

/**
 * Fill in the state slices the caller did not supply.
 *
 * `table.getState()` returns `options.state` verbatim — it does not merge
 * anything underneath. So a partial `state` leaves every other slice
 * `undefined`, and the first feature to read one (column pinning reads
 * `.left`) throws while building the header groups. `table.initialState`
 * carries a default for every registered feature, so it is the floor.
 */
export function withDefaultState<T extends RowData>(
	options: TableOptions<T>,
	initialState: TableState
): TableOptionsResolved<T> {
	return {
		...options,
		state: { ...initialState, ...(options.state ?? {}) },
		onStateChange: options.onStateChange ?? (() => {}),
		renderFallbackValue: null
	};
}

/**
 * `@tanstack/table-core` as a derived view model.
 *
 * The adapter is deliberately thin, and the instance owns no state of its own:
 * `options.state` is authoritative, supplied by whoever already persists it.
 * Rows arrive filtered, sorted and paginated, and the table is given only the
 * core row model — so there is no code path here that could reorder or drop a
 * row, and no second source of truth to drift from the first.
 *
 * `getOptions` is re-read inside a `$derived`, so every rune it touches
 * invalidates the snapshot. Re-applying options inside the derived rather than
 * an effect is what makes the first render correct: an effect runs after the
 * template has already read stale header groups.
 */
export function createSvelteTable<T extends RowData>(getOptions: () => TableOptions<T>) {
	// Seeded with an empty state purely so the instance exists; `initialState`
	// is derived from the registered features, not from what is passed here.
	const table: Table<T> = createTable({
		...getOptions(),
		state: {},
		onStateChange: () => {},
		renderFallbackValue: null
	});

	const initialState = table.initialState;
	table.setOptions(() => withDefaultState(getOptions(), initialState));

	const snapshot = $derived.by(() => {
		table.setOptions(() => withDefaultState(getOptions(), initialState));
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
