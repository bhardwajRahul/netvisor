<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { Share2 } from 'lucide-svelte';
	import type { Share } from '../types/base';
	import { createEmptyShare } from '../types/base';
	import {
		useCreateShareMutation,
		useUpdateShareMutation,
		useDeleteShareMutation
	} from '../queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans, entities } from '$lib/shared/stores/metadata';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import { generateShareUrl, generateEmbedCode } from '../queries';

	let {
		isOpen = false,
		onClose,
		share = null,
		topologyId = '',
		networkId = ''
	}: {
		isOpen?: boolean;
		onClose: () => void;
		share?: Share | null;
		topologyId?: string;
		networkId?: string;
	} = $props();

	// Mutations
	const createShareMutation = useCreateShareMutation();
	const updateShareMutation = useUpdateShareMutation();
	const deleteShareMutation = useDeleteShareMutation();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);
	let createdShare = $state<Share | null>(null);

	let isEditing = $derived(share !== null);
	let title = $derived(isEditing ? `Edit ${share?.name || 'Share'}` : 'Share Topology');
	let saveLabel = $derived(isEditing ? 'Save' : 'Create');

	let hasEmbedsFeature = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features.embeds : true
	);

	function getDefaultValues() {
		const s = share ? { ...share } : createEmptyShare(topologyId, networkId);
		return {
			name: s.name || '',
			password: '',
			allowed_domains: s.allowed_domains?.join(', ') || '',
			expires_at: s.expires_at || '',
			is_enabled: s.is_enabled ?? true,
			show_zoom_controls: s.options?.show_zoom_controls ?? true,
			show_inspect_panel: s.options?.show_inspect_panel ?? true,
			show_export_button: s.options?.show_export_button ?? true,
			embed_width: '800',
			embed_height: '600',
			// Preserve other share fields
			id: s.id,
			topology_id: s.topology_id,
			network_id: s.network_id,
			created_by: s.created_by
		};
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const formData = {
				id: value.id,
				name: value.name.trim(),
				topology_id: value.topology_id,
				network_id: value.network_id,
				created_by: currentUser?.id || value.created_by,
				allowed_domains: value.allowed_domains.trim()
					? value.allowed_domains
							.split(',')
							.map((d: string) => d.trim())
							.filter(Boolean)
					: null,
				expires_at: value.expires_at || null,
				is_enabled: value.is_enabled,
				options: {
					show_zoom_controls: value.show_zoom_controls,
					show_inspect_panel: value.show_inspect_panel,
					show_export_button: value.show_export_button
				}
			} as Share;

			loading = true;
			try {
				if (isEditing && share) {
					const password = value.password || undefined;
					await updateShareMutation.mutateAsync({
						id: share.id,
						request: { share: formData, password }
					});
					handleClose();
				} else {
					const result = await createShareMutation.mutateAsync({
						share: formData,
						password: value.password || undefined
					});
					createdShare = result;
				}
			} catch (error) {
				pushError(error instanceof Error ? error.message : 'Failed to save share');
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset(getDefaultValues());
		createdShare = null;
	}

	function handleClose() {
		createdShare = null;
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (!share) return;

		deleting = true;
		try {
			await deleteShareMutation.mutateAsync(share.id);
			handleClose();
		} catch (error) {
			pushError(error instanceof Error ? error.message : 'Failed to delete share');
		} finally {
			deleting = false;
		}
	}

	// For embed code display - use any to avoid type issues with dynamic form
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let formValues = $derived(form.state.values as any);
	let embedWidth = $derived(parseInt(String(formValues.embed_width)) || 800);
	let embedHeight = $derived(parseInt(String(formValues.embed_height)) || 600);
	let shareId: string = $derived(createdShare?.id ?? share?.id ?? '');
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={Share2} color={entities.getColorHelper('Share').color} />
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (!createdShare) handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				{#if isEditing}
					<InlineInfo
						title="Changes may take up to 5 minutes to appear"
						body="Share links and embeds are cached. Any updates you make won't be visible immediately."
						dismissableKey="share-cache-info"
					/>
				{/if}

				<!-- Name -->
				<div class="card card-static">
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Name"
								id="share-name"
								{field}
								placeholder="My shared topology"
								required
								disabled={!!createdShare}
							/>
						{/snippet}
					</form.Field>
				</div>

				<div class="card card-static space-y-3">
					<span class="text-secondary text-m">Access Control</span>

					<!-- Password -->
					<form.Field name="password">
						{#snippet children(field)}
							<TextInput
								label="Password"
								id="share-password"
								type="password"
								{field}
								placeholder="Enter password"
								disabled={!!createdShare}
								helpText={isEditing
									? 'Leave empty to keep the current password'
									: 'Leave empty to allow public access with no password'}
							/>
						{/snippet}
					</form.Field>

					<!-- Enabled & Expiration -->
					<div class="grid grid-cols-2 gap-4">
						<form.Field name="expires_at">
							{#snippet children(field)}
								<DateInput
									{field}
									label="Expiration Date"
									id="expires-at"
									disabled={!!createdShare}
									helpText="Leave empty to never expire"
								/>
							{/snippet}
						</form.Field>
						<div class="flex items-center">
							<form.Field name="is_enabled">
								{#snippet children(field)}
									<Checkbox
										label="Enabled"
										id="is-enabled"
										{field}
										disabled={!!createdShare}
										helpText="Disable to temporarily prevent access"
									/>
								{/snippet}
							</form.Field>
						</div>
					</div>

					<!-- Allowed Domains -->
					<form.Field name="allowed_domains">
						{#snippet children(field)}
							<TextInput
								label="Allowed Embed Domains"
								id="allowed-domains"
								{field}
								placeholder="example.com, *.mysite.com"
								disabled={!!createdShare}
								helpText="Restrict which domains can embed this share. Leave empty to allow all domains."
							/>
						{/snippet}
					</form.Field>
				</div>

				<div class="card card-static space-y-3">
					<span class="text-secondary text-m">Display Options</span>
					<form.Field name="show_zoom_controls">
						{#snippet children(field)}
							<Checkbox
								label="Show zoom controls"
								id="show-zoom-controls"
								{field}
								disabled={!!createdShare}
							/>
						{/snippet}
					</form.Field>
					<form.Field name="show_inspect_panel">
						{#snippet children(field)}
							<Checkbox
								label="Show inspect panel"
								id="show-inspect-panel"
								{field}
								disabled={!!createdShare}
							/>
						{/snippet}
					</form.Field>
					<form.Field name="show_export_button">
						{#snippet children(field)}
							<Checkbox
								label="Show export button"
								id="show-export-button"
								{field}
								disabled={!!createdShare}
							/>
						{/snippet}
					</form.Field>
					<span class="block text-sm font-medium text-gray-300">Embed Dimensions</span>
					<div class="grid grid-cols-2 gap-4">
						<form.Field name="embed_width">
							{#snippet children(field)}
								<TextInput label="Width" id="embed-width" type="number" {field} placeholder="800" />
							{/snippet}
						</form.Field>
						<form.Field name="embed_height">
							{#snippet children(field)}
								<TextInput
									label="Height"
									id="embed-height"
									type="number"
									{field}
									placeholder="600"
								/>
							{/snippet}
						</form.Field>
					</div>
				</div>

				<!-- Share URL / Embed Code (shown after creation or when editing) -->
				{#if createdShare || isEditing}
					<div class="space-y-4">
						{#if createdShare}
							<InlineSuccess
								title="Share created"
								body="To edit this share's settings, go to the Sharing tab."
							/>
						{/if}
						<div>
							<span class="mb-1 block text-sm font-medium text-gray-300">Share URL</span>
							<CodeContainer language="bash" expandable={false} code={generateShareUrl(shareId)} />
						</div>
						<div class="space-y-2">
							<span class="mb-1 block text-sm font-medium text-gray-300">Embed Code</span>
							{#if !hasEmbedsFeature}
								<InlineInfo
									title="Embeds require an upgraded plan"
									body="Upgrade your plan to embed this share on external websites."
								/>
							{:else}
								<CodeContainer
									language="html"
									expandable={false}
									code={generateEmbedCode(shareId, embedWidth, embedHeight)}
								/>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing}
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
						onclick={handleClose}
						class="btn-secondary"
					>
						{createdShare ? 'Done' : 'Cancel'}
					</button>
					{#if !createdShare}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? 'Saving...' : saveLabel}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
