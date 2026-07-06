<script lang="ts">
	import type { ModalTab } from './GenericModal.svelte';
	import { Check } from 'lucide-svelte';
	import { common_progressSteps } from '$lib/paraglide/messages';

	let {
		tabs = [],
		activeTab = '',
		onTabClick
	}: {
		tabs: ModalTab[];
		activeTab: string;
		onTabClick: (tabId: string) => void;
	} = $props();

	let activeIndex = $derived(tabs.findIndex((t) => t.id === activeTab));
</script>

<nav class="flex w-full items-start pb-4 pt-4" aria-label={common_progressSteps()}>
	{#each tabs as tab, i (tab.id)}
		{@const isCompleted = i < activeIndex && !tab.disabled}
		{@const isCurrent = tab.id === activeTab}
		{@const isClickable = isCompleted}

		<!-- Connecting line before this step (not for first) -->
		{#if i > 0}
			<div class="mt-4 flex-1 px-1">
				<div
					class="h-0.5 {i <= activeIndex ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'}"
				></div>
			</div>
		{/if}

		<!-- Step -->
		<button
			type="button"
			onclick={() => isClickable && onTabClick(tab.id)}
			class="flex flex-col items-center gap-1.5 {isClickable ? 'cursor-pointer' : 'cursor-default'}"
			aria-current={isCurrent ? 'step' : undefined}
			aria-disabled={tab.disabled ? 'true' : undefined}
			tabindex={isClickable ? 0 : -1}
		>
			<!-- Circle -->
			<div
				class="flex h-8 w-8 items-center justify-center rounded-full text-sm font-semibold transition-colors
				{isCurrent
					? 'bg-blue-600 text-white'
					: isCompleted
						? 'bg-blue-600 text-white'
						: tab.disabled
							? 'bg-gray-200 text-gray-400 opacity-50 dark:bg-gray-700 dark:text-gray-500'
							: 'bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}"
			>
				{#if isCompleted}
					<Check class="h-4 w-4" />
				{:else}
					{i + 1}
				{/if}
			</div>

			<!-- Label -->
			<span
				class="whitespace-nowrap text-center text-xs
				{isCurrent
					? 'text-primary font-semibold'
					: isCompleted
						? 'text-secondary'
						: tab.disabled
							? 'text-muted opacity-50'
							: 'text-muted'}"
			>
				{tab.label}
			</span>
		</button>
	{/each}
</nav>
