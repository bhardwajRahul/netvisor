<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		email,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../stores/onboarding';
	import type { RegisterRequest } from '../types/base';

	let {
		orgName = null,
		invitedBy = null,
		isOpen = false,
		onRegister,
		onClose
	}: {
		orgName?: string | null;
		invitedBy?: string | null;
		isOpen?: boolean;
		onRegister: (data: RegisterRequest, subscribed: boolean) => Promise<void> | void;
		onClose: () => void;
	} = $props();

	let registering = $state(false);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);
	let enableEmailOptIn = $derived(configData?.has_email_opt_in ?? false);
	let enableTermsCheckbox = $derived(configData?.billing_enabled ?? false);

	// Get networks with daemon setups that will scan after registration
	let networksWithDaemons = $derived.by(() => {
		const networks = $onboardingStore.networks;
		const daemonSetups = $onboardingStore.daemonSetups;

		return networks.filter((n) => {
			if (!n.id) return false;
			const setup = daemonSetups.get(n.id);
			return setup?.installNow === true;
		});
	});

	let hasPendingDaemons = $derived(networksWithDaemons.length > 0);
	let pendingNetworkNames = $derived(networksWithDaemons.map((n) => n.name).join(', '));

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			email: '',
			password: '',
			confirmPassword: '',
			subscribed: true,
			terms_accepted: false
		},
		onSubmit: async ({ value }) => {
			registering = true;
			try {
				await onRegister(
					{
						email: value.email.trim(),
						password: value.password,
						terms_accepted: enableTermsCheckbox && value.terms_accepted
					},
					value.subscribed
				);
			} finally {
				registering = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({
			email: '',
			password: '',
			confirmPassword: '',
			subscribed: true,
			terms_accepted: false
		});
	}

	function handleOidcRegister(providerSlug: string) {
		// Store subscribed preference for post-registration Plunk tracking
		if (form.state.values.subscribed) {
			sessionStorage.setItem('pendingPlunkRegistration', 'true');
		}

		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=register&return_url=${returnUrl}&terms_accepted=${enableTermsCheckbox && form.state.values.terms_accepted}`;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title="Create your account"
	size="md"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	<svelte:fragment slot="header-icon">
		<img
			src="https://cdn.jsdelivr.net/gh/scanopy/website@main/static/scanopy-logo.png"
			alt="Scanopy Logo"
			class="h-8 w-8"
		/>
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			{#if orgName && invitedBy}
				<div class="mb-6">
					<InlineInfo
						title="You're invited!"
						body={`You have been invited to join ${orgName} by ${invitedBy}. Please sign in or register to continue.`}
					/>
				</div>
			{/if}

			{#if hasPendingDaemons}
				<div class="mb-6">
					<InlineSuccess
						title="You're all set"
						body={networksWithDaemons.length === 1
							? `Register to finish setup. Your daemon will begin scanning "${pendingNetworkNames}" and populating your network map.`
							: `Register to finish setup. Your daemons will begin scanning "${pendingNetworkNames}" and populating your network map.`}
					/>
				</div>
			{/if}

			<div class="space-y-6">
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
						onBlur: ({ value }) => required(value) || passwordValidator(value)
					}}
				>
					{#snippet children(passwordField)}
						<form.Field
							name="confirmPassword"
							validators={{
								onBlur: ({ value, fieldApi }) =>
									required(value) ||
									confirmPasswordMatch(() => fieldApi.form.getFieldValue('password'))(value)
							}}
						>
							{#snippet children(confirmPasswordField)}
								<Password {passwordField} {confirmPasswordField} required={true} />
							{/snippet}
						</form.Field>
					{/snippet}
				</form.Field>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<form.Subscribe selector={(state) => state.values.terms_accepted}>
				{#snippet children(termsAccepted)}
					<div class="flex w-full flex-col gap-4">
						<div class="flex flex-grow flex-col items-center gap-2">
							{#if enableTermsCheckbox}
								<form.Field name="terms_accepted">
									{#snippet children(field)}
										<Checkbox
											label="I agree to the <a class='text-link' target='_blank' href='https://scanopy.net/terms'>terms</a> and <a target='_blank' class='text-link' href='https://scanopy.net/privacy'>privacy policy</a>"
											helpText=""
											{field}
											id="terms"
										/>
									{/snippet}
								</form.Field>
							{/if}
						</div>

						<button
							type="submit"
							disabled={registering || (enableTermsCheckbox && !termsAccepted)}
							class="btn-primary w-full"
						>
							{registering ? 'Creating account...' : 'Create Account with Email'}
						</button>

						{#if hasOidcProviders}
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
										onclick={() => handleOidcRegister(provider.slug)}
										disabled={enableTermsCheckbox && !termsAccepted}
										type="button"
										class="btn-secondary flex w-full items-center justify-center gap-3"
									>
										{#if provider.logo}
											<img src={provider.logo} alt={provider.name} class="h-5 w-5" />
										{/if}
										Create Account with {provider.name}
									</button>
								{/each}
							</div>
						{/if}

						<div class="flex flex-grow flex-col items-center gap-2">
							{#if enableEmailOptIn}
								<form.Field name="subscribed">
									{#snippet children(field)}
										<Checkbox
											{field}
											label="Sign up for product updates via email"
											id="subscribe"
											helpText=""
										/>
									{/snippet}
								</form.Field>
							{/if}
						</div>
					</div>
				{/snippet}
			</form.Subscribe>
		</div>
	</form>
</GenericModal>
