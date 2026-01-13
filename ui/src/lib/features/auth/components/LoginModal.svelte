<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, email } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { LoginRequest } from '../types/base';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import * as m from '$lib/paraglide/messages';

	interface Props {
		orgName?: string | null;
		invitedBy?: string | null;
		demoMode?: boolean;
		isOpen?: boolean;
		onLogin: (data: LoginRequest) => Promise<void> | void;
		onClose: () => void;
		onSwitchToRegister?: (() => void) | null;
		onSwitchToForgot?: (() => void) | null;
	}

	let {
		orgName = null,
		invitedBy = null,
		demoMode = false,
		isOpen = false,
		onLogin,
		onClose,
		onSwitchToRegister = null,
		onSwitchToForgot = null
	}: Props = $props();

	let signingIn = $state(false);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let disableRegistration = $derived(configData?.disable_registration ?? false);
	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);
	let enablePasswordReset = $derived(configData?.has_email_service ?? false);

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			email: '',
			password: ''
		},
		onSubmit: async ({ value }) => {
			signingIn = true;
			try {
				await onLogin({
					email: value.email.trim(),
					password: value.password
				});
			} finally {
				signingIn = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ email: '', password: '' });
	}

	function handleOidcLogin(providerSlug: string) {
		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=login&return_url=${returnUrl}`;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title={m.auth_signInToScanopy()}
	size="md"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={m.auth_scanopyLogo()} class="h-8 w-8" />
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
			{#if demoMode}
				<div class="mb-6">
					<InlineInfo title={m.auth_demoModeTitle()} body={m.auth_demoModeBody()} />
					<div class="mt-3 rounded-md bg-gray-800 p-3 font-mono text-sm">
						<div class="text-secondary">
							<span class="text-gray-400">{m.auth_demoEmail()}</span>
							<span class="text-primary ml-2">demo@scanopy.net</span>
						</div>
						<div class="text-secondary mt-1">
							<span class="text-gray-400">{m.auth_demoPassword()}</span>
							<span class="text-primary ml-2">password123</span>
						</div>
					</div>
				</div>
			{:else if orgName && invitedBy}
				<div class="mb-6">
					<InlineInfo
						title={m.auth_youreInvitedTitle()}
						body={m.auth_youreInvitedBody({ orgName, invitedBy })}
					/>
				</div>
			{/if}

			<div class="space-y-6">
				<div class="space-y-4">
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
								placeholder={m.auth_enterYourEmail()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="password"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={m.common_password()}
								id="password"
								type="password"
								{field}
								placeholder={m.auth_enterYourPassword()}
								required
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				<!-- Sign In Button -->
				<button type="submit" disabled={signingIn} class="btn-primary w-full">
					{signingIn ? m.auth_signingIn() : m.auth_signInWithEmail()}
				</button>

				<!-- OIDC Providers -->
				{#if hasOidcProviders && !demoMode}
					<div class="relative">
						<div class="absolute inset-0 flex items-center">
							<div class="w-full border-t border-gray-600"></div>
						</div>
						<div class="relative flex justify-center text-sm">
							<span class="bg-gray-900 px-2 text-gray-400">{m.common_or()}</span>
						</div>
					</div>

					<div class="space-y-2">
						{#each oidcProviders as provider (provider.slug)}
							<button
								type="button"
								onclick={() => handleOidcLogin(provider.slug)}
								class="btn-secondary flex w-full items-center justify-center gap-3"
							>
								{#if provider.logo}
									<img src={provider.logo} alt={provider.name} class="h-5 w-5" />
								{/if}
								{m.auth_signInWith({ provider: provider.name })}
							</button>
						{/each}
					</div>
				{/if}

				<!-- Register Link -->
				{#if onSwitchToRegister && !disableRegistration && !demoMode}
					<div class="text-center">
						<p class="text-sm text-gray-400">
							{m.auth_dontHaveAccount()}
							<button
								type="button"
								onclick={onSwitchToRegister}
								class="font-medium text-blue-400 hover:text-blue-300"
							>
								{m.auth_registerHere()}
							</button>
						</p>
					</div>
				{/if}

				{#if enablePasswordReset && !demoMode}
					<div class="text-center">
						<p class="text-sm text-gray-400">
							{m.auth_forgotYourPassword()}
							<button
								type="button"
								onclick={onSwitchToForgot}
								class="font-medium text-blue-400 hover:text-blue-300"
							>
								{m.auth_resetPassword()}
							</button>
						</p>
					</div>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
