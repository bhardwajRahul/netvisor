import { describe, it, expect } from 'vitest';
import {
	matchesFilters,
	matchesSearch,
	hasActiveFilters,
	type FilterState,
	type ServerFilterMode
} from '$lib/shared/components/data/controls/filtering';
import { getUniqueValues } from '$lib/shared/components/data/controls/fieldValues';
import type { FieldConfig } from '$lib/shared/components/data/types';

interface Row {
	name: string | null;
	category: string | null;
	hidden: boolean | null;
	tags: string[];
}

function row(partial: Partial<Row>): Row {
	return { name: null, category: null, hidden: null, tags: [], ...partial };
}

const CLIENT_ONLY: ServerFilterMode = { tags: false, fields: false };

const nameField: FieldConfig<Row> = {
	key: 'name',
	label: 'Name',
	type: 'string',
	searchable: true,
	filterable: true,
	getValue: (r) => r.name
};
const hiddenField: FieldConfig<Row> = {
	key: 'hidden',
	label: 'Hidden',
	type: 'boolean',
	filterable: true,
	getValue: (r) => r.hidden
};
const tagsField: FieldConfig<Row> = {
	key: 'tags',
	label: 'Tags',
	type: 'array',
	filterable: true,
	getValue: (r) => r.tags
};

function stringFilter(values: string[]): FilterState[string] {
	return { type: 'string', values: new Set(values) };
}

describe('matchesFilters — string fields', () => {
	it('matches everything while nothing is selected', () => {
		// Otherwise opening the filter panel would blank the list before the user
		// has chosen anything.
		const state: FilterState = { name: stringFilter([]) };

		expect(matchesFilters(row({ name: 'a' }), [nameField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: null }), [nameField], state, CLIENT_ONLY)).toBe(true);
	});

	it('keeps only selected values in include mode, dropping nulls', () => {
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'a' }), [nameField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: 'b' }), [nameField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ name: null }), [nameField], state, CLIENT_ONLY)).toBe(false);
	});

	it('hides selected values in exclude mode but keeps nulls', () => {
		// Asymmetric with include mode on purpose: a row with no value has nothing
		// to exclude, so excluding "a" must not also hide the unset rows.
		const excludeField: FieldConfig<Row> = { ...nameField, filterMode: 'exclude' };
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'a' }), [excludeField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ name: 'b' }), [excludeField], state, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ name: null }), [excludeField], state, CLIENT_ONLY)).toBe(true);
	});
});

describe('matchesFilters — boolean fields', () => {
	it('treats a null boolean as unknown, so neither box excludes it', () => {
		const state: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: false, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: null }), [hiddenField], state, CLIENT_ONLY)).toBe(true);
	});

	it('matches nothing when both boxes are unchecked', () => {
		const state: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: false, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: true }), [hiddenField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ hidden: false }), [hiddenField], state, CLIENT_ONLY)).toBe(false);
	});

	it('selects each side independently', () => {
		const onlyTrue: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: false }
		};

		expect(matchesFilters(row({ hidden: true }), [hiddenField], onlyTrue, CLIENT_ONLY)).toBe(true);
		expect(matchesFilters(row({ hidden: false }), [hiddenField], onlyTrue, CLIENT_ONLY)).toBe(
			false
		);
	});
});

describe('matchesFilters — array fields', () => {
	it('matches when any value is selected', () => {
		const state: FilterState = { tags: { type: 'array', values: new Set(['prod']) } };

		expect(matchesFilters(row({ tags: ['prod', 'db'] }), [tagsField], state, CLIENT_ONLY)).toBe(
			true
		);
		expect(matchesFilters(row({ tags: ['db'] }), [tagsField], state, CLIENT_ONLY)).toBe(false);
		expect(matchesFilters(row({ tags: [] }), [tagsField], state, CLIENT_ONLY)).toBe(false);
	});
});

describe('matchesFilters — server-side handover', () => {
	it('skips a serverFiltered field only when the parent handles it', () => {
		// Filtering a server-paginated list on the client would only ever narrow
		// the page in hand, silently hiding every match on other pages.
		const serverField: FieldConfig<Row> = {
			key: 'category',
			label: 'Category',
			type: 'string',
			filterable: true,
			serverFiltered: true,
			getValue: (r) => r.category
		};
		const state: FilterState = { category: stringFilter(['a']) };
		const item = row({ category: 'b' });

		expect(matchesFilters(item, [serverField], state, { tags: false, fields: true })).toBe(true);
		expect(matchesFilters(item, [serverField], state, { tags: false, fields: false })).toBe(false);
	});

	it('skips the tags field only when tag filtering is server-side', () => {
		const state: FilterState = { tags: { type: 'array', values: new Set(['prod']) } };
		const item = row({ tags: ['other'] });

		expect(matchesFilters(item, [tagsField], state, { tags: true, fields: false })).toBe(true);
		expect(matchesFilters(item, [tagsField], state, { tags: false, fields: false })).toBe(false);
	});

	it('ignores fields that are not filterable at all', () => {
		const plain: FieldConfig<Row> = { ...nameField, filterable: false };
		const state: FilterState = { name: stringFilter(['a']) };

		expect(matchesFilters(row({ name: 'zzz' }), [plain], state, CLIENT_ONLY)).toBe(true);
	});
});

describe('matchesSearch', () => {
	it('only searches fields that opted in', () => {
		// Matching every field by default meant a date field turned "2026" into a
		// hit on every row.
		const optedOut: FieldConfig<Row> = { ...nameField, searchable: false };

		expect(matchesSearch(row({ name: 'switch' }), [nameField], 'switch')).toBe(true);
		expect(matchesSearch(row({ name: 'switch' }), [optedOut], 'switch')).toBe(false);
	});

	it('matches any element of an array field', () => {
		const searchableTags: FieldConfig<Row> = { ...tagsField, searchable: true };

		expect(matchesSearch(row({ tags: ['prod', 'db'] }), [searchableTags], 'db')).toBe(true);
		expect(matchesSearch(row({ tags: ['prod'] }), [searchableTags], 'db')).toBe(false);
	});

	it('matches everything for a blank or whitespace query', () => {
		expect(matchesSearch(row({ name: null }), [nameField], '')).toBe(true);
		expect(matchesSearch(row({ name: null }), [nameField], '   ')).toBe(true);
	});

	it('is case insensitive', () => {
		expect(matchesSearch(row({ name: 'Switch' }), [nameField], 'sWiTcH')).toBe(true);
	});
});

describe('hasActiveFilters', () => {
	it('reports staleness on its own', () => {
		expect(hasActiveFilters([nameField], {}, true)).toBe(true);
	});

	it('reports a boolean field only when a side is unchecked', () => {
		const both: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: true }
		};
		const one: FilterState = {
			hidden: { type: 'boolean', values: new Set(), showTrue: true, showFalse: false }
		};

		expect(hasActiveFilters([hiddenField], both, false)).toBe(false);
		expect(hasActiveFilters([hiddenField], one, false)).toBe(true);
	});

	it('reports a value field only once something is selected', () => {
		expect(hasActiveFilters([nameField], { name: stringFilter([]) }, false)).toBe(false);
		expect(hasActiveFilters([nameField], { name: stringFilter(['a']) }, false)).toBe(true);
	});
});

describe('getUniqueValues', () => {
	it('flattens array fields so each element is its own option', () => {
		const items = [row({ tags: ['b', 'a'] }), row({ tags: ['a'] })];

		expect(getUniqueValues(items, tagsField)).toEqual(['a', 'b']);
	});

	it('drops nulls and empty strings', () => {
		const items = [row({ name: 'a' }), row({ name: null }), row({ name: '' })];

		expect(getUniqueValues(items, nameField)).toEqual(['a']);
	});
});
