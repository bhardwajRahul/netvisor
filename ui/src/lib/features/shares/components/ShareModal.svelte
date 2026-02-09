<script lang="ts">
	import { tick } from 'svelte';
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
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import { generateShareUrl, generateEmbedCode } from '../queries';
	import {
		common_cancel,
		common_create,
		common_delete,
		common_deleting,
		common_done,
		common_editName,
		common_enabled,
		common_failedToDelete,
		common_failedToSave,
		common_height,
		common_name,
		common_password,
		common_save,
		common_saving,
		common_width,
		shares_accessControl,
		shares_allowedDomainsHelp,
		shares_allowedDomainsPlaceholder,
		shares_allowedEmbedDomains,
		shares_cacheInfoBody,
		shares_cacheInfoTitle,
		shares_created,
		shares_createdHelp,
		shares_displayOptions,
		shares_embedCode,
		shares_embedDimensions,
		shares_embedsRequirePlan,
		shares_enabledHelp,
		shares_expirationDate,
		shares_expirationHelp,
		shares_namePlaceholder,
		shares_passwordHelpCreate,
		shares_passwordHelpEdit,
		shares_passwordPlaceholder,
		shares_shareTopology,
		shares_shareUrl,
		shares_showExportButton,
		shares_showInspectPanel,
		shares_showZoomControls,
		shares_upgradeForEmbeds
	} from '$lib/paraglide/messages';

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
	let scrollContainer: HTMLDivElement | null = $state(null);

	// Scroll to bottom when share is created to show the share URL
	$effect(() => {
		if (createdShare && scrollContainer) {
			// Use tick to ensure DOM has updated with the new share URL section
			tick().then(() => {
				scrollContainer?.scrollTo({ top: scrollContainer.scrollHeight, behavior: 'smooth' });
			});
		}
	});

	let isEditing = $derived(share !== null);
	let title = $derived(
		isEditing ? common_editName({ name: share?.name || '' }) : shares_shareTopology()
	);
	let saveLabel = $derived(isEditing ? common_save() : common_create());

	let hasShareViews = $derived(
		organization?.plan
			? billingPlans.getMetadata(organization.plan.type).features.share_views
			: true
	);

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
				pushError(error instanceof Error ? error.message : common_failedToSave());
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
			pushError(error instanceof Error ? error.message : common_failedToDelete());
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
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Share2} color={entities.getColorHelper('Share').color} />
	{/snippet}

	{#if !hasShareViews && !isEditing}
		<div class="flex min-h-0 flex-1 flex-col items-center justify-center p-6">
			<p class="text-secondary mb-2 text-lg">Sharing Not Available</p>
			<p class="text-tertiary mb-6 text-center">
				Upgrade your plan to share live network diagrams with others.
			</p>
			<UpgradeButton feature="sharing" />
		</div>
		<div class="modal-footer">
			<div class="flex items-center justify-end">
				<button type="button" onclick={handleClose} class="btn-secondary">
					{common_cancel()}
				</button>
			</div>
		</div>
	{:else}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				e.stopPropagation();
				if (!createdShare) handleSubmit();
			}}
			class="flex min-h-0 flex-1 flex-col"
		>
			<div class="flex-1 overflow-auto p-6" bind:this={scrollContainer}>
				<div class="space-y-6">
					{#if isEditing}
						<InlineInfo
							title={shares_cacheInfoTitle()}
							body={shares_cacheInfoBody()}
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
									label={common_name()}
									id="share-name"
									{field}
									placeholder={shares_namePlaceholder()}
									required
									disabled={!!createdShare}
								/>
							{/snippet}
						</form.Field>
					</div>

					<div class="card card-static space-y-3">
						<span class="text-secondary text-m">{shares_accessControl()}</span>

						<!-- Password -->
						<form.Field name="password">
							{#snippet children(field)}
								<TextInput
									label={common_password()}
									id="share-password"
									type="password"
									{field}
									placeholder={shares_passwordPlaceholder()}
									disabled={!!createdShare}
									helpText={isEditing ? shares_passwordHelpEdit() : shares_passwordHelpCreate()}
								/>
							{/snippet}
						</form.Field>

						<!-- Enabled & Expiration -->
						<div class="grid grid-cols-2 gap-4">
							<form.Field name="expires_at">
								{#snippet children(field)}
									<DateInput
										{field}
										label={shares_expirationDate()}
										id="expires-at"
										disabled={!!createdShare}
										helpText={shares_expirationHelp()}
									/>
								{/snippet}
							</form.Field>
							<div class="flex items-center">
								<form.Field name="is_enabled">
									{#snippet children(field)}
										<Checkbox
											label={common_enabled()}
											id="is-enabled"
											{field}
											disabled={!!createdShare}
											helpText={shares_enabledHelp()}
										/>
									{/snippet}
								</form.Field>
							</div>
						</div>

						<!-- Allowed Domains -->
						<form.Field name="allowed_domains">
							{#snippet children(field)}
								<TextInput
									label={shares_allowedEmbedDomains()}
									id="allowed-domains"
									{field}
									placeholder={shares_allowedDomainsPlaceholder()}
									disabled={!!createdShare}
									helpText={shares_allowedDomainsHelp()}
								/>
							{/snippet}
						</form.Field>
					</div>

					<div class="card card-static space-y-3">
						<span class="text-secondary text-m">{shares_displayOptions()}</span>
						<form.Field name="show_zoom_controls">
							{#snippet children(field)}
								<Checkbox
									label={shares_showZoomControls()}
									id="show-zoom-controls"
									{field}
									disabled={!!createdShare}
								/>
							{/snippet}
						</form.Field>
						<form.Field name="show_inspect_panel">
							{#snippet children(field)}
								<Checkbox
									label={shares_showInspectPanel()}
									id="show-inspect-panel"
									{field}
									disabled={!!createdShare}
								/>
							{/snippet}
						</form.Field>
						<form.Field name="show_export_button">
							{#snippet children(field)}
								<Checkbox
									label={shares_showExportButton()}
									id="show-export-button"
									{field}
									disabled={!!createdShare}
								/>
							{/snippet}
						</form.Field>
						<span class="block text-sm font-medium text-gray-300">{shares_embedDimensions()}</span>
						<div class="grid grid-cols-2 gap-4">
							<form.Field name="embed_width">
								{#snippet children(field)}
									<TextInput
										label={common_width()}
										id="embed-width"
										type="number"
										{field}
										placeholder="800"
									/>
								{/snippet}
							</form.Field>
							<form.Field name="embed_height">
								{#snippet children(field)}
									<TextInput
										label={common_height()}
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
								<InlineSuccess title={shares_created()} body={shares_createdHelp()} />
							{/if}
							<div>
								<span class="mb-1 block text-sm font-medium text-gray-300">{shares_shareUrl()}</span
								>
								<CodeContainer
									language="bash"
									expandable={false}
									code={generateShareUrl(shareId)}
								/>
							</div>
							<div class="space-y-2">
								<span class="mb-1 block text-sm font-medium text-gray-300"
									>{shares_embedCode()}</span
								>
								{#if !hasEmbedsFeature}
									<InlineInfo title={shares_embedsRequirePlan()} body={shares_upgradeForEmbeds()} />
									<div class="mt-2">
										<UpgradeButton feature="embeds" />
									</div>
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
								{deleting ? common_deleting() : common_delete()}
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
							{createdShare ? common_done() : common_cancel()}
						</button>
						{#if !createdShare}
							<button type="submit" disabled={loading || deleting} class="btn-primary">
								{loading ? common_saving() : saveLabel}
							</button>
						{/if}
					</div>
				</div>
			</div>
		</form>
	{/if}
</GenericModal>
