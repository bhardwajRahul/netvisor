<script lang="ts">
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';

	let {
		icon = null,
		color = 'Gray',
		disabled = false,
		label,
		badge = ''
	}: {
		icon?: IconComponent | null;
		color?: Color;
		disabled?: boolean;
		label: string;
		badge?: string;
	} = $props();

	// Make colorHelper reactive to color changes
	let colorHelper = $derived(createColorHelper(color));
	let bgColor = $derived(colorHelper.bg);
	let textColor = $derived(colorHelper.text);
</script>

<div class="inline-flex flex-shrink-0 items-center gap-1 whitespace-nowrap rounded">
	<!-- Main content -->
	<span
		class="inline-flex items-center gap-1 {!disabled ? bgColor : 'bg-gray-700/30'} {!disabled
			? textColor
			: 'text-tertiary'} rounded px-2 py-0.5 text-xs font-medium"
	>
		{#if icon}
			{@const Icon = icon}
			<Icon size={16} class={textColor} />
		{/if}

		<span class="truncate">{label}</span>
		{#if badge.length > 0}
			<span class="flex-shrink-0 {textColor}">{badge}</span>
		{/if}
	</span>
</div>
