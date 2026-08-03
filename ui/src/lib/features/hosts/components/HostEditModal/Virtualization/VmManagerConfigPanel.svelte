<script lang="ts">
	import type { Service } from '$lib/features/services/types/base';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Host, HostVirtualization } from '$lib/features/hosts/types/base';
	import {
		hosts_virtualization_addVmHost,
		hosts_virtualization_noVmsYet,
		hosts_virtualization_virtualMachines,
		hosts_virtualization_vmHelp
	} from '$lib/paraglide/messages';

	interface Props {
		service: Service;
		/** Effective host list (saved + staged edits) from VirtualizationForm. */
		hosts: Host[];
		/** Effective service list (saved + staged edits), for host service context. */
		services: Service[];
		onChange: (updatedHost: Host) => void;
	}

	let { service, hosts, services, onChange }: Props = $props();

	let serviceMetadata = $derived(serviceDefinitions.getItem(service.service_definition));

	// Derived from the effective host list keyed on this manager — so it updates as
	// VMs are added/removed and resets when a different manager is selected.
	let managedVms = $derived(hosts.filter((h) => h.virtualization_service_id === service.id));

	let vmIds = $derived(managedVms.map((h) => h.id));
	// Filter out the parent host and already managed VMs
	let selectableVms = $derived(
		hosts
			.filter((host) => service.host_id !== host.id && !vmIds.includes(host.id))
			.filter((h) => h.network_id == service.network_id)
	);

	function handleAddVm(vmId: string) {
		const host = hosts.find((h) => h.id === vmId);
		const variant = serviceMetadata?.metadata.virtualization_variant;
		if (host && variant) {
			const updatedHost: Host = {
				...host,
				virtualization_metadata: {
					type: variant,
					details: {
						vm_id: null,
						vm_name: null
					}
				} as HostVirtualization,
				virtualization_service_id: service.id
			};

			// Stage the change; managedVms re-derives from the effective host list.
			onChange(updatedHost);
		}
	}

	function handleRemoveVm(index: number) {
		const removedVm = managedVms.at(index);

		if (removedVm) {
			const updatedHost = {
				...removedVm,
				virtualization_metadata: null,
				virtualization_service_id: null
			};

			onChange(updatedHost);
		}
	}

	function getHostServices(host: Host): Service[] {
		return services.filter((s) => s.host_id == host.id);
	}
</script>

<div class="space-y-6">
	<ListManager
		label={hosts_virtualization_virtualMachines()}
		helpText={hosts_virtualization_vmHelp({ serviceName: serviceMetadata?.name ?? '' })}
		placeholder={hosts_virtualization_addVmHost()}
		emptyMessage={hosts_virtualization_noVmsYet()}
		allowReorder={false}
		allowDuplicates={false}
		showSearch={true}
		allowItemEdit={() => false}
		options={selectableVms}
		getItemContext={(item) => ({ services: getHostServices(item) })}
		getOptionContext={(item) => ({ services: getHostServices(item) })}
		items={managedVms}
		optionDisplayComponent={HostDisplay}
		itemDisplayComponent={HostDisplay}
		onAdd={handleAddVm}
		onRemove={handleRemoveVm}
	/>
</div>
