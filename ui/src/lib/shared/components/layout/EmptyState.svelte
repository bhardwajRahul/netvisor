<script lang="ts">
	import { Plus, Search } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import type { IconComponent as IconComponentType } from '$lib/shared/utils/types';

	let {
		IconComponent = Search,
		title = '',
		subtitle = '',
		onClick = () => {},
		cta = '',
		children
	}: {
		IconComponent?: IconComponentType;
		title?: string;
		subtitle?: string;
		onClick?: () => void;
		cta?: string;
		children?: Snippet;
	} = $props();
</script>

<div class="py-12 text-center">
	<IconComponent size={16} class="text-secondary mx-auto mb-4 h-16 w-16" />
	<p class="text-secondary mb-4 text-lg">{title}</p>
	<p class="text-tertiary mb-6">{subtitle}</p>
	{#if children}
		{@render children()}
	{:else if cta.length > 0}
		<button onclick={onClick} class="btn-primary">
			<Plus class="h-4 w-4" />
			{cta}
		</button>
	{/if}
</div>
