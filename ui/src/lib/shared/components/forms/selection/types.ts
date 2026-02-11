import type { TagProps } from '../../data/types';
import type { IconComponent } from '$lib/shared/utils/types';
import type { Component } from 'svelte';

// @typescript-eslint/no-explicit-any
export interface EntityDisplayComponent<T, C> {
	// Required methods
	getId(item: T): string;
	getLabel(item: T, context?: C): string;

	// Optional methods with defaults
	getDescription?(item: T, context: C): string;
	getIcon?(item: T, context: C): IconComponent | null;
	getIconColor?(item: T, context: C): string | null;
	getTags?(item: T, context: C): TagProps[];
	getCategory?(item: T, context: C): string | null;
	getDisabled?(item: T, context: C): boolean;

	// Inline editing support
	supportsInlineEdit?: boolean;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	InlineEditorComponent?: Component<any>;
}
