<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { email as emailValidatorFn, required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';

	interface Props {
		isOpen?: boolean;
		planName?: string;
		userEmail?: string;
		onClose: () => void;
		onSubmit: (email: string, message: string) => void | Promise<void>;
	}

	let { isOpen = false, planName = '', userEmail = '', onClose, onSubmit }: Props = $props();

	let loading = $state(false);

	function getDefaultValues() {
		return {
			email: userEmail,
			message: ''
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			loading = true;
			try {
				await onSubmit(value.email, value.message);
				onClose();
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		form.reset(getDefaultValues());
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	title="Request Information - {planName}"
	{isOpen}
	{onClose}
	onOpen={handleOpen}
	size="md"
	showCloseButton={true}
>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				<form.Field
					name="email"
					validators={{
						onBlur: ({ value }) => required(value) || emailValidatorFn(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label="Email"
							id="inquiry-email"
							{field}
							placeholder="your@email.com"
							required
						/>
					{/snippet}
				</form.Field>

				<form.Field name="message">
					{#snippet children(field)}
						<TextArea
							label="What information are you looking for?"
							id="inquiry-message"
							{field}
							placeholder="Tell us what you'd like to know about the {planName} plan..."
							rows={5}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
					Cancel
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? 'Sending...' : 'Send Request'}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
