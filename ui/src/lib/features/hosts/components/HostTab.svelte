<script lang="ts">
	import type {
		Host,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '../types/base';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import HostEditor from './HostEditModal/HostEditor.svelte';
	import HostConsolidationModal from './HostConsolidationModal.svelte';
	import HostExportModal from './HostExportModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import {
		entities,
		concepts,
		credentialTypes,
		serviceDefinitions
	} from '$lib/shared/stores/metadata';
	import { Plus, Trash2, RefreshCw, Replace, Eye, Edit } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmDeleteName,
		common_consolidate,
		common_create,
		common_created,
		common_delete,
		common_description,
		common_edit,
		common_hide,
		common_hidden,
		common_hostname,
		common_credentials,
		common_hosts,
		common_interfaces,
		common_ipAddresses,
		common_lastSeen,
		common_confirmBulkDelete,
		common_name,
		common_network,
		common_noEntityYet,
		common_rescan,
		common_service,
		common_services,
		common_tags,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptHosts,
		hosts_fields_virtualizedBy,
		hosts_notVirtualized,
		hosts_unnamedInterface
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	import {
		useHostsQuery,
		useCreateHostMutation,
		useUpdateHostMutation,
		useDeleteHostMutation,
		useBulkDeleteHostsMutation,
		useConsolidateHostsMutation,
		useRescanHostMutation,
		type HostQueryOptions
	} from '../queries';
	import { useServicesByIds, useServicesCacheQuery } from '$lib/features/services/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import type { Credential } from '$lib/features/credentials/types/base';
	import type { Interface } from '$lib/features/credentials/types/base';
	import { formatIPAddress } from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { components } from '$lib/api/schema';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type HostOrderField = components['schemas']['HostOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	// Pagination state
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<HostOrderField | undefined>(undefined);
	let orderBy = $state<HostOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);
	// Staleness filter state (server-side: the list is server-paginated)
	let stale = $state<boolean | null>(null);
	// Search state (server-side, for the same reason)
	let search = $state('');

	// Queries
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let hostLimit = $derived(org?.plan?.included_hosts ?? null);
	let canBuyMoreHosts = $derived(
		org?.plan?.host_cents !== undefined && org?.plan?.host_cents !== null
	);
	let onboarding = $derived((org?.onboarding ?? []) as OnboardingOperation[]);

	const tagsQuery = useTagsQuery();
	// Paginated hosts with server-side pagination, ordering, and tag filtering.
	//
	// Deliberately NOT gated on `isActive`, unlike the other tabs' list queries.
	// `useHostsQuery` is the last remaining writer of the ip-addresses / ports /
	// services / interfaces caches (it populates them from its nested response),
	// and those caches have no fetcher of their own. Gating this would leave the
	// services tab's binding chips and the host editor's interface lists empty for
	// anyone who never opens the hosts tab. Un-gate it only once those caches have
	// real queries — see planned-work/child-cache-rearchitecture.md.
	const hostsQuery = useHostsQuery(
		(): HostQueryOptions => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			stale: stale ?? undefined,
			search: search || undefined
		})
	);
	const networksQuery = useNetworksQuery();
	useDaemonsQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const interfacesQuery = useInterfacesQuery();
	const credentialsQuery = useCredentialsQuery();
	const subnetsQuery = useSubnetsQuery();

	// Selective service lookup - only fetches services needed for virtualization display
	// Extract service IDs from visible hosts for "Virtualized By" field
	const servicesQuery = useServicesByIds(() => {
		return (hostsQuery.data?.items ?? [])
			.filter((h) => h.virtualization?.details.service_id)
			.map((h) => h.virtualization!.details.service_id)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
	});

	// Mutations
	const createHostMutation = useCreateHostMutation();
	const updateHostMutation = useUpdateHostMutation();
	const deleteHostMutation = useDeleteHostMutation();
	const bulkDeleteHostsMutation = useBulkDeleteHostsMutation();
	const consolidateHostsMutation = useConsolidateHostsMutation();
	const rescanHostMutation = useRescanHostMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let hostsPagination = $derived(hostsQuery.data?.pagination ?? null);
	let servicesData = $derived(servicesQuery.data ?? []);
	const servicesCacheQuery = useServicesCacheQuery();
	let allServicesData = $derived(servicesCacheQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(hostsQuery.isPending && !hostsQuery.data);

	// Host limit tracking
	let totalHostCount = $derived(hostsPagination?.total_count ?? hostsData.length);
	let isAtHostLimit = $derived(
		hostLimit !== null && totalHostCount >= hostLimit && !canBuyMoreHosts
	);
	let isNearHostLimit = $derived(
		hostLimit !== null &&
			totalHostCount >= hostLimit - 5 &&
			totalHostCount < hostLimit &&
			!canBuyMoreHosts
	);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	// Values are now directly HostOrderField values from the orderField property
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as HostOrderField) ?? undefined;
		orderBy = (orderField as HostOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Tag filter change handler for server-side filtering
	function handleTagFilterChange(selectedTagIds: string[]) {
		tagIds = selectedTagIds;
		// Reset to page 1 is handled by DataControls
	}

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

	// Export modal state
	let showExportModal = $state(false);
	let exportParams = $derived({
		tag_ids: tagIds.length > 0 ? tagIds : undefined,
		order_by: orderBy,
		order_direction: orderDirection
	});

	let showHostEditor = $state(false);
	// Track the host being edited by id + a snapshot, and resolve `editingHost` from the
	// live query cache. So when an external change (e.g. a credential assignment removed
	// elsewhere) invalidates and refetches hosts, the open editor reflects it without a
	// page reload. The snapshot is the fallback for hosts not in the current page.
	let editingHostId = $state<string | null>(null);
	let editingHostSnapshot = $state<Host | null>(null);
	let editingHost = $derived(
		editingHostId ? (hostsData.find((h) => h.id === editingHostId) ?? editingHostSnapshot) : null
	);
	function setEditingHost(host: Host | null) {
		editingHostId = host?.id ?? null;
		editingHostSnapshot = host;
	}

	let otherHost = $state<Host | null>(null);
	let showHostConsolidationModal = $state(false);

	// Deep-link: open host editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'host-editor',
			hostsData,
			showHostEditor,
			editingHost?.id
		);
		if (result !== undefined) {
			setEditingHost(result);
			showHostEditor = true;
		}
	});

	// What a host holds. These were resolved inside HostCard, so the table had no
	// way to show them; resolving here gives both views the same columns.
	function hostCredentials(host: Host): Credential[] {
		return (host.credential_assignments ?? [])
			.map((a) => credentialsData.find((c) => c.id === a.credential_id))
			.filter((c): c is Credential => c != null);
	}

	function hostInterfaces(host: Host): Interface[] {
		return interfacesData.filter((i) => i.host_id === host.id);
	}

	function hostIPAddresses(host: Host) {
		return ipAddressesData.filter((i) => i.host_id === host.id);
	}

	function isContainerSubnetFn(subnetId: string): boolean {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all HostOrderField values are covered
	let hostFields = $derived(
		defineFields<Host, HostOrderField>(
			{
				// Identity fields: grouping by one would render a header per host.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { primary: true, width: 220, order: 0 }
				},
				hostname: {
					label: common_hostname(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { hiddenByDefault: true }
				},
				virtualized_by: {
					label: hosts_fields_virtualizedBy(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					// The server groups on the virtualizing service's name,
					// coalescing hosts without one to an empty string.
					getGroupValue: (host) =>
						servicesData.find((s) => s.id === host.virtualization?.details.service_id)?.name ?? '',
					getValue: (host) => {
						if (host.virtualization) {
							const virtualizationService = servicesData.find(
								(s) => s.id === host.virtualization?.details.service_id
							);
							if (virtualizationService) {
								return (
									virtualizationService?.name || common_unknownEntity({ entity: common_service() })
								);
							}
						}
						return hosts_notVirtualized();
					},
					display: {
						// Not in the default column set — it stays a filter and group axis.
						hiddenByDefault: true,
						// No chips when a host isn't virtualized, so the cell renders the
						// em dash rather than repeating "Not Virtualized" down the column.
						// `getValue` keeps the phrase, so the filter still offers it.
						getItems: (host) => {
							if (!host.virtualization) return [];
							const service = servicesData.find(
								(s) => s.id === host.virtualization?.details.service_id
							);
							if (!service) return [];
							return [
								{
									id: service.id,
									label: service.name,
									color: entities.getColorHelper('Service').color,
									entityRef: entityRef('Service', service.id, service)
								}
							];
						}
					}
				},
				interface_ip: {
					// The card calls this "IP Addresses"; it named one thing two ways.
					label: common_ipAddresses(),
					type: 'string',
					searchable: true,
					// Near-unique per host, so grouping by it is one header per host.
					groupable: false,
					getValue: (host) => {
						const iface = ipAddressesData
							.filter((i) => i.host_id === host.id)
							.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
						return iface?.ip_address ?? '';
					},
					display: {
						order: 5,
						// The server orders on the primary address, but a host usually has
						// several — showing only the first would misrepresent the row.
						getItems: (host) =>
							hostIPAddresses(host)
								.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
								.map((i) => ({
									id: i.id,
									label: formatIPAddress(i, isContainerSubnetFn),
									color: entities.getColorHelper('IPAddress').color,
									entityRef: entityRef('IPAddress', i.id, i, { subnets: subnetsData })
								}))
					}
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					filterOptions: networksData.map((n) => n.name),
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.network_id,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { order: 2, getItems: (item) => networkItems(item.network_id, networksData) }
				},
				// Audit dates stay available but off by default: 12 columns at once
				// is unreadable, and these are rarely what someone is scanning for.
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				last_seen_at: { label: common_lastSeen(), type: 'date', display: { order: 1 } }
			},
			[
				{
					key: 'description',
					label: common_description(),
					type: 'string',
					searchable: true,
					display: { hiddenByDefault: true }
				},
				{
					key: 'hidden',
					label: common_hidden(),
					type: 'boolean',
					filterable: true,
					// Useful as a filter, but almost always false — a column of "false"
					// earns none of the width it takes.
					display: { hiddenByDefault: true }
				},
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					// Search and filter match the same names the cell renders, so a row
					// can never show a tag the filter above it disagrees with.
					getValue: (entity) => tagNames(entity.tags, tagsData)
				},
				{
					key: 'credentials',
					label: common_credentials(),
					type: 'array',
					searchable: true,
					getValue: (host) => hostCredentials(host).map((c) => c.name),
					display: {
						order: 3,
						getItems: (host) =>
							hostCredentials(host).map((credential) => ({
								id: credential.id,
								label: credential.name,
								color: credentialTypes.getColorHelper(getCredentialTypeId(credential)).color,
								entityRef: entityRef('Credential', credential.id, credential)
							}))
					}
				},
				{
					key: 'interfaces',
					label: common_interfaces(),
					type: 'array',
					searchable: true,
					getValue: (host) =>
						hostInterfaces(host).map((i) => i.if_descr || hosts_unnamedInterface()),
					display: {
						order: 4,
						getItems: (host) =>
							hostInterfaces(host).map((iface) => ({
								id: iface.id,
								label: iface.if_descr || hosts_unnamedInterface(),
								color: entities.getColorHelper('Interface').color,
								entityRef: entityRef('Interface', iface.id, iface)
							}))
					}
				},
				{
					key: 'services',
					label: common_services(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (host) =>
						allServicesData.filter((s) => s.host_id === host.id).map((s) => s.name),
					display: {
						order: 6,
						// Containers are services too, so they share this column rather
						// than getting one of their own; the colour carries the
						// distinction instead of a second header.
						getItems: (host) =>
							allServicesData
								.filter((s) => s.host_id === host.id)
								.map((s) => ({
									id: s.id,
									label: s.name,
									color: s.virtualization
										? concepts.getColorHelper('Containerization').color
										: entities.getColorHelper('Service').color,
									entityRef: entityRef('Service', s.id, s)
								}))
					}
				}
			]
		)
	);

	function handleCreateHost() {
		setEditingHost(null);
		showHostEditor = true;
	}

	/**
	 * Row actions for table mode, matching what the card offers.
	 *
	 * The table never renders a card, so the actions the card builds for itself
	 * are not reachable from it — the tab already owns every handler, so it is
	 * the natural place to describe them once for both.
	 */
	function hostActions(host: Host): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditHost(host) },
			{ label: common_rescan(), icon: RefreshCw, onClick: () => handleRescanHost(host) },
			{ label: common_consolidate(), icon: Replace, onClick: () => handleStartConsolidate(host) },
			{
				label: common_hide(),
				icon: Eye,
				class: host.hidden ? 'text-blue-400' : '',
				onClick: () => handleHostHide(host)
			},
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteHost(host)
			}
		];
	}

	function handleEditHost(host: Host) {
		setEditingHost(host);
		showHostEditor = true;
	}

	function handleStartConsolidate(host: Host) {
		otherHost = host;
		showHostConsolidationModal = true;
	}

	function handleRescanHost(host: Host) {
		rescanHostMutation.mutate({ id: host.id, name: host.name });
	}

	function handleDeleteHost(host: Host) {
		if (confirm(common_confirmDeleteName({ name: host.name }))) {
			deleteHostMutation.mutate(host.id);
		}
	}

	async function handleHostCreate(data: CreateHostWithServicesRequest) {
		try {
			await createHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostUpdate(data: UpdateHostWithServicesRequest) {
		try {
			await updateHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleConsolidateHosts(destinationHostId: string, otherHostId: string) {
		try {
			await consolidateHostsMutation.mutateAsync({
				destinationHostId,
				otherHostId,
				otherHostName: otherHost?.name
			});
			showHostConsolidationModal = false;
			otherHost = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_hosts() }))) {
			await bulkDeleteHostsMutation.mutateAsync(ids);
		}
	}

	function getHostTags(host: Host): string[] {
		return host.tags;
	}

	async function handleHostHide(host: Host) {
		const updatedHost = { ...host, hidden: !host.hidden };
		await updateHostMutation.mutateAsync({
			host: updatedHost,
			ip_addresses: null,
			ports: null,
			services: null
		});
	}

	function handleCloseHostEditor() {
		showHostEditor = false;
		setEditingHost(null);
	}
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_hosts()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding)}
				<div class="flex items-center gap-3">
					{#if hostLimit !== null && !canBuyMoreHosts}
						<span
							class="text-sm {isAtHostLimit
								? 'text-amber-400'
								: isNearHostLimit
									? 'text-yellow-400'
									: 'text-tertiary'}"
						>
							{totalHostCount} / {hostLimit}
						</span>
					{/if}
					{#if !isReadOnly}
						{#if isAtHostLimit}
							<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
						{:else}
							{#if isNearHostLimit}
								<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
							{/if}
							<button class="btn-primary flex items-center" onclick={handleCreateHost}
								><Plus class="h-5 w-5" />{common_create()}</button
							>
						{/if}
					{/if}
				</div>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptHosts()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if hostsData.length === 0 && !hostsPagination}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_hosts() })}
			subtitle=""
			onClick={handleCreateHost}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={hostsData}
			fields={hostFields}
			storageKey="scanopy-hosts-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Host'}
			getItemTags={getHostTags}
			getItemId={(item) => item.id}
			getIcon={(host) => {
				const first = allServicesData.find(
					(s) => s.host_id === host.id && s.service_definition !== 'Unclaimed Open Ports'
				);
				return {
					icon: first
						? serviceDefinitions.getIconComponent(first.service_definition)
						: entities.getIconComponent('Host'),
					color: entities.getColorHelper('Host').icon
				};
			}}
			getLink={(host) => (host.hostname ? `http://${host.hostname}` : undefined)}
			serverPagination={hostsPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
			onStaleFilterChange={handleStaleFilterChange}
			onSearchChange={handleSearchChange}
			onExportClick={() => {
				showExportModal = true;
			}}
			getActions={hostActions}
			entityLabel={common_hosts()}
		></DataControls>
	{/if}
</div>

<HostEditor
	isOpen={showHostEditor}
	name="host-editor"
	host={editingHost}
	onCreate={handleHostCreate}
	onDelete={handleDeleteHost}
	onUpdate={handleHostUpdate}
	onClose={handleCloseHostEditor}
/>

<HostConsolidationModal
	isOpen={showHostConsolidationModal}
	{otherHost}
	onConsolidate={handleConsolidateHosts}
	onClose={() => (showHostConsolidationModal = false)}
/>

<HostExportModal
	isOpen={showExportModal}
	onClose={() => (showExportModal = false)}
	{exportParams}
/>
