import { getFieldKey, type FieldConfig, type GroupPosition } from '../types';
import { getFieldValue } from './fieldValues';

/**
 * Stands in for a null group key, which has no string form of its own.
 *
 * Prefixed with NUL because Postgres text values cannot contain one, so this
 * can never collide with a real group value. Built with `fromCharCode` rather
 * than a literal to keep a raw NUL byte out of the source file.
 */
export const UNGROUPED_KEY = `${String.fromCharCode(0)}ungrouped`;

/** A per-group total from the server, across every page. */
export interface ServerGroupCount {
	value?: string | null;
	count: number;
}

/**
 * Bucket items by the grouped field.
 *
 * When the server supplied group totals the rows already arrive in the server's
 * group order, so the buckets are left in insertion order — re-sorting here
 * would desync the headers from the cumulative offsets those totals index by.
 */
export function groupItems<T>(
	items: T[],
	fields: FieldConfig<T>[],
	groupFieldKey: string | null,
	ungroupedLabel: string,
	preserveOrder: boolean
): Map<string, T[]> {
	if (!groupFieldKey) return new Map();

	const field = fields.find((f) => getFieldKey(f) === groupFieldKey);
	if (!field) return new Map();

	const groups = new Map<string, T[]>();

	items.forEach((item) => {
		const value = getFieldValue(item, field);
		const groupKey = value !== null && value !== undefined ? String(value) : ungroupedLabel;

		if (!groups.has(groupKey)) {
			groups.set(groupKey, []);
		}
		groups.get(groupKey)!.push(item);
	});

	if (preserveOrder) return groups;

	return new Map([...groups.entries()].sort((a, b) => a[0].localeCompare(b[0])));
}

/**
 * Where each group starts in the full ordered result set.
 *
 * The server returns groups in the same order it orders rows, so a running sum
 * of the counts gives every group's global offset — which is what turns "this
 * page holds rows 100-199" into "this is rows 1-40 of that group".
 */
export function computeGroupOffsets(counts: ServerGroupCount[] | null): Map<string, GroupPosition> {
	const offsets = new Map<string, GroupPosition>();
	let cursor = 0;

	for (const group of counts ?? []) {
		offsets.set(group.value ?? UNGROUPED_KEY, { start: cursor, count: group.count });
		cursor += group.count;
	}

	return offsets;
}

/**
 * The value the server grouped these rows under, which is not always what the
 * header displays — a network group reads as a name but groups by id.
 */
export function serverGroupKey<T>(
	groupItems: T[],
	fields: FieldConfig<T>[],
	groupFieldKey: string | null
): string {
	const field = fields.find((f) => getFieldKey(f) === groupFieldKey);
	if (!field || groupItems.length === 0) return UNGROUPED_KEY;

	const raw = field.getGroupValue
		? field.getGroupValue(groupItems[0])
		: getFieldValue(groupItems[0], field);

	return raw === null || raw === undefined ? UNGROUPED_KEY : String(raw);
}
