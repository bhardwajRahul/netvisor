<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import type { IconComponent } from '$lib/shared/utils/types';

	interface Props {
		title: string;
		expanded?: boolean;
		description?: string;
		/** Rendered before the title, for a card whose subject has a symbol of its own. */
		icon?: IconComponent;
		/** Rendered beside the title — a count, a status. Sits left of the chevron. */
		badge?: Snippet;
		children: Snippet;
	}

	let { title, expanded = $bindable(true), description, icon, badge, children }: Props = $props();

	function toggle() {
		expanded = !expanded;
	}
</script>

<div class="card card-static">
	<button
		type="button"
		class="flex w-full items-center justify-between text-left focus:outline-none"
		onclick={toggle}
		aria-expanded={expanded}
	>
		<div class="min-w-0">
			<div class="flex items-center gap-2">
				{#if icon}
					{@const Icon = icon}
					<Icon class="text-secondary h-4 w-4 flex-shrink-0" />
				{/if}
				<h3 class="text-primary text-sm font-semibold">{title}</h3>
				{#if badge}
					{@render badge()}
				{/if}
			</div>
			{#if description}
				<p class="text-tertiary mt-0.5 text-xs">{description}</p>
			{/if}
		</div>
		{#if expanded}
			<ChevronDown class="text-secondary ml-3 h-4 w-4 flex-shrink-0" />
		{:else}
			<ChevronRight class="text-secondary ml-3 h-4 w-4 flex-shrink-0" />
		{/if}
	</button>

	{#if expanded}
		<div class="mt-3 space-y-3">
			{@render children()}
		</div>
	{/if}
</div>
