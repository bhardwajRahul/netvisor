<script lang="ts">
	import type { Service } from '$lib/features/services/types/base';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
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
		onChange: (updatedService: Service) => void;
	}

	let { service, onChange }: Props = $props();

	// TanStack Query hooks
	const servicesQuery = useServicesCacheQuery();
	let servicesData = $derived(servicesQuery.data ?? []);

	let serviceMetadata = $derived(serviceDefinitions.getItem(service.service_definition));

	// Use local state for managed containers to support immediate UI updates
	let managedContainers = $state<Service[]>([]);
	let initialized = $state(false);

	// Initialize managedContainers when servicesData is available (only once at mount)
	$effect(() => {
		if (servicesData.length > 0 && !initialized) {
			initialized = true;
			managedContainers = servicesData.filter(
				(s) =>
					s.virtualization &&
					s.virtualization?.type == 'Docker' &&
					s.virtualization.details.service_id == service.id
			);
		}
	});

	let containerIds = $derived(managedContainers.map((s) => s.id));

	// Filter out services on other hosts and already managed containers
	let selectableContainers = $derived(
		servicesData.filter(
			(s) => s.host_id === service.host_id && s.id !== service.id && !containerIds.includes(s.id)
		)
	);

	function handleAddContainer(serviceId: string) {
		const servicesForHost = servicesData.filter((s) => s.host_id === service.host_id);
		const containerizedService = servicesForHost.find((s) => s.id == serviceId);

		if (containerizedService) {
			const updatedService = {
				...containerizedService,
				virtualization: {
					type: 'Docker' as const,
					details: {
						container_id: null,
						container_name: null,
						service_id: service.id
					}
				}
			};

			managedContainers = [...managedContainers, updatedService];
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

			managedContainers = managedContainers.filter((s) => s.id !== removedContainer.id);
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
