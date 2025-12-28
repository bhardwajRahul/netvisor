<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, email } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { Mail } from 'lucide-svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';

	interface Props {
		isOpen?: boolean;
		onRequestReset: (email: string) => Promise<void> | void;
		onClose: () => void;
		onBackToLogin: () => void;
	}

	let { isOpen = false, onRequestReset, onClose, onBackToLogin }: Props = $props();

	let requesting = $state(false);
	let emailSent = $state(false);

	// Create form
	const form = createForm(() => ({
		defaultValues: { email: '' },
		onSubmit: async ({ value }) => {
			requesting = true;
			try {
				await onRequestReset(value.email.trim());
				emailSent = true;
			} finally {
				requesting = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ email: '' });
		emailSent = false;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title={emailSent ? 'Check Your Email' : 'Reset Password'}
	size="md"
	onClose={onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={Mail} color="Blue" />
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
			{#if emailSent}
				<InlineInfo
					title="Reset link sent"
					body="If an account exists with that email, you'll receive a password reset link shortly. Please check your inbox and spam folder."
				/>
			{:else}
				<div class="space-y-6">
					<p class="text-sm text-gray-400">
						Enter your email address and we'll send you a link to reset your password.
					</p>

					<form.Field
						name="email"
						validators={{
							onBlur: ({ value }) => required(value) || email(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Email"
								id="email"
								{field}
								placeholder="Enter your email"
								required
							/>
						{/snippet}
					</form.Field>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if emailSent}
					<button type="button" onclick={onBackToLogin} class="btn-primary w-full">
						Back to Login
					</button>
				{:else}
					<button type="submit" disabled={requesting} class="btn-primary w-full">
						{requesting ? 'Sending...' : 'Send Reset Link'}
					</button>

					<div class="text-center">
						<button
							type="button"
							onclick={onBackToLogin}
							class="text-sm font-medium text-blue-400 hover:text-blue-300"
						>
							Back to Login
						</button>
					</div>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
