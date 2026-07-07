<script lang="ts">
	import type { components } from '$lib/api/schema';
	import type { PublicServerConfig } from '$lib/shared/stores/config-query';
	import { useProfileUpdateMutation } from '$lib/features/auth/queries';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_companySize,
		common_companySize101To250,
		common_companySize1To10,
		common_companySize11To25,
		common_companySize251To500,
		common_companySize26To50,
		common_companySize501To1000,
		common_companySize51To100,
		common_companySizeOver1000,
		common_companySizeSelect,
		common_dismiss,
		common_other,
		common_role,
		common_submit,
		home_profileRoleDevops,
		home_profileRoleExecutive,
		home_profileRoleItAdmin,
		home_profileRoleManager,
		home_profileRoleNetworkEngineer,
		home_profileRoleSelect,
		home_profileSubtitle,
		home_profileTitle
	} from '$lib/paraglide/messages';

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
		{ value: '', label: home_profileRoleSelect(), disabled: true },
		{ value: 'it_admin', label: home_profileRoleItAdmin() },
		{ value: 'network_engineer', label: home_profileRoleNetworkEngineer() },
		{ value: 'devops', label: home_profileRoleDevops() },
		{ value: 'manager', label: home_profileRoleManager() },
		{ value: 'executive', label: home_profileRoleExecutive() },
		{ value: 'other', label: common_other() }
	];

	const companySizeOptions = [
		{ value: '', label: common_companySizeSelect(), disabled: true },
		{ value: '1-10', label: common_companySize1To10() },
		{ value: '11-25', label: common_companySize11To25() },
		{ value: '26-50', label: common_companySize26To50() },
		{ value: '51-100', label: common_companySize51To100() },
		{ value: '101-250', label: common_companySize101To250() },
		{ value: '251-500', label: common_companySize251To500() },
		{ value: '501-1000', label: common_companySize501To1000() },
		{ value: '1001+', label: common_companySizeOver1000() }
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
					<h3 class="text-primary text-sm font-semibold">{home_profileTitle()}</h3>
					<p class="text-secondary mt-1 text-xs">{home_profileSubtitle()}</p>
				</div>
				<button onclick={dismiss} class="text-tertiary hover:text-secondary text-sm">
					{common_dismiss()}
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
							<SelectInput
								label={common_role()}
								id="profile-job-title"
								{field}
								options={roleOptions}
							/>
						{/snippet}
					</form.Field>
					<form.Field name="company_size">
						{#snippet children(field)}
							<SelectInput
								label={common_companySize()}
								id="profile-company-size"
								{field}
								options={companySizeOptions}
							/>
						{/snippet}
					</form.Field>
				</div>
				<button type="submit" class="btn-primary mt-3 text-sm">{common_submit()}</button>
			</form>
		</div>
	</section>
{/if}
