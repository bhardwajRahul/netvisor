<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import { createEmptyGroupFormData } from '../../queries';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { Group, EdgeStyle } from '../../types/base';
	import type { Color } from '$lib/shared/utils/styling';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities, groupTypes } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { BindingWithServiceDisplay } from '$lib/shared/components/forms/selection/display/BindingWithServiceDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import EdgeStyleForm from './EdgeStyleForm.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import {
		common_cancel,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_editName,
		common_saving,
		common_update,
		groups_createGroup,
		groups_descriptionPlaceholder,
		groups_edgeAppearance,
		groups_groupDetails,
		groups_groupName,
		groups_groupNamePlaceholder,
		groups_groupType,
		groups_loadingServices,
		groups_noBindingsYet,
		groups_selectBinding,
		groups_serviceBindings,
		groups_serviceBindingsHelp
	} from '$lib/paraglide/messages';

	interface Props {
		group?: Group | null;
		isOpen?: boolean;
		onCreate: (data: Group) => Promise<void> | void;
		onUpdate: (id: string, data: Group) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	}

	let {
		group = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null
	}: Props = $props();

	// TanStack Query hooks
	const servicesQuery = useServicesCacheQuery();
	const networksQuery = useNetworksQuery();
	// Use limit: 0 to get all hosts for group edit modal
	const hostsQuery = useHostsQuery({ limit: 0 });
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();
	const subnetsQuery = useSubnetsQuery();

	let servicesData = $derived(servicesQuery.data ?? []);
	let isServicesLoading = $derived(servicesQuery.isLoading);
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	// Context for BindingWithServiceDisplay
	let bindingContext = $derived({
		services: servicesData,
		hosts: hostsData,
		interfaces: interfacesData,
		ports: portsData,
		isContainerSubnet: isContainerSubnetFn
	});

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(group !== null);
	let title = $derived(
		isEditing ? common_editName({ name: group?.name ?? '' }) : groups_createGroup()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	function getDefaultValues(): Group {
		return group ? { ...group } : createEmptyGroupFormData(defaultNetworkId);
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createEmptyGroupFormData(''),
		onSubmit: async ({ value }) => {
			const groupData: Group = {
				...(value as Group),
				name: value.name.trim(),
				description: value.description?.trim() || '',
				// Explicitly use local bindingIds state - form.setFieldValue doesn't reliably
				// sync array fields back to form.state.values in TanStack Svelte Form
				binding_ids: bindingIds
			};

			loading = true;
			try {
				if (isEditing && group) {
					await onUpdate(group.id, groupData);
				} else {
					await onCreate(groupData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Local state for binding_ids to enable Svelte 5 reactivity
	// (form.state.values is not tracked by $derived)
	let bindingIds = $state<string[]>([]);
	let selectedNetworkId = $state<string>('');

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		bindingIds = defaults.binding_ids ?? [];
		selectedNetworkId = defaults.network_id ?? '';
	}

	// Available service bindings (exclude already selected ones and Unclaimed Open Ports)
	let availableServiceBindings = $derived.by(() => {
		return servicesData
			.filter((s) => s.network_id == selectedNetworkId)
			.filter((s) => s.service_definition !== 'Unclaimed Open Ports')
			.flatMap((s) => s.bindings)
			.filter((sb) => !bindingIds.some((binding) => binding === sb.id));
	});

	let selectedServiceBindings = $derived.by(() => {
		return bindingIds
			.map((bindingId) => servicesData.flatMap((s) => s.bindings).find((sb) => sb.id === bindingId))
			.filter(Boolean);
	});

	// Handlers for service bindings
	function handleAdd(bindingId: string) {
		bindingIds = [...bindingIds, bindingId];
		form.setFieldValue('binding_ids', bindingIds);
	}

	function handleRemove(index: number) {
		bindingIds = bindingIds.filter((_, i) => i !== index);
		form.setFieldValue('binding_ids', bindingIds);
	}

	function handleServiceBindingsReorder(fromIndex: number, toIndex: number) {
		if (fromIndex === toIndex) return;
		const current = [...bindingIds];
		const [movedBinding] = current.splice(fromIndex, 1);
		current.splice(toIndex, 0, movedBinding);
		bindingIds = current;
		form.setFieldValue('binding_ids', bindingIds);
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && group) {
			deleting = true;
			try {
				await onDelete(group.id);
			} finally {
				deleting = false;
			}
		}
	}

	// Group type options
	let groupTypeOptions = $derived(
		groupTypes.getItems().map((gt) => ({
			value: gt.id,
			label: gt.name ?? gt.id
		}))
	);

	let colorHelper = entities.getColorHelper('Group');

	// EdgeStyleForm needs direct formData binding - create a reactive wrapper

	let edgeStyleFormData = $derived({
		get color() {
			return form.state.values.color;
		},
		set color(v: Color) {
			form.setFieldValue('color', v);
		},
		get edge_style() {
			return form.state.values.edge_style;
		},
		set edge_style(v: EdgeStyle) {
			form.setFieldValue('edge_style', v);
		}
	} as Group);
</script>

<GenericModal {isOpen} {title} size="xl" {onClose} onOpen={handleOpen} showCloseButton={true}>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Group')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-8">
				<!-- Group Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{groups_groupDetails()}</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={groups_groupName()}
								id="name"
								{field}
								placeholder={groups_groupNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="network_id">
						{#snippet children(field)}
							<SelectNetwork
								selectedNetworkId={field.state.value}
								onNetworkChange={(id) => {
									field.handleChange(id);
									selectedNetworkId = id;
								}}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="group_type">
						{#snippet children(field)}
							<SelectInput
								label={groups_groupType()}
								id="group_type"
								{field}
								options={groupTypeOptions}
							/>
							<p class="text-tertiary text-xs">{groupTypes.getDescription(field.state.value)}</p>
						{/snippet}
					</form.Field>

					<form.Field
						name="description"
						validators={{
							onBlur: ({ value }) => max(500)(value || '')
						}}
					>
						{#snippet children(field)}
							<TextArea
								label={common_description()}
								id="description"
								{field}
								placeholder={groups_descriptionPlaceholder()}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>
				</div>

				<!-- Service Bindings Section -->
				<div class="space-y-4">
					<div class="border-t border-gray-700 pt-6">
						<div class="rounded-lg bg-gray-800/50 p-4">
							<ListManager
								label={groups_serviceBindings()}
								helpText={groups_serviceBindingsHelp()}
								placeholder={isServicesLoading ? groups_loadingServices() : groups_selectBinding()}
								emptyMessage={groups_noBindingsYet()}
								allowReorder={true}
								allowItemEdit={() => false}
								showSearch={true}
								options={availableServiceBindings}
								items={selectedServiceBindings}
								optionDisplayComponent={BindingWithServiceDisplay}
								itemDisplayComponent={BindingWithServiceDisplay}
								getItemContext={() => bindingContext}
								getOptionContext={() => bindingContext}
								onAdd={handleAdd}
								onRemove={handleRemove}
								onMoveUp={(index) => handleServiceBindingsReorder(index, index - 1)}
								onMoveDown={(index) => handleServiceBindingsReorder(index, index + 1)}
							/>
						</div>
					</div>
				</div>

				<!-- Edge Style Section -->
				<div class="space-y-4">
					<div class="border-t border-gray-700 pt-6">
						<h3 class="text-primary mb-4 text-lg font-medium">{groups_edgeAppearance()}</h3>
						<div class="rounded-lg bg-gray-800/50 p-4">
							<EdgeStyleForm formData={edgeStyleFormData} />
						</div>
					</div>
				</div>

				{#if isEditing && group}
					<EntityMetadataSection entities={[group]} />
				{/if}
			</div>
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
					<button
						type="button"
						disabled={loading || deleting}
						onclick={onClose}
						class="btn-secondary"
					>
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
