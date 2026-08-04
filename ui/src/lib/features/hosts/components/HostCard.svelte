<script lang="ts">
	import { Edit, Eye, RefreshCw, Replace, Trash2 } from 'lucide-svelte';
	import type { Host } from '../types/base';
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import {
		common_consolidate,
		common_delete,
		common_edit,
		common_hide,
		common_rescan,
		common_service,
		common_unknownEntity,
		hosts_vmManagedBy
	} from '$lib/paraglide/messages';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';

	// Queries
	const servicesQuery = useServicesCacheQuery();
	const networksQuery = useNetworksQuery();

	// Derived data
	let servicesData = $derived(servicesQuery.data ?? []);

	let {
		host,
		columns,
		onEdit,
		onDelete,
		onHide,
		onConsolidate,
		onRescan,
		selected,
		onSelectionChange = () => {}
	}: {
		host: Host;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Host>[];
		onEdit?: (host: Host) => void;
		onDelete?: (host: Host) => void;
		onHide?: (host: Host) => void;
		onConsolidate?: (host: Host) => void;
		onRescan?: (host: Host) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Get filtered data for this host, sorted by position
	let hostServices = $derived(
		servicesData
			.filter((s) => s.host_id === host.id)
			.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
	);
	let virtualizationService = $derived(
		host.virtualization
			? servicesData.find((s) => s.id === host.virtualization?.details.service_id)
			: null
	);

	// Consolidate all reactive computations into a single derived to prevent cascading updates
	let cardData = $derived.by(() => {
		const visibleServices = hostServices.filter(
			(sv) => sv.service_definition !== 'Unclaimed Open Ports'
		);

		return {
			title: host.name,
			// Staleness is judged against this host's own network's window.
			// Occupies the same slot DaemonCard uses for daemon health.
			status: getFreshnessTag(
				host,
				(networksQuery.data ?? []).find((n) => n.id === host.network_id),
				{ entityTypeLabel: entities.getName('Host') || undefined }
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
				...(onRescan
					? [{ label: common_rescan(), icon: RefreshCw, onClick: () => onRescan(host) }]
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

<GenericCard {...cardData} {columns} item={host} {selected} {onSelectionChange} />
