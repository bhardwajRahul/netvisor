<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { email as emailValidatorFn, required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import { CheckCircle } from 'lucide-svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { billing_requestInfo, common_cancel, common_sending } from '$lib/paraglide/messages';
	import { apiClient } from '$lib/api/client';

	interface Props {
		isOpen?: boolean;
		planName?: string;
		planType?: string;
		userEmail?: string;
		orgName?: string;
		companySize?: string;
		onClose: () => void;
	}

	let {
		isOpen = false,
		planName = '',
		planType = '',
		userEmail = '',
		orgName = '',
		companySize = '',
		onClose
	}: Props = $props();

	let loading = $state(false);
	let status = $state<'idle' | 'success' | 'error'>('idle');
	let submitError = $state('');

	const teamSizeOptions = [
		{ value: '', label: 'Select team size', disabled: true },
		{ value: '1-10', label: '1-10 employees' },
		{ value: '11-25', label: '11-25 employees' },
		{ value: '26-50', label: '26-50 employees' },
		{ value: '51-100', label: '51-100 employees' },
		{ value: '101-250', label: '101-250 employees' },
		{ value: '251-500', label: '251-500 employees' },
		{ value: '501-1000', label: '501-1000 employees' },
		{ value: '1001+', label: '1001+ employees' }
	];

	const urgencyOptions = [
		{ value: '', label: 'Select timeline', disabled: true },
		{ value: 'immediately', label: 'Immediately' },
		{ value: '1-3 months', label: '1-3 months' },
		{ value: '3-6 months', label: '3-6 months' },
		{ value: 'exploring', label: 'Just exploring' }
	];

	function getDefaultValues() {
		return {
			email: userEmail,
			name: '',
			company: orgName,
			teamSize: companySize,
			message: '',
			urgency: '',
			networkCount: undefined as number | undefined
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			loading = true;
			submitError = '';

			try {
				const response = await apiClient.POST('/api/billing/inquiry', {
					body: {
						email: value.email.trim(),
						name: value.name.trim(),
						company: value.company.trim(),
						team_size: value.teamSize,
						message: value.message.trim(),
						urgency: value.urgency || undefined,
						network_count: value.networkCount ?? undefined,
						plan_type: planType || undefined
					}
				});

				if (response.data?.success) {
					status = 'success';
					trackEvent('plan_inquiry_submitted', { planType, success: true });
				} else {
					throw new Error(response.data?.error || 'Failed to submit');
				}
			} catch (err) {
				console.error('Plan inquiry form error:', err);
				submitError = 'Something went wrong. Please try again.';
				trackEvent('plan_inquiry_submitted', { planType, success: false });
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		form.reset(getDefaultValues());
		status = 'idle';
		submitError = '';
	}

	function handleClose() {
		status = 'idle';
		submitError = '';
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	title={billing_requestInfo({ planName })}
	{isOpen}
	onClose={handleClose}
	onOpen={handleOpen}
	size="md"
	showCloseButton={true}
>
	{#if status === 'success'}
		<div class="flex flex-col items-center justify-center p-8 text-center">
			<div class="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-500/20">
				<CheckCircle class="h-8 w-8 text-green-400" />
			</div>
			<h3 class="text-primary mb-2 text-xl font-semibold">Thank you!</h3>
			<p class="text-secondary mb-6">
				We've received your inquiry about the {planName} plan. We'll be in touch soon.
			</p>
			<button type="button" onclick={handleClose} class="btn-primary">Close</button>
		</div>
	{:else}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				e.stopPropagation();
				handleSubmit();
			}}
			class="flex min-h-0 flex-1 flex-col"
		>
			<div class="flex-1 overflow-auto p-6">
				<p class="text-secondary mb-6 text-sm">
					Tell us about your needs and we'll get back to you shortly.
				</p>

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
								placeholder="you@company.com"
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<TextInput label="Name" id="inquiry-name" {field} placeholder="Your name" required />
						{/snippet}
					</form.Field>

					<form.Field
						name="company"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Company"
								id="inquiry-company"
								{field}
								placeholder="Your company"
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="teamSize"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<SelectInput
								label="Company Size"
								id="inquiry-team-size"
								{field}
								options={teamSizeOptions}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="urgency">
						{#snippet children(field)}
							<SelectInput
								label="How soon do you need a solution?"
								id="inquiry-urgency"
								{field}
								options={urgencyOptions}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="networkCount">
						{#snippet children(field)}
							<TextInput
								label="How many networks/sites?"
								id="inquiry-network-count"
								{field}
								type="number"
								placeholder="e.g., 10"
							/>
						{/snippet}
					</form.Field>

					<form.Field name="message">
						{#snippet children(field)}
							<TextArea
								label="Message"
								id="inquiry-message"
								{field}
								placeholder="Tell us about your use case..."
								rows={3}
							/>
						{/snippet}
					</form.Field>

					{#if submitError}
						<p class="text-sm text-red-400">{submitError}</p>
					{/if}
				</div>
			</div>

			<div class="modal-footer">
				<div class="flex items-center justify-end gap-3">
					<button type="button" disabled={loading} onclick={handleClose} class="btn-secondary">
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading} class="btn-primary">
						{loading ? common_sending() : 'Submit'}
					</button>
				</div>
			</div>
		</form>
	{/if}
</GenericModal>
