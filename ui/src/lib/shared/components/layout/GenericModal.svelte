<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';

	/**
	 * Tab definition for modal header tabs
	 */
	export interface ModalTab {
		id: string;
		label: string;
		icon?: IconComponent;
		notification?: boolean;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { X } from 'lucide-svelte';
	import { common_closeModal, common_modal } from '$lib/paraglide/messages';

	let {
		title = common_modal(),
		centerTitle = false,
		isOpen = false,
		onClose = null,
		size = 'lg',
		preventCloseOnClickOutside = false,
		showCloseButton = true,
		showBackdrop = true,
		tabs = [],
		activeTab = $bindable(''),
		onTabChange = null,
		onOpen = null,
		instanceKey = $bindable(0),
		headerIcon,
		children,
		footer
	}: {
		title?: string;
		centerTitle?: boolean;
		isOpen?: boolean;
		onClose?: (() => void) | null;
		size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
		preventCloseOnClickOutside?: boolean;
		showCloseButton?: boolean;
		showBackdrop?: boolean;
		tabs?: ModalTab[];
		activeTab?: string;
		onTabChange?: ((tabId: string) => void) | null;
		onOpen?: (() => void) | null;
		instanceKey?: number;
		headerIcon?: Snippet;
		children?: Snippet<[number]>;
		footer?: Snippet;
	} = $props();

	// Track previous open state to detect open transition
	let wasOpen = $state(false);

	function handleTabClick(tabId: string) {
		activeTab = tabId;
		onTabChange?.(tabId);
	}

	// Lock body scroll when modal is open
	$effect(() => {
		if (typeof window !== 'undefined' && isOpen) {
			document.body.style.overflow = 'hidden';
			return () => {
				document.body.style.overflow = '';
			};
		}
	});

	// Fire onOpen callback when modal transitions from closed to open
	$effect(() => {
		if (isOpen && !wasOpen) {
			instanceKey++;
			if (tabs.length > 0 && !tabs.some((t) => t.id === activeTab)) {
				activeTab = tabs[0].id;
				onTabChange?.(activeTab);
			}
			onOpen?.();
		}
		wasOpen = isOpen;
	});

	// Size classes
	const sizeClasses: Record<string, string> = {
		sm: 'max-w-md',
		md: 'max-w-lg',
		lg: 'max-w-2xl',
		xl: 'max-w-4xl',
		full: 'max-w-7xl'
	};

	function handleClose() {
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

<svelte:window onkeydown={handleKeydown} />

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
			<!-- Header (hidden when no title, no close button, and no tabs) -->
			{#if title || showCloseButton || tabs.length > 0}
				<div class="modal-header flex-col gap-0 {tabs.length > 0 ? 'pb-0' : ''}">
					<!-- Title row -->
					<div class="flex w-full items-center justify-between">
						{#if centerTitle}
							{@render headerIcon?.()}
							<h2
								id="modal-title"
								class="text-primary absolute left-1/2 -translate-x-1/2 text-xl font-semibold"
							>
								{title}
							</h2>
						{:else}
							<div class="flex items-center gap-3">
								{@render headerIcon?.()}
								<h2 id="modal-title" class="text-primary text-xl font-semibold">
									{title}
								</h2>
							</div>
						{/if}

						{#if showCloseButton}
							<button
								type="button"
								onclick={handleClose}
								class="btn-icon"
								aria-label={common_closeModal()}
							>
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
										? 'text-primary border-blue-500'
										: 'text-muted hover:text-secondary border-transparent'}"
									aria-current={activeTab === tab.id ? 'page' : undefined}
								>
									<div class="flex items-center gap-2">
										{#if tab.icon}
											<span class="relative">
												<tab.icon class="h-4 w-4" />
												{#if tab.notification}
													<span class="absolute -right-1 -top-1 h-2 w-2 rounded-full bg-amber-500"
													></span>
												{/if}
											</span>
										{/if}
										{tab.label}
									</div>
								</button>
							{/each}
						</nav>
					{/if}
				</div>
			{/if}

			<!-- Content slot -->
			<div class="modal-content">
				{@render children?.(instanceKey)}
			</div>

			<!-- Footer slot -->
			{@render footer?.()}
		</div>
	</div>
{/if}
