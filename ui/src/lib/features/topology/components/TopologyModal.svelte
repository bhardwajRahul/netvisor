<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { Topology } from '../types/base';
	import {
		createEmptyTopologyFormData,
		createTopology,
		topologies,
		topology,
		topologyOptions,
		updateTopology
	} from '../store';
	import { entities } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let {
		isOpen = $bindable(false),
		onSubmit,
		onClose,
		topo = null
	}: {
		isOpen: boolean;
		onSubmit: () => Promise<void> | void;
		onClose: () => void;
		topo: Topology | null;
	} = $props();

	let isEditing = $derived(topo != null);
	let title = $derived(isEditing ? `Edit ${topo?.name}` : 'Create Topology');

	let loading = $state(false);

	$effect(() => {
		void $topology;
		void $topologies;
	});

	function getDefaultValues(): Topology {
		return topo ? { ...topo } : createEmptyTopologyFormData(defaultNetworkId);
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createEmptyTopologyFormData(''),
		onSubmit: async ({ value }) => {
			const topologyData: Topology = {
				...(value as Topology),
				name: value.name.trim(),
				options: $topologyOptions
			};

			loading = true;
			try {
				if (isEditing) {
					await updateTopology(topologyData);
				} else {
					await createTopology(topologyData);
				}
				await onSubmit();
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	// Available topologies for parent selection (exclude current and filter by network)
	let availableTopologies = $derived(() => {
		const networkId = form.state.values.network_id;
		const currentId = form.state.values.id;
		return $topologies.filter((t) => t.id !== currentId && t.network_id == networkId);
	});

	let colorHelper = entities.getColorHelper('Topology');
	let Icon = entities.getIconComponent('Topology');
</script>

<GenericModal
	{isOpen}
	{title}
	size="md"
	onClose={onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon {Icon} color={colorHelper.color} />
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex h-full flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				<form.Field name="network_id">
					{#snippet children(field)}
						<SelectNetwork
							selectedNetworkId={field.state.value}
							onNetworkChange={(id) => field.handleChange(id)}
							disabled={isEditing}
						/>
					{/snippet}
				</form.Field>

				<form.Field name="parent_id">
					{#snippet children(field)}
						<div>
							<RichSelect
								label="(Optional) Select a parent to branch off of"
								displayComponent={TopologyDisplay}
								required={false}
								disabled={isEditing}
								selectedValue={field.state.value}
								onSelect={(id) => field.handleChange(id)}
								options={availableTopologies()}
							/>
						</div>
					{/snippet}
				</form.Field>

				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value) || min(3)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label="Name"
							id="name"
							{field}
							placeholder="Enter topology name"
							required
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
					Cancel
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? 'Saving...' : 'Save'}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
