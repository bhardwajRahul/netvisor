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
	import {
		formDataToHostPrimitive,
		hydrateHostToFormData,
		createEmptyHostFormData
	} from '$lib/features/hosts/queries';
	import { useQueryClient } from '@tanstack/svelte-query';
	import DetailsForm from './Details/HostDetailsForm.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import InterfacesForm from './Interfaces/InterfacesForm.svelte';
	import ServicesForm from './Services/ServicesForm.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Service } from '$lib/features/services/types/base';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import PortsForm from './Ports/PortsForm.svelte';
	import VirtualizationForm from './Virtualization/VirtualizationForm.svelte';
	import SnmpForm from './Snmp/SnmpForm.svelte';
	import IfEntriesForm from './IfEntries/IfEntriesForm.svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		common_back,
		common_cancel,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_interfaces,
		common_next,
		common_ports,
		common_saving,
		common_serviceConfiguration,
		common_services,
		common_virtualization,
		hosts_createHost,
		hosts_editor_basicInfo,
		hosts_editor_ifEntriesTab,
		hosts_editor_interfacesDesc,
		hosts_editor_snmpTab,
		hosts_editor_snmpTabDesc,
		hosts_editor_updateHost,
		hosts_editor_virtualizationDesc,
		hosts_failedToDelete,
		hosts_failedToSave,
		hosts_ifEntries_subtitle
	} from '$lib/paraglide/messages';

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
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(host !== null);
	let title = $derived(
		isEditing ? common_editName({ name: host?.name ?? '' }) : hosts_createHost()
	);

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
						services: formData.services
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
				await onCreate({ host: formData, services: formData.services });
				handleClose();
			}
		} catch (error) {
			pushError(error instanceof Error ? error.message : hosts_failedToSave());
		} finally {
			loading = false;
		}
	}

	// TanStack Form - onSubmit delegates to performSubmission for access to latest state
	// Note: We intentionally do NOT recreate the form on modal open. Reassigning `form` causes
	// Field components to remain bound to the old form (TanStack Form is not reactive to Svelte).
	// Instead, we use form.reset() which properly resets values, validation, and touched state.
	// Fields unmount when modal closes ({#if isOpen} in GenericModal) and re-register on open.
	let form = createForm(() => ({
		defaultValues: {
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || [],
			services: formData.services || [],
			credential_mode: (formData.snmp_credential_id ? 'override' : 'default') as
				| 'default'
				| 'override'
		},
		onSubmit: async ({ value }) => {
			await performSubmission(value);
		}
	}));

	// Initialize form data when host changes or modal opens
	let modalInitialized = $state(false);
	function handleOpen() {
		modalInitialized = true;
		lastHostId = host?.id ?? null;
		resetForm();
	}

	// Track host ID to detect when host changes WHILE modal is already open
	// (e.g., after createAndContinue creates a new host).
	// Initial open is handled by handleOpen(), so we check modalInitialized.
	let lastHostId = $state<string | null>(null);
	$effect(() => {
		const currentHostId = host?.id ?? null;
		// Only reset if modal was already initialized and host changed
		if (isOpen && modalInitialized && currentHostId !== lastHostId) {
			const currentTab = activeTab;
			resetForm();
			activeTab = currentTab; // Preserve tab position
			lastHostId = currentHostId;
		}
		// Reset flag when modal closes
		if (!isOpen) {
			modalInitialized = false;
		}
	});

	let vmManagerServices = $derived(
		(formData.services || []).filter(
			(s) => serviceDefinitions.getMetadata(s.service_definition).manages_virtualization != null
		)
	);

	function handleVirtualizationServiceChange(updatedService: Service) {
		// Find the actual index in formData.services
		const actualIndex = formData.services.findIndex((s) => s.id === updatedService.id);
		if (actualIndex >= 0) {
			const updatedServices = [...formData.services];
			updatedServices[actualIndex] = updatedService;
			formData.services = updatedServices;
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
	// Get network for passing to SNMP form
	let currentNetwork = $derived(networksData.find((n) => n.id === formData.network_id) ?? null);

	let tabs = $derived([
		{
			id: 'details',
			label: common_details(),
			icon: Info,
			description: hosts_editor_basicInfo()
		},
		{
			id: 'interfaces',
			label: common_interfaces(),
			icon: entities.getIconComponent('Interface'),
			description: hosts_editor_interfacesDesc()
		},
		{
			id: 'ports',
			label: common_ports(),
			icon: entities.getIconComponent('Port'),
			description: common_serviceConfiguration()
		},
		{
			id: 'services',
			label: common_services(),
			icon: entities.getIconComponent('Service'),
			description: common_serviceConfiguration()
		},
		...(vmManagerServices && vmManagerServices.length > 0
			? [
					{
						id: 'virtualization',
						label: common_virtualization(),
						icon: concepts.getIconComponent('Virtualization'),
						description: hosts_editor_virtualizationDesc()
					}
				]
			: []),
		// SNMP tab - always shown (users can set credential before discovery)
		{
			id: 'snmp',
			label: hosts_editor_snmpTab(),
			icon: concepts.getIconComponent('SNMP'),
			description: hosts_editor_snmpTabDesc()
		},
		// IfEntries tab - only show when data exists (populated by discovery)
		...(formData.if_entries && formData.if_entries.length > 0
			? [
					{
						id: 'if-entries',
						label: hosts_editor_ifEntriesTab(),
						icon: entities.getIconComponent('IfEntry'),
						description: hosts_ifEntries_subtitle()
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

		// Sort services by position
		if (formData.services) {
			formData.services = formData.services.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
		}

		// Reset TanStack form
		form.reset({
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || [],
			services: formData.services || [],
			credential_mode: formData.snmp_credential_id ? 'override' : 'default'
		});

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
				pushError(error instanceof Error ? error.message : hosts_failedToDelete());
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

	// Dynamic labels based on create/edit mode and tab position
	let saveLabel = $derived(
		isEditing
			? hosts_editor_updateHost()
			: currentTabIndex === tabs.length - 1
				? hosts_createHost()
				: common_next()
	);
	let cancelLabel = $derived(isEditing ? common_cancel() : common_back());
	let showCancel = $derived(isEditing ? true : currentTabIndex !== 0);
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
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={entities.getIconComponent('Host')}
			color={entities.getColorString('Host')}
		/>
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleFormSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
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
							{isEditing}
							currentServices={formData.services}
							onServicesChange={(services) => (formData.services = services)}
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
							currentServices={formData.services}
							onServicesChange={(services) => (formData.services = services)}
						/>
					</div>
				</div>
			{/if}

			<!-- Services Tab -->
			{#if activeTab === 'services'}
				<div class="h-full">
					<div class="relative flex-1">
						<ServicesForm bind:formData {form} />
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

			<!-- SNMP Tab -->
			{#if activeTab === 'snmp'}
				<div class="h-full">
					<div class="relative flex-1">
						<SnmpForm bind:formData {form} {isEditing} network={currentNetwork} />
					</div>
				</div>
			{/if}

			<!-- IfEntries Tab -->
			{#if activeTab === 'if-entries'}
				<div class="h-full">
					<div class="relative flex-1">
						<IfEntriesForm {formData} />
					</div>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
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
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
