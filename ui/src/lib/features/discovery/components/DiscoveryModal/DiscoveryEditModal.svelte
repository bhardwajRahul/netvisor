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
	import { createEmptyDiscoveryFormData, parseCronToHours } from '../../queries';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import * as m from '$lib/paraglide/messages';

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
				? m.discovery_viewRun({ name: discovery?.name ?? '' })
				: m.discovery_edit({ name: discovery?.name ?? '' })
			: m.discovery_createScheduled()
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

	// TanStack Form for validation
	// NOTE: defaultValues must NOT read from $state to avoid reactivity loops
	const form = createForm(() => ({
		defaultValues: {
			name: '',
			run_type_type: 'AdHoc' as 'AdHoc' | 'Scheduled',
			discovery_type_type: 'Network' as 'Network' | 'Docker' | 'SelfReport',
			host_naming_fallback: 'BestService' as 'BestService' | 'Ip',
			schedule_days: '1',
			schedule_hours: '0'
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

		// Compute schedule days/hours from cron
		let scheduleDays = '1';
		let scheduleHours = '0';
		if (formData.run_type.type === 'Scheduled' && formData.run_type.cron_schedule) {
			const totalHours = parseCronToHours(formData.run_type.cron_schedule);
			if (totalHours !== null) {
				scheduleDays = String(Math.floor(totalHours / 24));
				scheduleHours = String(totalHours % 24);
			}
		}

		// Compute host naming fallback
		const hostNamingFallback =
			formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Docker'
				? formData.discovery_type.host_naming_fallback
				: 'BestService';

		form.reset({
			name: formData.name,
			run_type_type: formData.run_type.type === 'Historical' ? 'AdHoc' : formData.run_type.type,
			discovery_type_type: formData.discovery_type.type,
			host_naming_fallback: hostNamingFallback,
			schedule_days: scheduleDays,
			schedule_hours: scheduleHours
		});
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

	let saveLabel = $derived(
		isEditing ? m.discovery_updateDiscovery() : m.discovery_createDiscovery()
	);
	let showSave = $derived(!isHistoricalRun);

	let colorHelper = entities.getColorHelper('Discovery');
</script>

<GenericModal {isOpen} {title} {onClose} onOpen={handleOpen} size="xl" showCloseButton={true}>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Discovery')} color={colorHelper.color} />
	{/snippet}

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
					<DiscoveryTypeForm {form} bind:formData {readOnly} {daemonHostId} {daemon} />
				{:else}
					<InlineWarning body={m.discovery_noDaemonSelected()} />
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
							{deleting ? m.discovery_deleting() : m.common_delete()}
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
						{isHistoricalRun ? m.common_close() : m.common_cancel()}
					</button>
					{#if showSave}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? m.discovery_saving() : saveLabel}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
