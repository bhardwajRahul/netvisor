<script lang="ts">
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { Component } from 'svelte';
	import { common_contentTabs } from '$lib/paraglide/messages';

	export interface SubTab {
		id: string;
		label: string;
		icon: IconComponent;
		component: Component;
	}

	let {
		tabs,
		activeTab = $bindable(),
		isReadOnly = false,
		notifications
	}: {
		tabs: SubTab[];
		activeTab: string;
		isReadOnly: boolean;
		notifications?: Record<string, string>;
	} = $props();
</script>

<div>
	<!-- Sub-tab navigation bar -->
	<nav
		class="mb-6 flex space-x-6 border-b"
		style="border-color: var(--color-border)"
		aria-label={common_contentTabs()}
	>
		{#each tabs as tab (tab.id)}
			<button
				type="button"
				onclick={() => (activeTab = tab.id)}
				class="border-b-2 px-1 pb-3 text-sm font-medium transition-colors
				{activeTab === tab.id
					? 'text-primary border-blue-500'
					: 'text-muted hover:text-secondary border-transparent'}"
				aria-current={activeTab === tab.id ? 'page' : undefined}
			>
				<div class="flex items-center gap-2">
					<span class="relative">
						<tab.icon class="h-4 w-4" />
						{#if notifications?.[tab.id]}
							<span
								class="absolute -right-1 -top-1 h-2 w-2 rounded-full"
								style="background-color: {notifications[tab.id]}"
							></span>
						{/if}
					</span>
					{tab.label}
				</div>
			</button>
		{/each}
	</nav>

	<!-- Sub-tab content -->
	{#each tabs as tab (tab.id)}
		<div class={activeTab !== tab.id ? 'h-0 overflow-hidden' : ''}>
			<tab.component {isReadOnly} isActive={activeTab === tab.id} />
		</div>
	{/each}
</div>
