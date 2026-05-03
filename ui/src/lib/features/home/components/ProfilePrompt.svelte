<script lang="ts">
	import type { components } from '$lib/api/schema';
	import type { PublicServerConfig } from '$lib/shared/stores/config-query';
	import { useProfileUpdateMutation } from '$lib/features/auth/queries';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';

	type Organization = components['schemas']['Organization'];
	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let {
		organization,
		configData = null
	}: {
		organization: Organization;
		configData?: PublicServerConfig | null;
	} = $props();

	const onboarding = $derived(organization.onboarding ?? []);
	const has = (op: OnboardingOperation) => onboarding.includes(op);

	const isCloud = $derived(configData?.deployment_type === 'cloud');

	const visible = $derived(has('FirstDiscoveryCompleted') && !has('ProfileCompleted') && isCloud);

	const profileMutation = useProfileUpdateMutation();

	const roleOptions = [
		{ value: '', label: 'Select your role', disabled: true },
		{ value: 'it_admin', label: 'IT Admin' },
		{ value: 'network_engineer', label: 'Network Engineer' },
		{ value: 'devops', label: 'DevOps' },
		{ value: 'manager', label: 'Manager / Director' },
		{ value: 'executive', label: 'Owner / Executive' },
		{ value: 'other', label: 'Other' }
	];

	const companySizeOptions = [
		{ value: '', label: 'Select company size', disabled: true },
		{ value: '1-10', label: '1-10 employees' },
		{ value: '11-25', label: '11-25 employees' },
		{ value: '26-50', label: '26-50 employees' },
		{ value: '51-100', label: '51-100 employees' },
		{ value: '101-250', label: '101-250 employees' },
		{ value: '251-500', label: '251-500 employees' },
		{ value: '501-1000', label: '501-1000 employees' },
		{ value: '1001+', label: '1001+ employees' }
	];

	const form = createForm(() => ({
		defaultValues: {
			job_title: '',
			company_size: ''
		},
		onSubmit: async ({ value }) => {
			profileMutation.mutate({
				job_title: value.job_title || undefined,
				company_size: value.company_size || undefined
			});
		}
	}));

	function dismiss() {
		// Submit empty payload — still records ProfileCompleted milestone
		profileMutation.mutate({ job_title: undefined, company_size: undefined });
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

{#if visible}
	<section>
		<div class="card card-static !rounded-lg !p-4">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-primary text-sm font-semibold">Tell us about your team</h3>
					<p class="text-secondary mt-1 text-xs">Helps us prioritize features for your use case.</p>
				</div>
				<button onclick={dismiss} class="text-tertiary hover:text-secondary text-sm">
					Dismiss
				</button>
			</div>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					e.stopPropagation();
					handleSubmit();
				}}
			>
				<div class="mt-3 grid gap-3 sm:grid-cols-2">
					<form.Field name="job_title">
						{#snippet children(field)}
							<SelectInput label="Role" id="profile-job-title" {field} options={roleOptions} />
						{/snippet}
					</form.Field>
					<form.Field name="company_size">
						{#snippet children(field)}
							<SelectInput
								label="Company size"
								id="profile-company-size"
								{field}
								options={companySizeOptions}
							/>
						{/snippet}
					</form.Field>
				</div>
				<button type="submit" class="btn-primary mt-3 text-sm">Submit</button>
			</form>
		</div>
	</section>
{/if}
