<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';

	/**
	 * Tab definition for modal header tabs
	 */
	export interface ModalTab {
		id: string;
		label: string;
		icon?: IconComponent;
	}
</script>

<script lang="ts">
	import { X } from 'lucide-svelte';
	import { onDestroy } from 'svelte';

	export let title: string = 'Modal';
	export let centerTitle: boolean = false;
	export let isOpen: boolean = false;
	export let onClose: (() => void) | null = null;
	export let size: 'sm' | 'md' | 'lg' | 'xl' | 'full' = 'lg';
	export let preventCloseOnClickOutside: boolean = false;
	export let showCloseButton: boolean = true;
	export let showBackdrop: boolean = true;

	/** Optional tabs to display in modal header */
	export let tabs: ModalTab[] = [];
	/** Currently active tab id (bindable) */
	export let activeTab: string = '';
	/** Callback when tab changes */
	export let onTabChange: ((tabId: string) => void) | null = null;
	/** Callback when modal opens (fires on transition from closed to open) */
	export let onOpen: (() => void) | null = null;

	// Track previous open state to detect open transition
	let wasOpen = false;

	function handleTabClick(tabId: string) {
		activeTab = tabId;
		onTabChange?.(tabId);
	}

	$: if (typeof window !== 'undefined' && isOpen) {
		document.body.style.overflow = 'hidden';
	} else if (typeof window !== 'undefined') {
		document.body.style.overflow = '';
	}

	// Fire onOpen callback when modal transitions from closed to open
	// Also reset activeTab to first tab (or keep current if valid)
	$: {
		if (isOpen && !wasOpen) {
			// Reset to first tab if current activeTab is not in tabs list
			if (tabs.length > 0 && !tabs.some((t) => t.id === activeTab)) {
				activeTab = tabs[0].id;
				onTabChange?.(activeTab);
			}
			onOpen?.();
		}
		wasOpen = isOpen;
	}

	onDestroy(() => {
		if (typeof window !== 'undefined') {
			document.body.style.overflow = '';
		}
	});

	// Size classes
	const sizeClasses = {
		sm: 'max-w-md',
		md: 'max-w-lg',
		lg: 'max-w-2xl',
		xl: 'max-w-4xl',
		full: 'max-w-7xl'
	};

	function handleClose() {
		// Reset tab state on close
		activeTab = tabs.length > 0 ? tabs[0].id : '';
		onClose?.();
	}

	function handleBackdropClick(event: MouseEvent) {
		if (!preventCloseOnClickOutside && event.target === event.currentTarget) {
			handleClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && isOpen) {
			handleClose();
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
	<!-- Modal backdrop -->
	<div
		class={showBackdrop ? 'modal-page modal-background' : 'modal-page'}
		onclick={handleBackdropClick}
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		onkeydown={(e) => e.key === 'Escape' && handleClose()}
		tabindex="-1"
	>
		<!-- Modal content -->
		<div
			class="modal-container {sizeClasses[size]} {size === 'full'
				? 'h-[calc(100vh-8rem)]'
				: 'max-h-[calc(100vh-8rem)]'} flex flex-col"
		>
			<!-- Header -->
			<div class="modal-header flex-col gap-0 {tabs.length > 0 ? 'pb-0' : ''}">
				<!-- Title row -->
				<div class="flex w-full items-center justify-between">
					{#if centerTitle}
						{#if $$slots['header-icon']}
							<slot name="header-icon" />
						{/if}
						<h2
							id="modal-title"
							class="text-primary absolute left-1/2 -translate-x-1/2 text-xl font-semibold"
						>
							{title}
						</h2>
					{:else}
						<div class="flex items-center gap-3">
							{#if $$slots['header-icon']}
								<slot name="header-icon" />
							{/if}
							<h2 id="modal-title" class="text-primary text-xl font-semibold">
								{title}
							</h2>
						</div>
					{/if}

					{#if showCloseButton}
						<button type="button" onclick={handleClose} class="btn-icon" aria-label="Close modal">
							<X class="h-5 w-5" />
						</button>
					{/if}
				</div>

				<!-- Tab navigation (if tabs provided) -->
				{#if tabs.length > 0}
					<nav class="flex w-full space-x-6 pt-4" aria-label="Modal tabs">
						{#each tabs as tab (tab.id)}
							<button
								type="button"
								onclick={() => handleTabClick(tab.id)}
								class="border-b-2 px-1 pb-3 text-sm font-medium transition-colors
									{activeTab === tab.id
									? 'border-blue-500 text-primary'
									: 'border-transparent text-muted hover:text-secondary'}"
								aria-current={activeTab === tab.id ? 'page' : undefined}
							>
								<div class="flex items-center gap-2">
									{#if tab.icon}
										<svelte:component this={tab.icon} class="h-4 w-4" />
									{/if}
									{tab.label}
								</div>
							</button>
						{/each}
					</nav>
				{/if}
			</div>

			<!-- Content slot -->
			<div class="modal-content">
				<slot />
			</div>

			<!-- Footer slot -->
			{#if $$slots.footer}
				<div class="modal-footer">
					<slot name="footer" />
				</div>
			{/if}
		</div>
	</div>
{/if}
