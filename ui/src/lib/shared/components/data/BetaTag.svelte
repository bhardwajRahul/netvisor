<script lang="ts">
	let { tooltip = '' }: { tooltip?: string } = $props();
</script>

<span class="group relative inline-flex items-center">
	<span
		class="inline-flex items-center rounded bg-purple-500/20 px-1.5 py-0.5 text-xs font-medium text-purple-400"
	>
		Beta
	</span>
	{#if tooltip}
		<span
			class="pointer-events-none absolute left-1/2 top-full z-50 mt-1 -translate-x-1/2 whitespace-nowrap rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 opacity-0 shadow-lg transition-opacity group-hover:opacity-100"
		>
			{tooltip}
		</span>
	{/if}
</span>
