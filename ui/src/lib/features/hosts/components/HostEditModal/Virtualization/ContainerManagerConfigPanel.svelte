<script lang="ts">
	import type { Service, ServiceVirtualization } from '$lib/features/services/types/base';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import {
		common_containers,
		hosts_virtualization_addContainer,
		hosts_virtualization_containerHelp,
		hosts_virtualization_noContainersYet
	} from '$lib/paraglide/messages';

	interface Props {
		service: Service;
		/** Effective service list (saved + staged edits) from VirtualizationForm. */
		services: Service[];
		onChange: (updatedService: Service) => void;
	}

	let { service, services, onChange }: Props = $props();

	let serviceMetadata = $derived(serviceDefinitions.getItem(service.service_definition));

	// Derived from the effective service list keyed on this manager — updates as
	// containers are added/removed and resets when a different manager is selected.
	let managedContainers = $derived(
		services.filter((s) => s.virtualization && s.virtualization.details.service_id === service.id)
	);

	let containerIds = $derived(managedContainers.map((s) => s.id));

	// Filter out services on other hosts and already managed containers
	let selectableContainers = $derived(
		services.filter(
			(s) => s.host_id === service.host_id && s.id !== service.id && !containerIds.includes(s.id)
		)
	);

	function handleAddContainer(serviceId: string) {
		const containerizedService = services.find(
			(s) => s.host_id === service.host_id && s.id == serviceId
		);

		const variant = serviceMetadata?.metadata.virtualization_variant;
		if (containerizedService && variant) {
			const updatedService: Service = {
				...containerizedService,
				virtualization: {
					type: variant,
					details: {
						container_id: null,
						container_name: null,
						service_id: service.id
					}
				} as ServiceVirtualization
			};

			// Stage the change; managedContainers re-derives from the effective list.
			onChange(updatedService);
		}
	}

	function handleRemoveContainer(index: number) {
		const removedContainer = managedContainers.at(index);

		if (removedContainer) {
			const updatedService = {
				...removedContainer,
				virtualization: null
			};

			onChange(updatedService);
		}
	}
</script>

<div class="space-y-6">
	<ListManager
		label={common_containers()}
		helpText={hosts_virtualization_containerHelp({ serviceName: serviceMetadata?.name ?? '' })}
		placeholder={hosts_virtualization_addContainer()}
		emptyMessage={hosts_virtualization_noContainersYet()}
		allowReorder={false}
		allowDuplicates={false}
		allowItemEdit={() => false}
		showSearch={true}
		options={selectableContainers}
		items={managedContainers}
		getItemContext={() => ({})}
		optionDisplayComponent={ServiceDisplay}
		itemDisplayComponent={ServiceDisplay}
		onAdd={handleAddContainer}
		onRemove={handleRemoveContainer}
	/>
</div>
