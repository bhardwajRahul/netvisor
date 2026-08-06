import { describe, it, expect } from 'vitest';
import { createTable, getCoreRowModel, type TableOptions } from '@tanstack/table-core';
import { withDefaultState } from '$lib/shared/components/data/table/createSvelteTable.svelte';

/**
 * The adapter's contract, exercised against real table-core.
 *
 * `createSvelteTable` itself needs runes and a component instance, but the part
 * that broke is pure: how a caller's partial `state` is resolved into the full
 * state every feature expects. That is worth pinning, because the failure mode
 * is a runtime throw while building header groups — nothing a type check or a
 * lint can see.
 */

interface Row {
	id: string;
	name: string;
}

const data: Row[] = [
	{ id: 'a', name: 'alpha' },
	{ id: 'b', name: 'beta' }
];

const columns = [
	{ id: 'name', accessorFn: (row: Row) => row.name, enableSorting: true },
	{ id: 'id', accessorFn: (row: Row) => row.id, enableSorting: false }
];

/** Builds a table the same way the adapter does, from a caller's partial state. */
function buildTable(state: TableOptions<Row>['state']) {
	const base: TableOptions<Row> = {
		data,
		columns,
		getCoreRowModel: getCoreRowModel(),
		manualSorting: true,
		manualFiltering: true,
		manualPagination: true,
		getRowId: (row) => row.id,
		state
	};

	const table = createTable({
		...base,
		state: {},
		onStateChange: () => {},
		renderFallbackValue: null
	});

	table.setOptions(() => withDefaultState(base, table.initialState));
	return table;
}

describe('table-core option resolution', () => {
	it('builds header groups when the caller supplies only some state', () => {
		// getState() returns options.state verbatim — nothing is merged underneath —
		// so a partial state used to leave columnPinning undefined and throw on
		// `.left` while assembling the headers.
		const table = buildTable({ sorting: [{ id: 'name', desc: false }] });

		expect(() => table.getHeaderGroups()).not.toThrow();
		expect(table.getHeaderGroups()[0].headers.map((h) => h.column.id)).toEqual(['name', 'id']);
	});

	it('builds header groups when the caller supplies no state at all', () => {
		const table = buildTable({});

		expect(() => table.getHeaderGroups()).not.toThrow();
		expect(table.getRowModel().rows).toHaveLength(2);
	});

	it('leaves every feature slice defined after resolution', () => {
		const table = buildTable({ sorting: [] });
		const state = table.getState();

		// The specific slice that broke, plus the neighbours that would break next.
		expect(state.columnPinning).toBeDefined();
		expect(state.columnPinning.left).toBeDefined();
		expect(state.columnVisibility).toBeDefined();
		expect(state.columnOrder).toBeDefined();
		expect(state.columnSizing).toBeDefined();
	});

	it('lets the caller override a default rather than the other way round', () => {
		const table = buildTable({ sorting: [{ id: 'name', desc: true }] });

		expect(table.getState().sorting).toEqual([{ id: 'name', desc: true }]);
	});

	it('keeps rows in the order given, since sorting is the caller’s job', () => {
		// manualSorting plus core-only row model: table-core must not reorder.
		const table = buildTable({ sorting: [{ id: 'name', desc: true }] });

		expect(table.getRowModel().rows.map((r) => r.original.name)).toEqual(['alpha', 'beta']);
	});

	it('reports sortability per column', () => {
		const table = buildTable({ sorting: [] });
		const headers = table.getHeaderGroups()[0].headers;

		expect(headers.find((h) => h.column.id === 'name')!.column.getCanSort()).toBe(true);
		expect(headers.find((h) => h.column.id === 'id')!.column.getCanSort()).toBe(false);
	});
});
