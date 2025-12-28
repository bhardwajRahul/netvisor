<script lang="ts">
	import { useCurrentUserQuery, useLogoutMutation } from '$lib/features/auth/queries';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { apiClient } from '$lib/api/client';
	import type { User } from '$lib/features/users/types';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { Link, Key, LogOut, User as UserIcon } from 'lucide-svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		email,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';

	let {
		isOpen = false,
		onClose
	}: {
		isOpen?: boolean;
		onClose: () => void;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	const logoutMutation = useLogoutMutation();
	const queryClient = useQueryClient();
	const organizationQuery = useOrganizationQuery();

	let user = $derived(currentUserQuery.data);
	let organization = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);

	let activeSection = $state<'main' | 'credentials'>('main');
	let linkingProviderSlug: string | null = $state(null);
	let savingCredentials = $state(false);

	// Create form for credentials section
	const form = createForm(() => ({
		defaultValues: { email: '', password: '', confirmPassword: '' },
		onSubmit: async ({ value }) => {
			savingCredentials = true;
			try {
				const updateRequest: { email?: string; password?: string } = {};

				if (value.email !== user?.email) {
					updateRequest.email = value.email;
				}

				if (value.password) {
					updateRequest.password = value.password;
				}

				if (Object.keys(updateRequest).length === 0) {
					pushError('No changes to save');
					return;
				}

				const { data } = await apiClient.POST('/api/auth/update', {
					body: updateRequest
				});

				if (data?.success && data.data) {
					queryClient.setQueryData<User>(queryKeys.auth.currentUser(), data.data);
					pushSuccess('Credentials updated successfully');
					activeSection = 'main';
				} else {
					pushError(data?.error || 'Failed to update credentials');
				}
			} finally {
				savingCredentials = false;
			}
		}
	}));

	// Reset to main view when modal opens
	function handleOpen() {
		activeSection = 'main';
		linkingProviderSlug = null;
		form.reset({ email: user?.email || '', password: '', confirmPassword: '' });
	}

	// Find which provider (if any) is linked to this user
	let linkedProvider = $derived(
		user?.oidc_provider ? oidcProviders.find((p) => p.slug === user.oidc_provider) : null
	);

	function linkOidcAccount(providerSlug: string) {
		linkingProviderSlug = providerSlug;
		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=link&return_url=${returnUrl}`;
	}

	async function unlinkOidcAccount(providerSlug: string) {
		const { data } = await apiClient.POST('/api/auth/oidc/{slug}/unlink', {
			params: { path: { slug: providerSlug } }
		});

		if (data?.success && data.data) {
			queryClient.setQueryData<User>(queryKeys.auth.currentUser(), data.data);
			pushSuccess('OIDC account unlinked successfully');
		} else {
			pushError(data?.error || 'Failed to unlink OIDC account');
		}
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleCancel() {
		if (activeSection === 'credentials') {
			activeSection = 'main';
			form.reset({ email: user?.email || '', password: '', confirmPassword: '' });
		} else {
			onClose();
		}
	}

	async function handleLogout() {
		try {
			await logoutMutation.mutateAsync();
			window.location.reload();
			onClose();
		} catch {
			// Error handled by mutation
		}
	}

	let hasLinkedOidc = $derived(!!user?.oidc_provider);
	let modalTitle = $derived(activeSection === 'main' ? 'Account Settings' : 'Update Credentials');
	let showSave = $derived(activeSection === 'credentials');
	let cancelLabel = $derived(activeSection === 'main' ? 'Close' : 'Back');
</script>

<GenericModal
	{isOpen}
	title={modalTitle}
	size="md"
	onClose={handleCancel}
	onOpen={handleOpen}
	showCloseButton={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={activeSection === 'main' ? UserIcon : Key} color="Blue" />
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (showSave) handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			{#if activeSection === 'main'}
				{#if user}
					<div class="space-y-6">
						<!-- User Info -->
						<InfoCard title="User Information">
							<InfoRow label="Organization">{organization?.name}</InfoRow>
							<InfoRow label="Email">{user.email}</InfoRow>
							<InfoRow label="Permissions" mono={true}>{user.permissions}</InfoRow>
							<InfoRow label="User ID" mono={true}>{user.id}</InfoRow>
						</InfoCard>

						<!-- Authentication Methods -->
						<div>
							<h3 class="text-primary mb-3 text-sm font-semibold">Authentication Methods</h3>
							<div class="space-y-3">
								<!-- Email & Password -->
								<InfoCard variant="compact">
									<div class="flex items-center justify-between">
										<div class="flex items-center gap-4">
											<Key class="text-secondary h-5 w-5 flex-shrink-0" />
											<div>
												<p class="text-primary text-sm font-medium">Email & Password</p>
												<p class="text-secondary text-xs">Update email and password</p>
											</div>
										</div>
										<button
											type="button"
											onclick={() => {
												activeSection = 'credentials';
												form.reset({ email: user.email, password: '', confirmPassword: '' });
											}}
											class="btn-primary"
										>
											Update
										</button>
									</div>
								</InfoCard>

								<!-- OIDC Providers -->
								{#if hasOidcProviders}
									<div class="space-y-3">
										<p class="text-secondary text-xs">
											Link your account with an identity provider for faster sign-in. You can only
											link one provider at a time.
										</p>

										{#each oidcProviders as provider (provider.slug)}
											{@const isLinked = hasLinkedOidc && user.oidc_provider === provider.slug}
											{@const isDisabled = hasLinkedOidc && !isLinked}
											<InfoCard variant="compact">
												<div class="flex items-center justify-between">
													<div class="mr-2 flex items-center gap-4">
														{#if provider.logo}
															<img src={provider.logo} alt={provider.name} class="h-5 w-5" />
														{:else}
															<Link class="text-secondary h-5 w-5 flex-shrink-0" />
														{/if}
														<div>
															<p class="text-primary text-sm font-medium">{provider.name}</p>
															{#if isLinked}
																<p class="text-secondary text-xs">
																	Linked on {new Date(
																		user.oidc_linked_at || ''
																	).toLocaleDateString()}
																</p>
															{:else if isDisabled}
																<p class="text-secondary text-xs">
																	Unlink {linkedProvider?.name} first to link this provider
																</p>
															{:else}
																<p class="text-secondary text-xs">Not linked</p>
															{/if}
														</div>
													</div>
													{#if isLinked}
														<button
															type="button"
															onclick={() => unlinkOidcAccount(provider.slug)}
															class="btn-danger"
														>
															Unlink
														</button>
													{:else if !hasLinkedOidc}
														<button
															type="button"
															onclick={() => linkOidcAccount(provider.slug)}
															disabled={(linkingProviderSlug &&
																linkingProviderSlug != provider.slug) ||
																isDisabled}
															class={isDisabled ? 'btn-disabled' : 'btn-primary'}
														>
															{linkingProviderSlug == provider.slug ? 'Redirecting...' : 'Link'}
														</button>
													{:else}
														<button type="button" disabled={isDisabled} class="btn-primary">
															Link
														</button>
													{/if}
												</div>
											</InfoCard>
										{/each}
									</div>
								{/if}
							</div>
						</div>

						<!-- Logout -->
						<InfoCard variant="compact">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-4">
									<LogOut class="text-secondary h-5 w-5" />
									<span class="text-primary text-sm">Sign out of your account</span>
								</div>
								<button type="button" onclick={handleLogout} class="btn-secondary"> Logout </button>
							</div>
						</InfoCard>
					</div>
				{:else}
					<div class="text-secondary py-8 text-center">Loading user information...</div>
				{/if}
			{:else if activeSection === 'credentials'}
				<div class="space-y-2">
					<p class="text-secondary mb-2 text-sm">Update your email address and/or password</p>
					<div class="space-y-6">
						<form.Field
							name="email"
							validators={{
								onBlur: ({ value }) => required(value) || email(value)
							}}
						>
							{#snippet children(field)}
								<TextInput label="Email" id="email" {field} placeholder="Enter email" />
							{/snippet}
						</form.Field>

						<form.Field
							name="password"
							validators={{
								onBlur: ({ value }) => passwordValidator(value)
							}}
						>
							{#snippet children(passwordField)}
								<form.Field
									name="confirmPassword"
									validators={{
										onBlur: ({ value, fieldApi }) =>
											confirmPasswordMatch(() => fieldApi.form.getFieldValue('password'))(value)
									}}
								>
									{#snippet children(confirmPasswordField)}
										<Password {passwordField} {confirmPasswordField} required={false} />
									{/snippet}
								</form.Field>
							{/snippet}
						</form.Field>
					</div>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={handleCancel} class="btn-secondary">
					{cancelLabel}
				</button>
				{#if showSave}
					<button type="submit" disabled={savingCredentials} class="btn-primary">
						{savingCredentials ? 'Saving...' : 'Save Changes'}
					</button>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
