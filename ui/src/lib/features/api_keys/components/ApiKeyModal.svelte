<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { entities } from '$lib/shared/stores/metadata';
	import { RotateCcwKey } from 'lucide-svelte';
	import type { ApiKey } from '../types/base';
	import { createEmptyApiKeyFormData, createNewApiKey, rotateKey } from '../store';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onUpdate: (data: ApiKey) => Promise<void> | void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		apiKey?: ApiKey | null;
	}

	let { isOpen = false, onClose, onUpdate, onDelete = null, apiKey = null }: Props = $props();

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let loading = $state(false);
	let deleting = $state(false);
	let key = $state<string | null>(null);

	let isEditing = $derived(apiKey !== null);
	let title = $derived(isEditing ? `Edit ${apiKey?.name || 'API Key'}` : 'Create API Key');

	// Get minimum date (today)
	const today = new Date().toISOString().slice(0, 16);

	function getDefaultValues(): ApiKey {
		return apiKey ? { ...apiKey } : createEmptyApiKeyFormData(defaultNetworkId);
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createEmptyApiKeyFormData(''),
		onSubmit: async ({ value }) => {
			loading = true;
			try {
				if (isEditing) {
					await onUpdate(value as ApiKey);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		key = null;

		// If network_id is empty but we have a default, set it
		if (!defaults.network_id && defaultNetworkId) {
			form.setFieldValue('network_id', defaultNetworkId);
		}
	}

	function handleOnClose() {
		key = null;
		onClose();
	}

	async function handleGenerateKey() {
		const formData = form.state.values as ApiKey;

		// Ensure network_id is set
		if (!formData.network_id) {
			if (defaultNetworkId) {
				formData.network_id = defaultNetworkId;
			} else {
				pushError('No network available. Please create a network first.');
				return;
			}
		}

		loading = true;
		try {
			const generatedKey = await createNewApiKey(formData);
			if (generatedKey) {
				key = generatedKey;
			} else {
				pushError('Failed to generate API key');
			}
		} finally {
			loading = false;
		}
	}

	async function handleRotateKey() {
		const formData = form.state.values as ApiKey;
		const generatedKey = await rotateKey(formData.id);
		if (generatedKey) {
			key = generatedKey;
		} else {
			pushError('Failed to generate API key');
		}
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && apiKey) {
			deleting = true;
			try {
				await onDelete(apiKey.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('ApiKey');
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleOnClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={entities.getIconComponent('ApiKey')} color={colorHelper.color} />
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
			<div class="space-y-6">
				<!-- Key Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">Key Details</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Name"
								id="name"
								{field}
								placeholder="e.g., Production Daemon Key, Terraform Deployment"
								helpText="A friendly name to help you identify this key"
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="network_id">
						{#snippet children(field)}
							<SelectNetwork
								selectedNetworkId={field.state.value}
								onNetworkChange={(id) => field.handleChange(id)}
								disabled={isEditing}
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

					<form.Field name="expires_at">
						{#snippet children(field)}
							<DateInput
								label="Expiration Date (Optional)"
								id="expires_at"
								{field}
								helpText="Leave empty for keys that never expire"
								min={today}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="is_enabled">
						{#snippet children(field)}
							<Checkbox
								{field}
								label="Enable API Key"
								helpText="Manually enable or disable API Key. Will be auto-disabled if used in a request after expiry date passes."
								id="enableApiKey"
							/>
						{/snippet}
					</form.Field>
				</div>

				<!-- Key generation section -->
				<div class="space-y-3">
					{#if !key && isEditing}
						<InlineWarning
							title="Generating a new key will invalidate your old key"
							body="Click the button below to generate a new API key. You'll only see it once, so make sure to copy it."
						/>
					{/if}

					{#if key}
						<InlineWarning
							title="Save this key now"
							body="This key will not be shown again. Copy it now and store it securely."
						/>
					{/if}

					<div class="flex items-start gap-2">
						<button
							type="button"
							class="btn-primary flex-shrink-0 self-stretch"
							onclick={apiKey != null ? handleRotateKey : handleGenerateKey}
							disabled={loading}
						>
							<RotateCcwKey />
							<span>{loading ? 'Generating...' : isEditing ? 'Rotate Key' : 'Generate Key'}</span>
						</button>

						<div class="flex-1">
							<CodeContainer
								language="bash"
								expandable={false}
								code={key ? key : 'Press Generate Key...'}
							/>
						</div>
					</div>
				</div>

				<!-- Metadata section for existing keys -->
				{#if isEditing && apiKey}
					<EntityMetadataSection entities={[apiKey]} />
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
							{deleting ? 'Deleting...' : 'Delete'}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={handleOnClose}
						class="btn-secondary"
					>
						Close
					</button>
					{#if isEditing}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? 'Saving...' : 'Save'}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
