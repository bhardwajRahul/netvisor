<script lang="ts">
	import type { CardAction, CardField, TagProps } from './types';
	import Tag from './Tag.svelte';
	import EntityTag from './EntityTag.svelte';
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
		fields?: CardField[];
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
		children,
		selected = false,
		selectable = true,
		onSelectionChange = () => {}
	}: Props = $props();

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
									{#each field.value as item (item.id)}
										{#if item.entityRef}
											<EntityTag
												entityRef={item.entityRef}
												icon={item.icon}
												disabled={item.disabled}
												color={field.color || item.color}
												badge={item.badge}
												label={item.label}
											/>
										{:else}
											<Tag
												icon={item.icon}
												disabled={item.disabled}
												color={field.color || item.color}
												badge={item.badge}
												label={item.label}
												title={item.title}
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
			{#each actions as action, index (action.label)}
				{@const cls = action.class ? action.class : 'btn-icon'}
				{@const tooltipText =
					typeof action.tooltip === 'function'
						? action.tooltip(!!action.disabled)
						: (action.tooltip ?? null)}
				{@const isLeftEdge = index === 0}
				{@const isRightEdge = index === actions.length - 1}
				<div use:tooltip data-tooltip={tooltipText}>
					<button
						onclick={action.onClick}
						disabled={action.disabled}
						class="{action.disabled
							? ''
							: 'group'} relative overflow-visible transition-all duration-200 ease-in-out {cls}"
						title={tooltipText ? undefined : action.label}
					>
						{#if action.forceLabel}
							<!-- Always-labelled action: render the label IN FLOW so the button sizes to
							     its content. An absolutely-positioned label (as used for hover-reveal
							     below) overflows the icon-width button and overlaps the adjacent action. -->
							<div class="flex items-center justify-center whitespace-nowrap">
								<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
								<span class="ml-2">{action.label}</span>
							</div>
						{:else}
							<div
								class="flex items-center justify-center transition-opacity duration-200 group-hover:opacity-0"
							>
								<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
							</div>

							<!-- pointer-events-none: the invisible label still spans its neighbours. -->
							<div
								class="pointer-events-none absolute top-1/2 flex -translate-y-1/2 items-center justify-center whitespace-nowrap {action.disabled
									? 'opacity-0'
									: 'opacity-0 transition-all duration-200 ease-in-out group-hover:opacity-100'} {isLeftEdge
									? 'left-0'
									: isRightEdge
										? 'right-0'
										: 'left-1/2 -translate-x-1/2'} {cls}"
							>
								<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
								<span class="ml-2">{action.label}</span>
							</div>
						{/if}
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>
