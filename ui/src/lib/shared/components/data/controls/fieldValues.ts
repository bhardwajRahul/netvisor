import { getFieldKey, type FieldConfig } from '../types';

/** What a field resolves to for one item. */
export type FieldValue = string | boolean | Date | string[] | null;

/**
 * Resolve a field's value for an item.
 *
 * Falls back to a property lookup by field key so a field that only needs to
 * read `item.name` doesn't have to declare a `getValue`.
 */
export function getFieldValue<T>(item: T, field: FieldConfig<T>): FieldValue {
	if (field.getValue) {
		return field.getValue(item);
	}
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	return (item as any)[getFieldKey(field)] ?? null;
}

/**
 * The distinct values a field takes across the given items, sorted.
 *
 * Array-typed fields are flattened, so a "tags" field offers each individual
 * tag as an option rather than one option per combination.
 *
 * Only meaningful for lists holding every row: on a server-paginated list this
 * would only ever see the loaded page, which is why `FieldConfig.filterOptions`
 * exists to bypass it.
 */
export function getUniqueValues<T>(items: T[], field: FieldConfig<T>): string[] {
	const values = new Set<string>();

	items.forEach((item) => {
		const value = getFieldValue(item, field);
		if (value === null || value === undefined) return;

		if (field.type === 'array' && Array.isArray(value)) {
			value.forEach((v) => {
				if (v !== null && v !== undefined && v !== '') {
					values.add(String(v));
				}
			});
		} else if (value !== '') {
			values.add(String(value));
		}
	});

	return Array.from(values).sort();
}
