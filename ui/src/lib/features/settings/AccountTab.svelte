<script lang="ts">
	import { useCurrentUserQuery, useLogoutMutation } from '$lib/features/auth/queries';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { apiClient } from '$lib/api/client';
	import type { User } from '$lib/features/users/types';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { Link, Key, LogOut } from 'lucide-svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		email,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import * as m from '$lib/paraglide/messages';

	let {
		subView = $bindable<'main' | 'credentials'>('main'),
		onClose
	}: {
		subView?: 'main' | 'credentials';
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
					pushError(m.settings_account_noChanges());
					return;
				}

				const { data } = await apiClient.POST('/api/auth/update', {
					body: updateRequest
				});

				if (data?.success && data.data) {
					queryClient.setQueryData<User>(queryKeys.auth.currentUser(), data.data);
					pushSuccess(m.settings_account_credentialsUpdated());
					subView = 'main';
				} else {
					pushError(data?.error || 'Failed to update credentials');
				}
			} finally {
				savingCredentials = false;
			}
		}
	}));

	// Reset form when switching to credentials view
	export function resetForm() {
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
			pushSuccess(m.settings_account_oidcUnlinked());
		} else {
			pushError(data?.error || m.settings_account_failedToUnlink());
		}
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleCancel() {
		if (subView === 'credentials') {
			subView = 'main';
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
	let showSave = $derived(subView === 'credentials');
	let cancelLabel = $derived(subView === 'main' ? m.common_close() : m.common_back());
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		e.stopPropagation();
		if (showSave) handleSubmit();
	}}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="flex-1 overflow-auto p-6">
		{#if subView === 'main'}
			{#if user}
				<div class="space-y-6">
					<!-- User Info -->
					<InfoCard title={m.settings_account_userInfo()}>
						<InfoRow label={m.settings_account_organization()}>{organization?.name}</InfoRow>
						<InfoRow label={m.common_email()}>{user.email}</InfoRow>
						<InfoRow label={m.settings_account_permissions()} mono={true}
							>{user.permissions}</InfoRow
						>
						<InfoRow label={m.settings_account_userId()} mono={true}>{user.id}</InfoRow>
					</InfoCard>

					<!-- Authentication Methods -->
					<div>
						<h3 class="text-primary mb-3 text-sm font-semibold">
							{m.settings_account_authMethods()}
						</h3>
						<div class="space-y-3">
							<!-- Email & Password -->
							<InfoCard variant="compact">
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-4">
										<Key class="text-secondary h-5 w-5 flex-shrink-0" />
										<div>
											<p class="text-primary text-sm font-medium">
												{m.settings_account_emailPassword()}
											</p>
											<p class="text-secondary text-xs">
												{m.settings_account_updateEmailPassword()}
											</p>
										</div>
									</div>
									<button
										type="button"
										onclick={() => {
											subView = 'credentials';
											form.reset({ email: user.email, password: '', confirmPassword: '' });
										}}
										class="btn-primary"
									>
										{m.settings_account_update()}
									</button>
								</div>
							</InfoCard>

							<!-- OIDC Providers -->
							{#if hasOidcProviders}
								<div class="space-y-3">
									<p class="text-secondary text-xs">
										{m.settings_account_oidcLinkHelp()}
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
																{m.settings_account_linkedOn({
																	date: new Date(user.oidc_linked_at || '').toLocaleDateString()
																})}
															</p>
														{:else if isDisabled}
															<p class="text-secondary text-xs">
																{m.settings_account_unlinkFirst({
																	provider: linkedProvider?.name || ''
																})}
															</p>
														{:else}
															<p class="text-secondary text-xs">{m.settings_account_notLinked()}</p>
														{/if}
													</div>
												</div>
												{#if isLinked}
													<button
														type="button"
														onclick={() => unlinkOidcAccount(provider.slug)}
														class="btn-danger"
													>
														{m.settings_account_unlink()}
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
														{linkingProviderSlug == provider.slug
															? m.settings_account_redirecting()
															: m.settings_account_link()}
													</button>
												{:else}
													<button type="button" disabled={isDisabled} class="btn-primary">
														{m.settings_account_link()}
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
								<span class="text-primary text-sm">{m.settings_account_signOut()}</span>
							</div>
							<button type="button" onclick={handleLogout} class="btn-secondary">
								{m.settings_account_logout()}
							</button>
						</div>
					</InfoCard>
				</div>
			{:else}
				<div class="text-secondary py-8 text-center">{m.settings_account_loadingUser()}</div>
			{/if}
		{:else if subView === 'credentials'}
			<div class="space-y-2">
				<p class="text-secondary mb-2 text-sm">{m.settings_account_updateCredentials()}</p>
				<div class="space-y-6">
					<form.Field
						name="email"
						validators={{
							onBlur: ({ value }) => required(value) || email(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={m.common_email()}
								id="email"
								{field}
								placeholder={m.settings_account_enterEmail()}
							/>
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
					{savingCredentials ? m.settings_account_saving() : m.settings_account_saveChanges()}
				</button>
			{/if}
		</div>
	</div>
</form>
