<script lang="ts">
	import { Edit, Eye, Replace, Trash2 } from 'lucide-svelte';
	import { formatInterface } from '../queries';
	import type { Host } from '../types/base';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		common_consolidate,
		common_containers,
		common_delete,
		common_description,
		common_edit,
		common_hide,
		common_interfaces,
		common_services,
		common_tags,
		hosts_noContainers,
		hosts_noInterfaces,
		hosts_noServicesAssigned,
		hosts_unknownService,
		hosts_vmManagedBy
	} from '$lib/paraglide/messages';

	// Queries
	const servicesQuery = useServicesCacheQuery();
	const interfacesQuery = useInterfacesQuery();
	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();

	// Derived data
	let servicesData = $derived(servicesQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

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

	let hasDaemon = $derived(daemonsData.some((d) => d.host_id == host.id));

	// Get filtered data for this host, sorted by position
	let hostServices = $derived(
		servicesData
			.filter((s) => s.host_id === host.id)
			.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
	);
	let hostInterfaces = $derived(interfacesData.filter((i) => i.host_id === host.id));
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

		return {
			title: host.name,
			...(host.virtualization !== null && virtualizationService
				? {
						subtitle: hosts_vmManagedBy({
							serviceName: virtualizationService.name || hosts_unknownService()
						})
					}
				: {}),
			link: host.hostname ? `http://${host.hostname}` : undefined,
			iconColor: entities.getColorHelper('Host').icon,
			Icon:
				hostServices.length > 0
					? serviceDefinitions.getIconComponent(hostServices[0].service_definition)
					: entities.getIconComponent('Host'),
			fields: [
				{
					label: common_description(),
					value: host.description
				},
				{
					label: common_services(),
					value: hostServices
						.filter((sv) => !containerIds.includes(sv.id))
						.map((sv) => {
							return {
								id: sv.id,
								label: sv.name,
								color: entities.getColorHelper('Service').color
							};
						}),
					emptyText: hosts_noServicesAssigned()
				},
				{
					label: common_containers(),
					value: containers
						.map((c) => {
							return {
								id: c.id,
								label: c.name,
								color: concepts.getColorHelper('Virtualization').color
							};
						})
						.sort((a) => (containerIds.includes(a.id) ? 1 : -1)),
					emptyText: hosts_noContainers()
				},
				{
					label: common_interfaces(),
					value: hostInterfaces.map((i) => {
						return {
							id: i.id,
							label: formatInterface(i, isContainerSubnetFn),
							color: entities.getColorHelper('Interface').color
						};
					}),
					emptyText: hosts_noInterfaces()
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
								onClick: () => onDelete(host),
								disabled: hasDaemon
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
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline selectedTagIds={host.tags} entityId={host.id} entityType="Host" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
