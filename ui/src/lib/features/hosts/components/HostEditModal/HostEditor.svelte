<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import { Info } from 'lucide-svelte';
	import type {
		Host,
		HostFormData,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '$lib/features/hosts/types/base';
	import { formDataToHostPrimitive, hydrateHostToFormData } from '$lib/features/hosts/queries';
	import { createEmptyHostFormData } from '$lib/features/hosts/store';
	import { useQueryClient } from '@tanstack/svelte-query';
	import DetailsForm from './Details/HostDetailsForm.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import InterfacesForm from './Interfaces/InterfacesForm.svelte';
	import ServicesForm from './Services/ServicesForm.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Service } from '$lib/features/services/types/base';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import PortsForm from './Ports/PortsForm.svelte';
	import VirtualizationForm from './Virtualization/VirtualizationForm.svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { pushError } from '$lib/shared/stores/feedback';

	interface Props {
		host?: Host | null;
		isOpen?: boolean;
		onCreate: (data: CreateHostWithServicesRequest) => Promise<void> | void;
		onCreateAndContinue: (data: CreateHostWithServicesRequest) => Promise<void> | void;
		onUpdate: (data: UpdateHostWithServicesRequest) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	}

	let {
		host = null,
		isOpen = false,
		onCreate,
		onCreateAndContinue,
		onUpdate,
		onClose,
		onDelete = null
	}: Props = $props();

	// TanStack Query hooks
	const queryClient = useQueryClient();
	const servicesQuery = useServicesQuery();
	const networksQuery = useNetworksQuery();
	let servicesData = $derived(servicesQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let loading = $state(false);
	let deleting = $state(false);

	let currentHostServices = $state<Service[]>([]);

	let isEditing = $derived(host !== null);
	let title = $derived(isEditing ? `Edit ${host?.name}` : 'Create Host');

	// formData holds structural data (ids, network_id, tags, etc.)
	// Form fields (name, hostname, description, interface IPs, port numbers) are synced at submission
	let formData = $state<HostFormData>(createEmptyHostFormData());

	// Track which submission action to take
	type SubmissionMode = 'create' | 'createAndContinue' | 'update';
	let submissionMode = $state<SubmissionMode>('create');

	// Sync form field values to formData structure
	function syncFormValuesToFormData(values: typeof form.state.values) {
		formData.name = values.name;
		formData.hostname = values.hostname;
		formData.description = values.description;

		// Sync interface field values (ip, mac, name) to formData.interfaces
		if (values.interfaces && formData.interfaces) {
			for (let i = 0; i < formData.interfaces.length && i < values.interfaces.length; i++) {
				const formIface = values.interfaces[i];
				const dataIface = formData.interfaces[i];
				if (formIface && dataIface) {
					dataIface.ip_address = formIface.ip_address ?? '';
					dataIface.mac_address = formIface.mac_address ?? null;
					dataIface.name = formIface.name ?? null;
				}
			}
		}

		// Sync port field values (number, protocol) to formData.ports
		if (values.ports && formData.ports) {
			for (let i = 0; i < formData.ports.length && i < values.ports.length; i++) {
				const formPort = values.ports[i];
				const dataPort = formData.ports[i];
				if (formPort && dataPort) {
					dataPort.number = formPort.number;
					dataPort.protocol = formPort.protocol;
				}
			}
		}
	}

	// Perform the actual submission - defined as a separate function so it
	// has access to latest state (TanStack Form's onSubmit captures state at creation time)
	async function performSubmission(value: typeof form.state.values) {

		// Sync form values to formData structure
		syncFormValuesToFormData(value);


		loading = true;
		try {
			if (submissionMode === 'update' && host) {
				// Update existing host
				const hostPrimitive = formDataToHostPrimitive(formData);
				const promises = [
					onUpdate({
						host: hostPrimitive,
						interfaces: formData.interfaces,
						ports: formData.ports,
						services: currentHostServices
					})
				];

				// VM managed hosts - only update host fields, not children
				for (const updatedHost of vmManagedHostUpdates.values()) {
					promises.push(
						onUpdate({
							host: updatedHost,
							interfaces: null,
							ports: null,
							services: null
						})
					);
				}

				await Promise.all(promises);
				handleClose();
			} else if (submissionMode === 'createAndContinue') {
				// Create and keep modal open for adding services
				await onCreateAndContinue({ host: formData, services: [] });
			} else {
				// Create and close
				await onCreate({ host: formData, services: currentHostServices });
				handleClose();
			}
		} catch (error) {
			pushError(error instanceof Error ? error.message : 'Failed to save host');
		} finally {
			loading = false;
		}
	}

	// TanStack Form - onSubmit delegates to performSubmission for access to latest state
	const form = createForm(() => ({
		defaultValues: {
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || []
		},
		onSubmit: async ({ value }) => {
			await performSubmission(value);
		}
	}));

	// Initialize form data when host changes or modal opens
	function handleOpen() {
		resetForm();
	}

	// Track host ID to detect when host changes (e.g., after createAndContinue)
	let lastHostId = $state<string | null>(null);
	$effect(() => {
		const currentHostId = host?.id ?? null;
		if (isOpen && currentHostId !== lastHostId) {
			// Host changed while modal is open (e.g., after createAndContinue)
			// Reset form to use the new host's data, but preserve current tab
			if (lastHostId !== null || currentHostId !== null) {
				const currentTab = activeTab;
				resetForm();
				activeTab = currentTab; // Preserve tab position
			}
			lastHostId = currentHostId;
		}
	});

	let vmManagerServices = $derived(
		currentHostServices.filter(
			(s) => serviceDefinitions.getMetadata(s.service_definition).manages_virtualization != null
		)
	);

	function handleVirtualizationServiceChange(updatedService: Service) {
		// Find the actual index in currentHostServices
		const actualIndex = currentHostServices.findIndex((s) => s.id === updatedService.id);
		if (actualIndex >= 0) {
			currentHostServices[actualIndex] = updatedService;
			currentHostServices = [...currentHostServices]; // Trigger reactivity
		}
	}

	let vmManagedHostUpdates: SvelteMap<string, Host> = new SvelteMap();

	function handleVirtualizationHostChange(updatedHost: Host) {
		// This is another host; ie not the current
		// Hold on to updates and only make them on submit
		vmManagedHostUpdates.set(updatedHost.id, updatedHost);
	}

	// Tab management
	let activeTab = $state('details');
	let tabs = $derived([
		{
			id: 'details',
			label: 'Details',
			icon: Info,
			description: 'Basic host information and connection details'
		},
		{
			id: 'interfaces',
			label: 'Interfaces',
			icon: entities.getIconComponent('Interface'),
			description: 'Network interfaces and subnet membership'
		},
		{
			id: 'ports',
			label: 'Ports',
			icon: entities.getIconComponent('Port'),
			description: 'Service configuration'
		},
		{
			id: 'services',
			label: 'Services',
			icon: entities.getIconComponent('Service'),
			description: 'Service configuration'
		},
		...(vmManagerServices && vmManagerServices.length > 0
			? [
					{
						id: 'virtualization',
						label: 'Virtualization',
						icon: concepts.getIconComponent('Virtualization'),
						description: 'VMs and containers managed by services on this host'
					}
				]
			: [])
	]);

	let currentTabIndex = $derived(tabs.findIndex((t) => t.id === activeTab) || 0);

	function nextTab() {
		if (currentTabIndex < tabs.length - 1) {
			activeTab = tabs[currentTabIndex + 1].id;
		}
	}

	function previousTab() {
		if (currentTabIndex > 0) {
			activeTab = tabs[currentTabIndex - 1].id;
		}
	}

	function resetForm() {
		// Hydrate host to HostFormData for form editing (includes interfaces, ports, services)
		formData = host
			? hydrateHostToFormData(host, queryClient)
			: createEmptyHostFormData(defaultNetworkId);

		// Reset TanStack form
		form.reset({
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || []
		});

		if (host && host.id) {
			// Get services for this host from query data
			currentHostServices = servicesData.filter((s) => s.host_id === host.id);
		} else {
			currentHostServices = [];
		}
		activeTab = 'details'; // Reset to first tab
	}

	// Submit the form with appropriate mode
	async function handleSubmit() {
		submissionMode = isEditing ? 'update' : 'create';
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && host) {
			deleting = true;
			try {
				await onDelete(host.id);
				handleClose();
			} catch (error) {
				pushError(error instanceof Error ? error.message : 'Failed to delete host');
			} finally {
				deleting = false;
			}
		}
	}

	// Handle form-based submission for create flow with steps
	async function handleFormSubmit() {
		if (isEditing || currentTabIndex === tabs.length - 1) {
			handleSubmit();
		} else {
			// Validate all fields before advancing to next tab
			const isValid = await validateForm(form);
			if (isValid) {
				nextTab();
			}
		}
	}

	function handleClose() {
		activeTab = 'details';
		onClose();
	}

	function handleFormCancel() {
		if (isEditing || currentTabIndex == 0) {
			handleClose();
		} else {
			previousTab();
		}
	}

	// Check if we're on the services tab during create mode
	let isServicesTabDuringCreate = $derived(!isEditing && activeTab === 'services');

	// Dynamic labels based on create/edit mode and tab position
	let saveLabel = $derived(
		isEditing ? 'Update Host' : currentTabIndex === tabs.length - 1 ? 'Create Host' : 'Next'
	);
	let cancelLabel = $derived(isEditing ? 'Cancel' : 'Previous');
	let showCancel = $derived(isEditing ? true : currentTabIndex !== 0);

	// Handler for "Create Host & Add Services" - creates host and keeps modal open
	async function handleCreateAndContinue() {
		submissionMode = 'createAndContinue';
		await submitForm(form);
	}

	// Handler for "Create Host" when on services tab - creates host and closes
	async function handleCreateAndClose() {
		submissionMode = 'create';
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	{title}
	onClose={handleClose}
	onOpen={handleOpen}
	size="full"
	showCloseButton={true}
	tabs={isEditing ? tabs : []}
	{activeTab}
	onTabChange={(tabId) => (activeTab = tabId)}
>
	<!-- Header icon -->
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon
			Icon={entities.getIconComponent('Host')}
			color={entities.getColorString('Host')}
		/>
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleFormSubmit();
		}}
		class="flex h-full min-h-0 flex-col"
	>
		<!-- Content -->
		<div class="flex-1 overflow-auto">
			<!-- Details Tab -->
			{#if activeTab === 'details'}
				<div class="h-full">
					<div class="relative flex-1">
						<DetailsForm {form} {isEditing} {host} bind:formData />
					</div>
				</div>
			{/if}

			<!-- Interfaces Tab -->
			{#if activeTab === 'interfaces'}
				<div class="h-full">
					<div class="relative flex-1">
						<InterfacesForm
							bind:formData
							{form}
							currentServices={currentHostServices}
							onServicesChange={(services) => (currentHostServices = services)}
						/>
					</div>
				</div>
			{/if}

			<!-- Ports Tab -->
			{#if activeTab === 'ports'}
				<div class="h-full">
					<div class="relative flex-1">
						<PortsForm
							bind:formData
							{form}
							currentServices={currentHostServices}
							onServicesChange={(services) => (currentHostServices = services)}
						/>
					</div>
				</div>
			{/if}

			<!-- Services Tab -->
			{#if activeTab === 'services'}
				<div class="h-full">
					<div class="relative flex-1">
						<ServicesForm
							bind:formData
							currentServices={currentHostServices}
							onServicesChange={(services) => (currentHostServices = services)}
							{isEditing}
						/>
					</div>
				</div>
			{/if}

			<!-- Virtualization Tab -->
			{#if activeTab === 'virtualization'}
				<div class="h-full">
					<div class="relative flex-1">
						<VirtualizationForm
							virtualizationManagerServices={vmManagerServices}
							onServiceChange={handleVirtualizationServiceChange}
							onVirtualizedHostChange={handleVirtualizationHostChange}
						/>
					</div>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			{#if isServicesTabDuringCreate}
				<!-- Special footer for services tab during create mode -->
				<div class="flex items-center justify-between">
					<div></div>
					<div class="flex items-center gap-3">
						<button
							type="button"
							disabled={loading}
							onclick={handleFormCancel}
							class="btn-secondary"
						>
							Previous
						</button>
						<button
							type="button"
							disabled={loading || deleting}
							onclick={handleCreateAndClose}
							class="btn-secondary"
						>
							{loading ? 'Creating...' : 'Create Host'}
						</button>
						<button
							type="button"
							disabled={loading || deleting}
							onclick={handleCreateAndContinue}
							class="btn-primary"
						>
							{loading ? 'Creating...' : 'Create Host & Add Services'}
						</button>
					</div>
				</div>
			{:else}
				<!-- Default footer behavior -->
				<div class="flex items-center justify-between">
					<div>
						{#if isEditing && onDelete}
							<button
								type="button"
								disabled={deleting || loading}
								onclick={handleDelete}
								class="btn-danger"
							>
								{deleting ? 'Deleting...' : 'Delete'}
							</button>
						{/if}
					</div>
					<div class="flex items-center gap-3">
						{#if showCancel}
							<button
								type="button"
								disabled={loading || deleting}
								onclick={handleFormCancel}
								class="btn-secondary"
							>
								{cancelLabel}
							</button>
						{/if}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? 'Saving...' : saveLabel}
						</button>
					</div>
				</div>
			{/if}
		</div>
	</form>
</GenericModal>
