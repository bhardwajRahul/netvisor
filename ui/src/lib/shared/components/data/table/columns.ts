import {
	getFieldKey,
	isDisplayField,
	isOrderableField,
	type ColumnConfig,
	type FieldConfig
} from '../types';

/**
 * A table column, derived from the field that already describes this data.
 *
 * There is deliberately no second list of columns: `FieldConfig` already
 * carries the label, the type and the value accessor, and `defineFields` forces
 * exhaustive coverage of the backend order-field union — so every column the
 * server can sort is guaranteed to exist here.
 */
export interface EntityColumn<T> {
	/**
	 * Always `getFieldKey(field)`.
	 *
	 * This is the load-bearing invariant of the whole table: it is what makes a
	 * header click dispatch a sort the backend actually accepts, because the same
	 * key is the field's `orderField` and therefore a valid `*OrderField` value.
	 */
	id: string;
	label: string;
	field: FieldConfig<T>;
	column: ColumnConfig<T>;
	/** Whether a header click can sort this column. */
	sortable: boolean;
	align: 'left' | 'right';
	width?: number;
	primary: boolean;
}

/** Column state the table owns but `DataControls` persists. */
export interface ColumnState {
	visibility: Record<string, boolean>;
	order: string[];
}

/**
 * Turn field configs into columns.
 *
 * Fields marked `column.hidden` produce nothing — they exist only to drive a
 * filter (a port number, say) and have no value worth a column of its own.
 */
export function fieldsToColumns<T>(fields: FieldConfig<T>[]): EntityColumn<T>[] {
	return fields
		.filter((field) => !field.column?.hidden)
		.map((field) => {
			const column = field.column ?? {};
			return {
				id: getFieldKey(field),
				label: field.label,
				field,
				column,
				// Mirrors the sort dropdown's rule, so a header can never offer a sort
				// the dropdown doesn't and vice versa.
				sortable: isOrderableField(field) || (isDisplayField(field) && field.sortable === true),
				align: column.align ?? 'left',
				width: column.width,
				primary: column.primary === true
			};
		});
}

/** Default visibility: everything except fields that opted out of first paint. */
export function defaultColumnVisibility<T>(columns: EntityColumn<T>[]): Record<string, boolean> {
	const visibility: Record<string, boolean> = {};
	for (const column of columns) {
		visibility[column.id] = column.column.hiddenByDefault !== true;
	}
	return visibility;
}

/** Default order: declaration order, which is the order the tab authored. */
export function defaultColumnOrder<T>(columns: EntityColumn<T>[]): string[] {
	return columns.map((c) => c.id);
}

/**
 * Fold persisted column state onto the columns that exist now.
 *
 * Renaming or removing a field must not leave a stale entry deciding anything,
 * and a newly added field must appear where it was declared rather than being
 * appended after the date columns at the end.
 */
export function reconcileColumnState<T>(
	columns: EntityColumn<T>[],
	stored: Partial<ColumnState> | undefined
): ColumnState {
	const defaults = defaultColumnVisibility(columns);
	const visibility: Record<string, boolean> = {};

	for (const column of columns) {
		const persisted = stored?.visibility?.[column.id];
		visibility[column.id] = typeof persisted === 'boolean' ? persisted : defaults[column.id];
	}

	const known = new Set(columns.map((c) => c.id));
	const storedOrder = (stored?.order ?? []).filter((id) => known.has(id));
	const ordered = new Set(storedOrder);

	// Splice unknown-to-the-stored-order columns back in at their declared index,
	// so inserting a field mid-list doesn't push it to the end for existing users.
	const order: string[] = [...storedOrder];
	columns.forEach((column, index) => {
		if (ordered.has(column.id)) return;
		order.splice(Math.min(index, order.length), 0, column.id);
	});

	return { visibility, order };
}

/** Columns to render, in persisted order, minus the hidden ones. */
export function visibleColumns<T>(
	columns: EntityColumn<T>[],
	state: ColumnState
): EntityColumn<T>[] {
	const byId = new Map(columns.map((c) => [c.id, c]));

	return state.order
		.map((id) => byId.get(id))
		.filter((c): c is EntityColumn<T> => Boolean(c) && state.visibility[c!.id] !== false);
}
