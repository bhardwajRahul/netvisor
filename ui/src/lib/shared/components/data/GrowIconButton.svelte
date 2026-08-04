<script lang="ts">
	import type { CardAction } from './types';

	let {
		action,
		isRightEdge = false,
		isLeftEdge = false
	}: {
		action: CardAction;
		isRightEdge?: boolean;
		isLeftEdge?: boolean;
	} = $props();

	let cls = $derived(action.class ? action.class : 'btn-icon');
</script>

<button
	onclick={action.onClick}
	disabled={action.disabled}
	class="group relative overflow-visible transition-all duration-200 ease-in-out {cls}"
	title={action.label}
>
	<div class="flex items-center justify-center {action.forceLabel ? 'opacity-0' : ''}">
		<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
	</div>

	<!--
		pointer-events-none: this label is absolutely positioned and, while
		invisible, still spans its neighbours — without it the widest label sits on
		top of the adjacent buttons and swallows their hover and clicks.
	-->
	<div
		class="pointer-events-none absolute top-1/2 flex -translate-y-1/2 items-center justify-center whitespace-nowrap {action.disabled
			? 'opacity-0'
			: action.forceLabel
				? 'opacity-100'
				: 'opacity-0 transition-all duration-200 ease-in-out group-hover:opacity-100'} {isLeftEdge
			? 'left-0'
			: isRightEdge
				? 'right-0'
				: 'left-1/2 -translate-x-1/2'} {cls}"
	>
		<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
		<span class="ml-2">{action.label}</span>
	</div>
</button>
