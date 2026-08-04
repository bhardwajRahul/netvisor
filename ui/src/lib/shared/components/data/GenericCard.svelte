<script lang="ts" generics="T">
	import type { CardAction, CardField, TagProps } from './types';
	import type { EntityColumn } from './table/columns';
	import Tag from './Tag.svelte';
	import EntityTag from './EntityTag.svelte';
	import FieldValue from './FieldValue.svelte';
	import type { Snippet } from 'svelte';
	import { type IconComponent } from '$lib/shared/utils/types';
	import { tooltip } from '$lib/shared/actions/tooltip';

	interface Props {
		title: string;
		link?: string;
		subtitle?: string;
		status?: TagProps | null;
		Icon?: IconComponent | null;
		iconColor?: string;
		actions?: CardAction[];
		/** Legacy per-card field list, for cards not yet on the shared definition. */
		fields?: CardField[];
		/**
		 * The shared field definition, rendered exactly as the table renders it.
		 *
		 * Supplying this (with `item`) is what puts card and table on one list:
		 * the same fields, the same chips, and the same visibility choices. It
		 * takes precedence over `fields`.
		 */
		columns?: EntityColumn<T>[] | null;
		item?: T | null;
		children?: Snippet;
		selected?: boolean;
		onSelectionChange?: (selected: boolean) => void;
		selectable?: boolean;
	}

	let {
		title,
		link = '',
		subtitle = '',
		status = null,
		Icon = null,
		iconColor = 'text-blue-400',
		actions = [],
		fields = [],
		columns = null,
		item = null,
		children,
		selected = false,
		selectable = true,
		onSelectionChange = () => {}
	}: Props = $props();

	let sharedColumns = $derived(columns && item !== null ? columns : null);

	// Helper to check if value is an array
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function isArrayValue(value: string | any[]): value is any[] {
		return Array.isArray(value);
	}

	function handleCheckboxChange(e: Event) {
		const target = e.target as HTMLInputElement;
		onSelectionChange(target.checked);
	}
</script>

<div class="card flex h-full flex-col {selected ? 'card-selected' : ''}">
	<!-- Checkbox (shown when selectable) -->
	{#if selectable}
		<div class="absolute right-4 top-4 flex-shrink-0">
			<input
				type="checkbox"
				checked={selected}
				onchange={handleCheckboxChange}
				onclick={(e) => e.stopPropagation()}
				class="checkbox-card h-5 w-5"
			/>
		</div>
	{/if}

	<!-- Header -->
	<div class="mb-4 flex items-start">
		<div class="flex items-center space-x-3">
			{#if Icon}
				<Icon size={28} class={iconColor} />
			{/if}
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<div class="min-w-0">
						{#if link}
							<a
								href={link}
								class="text-primary hover:text-info text-lg font-semibold"
								target="_blank"
							>
								{title}
							</a>
						{:else}
							<h3 class="text-primary text-lg font-semibold">
								{title}
							</h3>
						{/if}
					</div>
					{#if status}
						<div class="flex-shrink-0">
							<Tag {...status} />
						</div>
					{/if}
				</div>
				{#if subtitle}
					<p class="text-secondary text-sm">
						{subtitle}
					</p>
				{/if}
			</div>
		</div>
	</div>

	<!-- Content - grows to fill available space -->
	<div class="flex-grow space-y-3">
		{#if sharedColumns}
			{#each sharedColumns as column (column.id)}
				<div class="text-sm">
					<div class="flex flex-wrap items-center gap-2">
						<span class="text-secondary">{column.label}:</span>
						<FieldValue item={item as T} {column} />
					</div>
				</div>
			{/each}
		{:else}
			{#each fields as field, i (field.label + i)}
				{#if field.snippet}
					<div>
						{@render field.snippet()}
					</div>
				{:else}
					<div class="text-sm">
						{#if field.value}
							{#if isArrayValue(field.value)}
								<div class="flex flex-wrap items-center gap-2">
									<span class="text-secondary">{field.label}:</span>
									{#if field.value.length > 0}
										{#each field.value as entry (entry.id)}
											{#if entry.entityRef}
												<EntityTag
													entityRef={entry.entityRef}
													icon={entry.icon}
													disabled={entry.disabled}
													color={field.color || entry.color}
													badge={entry.badge}
													label={entry.label}
												/>
											{:else}
												<Tag
													icon={entry.icon}
													disabled={entry.disabled}
													color={field.color || entry.color}
													badge={entry.badge}
													label={entry.label}
													title={entry.title}
												/>
											{/if}
										{/each}
									{:else}
										<span class="text-muted"
											>{field.emptyText || `No ${field.label.toLowerCase()}`}</span
										>
									{/if}
								</div>
							{:else}
								<div class="text-sm">
									<span class="text-secondary">{field.label}: </span><span
										class="text-tertiary ml-2"
										style="word-wrap: break-word; word-break: break-word;">{field.value}</span
									>
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			{/each}
		{/if}
	</div>

	<!-- Optional additional content -->
	{#if children}
		<div>
			{@render children()}
		</div>
	{/if}

	<!-- Action Buttons -->
	{#if actions.length > 0}
		<div class="card-divider-h mt-4 flex items-center justify-between pt-4">
			{#each actions as action (action.label)}
				{@const cls = action.class ? action.class : 'btn-icon'}
				{@const explicitTooltip =
					typeof action.tooltip === 'function'
						? action.tooltip(!!action.disabled)
						: (action.tooltip ?? null)}
				<!--
					The label floats in a tooltip rather than growing inside the button.
					An in-flow label has to span its neighbours to fit its text, which
					made the widest action cover the ones beside it. Matches the table.
				-->
				<button
					onclick={action.onClick}
					disabled={action.disabled}
					use:tooltip
					data-tooltip={explicitTooltip ?? action.label}
					aria-label={action.label}
					class="{cls} disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if action.forceLabel}
						<!-- Always-labelled action: the label is the affordance, not a hover
						     reveal, so it renders in flow and the button sizes to it. -->
						<div class="flex items-center justify-center whitespace-nowrap">
							<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
							<span class="ml-2">{action.label}</span>
						</div>
					{:else}
						<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
