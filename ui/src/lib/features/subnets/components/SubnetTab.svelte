<script lang="ts">
	import SubnetCard from './SubnetCard.svelte';
	import SubnetEditModal from './SubnetEditModal/SubnetEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import type { Subnet } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, type CardAction } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { Plus, Trash2, Edit } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		useSubnetsQuery,
		useCreateSubnetMutation,
		useUpdateSubnetMutation,
		useDeleteSubnetMutation,
		useBulkDeleteSubnetsMutation
	} from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_cidr,
		common_confirmDeleteName,
		common_create,
		common_confirmBulkDelete,
		common_created,
		common_description,
		common_lastSeen,
		common_name,
		common_network,
		common_noEntityYet,
		common_delete,
		common_edit,
		common_subnets,
		common_tags,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptSubnets,
		subnets_subnetType
	} from '$lib/paraglide/messages';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { subnetTypes } from '$lib/shared/stores/metadata';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type SubnetOrderField = components['schemas']['SubnetOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	// Staleness filter state. Server-side so it applies to the whole set, and
	// keyed separately from the shared subnets cache.
	let stale = $state<boolean | null>(null);
	const subnetsQuery = useSubnetsQuery(undefined, () => stale ?? undefined);

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}
	const networksQuery = useNetworksQuery();

	// Mutations
	const createSubnetMutation = useCreateSubnetMutation();
	const updateSubnetMutation = useUpdateSubnetMutation();
	const deleteSubnetMutation = useDeleteSubnetMutation();
	const bulkDeleteSubnetsMutation = useBulkDeleteSubnetsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let subnetsData = $derived(
		(subnetsQuery.data ?? []).filter(
			(s) => !subnetTypes.getMetadata(s.subnet_type).hide_from_subnet_list
		)
	);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(subnetsQuery.isPending);

	let showSubnetEditor = $state(false);
	let editingSubnet = $state<Subnet | null>(null);

	// Deep-link: open subnet editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'subnet-editor',
			subnetsData,
			showSubnetEditor,
			editingSubnet?.id
		);
		if (result !== undefined) {
			editingSubnet = result;
			showSubnetEditor = true;
		}
	});

	function handleCreateSubnet() {
		editingSubnet = null;
		showSubnetEditor = true;
	}

	/** Row actions for table mode, matching what the card offers. */
	function subnetActions(subnet: Subnet): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditSubnet(subnet) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteSubnet(subnet)
			}
		];
	}

	function handleEditSubnet(subnet: Subnet) {
		editingSubnet = subnet;
		showSubnetEditor = true;
	}

	function handleDeleteSubnet(subnet: Subnet) {
		if (confirm(common_confirmDeleteName({ name: subnet.name }))) {
			deleteSubnetMutation.mutate(subnet.id);
		}
	}

	async function handleSubnetCreate(data: Subnet) {
		try {
			await createSubnetMutation.mutateAsync(data);
			showSubnetEditor = false;
			editingSubnet = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleSubnetUpdate(_id: string, data: Subnet) {
		try {
			await updateSubnetMutation.mutateAsync(data);
			showSubnetEditor = false;
			editingSubnet = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseSubnetEditor() {
		showSubnetEditor = false;
		editingSubnet = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_subnets() }))) {
			await bulkDeleteSubnetsMutation.mutateAsync(ids);
		}
	}

	function getSubnetTags(subnet: Subnet): string[] {
		return subnet.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Subnet', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all SubnetOrderField values are covered
	let subnetFields = $derived(
		defineFields<Subnet, SubnetOrderField>(
			{
				// Identity fields: grouping by one would render a header per subnet.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { primary: true, width: 220 }
				},
				cidr: { label: common_cidr(), type: 'string', searchable: true, groupable: false },
				subnet_type: {
					label: subnets_subnetType(),
					type: 'string',
					searchable: true,
					filterable: true,
					display: {
						getItems: (subnet) => [
							{
								id: subnet.subnet_type,
								label: subnetTypes.getName(subnet.subnet_type),
								color: subnetTypes.getColorHelper(subnet.subnet_type).color,
								icon: subnetTypes.getIconComponent(subnet.subnet_type)
							}
						]
					}
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { getItems: (item) => networkItems(item.network_id, networksData) }
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				last_seen_at: { label: common_lastSeen(), type: 'date' }
			},
			[
				{ key: 'description', label: common_description(), type: 'string', searchable: true },
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) => tagNames(entity.tags, tagsData)
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_subnets()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateSubnet}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptSubnets()} />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if subnetsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_subnets() })}
			subtitle=""
			onClick={handleCreateSubnet}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={subnetsData}
			fields={subnetFields}
			storageKey="scanopy-subnets-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Subnet'}
			getItemTags={getSubnetTags}
			getItemId={(item) => item.id}
			onStaleFilterChange={handleStaleFilterChange}
			onCsvExport={handleCsvExport}
			getActions={subnetActions}
			entityLabel={common_subnets()}
		>
			{#snippet children(
				item: Subnet,
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<SubnetCard
					subnet={item}
					selected={isSelected}
					{onSelectionChange}
					onEdit={isReadOnly ? undefined : handleEditSubnet}
					onDelete={isReadOnly ? undefined : handleDeleteSubnet}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<SubnetEditModal
	name="subnet-editor"
	isOpen={showSubnetEditor}
	subnet={editingSubnet}
	onCreate={handleSubnetCreate}
	onUpdate={handleSubnetUpdate}
	onClose={handleCloseSubnetEditor}
	onDelete={editingSubnet
		? () => {
				handleDeleteSubnet(editingSubnet!);
				handleCloseSubnetEditor();
			}
		: null}
/>
