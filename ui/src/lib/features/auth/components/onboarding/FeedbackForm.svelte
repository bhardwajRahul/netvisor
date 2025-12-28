<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { config } from '$lib/shared/stores/config';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { pushSuccess, pushError } from '$lib/shared/stores/feedback';
	import { email as emailValidator } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';

	interface Props {
		blocker: string;
		showActions?: boolean;
		onOtherIssue?: (() => void) | null;
	}

	let { blocker, showActions = true, onOtherIssue = null }: Props = $props();

	let isSubmitting = $state(false);
	let hasSubmitted = $state(false);

	const form = createForm(() => ({
		defaultValues: {
			feedback: '',
			email: '',
			subscribe: true
		},
		onSubmit: async ({ value }) => {
			const feedbackText = value.feedback;
			const email = value.email;

			if (!feedbackText.trim()) return;

			isSubmitting = true;

			try {
				// Submit to Plunk if configured
				if ($config?.plunk_key && email.trim()) {
					await fetch('https://api.useplunk.com/v1/track', {
						method: 'POST',
						headers: {
							'Content-Type': 'application/json',
							Authorization: `Bearer ${$config.plunk_key}`
						},
						body: JSON.stringify({
							event: 'onboarding_feedback',
							email: email.trim(),
							subscribed: value.subscribe,
							data: {
								blocker,
								message: feedbackText.trim()
							}
						})
					});
				}

				// Track analytics
				trackEvent('onboarding_feedback_submitted', {
					email_provided: !!email.trim(),
					blocker,
					message: feedbackText.trim()
				});

				pushSuccess('Thank you for your feedback!');
				hasSubmitted = true;
			} catch (error) {
				console.error('Failed to submit feedback:', error);
				pushError('Failed to submit feedback. Please try again.');
			} finally {
				isSubmitting = false;
			}
		}
	}));

	export async function handleSubmit() {
		await form.handleSubmit();
	}

	let canSubmit = $derived(
		form.state.values.feedback.trim() !== '' && !isSubmitting && !hasSubmitted
	);
</script>

<div class="space-y-3">
	<form.Field name="feedback">
		{#snippet children(field)}
			<TextArea
				label="What's blocking you from getting started?"
				id="feedback"
				{field}
				placeholder="Tell us what you need help with..."
				rows={3}
			/>
		{/snippet}
	</form.Field>

	<form.Field
		name="email"
		validators={{
			onBlur: ({ value }) => emailValidator(value)
		}}
	>
		{#snippet children(field)}
			<TextInput
				label="Email"
				id="email"
				{field}
				placeholder="your@email.com"
				helpText="Optional - we'll follow up if we can help"
			/>
		{/snippet}
	</form.Field>

	<form.Field name="subscribe">
		{#snippet children(field)}
			<Checkbox {field} id="subscribe" label="Also email me about product news and updates" />
		{/snippet}
	</form.Field>

	{#if showActions}
		<div class="flex items-center justify-between pt-2">
			{#if onOtherIssue}
				<button
					type="button"
					class="text-secondary hover:text-primary text-sm"
					onclick={onOtherIssue}
				>
					I have another issue
				</button>
			{:else}
				<div></div>
			{/if}
			<button type="button" class="btn-primary" disabled={!canSubmit} onclick={handleSubmit}>
				{#if hasSubmitted}
					Submitted
				{:else if isSubmitting}
					Sending...
				{:else}
					Submit
				{/if}
			</button>
		</div>
	{/if}
</div>
