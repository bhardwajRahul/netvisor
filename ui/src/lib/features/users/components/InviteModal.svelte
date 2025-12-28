<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import {
		UserPlus,
		Copy,
		Check,
		Calendar,
		Link as LinkIcon,
		RotateCcw,
		Send
	} from 'lucide-svelte';
	import { pushSuccess, pushError } from '$lib/shared/stores/feedback';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { OrganizationInvite } from '$lib/features/organizations/types';
	import { formatInviteUrl, useCreateInviteMutation } from '$lib/features/organizations/queries';
	import type { UserOrgPermissions } from '../types';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import { email } from '$lib/shared/components/forms/validators';
	import { permissions, metadata, entities } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import type { Network } from '$lib/features/networks/types';

	let { isOpen = $bindable(false), onClose }: { isOpen: boolean; onClose: () => void } = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Mutation for creating invite
	const createInviteMutation = useCreateInviteMutation();

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let enableEmail = $derived(configData?.has_email_service ?? false);

	// Force Svelte to track reactivity
	$effect(() => {
		void $metadata;
		void currentUser;
	});

	let copied = $state(false);
	let copyTimeoutId = $state<ReturnType<typeof setTimeout> | null>(null);
	let generatingInvite = $derived(createInviteMutation.isPending);
	let invite = $state<OrganizationInvite | null>(null);

	const networksNotNeeded: string[] = permissions
		.getItems()
		.filter((p) => p.metadata.manage_org_entities)
		.map((p) => p.id);

	// Make permission options reactive to metadata and currentUser changes
	let permissionOptions = $derived(
		permissions
			.getItems()
			.filter((p) =>
				currentUser
					? permissions
							.getMetadata(currentUser.permissions)
							.can_manage_user_permissions.includes(p.id)
					: false
			)
			.map((p) => ({ value: p.id, label: p.name ?? '', description: p.description ?? '' }))
	);

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			permissions: 'Viewer' as UserOrgPermissions,
			email: ''
		},
		onSubmit: async () => {
			// Not used - we handle submission with handleGenerateInvite
		}
	}));

	let permissionsValue = $derived(form.state.values.permissions);
	let emailValue = $derived(form.state.values.email);
	let emailValid = $derived(!emailValue || !email(emailValue));

	let usingEmail = $derived(enableEmail && emailValue && emailValid);
	let ctaText = $derived(usingEmail ? 'Send Invite Link' : 'Generate Invite Link');
	let ctaLoadingText = $derived(usingEmail ? 'Sending...' : 'Generating...');
	let CtaIcon = $derived(usingEmail ? Send : RotateCcw);

	let selectedNetworks: Network[] = $state([]);

	let networkOptions = $derived(
		networksData
			.filter((n) => {
				if (currentUser) {
					return networksNotNeeded.includes(currentUser.permissions)
						? true
						: currentUser.network_ids.includes(n.id);
				}
				return false;
			})
			.filter((n) => !selectedNetworks.map((net) => net.id).includes(n.id) && n != undefined)
	);

	function handleAddNetwork(id: string) {
		const network = networksData.find((n) => n.id == id);
		if (network) {
			selectedNetworks.push(network);
			selectedNetworks = [...selectedNetworks];
		}
	}

	function handleRemoveNetwork(index: number) {
		selectedNetworks.splice(index, 1);
		selectedNetworks = [...selectedNetworks];
	}

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ permissions: 'Viewer', email: '' });
		selectedNetworks = [];
		invite = null;
	}

	function handleClose() {
		invite = null;
		onClose();
	}

	async function handleGenerateInvite() {
		try {
			// Read values directly from form state to ensure we get current values
			const currentPermissions = form.state.values.permissions;
			const currentEmail = form.state.values.email;

			const result = await createInviteMutation.mutateAsync({
				permissions: currentPermissions,
				network_ids: selectedNetworks.map((n) => n.id),
				email: currentEmail
			});
			invite = result;
			pushSuccess(`Invite ${currentEmail ? 'sent' : 'generated'} successfully`);
		} catch (err) {
			pushError(`Failed to ${form.state.values.email ? 'send' : 'generate'} invite: ${err}`);
		}
	}

	const isSecureContext =
		window.isSecureContext ||
		window.location.hostname === 'localhost' ||
		window.location.hostname === '127.0.0.1';

	async function handleCopy() {
		if (!invite) return;

		try {
			await navigator.clipboard.writeText(formatInviteUrl(invite));
			copied = true;
			pushSuccess('Invite link copied to clipboard');

			// Reset copied state after 2 seconds
			if (copyTimeoutId) {
				clearTimeout(copyTimeoutId);
			}
			copyTimeoutId = setTimeout(() => {
				copied = false;
			}, 2000);
		} catch (err) {
			pushError('Failed to copy link to clipboard');
			console.error('Failed to copy:', err);
		}
	}

	// Cleanup timeout on component destroy
	$effect(() => {
		if (!isOpen && copyTimeoutId) {
			clearTimeout(copyTimeoutId);
			copyTimeoutId = null;
			copied = false;
		}
	});
</script>

<GenericModal
	{isOpen}
	title="Invite User"
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={UserPlus} color={entities.getColorHelper('User').color} />
	</svelte:fragment>

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary text-sm">
					Select the permissions level for the new user, then generate an invite link. They can use
					it to register or join your organization.
				</p>

				<!-- Permissions Selection -->
				<form.Field name="permissions">
					{#snippet children(field)}
						<SelectInput
							label="Permissions Level"
							id="permissions"
							{field}
							options={permissionOptions}
							disabled={!!invite}
							helpText="Choose the access level for the invited user"
						/>
					{/snippet}
				</form.Field>

				{#if !networksNotNeeded.includes(permissionsValue)}
					<ListManager
						label="Networks"
						helpText="Select networks this user will have access to"
						required={true}
						allowReorder={false}
						allowAddFromOptions={true}
						allowCreateNew={false}
						allowItemEdit={() => false}
						disableCreateNewButton={false}
						onAdd={handleAddNetwork}
						onRemove={handleRemoveNetwork}
						options={networkOptions}
						optionDisplayComponent={NetworkDisplay}
						items={selectedNetworks}
						itemDisplayComponent={NetworkDisplay}
					/>
				{:else}
					<div class="card card-static">
						<p class="text-secondary text-sm">
							Users with {permissionsValue} permissions have access to all networks.
						</p>
					</div>
				{/if}

				{#if enableEmail}
					<form.Field name="email" validators={{ onBlur: ({ value }) => email(value) }}>
						{#snippet children(field)}
							<TextInput
								label="Email"
								id="email"
								placeholder="Enter email address..."
								helpText="Enter the email you would like to send this invite to, or leave empty to just generate a link"
								{field}
							/>
						{/snippet}
					</form.Field>
				{/if}

				<!-- Generate Invite Button (shown when no invite exists) -->
				{#if !invite}
					<button
						onclick={handleGenerateInvite}
						type="button"
						disabled={generatingInvite || !emailValid}
						class="btn-primary w-full"
					>
						<CtaIcon class="mr-2 h-4 w-4" />
						{generatingInvite ? ctaLoadingText : ctaText}
					</button>
				{/if}

				<!-- Invite URL Card (shown when invite exists) -->
				{#if invite}
					<div class="card card-static">
						<div class="space-y-3">
							<div class="flex items-center gap-2">
								<LinkIcon class="text-secondary h-4 w-4 flex-shrink-0" />
								<h3 class="text-primary text-sm font-semibold">Invite Link</h3>
							</div>

							<!-- URL Display -->
							<div class="rounded-md border border-gray-600 bg-gray-800/50 p-3">
								<code class="text-primary block break-all text-sm">{formatInviteUrl(invite)}</code>
							</div>

							<!-- Copy Button -->
							{#if isSecureContext}
								<button
									onclick={handleCopy}
									type="button"
									class="btn-primary w-full"
									disabled={copied}
								>
									{#if copied}
										<Check class="mr-2 h-4 w-4" />
										Copied!
									{:else}
										<Copy class="mr-2 h-4 w-4" />
										Copy Link
									{/if}
								</button>
							{/if}
						</div>
					</div>

					<!-- Expiration Info -->
					<div class="card card-static">
						<div class="flex items-center gap-3">
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg bg-blue-500/10"
							>
								<Calendar class="h-5 w-5 text-blue-400" />
							</div>
							<div class="flex-1">
								<p class="text-primary text-sm font-medium">
									{'Expires ' + formatTimestamp(invite.expires_at)}
								</p>
							</div>
						</div>
					</div>

					<InlineWarning
						title="Sensitive Link"
						body="Anyone with this link can join your organization with {permissionsValue} permissions. Keep it secure and only share it with people you trust."
					/>
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={handleClose} class="btn-secondary"> Close </button>
			</div>
		</div>
	</div>
</GenericModal>
