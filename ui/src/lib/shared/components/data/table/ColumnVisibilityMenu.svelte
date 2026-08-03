<script lang="ts" generics="T">
	import { Columns3 } from 'lucide-svelte';
	import type { EntityColumn } from './columns';
	import { common_columns, common_resetColumns } from '$lib/paraglide/messages';

	let {
		columns,
		visibility,
		onToggle,
		onReset
	}: {
		columns: EntityColumn<T>[];
		visibility: Record<string, boolean>;
		onToggle: (id: string) => void;
		onReset: () => void;
	} = $props();

	let open = $state(false);
	let menuElement = $state<HTMLDivElement | undefined>();

	/**
	 * The primary column stays put: it carries the row's identity and its
	 * checkbox, so hiding it would leave rows with nothing to identify them by.
	 */
	let toggleable = $derived(columns.filter((c) => !c.primary));

	function handleFocusOut(event: FocusEvent) {
		const next = event.relatedTarget as Node | null;
		if (next && menuElement?.contains(next)) return;
		open = false;
	}
</script>

<div bind:this={menuElement} class="relative" onfocusout={handleFocusOut} role="presentation">
	<button
		type="button"
		onclick={() => (open = !open)}
		class="btn-secondary h-[42px]"
		aria-expanded={open}
		aria-haspopup="true"
		title={common_columns()}
	>
		<Columns3 class="h-5 w-5" />
	</button>

	{#if open}
		<div
			class="card absolute right-0 z-30 mt-1 max-h-72 w-56 overflow-y-auto !rounded-lg !p-3 shadow-lg"
		>
			<div class="mb-2 flex items-center justify-between">
				<span class="text-primary text-sm font-semibold">{common_columns()}</span>
				<button
					type="button"
					onclick={onReset}
					class="text-tertiary hover:text-secondary text-xs transition-colors"
				>
					{common_resetColumns()}
				</button>
			</div>
			<div class="space-y-1.5">
				{#each toggleable as column (column.id)}
					<label class="flex cursor-pointer items-center gap-2">
						<input
							type="checkbox"
							checked={visibility[column.id] !== false}
							onchange={() => onToggle(column.id)}
							class="checkbox-card h-4 w-4 rounded"
						/>
						<span class="text-secondary truncate text-sm" title={column.label}>{column.label}</span>
					</label>
				{/each}
			</div>
		</div>
	{/if}
</div>
