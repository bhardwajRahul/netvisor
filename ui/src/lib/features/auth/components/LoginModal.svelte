<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, email } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { LoginRequest } from '../types/base';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';

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
	title="Sign in to Scanopy"
	size="lg"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt="Scanopy Logo" class="h-8 w-8" />
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
					<InlineInfo
						title="Demo Mode"
						body="You're about to log in to a demo account. Use the credentials below to explore Scanopy. Note that some actions are disabled in demo mode."
					/>
					<div class="mt-3 rounded-md bg-gray-800 p-3 font-mono text-sm">
						<div class="text-secondary">
							<span class="text-gray-400">Email:</span>
							<span class="text-primary ml-2">demo@scanopy.net</span>
						</div>
						<div class="text-secondary mt-1">
							<span class="text-gray-400">Password:</span>
							<span class="text-primary ml-2">password123</span>
						</div>
					</div>
				</div>
			{:else if orgName && invitedBy}
				<div class="mb-6">
					<InlineInfo
						title="You're invited!"
						body={`You have been invited to join ${orgName} by ${invitedBy}. Please sign in or register to continue.`}
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
							<TextInput label="Email" id="email" {field} placeholder="Enter your email" required />
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
								label="Password"
								id="password"
								type="password"
								{field}
								placeholder="Enter your password"
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
					{signingIn ? 'Signing in...' : 'Sign In with Email'}
				</button>

				<!-- OIDC Providers -->
				{#if hasOidcProviders && !demoMode}
					<div class="relative">
						<div class="absolute inset-0 flex items-center">
							<div class="w-full border-t border-gray-600"></div>
						</div>
						<div class="relative flex justify-center text-sm">
							<span class="bg-gray-900 px-2 text-gray-400">or</span>
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
								Sign in with {provider.name}
							</button>
						{/each}
					</div>
				{/if}

				<!-- Register Link -->
				{#if onSwitchToRegister && !disableRegistration && !demoMode}
					<div class="text-center">
						<p class="text-sm text-gray-400">
							Don't have an account?
							<button
								type="button"
								onclick={onSwitchToRegister}
								class="font-medium text-blue-400 hover:text-blue-300"
							>
								Register here
							</button>
						</p>
					</div>
				{/if}

				{#if enablePasswordReset && !demoMode}
					<div class="text-center">
						<p class="text-sm text-gray-400">
							Forgot your password?
							<button
								type="button"
								onclick={onSwitchToForgot}
								class="font-medium text-blue-400 hover:text-blue-300"
							>
								Reset password
							</button>
						</p>
					</div>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
