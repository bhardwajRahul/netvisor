<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { pushSuccess, pushError } from '$lib/shared/stores/feedback';
	import { email as emailValidator } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import {
		common_email,
		common_emailPlaceholder,
		common_sending,
		common_submit,
		common_submitted,
		onboarding_anotherIssue,
		onboarding_emailNewsUpdates,
		onboarding_emailOptional,
		onboarding_feedbackError,
		onboarding_feedbackPlaceholder,
		onboarding_feedbackQuestion,
		onboarding_feedbackThankYou
	} from '$lib/paraglide/messages';

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
				// Track analytics
				trackEvent('onboarding_feedback_submitted', {
					email_provided: !!email.trim(),
					blocker,
					message: feedbackText.trim()
				});

				pushSuccess(onboarding_feedbackThankYou());
				hasSubmitted = true;
			} catch (error) {
				console.error('Failed to submit feedback:', error);
				pushError(onboarding_feedbackError());
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
				label={onboarding_feedbackQuestion()}
				id="feedback"
				{field}
				placeholder={onboarding_feedbackPlaceholder()}
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
				label={common_email()}
				id="email"
				{field}
				placeholder={common_emailPlaceholder()}
				helpText={onboarding_emailOptional()}
			/>
		{/snippet}
	</form.Field>

	<form.Field name="subscribe">
		{#snippet children(field)}
			<Checkbox {field} id="subscribe" label={onboarding_emailNewsUpdates()} />
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
					{onboarding_anotherIssue()}
				</button>
			{:else}
				<div></div>
			{/if}
			<button type="button" class="btn-primary" disabled={!canSubmit} onclick={handleSubmit}>
				{#if hasSubmitted}
					{common_submitted()}
				{:else if isSubmitting}
					{common_sending()}
				{:else}
					{common_submit()}
				{/if}
			</button>
		</div>
	{/if}
</div>
