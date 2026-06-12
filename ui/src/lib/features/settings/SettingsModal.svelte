<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { User, Building2, CreditCard, Settings, Monitor } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import AccountTab from './AccountTab.svelte';
	import OrganizationTab from './OrganizationTab.svelte';
	import BillingTab from './BillingTab.svelte';
	import SystemTab from './SystemTab.svelte';
	import {
		common_account,
		common_billing,
		common_organization,
		common_settings,
		common_system
	} from '$lib/paraglide/messages';

	let {
		isOpen = false,
		onClose,
		initialTab = 'account',
		dismissible = true,
		name = undefined
	}: {
		isOpen: boolean;
		onClose: () => void;
		initialTab?: string;
		dismissible?: boolean;
		name?: string;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();

	let currentUser = $derived(currentUserQuery.data);
	let org = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isBillingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let billingNeedsAttention = $derived(
		!org?.plan ||
			org?.plan_status === 'past_due' ||
			org?.plan_status === 'cancelled' ||
			(org?.plan_status === 'trialing' && !org?.has_payment_method)
	);

	// Tab and sub-view state
	let activeTab = $state('account');
	let accountSubView = $state<'main' | 'credentials' | 'cookies'>('main');
	let orgSubView = $state<'main' | 'edit'>('main');

	// Define base tabs
	let baseTabs = $derived<ModalTab[]>([
		{ id: 'account', label: common_account(), icon: User },
		{ id: 'organization', label: common_organization(), icon: Building2 },
		{
			id: 'billing',
			label: common_billing(),
			icon: CreditCard,
			notification: billingNeedsAttention
		},
		{ id: 'system', label: common_system(), icon: Monitor }
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
		activeTab = initialTab;
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
		if (!dismissible) return;
		// Reset sub-views on close
		accountSubView = 'main';
		orgSubView = 'main';
		onClose();
	}
</script>

<GenericModal
	{isOpen}
	title={common_settings()}
	{name}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={dismissible}
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
			<BillingTab {isOpen} onClose={handleClose} {dismissible} />
		{:else if activeTab === 'system'}
			<SystemTab />
		{/if}
	</div>
</GenericModal>
