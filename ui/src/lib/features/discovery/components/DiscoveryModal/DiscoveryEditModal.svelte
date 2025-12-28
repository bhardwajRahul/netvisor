<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import DiscoveryDetailsForm from './DiscoveryDetailsForm.svelte';
	import DiscoveryTypeForm from './DiscoveryTypeForm.svelte';
	import type { Discovery } from '../../types/base';
	import DiscoveryHistoricalSummary from './DiscoveryHistoricalSummary.svelte';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import { createEmptyDiscoveryFormData } from '../../queries';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Host } from '$lib/features/hosts/types/base';

	interface Props {
		discovery?: Discovery | null;
		isOpen?: boolean;
		daemons?: Daemon[];
		hosts?: Host[];
		onCreate: (data: Discovery) => Promise<void> | void;
		onUpdate: (id: string, data: Discovery) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	}

	let {
		discovery = null,
		isOpen = false,
		daemons = [],
		hosts = [],
		onCreate,
		onUpdate,
		onClose,
		onDelete = null
	}: Props = $props();

	let loading = $state(false);
	let deleting = $state(false);

	// Mutable form data that sub-components can update
	let formData = $state<Discovery>(createEmptyDiscoveryFormData(null));

	let isEditing = $derived(discovery !== null);
	let isHistoricalRun = $derived(discovery?.run_type.type === 'Historical');
	let readOnly = $derived(formData.run_type.type == 'Historical');

	let title = $derived(
		isEditing
			? isHistoricalRun
				? `View Discovery Run: ${discovery?.name}`
				: `Edit Discovery: ${discovery?.name}`
			: 'Create Scheduled Discovery'
	);

	let daemon = $derived(daemons.find((d) => d.id === formData.daemon_id) || null);
	let daemonHostId = $derived(
		(daemon ? hosts.find((h) => h.id === daemon.host_id)?.id : null) || null
	);

	function getDefaultFormData(): Discovery {
		const defaultDaemon = daemons.length > 0 ? daemons[0] : null;
		if (discovery) {
			return { ...discovery };
		}
		const empty = createEmptyDiscoveryFormData(defaultDaemon);
		if (defaultDaemon) {
			empty.daemon_id = defaultDaemon.id;
			empty.network_id = defaultDaemon.network_id;
		}
		return empty;
	}

	// TanStack Form for validation (name field)
	// NOTE: defaultValues must NOT read from $state to avoid reactivity loops
	const form = createForm(() => ({
		defaultValues: {
			name: ''
		},
		onSubmit: async ({ value }) => {
			// Update formData with form values
			formData.name = value.name.trim();

			if (daemon) {
				loading = true;
				try {
					if (isEditing && discovery) {
						await onUpdate(discovery.id, formData);
					} else {
						await onCreate(formData);
					}
					onClose();
				} catch (error) {
					pushError(error instanceof Error ? error.message : 'Failed to save discovery');
				} finally {
					loading = false;
				}
			} else {
				pushError('Could not get network ID from selected daemon. Please try a different daemon.');
			}
		}
	}));

	function handleOpen() {
		formData = getDefaultFormData();
		form.reset({ name: formData.name });
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && discovery) {
			deleting = true;
			try {
				await onDelete(discovery.id);
				onClose();
			} catch (error) {
				pushError(error instanceof Error ? error.message : 'Failed to delete discovery');
			} finally {
				deleting = false;
			}
		}
	}

	// Set default daemon when available and formData has sentinel
	$effect(() => {
		if (formData.daemon_id === uuidv4Sentinel && daemons.length > 0) {
			formData.daemon_id = daemons[0].id;
			formData.network_id = daemons[0].network_id;
		}
	});

	let saveLabel = $derived(isEditing ? 'Update Discovery' : 'Create Discovery');
	let showSave = $derived(!isHistoricalRun);

	let colorHelper = entities.getColorHelper('Discovery');
</script>

<GenericModal {isOpen} {title} {onClose} onOpen={handleOpen} size="xl" showCloseButton={true}>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={entities.getIconComponent('Discovery')} color={colorHelper.color} />
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (showSave) handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-y-auto">
			<div class="space-y-8 p-6">
				<DiscoveryDetailsForm {form} {daemons} bind:formData {readOnly} />

				{#if isHistoricalRun && discovery?.run_type.type === 'Historical'}
					<DiscoveryHistoricalSummary payload={discovery.run_type.results} />
				{:else if daemon}
					<DiscoveryTypeForm bind:formData {readOnly} {daemonHostId} {daemon} />
				{:else}
					<InlineWarning body="No daemon selected; can't set up discovery" />
				{/if}

				{#if isEditing}
					<EntityMetadataSection entities={[discovery]} />
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && !isHistoricalRun && onDelete}
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
					<button
						type="button"
						disabled={loading || deleting}
						onclick={onClose}
						class="btn-secondary"
					>
						{isHistoricalRun ? 'Close' : 'Cancel'}
					</button>
					{#if showSave}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? 'Saving...' : saveLabel}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
