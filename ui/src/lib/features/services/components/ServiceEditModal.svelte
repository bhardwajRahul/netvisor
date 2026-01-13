<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Service } from '../types/base';
	import ServiceConfigPanel from '$lib/features/hosts/components/HostEditModal/Services/ServiceConfigPanel.svelte';
	import type { Host, HostFormData } from '$lib/features/hosts/types/base';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useServicesQuery } from '$lib/features/services/queries';
	import * as m from '$lib/paraglide/messages';

	// TanStack Query hooks to get child entities for hydrating host form data
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();
	const servicesQuery = useServicesQuery();
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let servicesData = $derived(servicesQuery.data?.items ?? []);

	// Hydrate host to form data for ServiceConfigPanel
	function hydrateHostToFormData(host: Host): HostFormData {
		const hostInterfaces = interfacesData.filter((i) => i.host_id === host.id);
		const hostPorts = portsData.filter((p) => p.host_id === host.id);
		const hostServices = servicesData.filter((s) => s.host_id === host.id);

		return {
			...host,
			interfaces: hostInterfaces,
			ports: hostPorts,
			services: hostServices
		};
	}

	interface Props {
		service: Service;
		host: Host;
		isOpen?: boolean;
		onUpdate: (id: string, data: Service) => Promise<void> | void;
		onClose: () => void;
	}

	let { service, host, isOpen = false, onUpdate, onClose }: Props = $props();

	let loading = $state(false);
	let formData = $state(service);

	// Hydrate host to form data for ServiceConfigPanel
	let hostFormData = $derived(hydrateHostToFormData(host));

	let title = $derived(`Edit ${service.name}`);

	// TanStack Form for validation
	let form = createForm(() => ({
		defaultValues: {
			services: [formData]
		},
		onSubmit: async () => {
			// Actual submission handled by handleSubmit
		}
	}));

	function handleOpen() {
		formData = { ...service };
		form.reset();
	}

	async function handleSubmit() {
		// Validate form first
		const isValid = await validateForm(form);
		if (!isValid) return;

		// Clean up the data before sending
		const serviceData: Service = {
			...formData,
			name: formData.name.trim()
		};

		loading = true;
		try {
			await onUpdate(service.id, serviceData);
			onClose();
		} finally {
			loading = false;
		}
	}

	function handleServiceUpdate(updatedService: Service) {
		formData = { ...updatedService };
	}
</script>

<GenericModal {isOpen} {title} {onClose} onOpen={handleOpen} size="xl">
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={serviceDefinitions.getIconComponent(service.service_definition)}
			color={serviceDefinitions.getColorHelper(service.service_definition).color}
		/>
	{/snippet}

	<!-- Content -->
	<div class="flex h-full flex-col overflow-hidden">
		<div class="flex-1 overflow-y-auto">
			<div class="space-y-8 p-6">
				<ServiceConfigPanel
					host={hostFormData}
					service={formData}
					{form}
					index={0}
					onChange={handleServiceUpdate}
				/>

				<EntityMetadataSection entities={[service]} />
			</div>
		</div>
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={onClose} class="btn-secondary"> {m.common_cancel()} </button>
				<button type="button" onclick={handleSubmit} disabled={loading} class="btn-primary">
					{loading ? m.services_updating() : m.services_updateService()}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>
