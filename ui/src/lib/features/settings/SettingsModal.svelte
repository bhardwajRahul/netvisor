<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { User, Building2, CreditCard, Settings } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import AccountTab from './AccountTab.svelte';
	import OrganizationTab from './OrganizationTab.svelte';
	import BillingTab from './BillingTab.svelte';
	import * as m from '$lib/paraglide/messages';

	let {
		isOpen = false,
		onClose
	}: {
		isOpen: boolean;
		onClose: () => void;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();

	let currentUser = $derived(currentUserQuery.data);
	let org = $derived(organizationQuery.data);

	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isBillingEnabled = $derived(org ? isBillingPlanActive(org) : false);

	// Tab and sub-view state
	let activeTab = $state('account');
	let accountSubView = $state<'main' | 'credentials'>('main');
	let orgSubView = $state<'main' | 'edit'>('main');

	// Define base tabs
	let baseTabs = $derived<ModalTab[]>([
		{ id: 'account', label: m.settings_account(), icon: User },
		{ id: 'organization', label: m.settings_organization(), icon: Building2 },
		{ id: 'billing', label: m.settings_billing(), icon: CreditCard }
	]);

	// Filter tabs based on permissions
	let visibleTabs = $derived(
		baseTabs.filter((tab) => {
			if (tab.id === 'organization') return isOwner;
			if (tab.id === 'billing') return isOwner && isBillingEnabled;
			return true;
		})
	);

	// Reset sub-views when modal opens or tab changes
	function handleOpen() {
		activeTab = 'account';
		accountSubView = 'main';
		orgSubView = 'main';
	}

	function handleTabChange(tabId: string) {
		activeTab = tabId;
		// Reset sub-views when switching tabs
		accountSubView = 'main';
		orgSubView = 'main';
	}

	function handleClose() {
		// Reset sub-views on close
		accountSubView = 'main';
		orgSubView = 'main';
		onClose();
	}
</script>

<GenericModal
	{isOpen}
	title={m.settings_title()}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
	tabs={visibleTabs}
	{activeTab}
	onTabChange={handleTabChange}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Settings} color="Blue" />
	{/snippet}

	<div class="flex h-[calc(100vh-16rem)] flex-col">
		{#if activeTab === 'account'}
			<AccountTab bind:subView={accountSubView} onClose={handleClose} />
		{:else if activeTab === 'organization'}
			<OrganizationTab bind:subView={orgSubView} onClose={handleClose} />
		{:else if activeTab === 'billing'}
			<BillingTab {isOpen} onClose={handleClose} />
		{/if}
	</div>
</GenericModal>
