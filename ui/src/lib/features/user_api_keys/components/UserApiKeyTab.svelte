<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import UserApiKeyCard from './UserApiKeyCard.svelte';
	import UserApiKeyModal from './UserApiKeyModal.svelte';
	import {
		useUserApiKeysQuery,
		useUpdateUserApiKeyMutation,
		useDeleteUserApiKeyMutation,
		useBulkDeleteUserApiKeysMutation,
		type UserApiKey
	} from '../queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';

	import {
		common_apiKeys,
		common_create,
		common_name,
		common_networks,
		common_permissions,
		common_tags,
		userApiKeys_confirmBulkDelete,
		userApiKeys_confirmDelete,
		userApiKeys_noApiKeysSubtitle,
		userApiKeys_noApiKeysYet,
		userApiKeys_subtitle
	} from '$lib/paraglide/messages';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import { modalState } from '$lib/shared/stores/modal-registry';

	let { isReadOnly = false }: TabProps = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let isEmailVerified = $derived(currentUser?.email_verified ?? true);

	// Check if plan has api_access feature before querying
	const organizationQuery = useOrganizationQuery();
	let hasApiAccess = $derived.by(() => {
		const org = organizationQuery.data;
		if (!org?.plan) return false;
		return billingPlans.getMetadata(org.plan.type).features.api_access;
	});

	// Queries
	const tagsQuery = useTagsQuery();
	const userApiKeysQuery = useUserApiKeysQuery({ enabled: () => hasApiAccess });
	const networksQuery = useNetworksQuery();

	// Mutations
	const updateMutation = useUpdateUserApiKeyMutation();
	const deleteMutation = useDeleteUserApiKeyMutation();
	const bulkDeleteMutation = useBulkDeleteUserApiKeysMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let userApiKeysData = $derived(userApiKeysQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(userApiKeysQuery.isPending);

	let showModal = $state(false);
	let editingApiKey = $state<UserApiKey | null>(null);

	// Deep-link: open user API key editor from URL
	$effect(() => {
		if ($modalState.name === 'user-api-key' && !showModal) {
			if ($modalState.id) {
				const entity = userApiKeysData.find((e) => e.id === $modalState.id);
				if (entity) {
					editingApiKey = entity;
					showModal = true;
				}
			} else {
				editingApiKey = null;
				showModal = true;
			}
		}
	});

	async function handleDelete(apiKey: UserApiKey) {
		if (confirm(userApiKeys_confirmDelete({ name: apiKey.name }))) {
			deleteMutation.mutate(apiKey.id);
		}
	}

	async function handleUpdate(apiKey: UserApiKey) {
		await updateMutation.mutateAsync(apiKey);
		showModal = false;
		editingApiKey = null;
	}

	function handleCreate() {
		showModal = true;
		editingApiKey = null;
	}

	function handleClose() {
		showModal = false;
		editingApiKey = null;
	}

	function handleEdit(apiKey: UserApiKey) {
		showModal = true;
		editingApiKey = apiKey;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(userApiKeys_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteMutation.mutateAsync(ids);
		}
	}

	function getUserApiKeyTags(apiKey: UserApiKey): string[] {
		return apiKey.tags ?? [];
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('UserApiKey', {});
	}

	const apiKeyFields: FieldConfig<UserApiKey>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true
		},
		{
			key: 'permissions',
			type: 'string',
			label: common_permissions(),
			filterable: true
		},
		{
			key: 'network_ids',
			type: 'array',
			label: common_networks(),
			getValue(item) {
				const ids = item.network_ids ?? [];
				return ids
					.map((id) => networksData.find((n) => n.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				return (entity.tags ?? [])
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		}
	];
</script>

<div class="space-y-6">
	<TabHeader title={common_apiKeys()} subtitle={userApiKeys_subtitle()}>
		<svelte:fragment slot="actions">
			{#if !isReadOnly && hasApiAccess}
				{#if !isEmailVerified}
					<span data-tooltip="Please verify email to create an API key" use:tooltip>
						<button class="btn-primary flex items-center opacity-50" disabled>
							<Plus class="h-5 w-5" />{common_create()}
						</button>
					</span>
				{:else}
					<button class="btn-primary flex items-center" onclick={handleCreate}>
						<Plus class="h-5 w-5" />{common_create()}
					</button>
				{/if}
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasApiAccess}
		<EmptyState
			title="API Access Not Available"
			subtitle="Your current plan does not include API access. Upgrade to a plan with API access to create and manage API keys."
		>
			<UpgradeButton feature="api_access" surface="api_keys_tab" />
		</EmptyState>
	{:else if isLoading}
		<Loading />
	{:else if userApiKeysData.length === 0}
		<EmptyState
			title={userApiKeys_noApiKeysYet()}
			subtitle={userApiKeys_noApiKeysSubtitle()}
			onClick={isEmailVerified ? handleCreate : undefined}
			cta={isEmailVerified ? common_create() : undefined}
		/>
	{:else}
		<DataControls
			items={userApiKeysData}
			fields={apiKeyFields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'UserApiKey'}
			getItemTags={getUserApiKeyTags}
			storageKey="scanopy-user-api-keys-table-state"
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: UserApiKey,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<UserApiKeyCard
					apiKey={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDelete}
					onEdit={isReadOnly ? undefined : handleEdit}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<UserApiKeyModal
	name="user-api-key"
	isOpen={showModal}
	onClose={handleClose}
	onUpdate={handleUpdate}
	apiKey={editingApiKey}
/>
