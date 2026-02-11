<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import Sidebar from '$lib/shared/components/layout/Sidebar.svelte';
	import { onDestroy, onMount } from 'svelte';
	import { discoverySSEManager } from '$lib/features/discovery/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { getMetadata } from '$lib/shared/stores/metadata';
	import { topologySSEManager } from '$lib/features/topology/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import BillingPlanModal from '$lib/features/billing/BillingPlanModal.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { showBillingPlanModal } from '$lib/features/billing/stores';

	// Read hash immediately during script initialization, before onMount
	const initialHash = typeof window !== 'undefined' ? window.location.hash.substring(1) : '';

	// After first billing checkout, trigger daemon setup
	// Don't clean billing_flow from URL here — AppShell needs it to fire billing_completed
	if (typeof window !== 'undefined') {
		const params = new URLSearchParams(window.location.search);
		if (params.get('billing_flow') === 'checkout') {
			sessionStorage.setItem('showDaemonSetup', 'true');
		}
	}

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let isAuthenticated = $derived(currentUserQuery.data != null);
	let isCheckingAuth = $derived(currentUserQuery.isPending);

	// TanStack Query for daemons - used to determine default tab
	// Only fetch when authenticated to avoid 401 errors during onboarding
	const daemonsQuery = useDaemonsQuery({ enabled: () => isAuthenticated });

	// Billing modal: show when billing is enabled but user has no active plan
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let organization = $derived(organizationQuery.data);
	let needsPlanSelection = $derived(
		billingEnabled && organization != null && !isBillingPlanActive(organization)
	);
	let showBillingModal = $derived(needsPlanSelection || $showBillingPlanModal);

	let activeTab = $state(initialHash || 'topology');
	let appInitialized = $state(false);
	let sidebarCollapsed = $state(false);
	let dataLoadingStarted = $state(false);
	let showSettings = $state(false);
	let isPastDue = $derived(organization?.plan_status === 'past_due');
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let allTabs = $state<Array<{ id: string; component: any; isReadOnly: boolean }>>([]);

	// Update URL hash when activeTab changes
	$effect(() => {
		if (typeof window !== 'undefined' && activeTab) {
			window.location.hash = activeTab;
		}
	});

	// Set initial tab based on daemons (only if no hash was specified in URL)
	// Suppress when billing modal is showing — user must pick a plan first
	let initialTabSet = $state(false);
	$effect(() => {
		if (!initialHash && !initialTabSet && daemonsQuery.isSuccess && !showBillingModal) {
			const hasDaemons = (daemonsQuery.data?.length ?? 0) > 0;
			const wantsDaemonSetup =
				typeof sessionStorage !== 'undefined' &&
				sessionStorage.getItem('showDaemonSetup') === 'true';
			activeTab = hasDaemons && !wantsDaemonSetup ? 'topology' : 'daemons';
			initialTabSet = true;
		}
	});

	// Auto-open settings modal to billing tab when past_due
	$effect(() => {
		if (isPastDue && appInitialized) {
			showSettings = true;
		}
	});

	// Function to handle browser navigation (back/forward)
	function handleHashChange() {
		if (typeof window !== 'undefined') {
			const hash = window.location.hash.substring(1);
			if (hash && hash !== activeTab) {
				activeTab = hash;
			}
		}
	}

	// Initialize app when authenticated
	// TanStack Query handles data fetching in components - no need for cascading loads
	async function initializeApp() {
		if (dataLoadingStarted) return;
		dataLoadingStarted = true;

		// Load metadata (static config) - required before components render
		await getMetadata();

		// Connect SSE managers for real-time updates
		topologySSEManager.connect();
		discoverySSEManager.connect();

		appInitialized = true;
	}

	// Reactive effect: initialize app when authenticated
	// The layout handles auth check via TanStack Query, so we just wait for it to complete
	$effect(() => {
		if (isAuthenticated && !isCheckingAuth && !dataLoadingStarted) {
			initializeApp();
		}
	});

	onMount(() => {
		// Listen for hash changes (browser back/forward)
		if (typeof window !== 'undefined') {
			window.addEventListener('hashchange', handleHashChange);
		}
	});

	onDestroy(() => {
		topologySSEManager.disconnect();
		discoverySSEManager.disconnect();

		if (typeof window !== 'undefined') {
			window.removeEventListener('hashchange', handleHashChange);
		}
	});
</script>

{#if appInitialized}
	<div class="flex h-screen">
		<!-- Sidebar -->
		<div class="flex-shrink-0">
			<Sidebar
				bind:activeTab
				bind:collapsed={sidebarCollapsed}
				bind:allTabs
				bind:showSettings
				settingsInitialTab={isPastDue ? 'billing' : 'account'}
				settingsDismissible={!isPastDue}
			/>
		</div>

		<!-- Main Content -->
		<main
			class="flex-1 overflow-auto transition-all duration-300"
			class:ml-16={sidebarCollapsed}
			class:ml-64={!sidebarCollapsed}
		>
			<div class="p-8 [&_.sticky]:sticky [&_.sticky]:top-0">
				<!-- Programmatically render all tabs based on sidebar config -->
				{#each allTabs as tab (tab.id)}
					<div class:hidden={activeTab !== tab.id}>
						<tab.component isReadOnly={tab.isReadOnly} />
					</div>
				{/each}
			</div>

			<Toast />
		</main>
	</div>

	<!-- Billing modal rendered last so it stacks on top of other modals -->
	<BillingPlanModal
		isOpen={showBillingModal}
		dismissible={!needsPlanSelection}
		onClose={() => showBillingPlanModal.set(false)}
	/>
{:else}
	<!-- Data still loading -->
	<Loading />
{/if}
