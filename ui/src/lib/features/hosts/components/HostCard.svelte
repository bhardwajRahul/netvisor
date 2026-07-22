<script lang="ts">
	import { Edit, Eye, Replace, Trash2 } from 'lucide-svelte';
	import { formatIPAddress } from '../queries';
	import type { Host } from '../types/base';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import {
		common_consolidate,
		common_credentials,
		common_delete,
		common_description,
		common_edit,
		common_hide,
		common_interfaces,
		common_ipAddresses,
		common_lastSeen,
		common_never,
		common_service,
		common_services,
		common_tags,
		common_containerizedServices,
		common_unknownEntity,
		hosts_noContainers,
		hosts_noInterfaces,
		hosts_noServicesAssigned,
		hosts_vmManagedBy
	} from '$lib/paraglide/messages';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';

	// Queries
	const servicesQuery = useServicesCacheQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const interfacesQuery = useInterfacesQuery();
	const subnetsQuery = useSubnetsQuery();
	const credentialsQuery = useCredentialsQuery();
	const networksQuery = useNetworksQuery();

	// Derived data
	let servicesData = $derived(servicesQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	let {
		host,
		onEdit,
		onDelete,
		onHide,
		onConsolidate,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		host: Host;
		onEdit?: (host: Host) => void;
		onDelete?: (host: Host) => void;
		onHide?: (host: Host) => void;
		onConsolidate?: (host: Host) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Get filtered data for this host, sorted by position
	let hostServices = $derived(
		servicesData
			.filter((s) => s.host_id === host.id)
			.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
	);
	let hostIPAddresses = $derived(ipAddressesData.filter((i) => i.host_id === host.id));
	let hostInterfaces = $derived(interfacesData.filter((i) => i.host_id === host.id));
	let hostCredentials = $derived(
		(host.credential_assignments ?? [])
			.map((a: { credential_id: string }) => credentialsData.find((c) => c.id === a.credential_id))
			.filter((c): c is NonNullable<typeof c> => c != null)
	);
	let virtualizationService = $derived(
		host.virtualization
			? servicesData.find((s) => s.id === host.virtualization?.details.service_id)
			: null
	);

	// Consolidate all reactive computations into a single derived to prevent cascading updates
	let cardData = $derived.by(() => {
		const servicesThatManageContainersIds = hostServices
			.filter(
				(sv) =>
					serviceDefinitions.getItem(sv.service_definition)?.metadata.manages_virtualization ==
					'containers'
			)
			.map((sv) => sv.id);

		const containers = hostServices.filter(
			(s) =>
				s.virtualization &&
				s.virtualization?.type == 'Docker' &&
				servicesThatManageContainersIds.includes(s.virtualization.details.service_id)
		);

		const containerIds = containers.map((c) => c.id);

		const visibleServices = hostServices.filter(
			(sv) => sv.service_definition !== 'Unclaimed Open Ports'
		);

		return {
			title: host.name,
			// Staleness is judged against this host's own network's window.
			// Occupies the same slot DaemonCard uses for daemon health.
			status: getFreshnessTag(
				host,
				(networksQuery.data ?? []).find((n) => n.id === host.network_id)
			),
			...(host.virtualization !== null && virtualizationService
				? {
						subtitle: hosts_vmManagedBy({
							serviceName:
								virtualizationService.name || common_unknownEntity({ entity: common_service() })
						})
					}
				: {}),
			link: host.hostname ? `http://${host.hostname}` : undefined,
			iconColor: entities.getColorHelper('Host').icon,
			Icon:
				visibleServices.length > 0
					? serviceDefinitions.getIconComponent(visibleServices[0].service_definition)
					: entities.getIconComponent('Host'),
			fields: [
				{
					label: common_description(),
					value: host.description
				},
				{
					label: common_services(),
					value: visibleServices
						.filter((sv) => !containerIds.includes(sv.id))
						.map((sv) => ({
							id: sv.id,
							label: sv.name,
							color: entities.getColorHelper('Service').color,
							entityRef: entityRef('Service', sv.id, sv, { ipAddressId: null })
						})),
					emptyText: hosts_noServicesAssigned()
				},
				{
					label: common_containerizedServices(),
					value: containers
						.map((c) => ({
							id: c.id,
							label: c.name,
							color: concepts.getColorHelper('Virtualization').color,
							entityRef: entityRef('Service', c.id, c, { ipAddressId: null })
						}))
						.sort((a) => (containerIds.includes(a.id) ? 1 : -1)),
					emptyText: hosts_noContainers()
				},
				{
					label: common_ipAddresses(),
					value: hostIPAddresses.map((i) => ({
						id: i.id,
						label: formatIPAddress(i, isContainerSubnetFn),
						color: entities.getColorHelper('IPAddress').color,
						entityRef: entityRef('IPAddress', i.id, i, { subnets: subnetsData })
					})),
					emptyText: hosts_noInterfaces()
				},
				{
					label: common_credentials(),
					value: hostCredentials.map((c) => ({
						id: c.id,
						label: c.name,
						color: entities.getColorHelper('Credential').color,
						entityRef: entityRef('Credential', c.id, c)
					}))
				},
				{
					label: common_interfaces(),
					value: hostInterfaces.map((i) => ({
						id: i.id,
						label: i.if_descr && i.if_descr.length > 0 ? i.if_descr : 'Unnamed interface',
						color: entities.getColorHelper('Interface').color,
						entityRef: entityRef('Interface', i.id, i)
					}))
				},
				{
					label: common_lastSeen(),
					value: host.last_seen_at ? formatRelativeTime(host.last_seen_at) : common_never()
				},
				{ label: common_tags(), snippet: tagsSnippet }
			],
			actions: [
				...(onDelete
					? [
							{
								label: common_delete(),
								icon: Trash2,
								class: 'btn-icon-danger',
								onClick: () => onDelete(host)
							}
						]
					: []),
				...(onConsolidate
					? [{ label: common_consolidate(), icon: Replace, onClick: () => onConsolidate(host) }]
					: []),
				...(onHide
					? [
							{
								label: common_hide(),
								icon: Eye,
								class: host.hidden ? 'text-blue-400' : '',
								onClick: () => onHide(host)
							}
						]
					: []),
				...(onEdit ? [{ label: common_edit(), icon: Edit, onClick: () => onEdit(host) }] : [])
			]
		};
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={host.tags} entityId={host.id} entityType="Host" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
