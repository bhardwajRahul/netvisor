<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import type { IfEntry, Interface } from '$lib/features/hosts/types/base';
	import { getHostByIdFromCache } from '$lib/features/hosts/queries';
	import { getAdminStatusLabels, getOperStatusLabels } from '$lib/features/snmp/types/base';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Color } from '$lib/shared/utils/styling';
	import {
		common_macAddress,
		common_speed,
		common_status,
		common_unknown,
		hosts_ifEntries_adminStatus,
		hosts_ifEntries_aliasDescription,
		hosts_ifEntries_cdpNeighbor,
		hosts_ifEntries_chassisId,
		hosts_ifEntries_details,
		hosts_ifEntries_index,
		hosts_ifEntries_interfaceId,
		hosts_ifEntries_lldpNeighbor,
		hosts_ifEntries_lldpSysDescr,
		hosts_ifEntries_managementAddress,
		hosts_ifEntries_neighbor,
		hosts_ifEntries_operStatus,
		hosts_ifEntries_portId,
		hosts_ifEntries_remoteAddress,
		hosts_ifEntries_remoteDevice,
		hosts_ifEntries_remotePlatform,
		hosts_ifEntries_remotePort,
		hosts_ifEntries_remoteSystemName,
		hosts_ifEntries_type
	} from '$lib/paraglide/messages';

	interface Props {
		ifEntry: IfEntry;
	}

	let { ifEntry }: Props = $props();

	const queryClient = useQueryClient();

	function formatSpeed(speed: number | null | undefined): string {
		if (!speed) return common_unknown();
		if (speed >= 1_000_000_000) return `${(speed / 1_000_000_000).toFixed(1)} Gbps`;
		if (speed >= 1_000_000) return `${(speed / 1_000_000).toFixed(1)} Mbps`;
		if (speed >= 1_000) return `${(speed / 1_000).toFixed(1)} Kbps`;
		return `${speed} bps`;
	}

	let adminStatusLabel = $derived(getAdminStatusLabels()[ifEntry.admin_status] ?? common_unknown());
	let operStatusLabel = $derived(getOperStatusLabels()[ifEntry.oper_status] ?? common_unknown());

	let operStatusColor: Color = $derived.by(() => {
		switch (ifEntry.oper_status) {
			case 'Up':
				return 'Green';
			case 'Down':
				return 'Red';
			case 'Dormant':
				return 'Yellow';
			default:
				return 'Gray';
		}
	});

	// Linked Interface display (interface_id → Interface name/IP)
	let linkedInterfaceDisplay = $derived.by(() => {
		if (!ifEntry.interface_id) return '-';
		const allInterfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
		const iface = allInterfaces.find((i) => i.id === ifEntry.interface_id);
		if (iface) {
			return iface.name ? `${iface.name}: ${iface.ip_address}` : iface.ip_address;
		}
		return '-';
	});

	// Neighbor display (neighbor object → Host/IfEntry name)
	let neighborDisplay = $derived.by(() => {
		if (!ifEntry.neighbor) return '-';

		if (ifEntry.neighbor.type === 'Host') {
			const host = getHostByIdFromCache(queryClient, ifEntry.neighbor.id);
			return host?.name ?? 'Unknown host';
		} else {
			// IfEntry type - look up the ifEntry
			const allIfEntries = queryClient.getQueryData<IfEntry[]>(queryKeys.ifEntries.all) ?? [];
			const remoteEntry = allIfEntries.find((e) => e.id === ifEntry.neighbor!.id);
			if (remoteEntry) {
				const host = getHostByIdFromCache(queryClient, remoteEntry.host_id);
				const hostName = host?.name ?? 'Unknown';
				const portName = remoteEntry.if_descr || `Index ${remoteEntry.if_index}`;
				return `${hostName} → ${portName}`;
			}
			return 'Unknown interface';
		}
	});
</script>

<div class="space-y-6">
	<ConfigHeader
		title={ifEntry.if_descr || `Interface ${ifEntry.if_index}`}
		subtitle={hosts_ifEntries_index({ index: ifEntry.if_index })}
	/>

	<!-- Status Section -->
	<InfoCard title={common_status()}>
		<InfoRow label={hosts_ifEntries_adminStatus()}>{adminStatusLabel}</InfoRow>
		<InfoRow label={hosts_ifEntries_operStatus()}>
			<Tag label={operStatusLabel} color={operStatusColor} />
		</InfoRow>
	</InfoCard>

	<!-- Interface Details Section -->
	<InfoCard title={hosts_ifEntries_details()}>
		<InfoRow label={hosts_ifEntries_type()}>{ifEntry.if_type || '-'}</InfoRow>
		<InfoRow label={common_macAddress()} mono>{ifEntry.mac_address || '-'}</InfoRow>
		<InfoRow label={common_speed()}>{formatSpeed(ifEntry.speed_bps)}</InfoRow>
		<InfoRow label={hosts_ifEntries_aliasDescription()}>{ifEntry.if_alias || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_interfaceId()}>{linkedInterfaceDisplay}</InfoRow>
		<InfoRow label={hosts_ifEntries_neighbor()}>{neighborDisplay}</InfoRow>
	</InfoCard>

	<!-- CDP Neighbor Info Section -->
	<InfoCard title={hosts_ifEntries_cdpNeighbor()}>
		<InfoRow label={hosts_ifEntries_remoteDevice()}>{ifEntry.cdp_device_id || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePort()}>{ifEntry.cdp_port_id || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remoteAddress()} mono>{ifEntry.cdp_address || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePlatform()}>{ifEntry.cdp_platform || '-'}</InfoRow>
	</InfoCard>

	<!-- LLDP Neighbor Info Section -->
	<InfoCard title={hosts_ifEntries_lldpNeighbor()}>
		<InfoRow label={hosts_ifEntries_chassisId()} mono
			>{ifEntry.lldp_chassis_id?.value || '-'}</InfoRow
		>
		<InfoRow label={hosts_ifEntries_portId()} mono>{ifEntry.lldp_port_id?.value || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remoteSystemName()}>{ifEntry.lldp_sys_name || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePort()}>{ifEntry.lldp_port_desc || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_managementAddress()} mono
			>{ifEntry.lldp_mgmt_addr || '-'}</InfoRow
		>
		<InfoRow label={hosts_ifEntries_lldpSysDescr()}>{ifEntry.lldp_sys_desc || '-'}</InfoRow>
	</InfoCard>
</div>
