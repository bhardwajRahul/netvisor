<script lang="ts">
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		useOrganizationQuery,
		useUpdateOrganizationMutation,
		useResetOrganizationDataMutation,
		usePopulateDemoDataMutation
	} from '$lib/features/organizations/queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { createForm } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import * as m from '$lib/paraglide/messages';

	let {
		subView = $bindable<'main' | 'edit'>('main'),
		onClose
	}: {
		subView?: 'main' | 'edit';
		onClose: () => void;
	} = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	const updateOrganizationMutation = useUpdateOrganizationMutation();
	const resetOrganizationDataMutation = useResetOrganizationDataMutation();
	const populateDemoDataMutation = usePopulateDemoDataMutation();

	let saving = $derived(updateOrganizationMutation.isPending);
	let resetting = $derived(resetOrganizationDataMutation.isPending);
	let populating = $derived(populateDemoDataMutation.isPending);

	let org = $derived(organizationQuery.data);
	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isDemoOrg = $derived(org?.plan?.type === 'Demo');

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: {
			name: org?.name ?? ''
		},
		onSubmit: async ({ value }) => {
			await handleSave(value.name);
		}
	}));

	// Reset form when switching to edit view
	export function resetForm() {
		if (org) {
			form.reset();
			form.setFieldValue('name', org.name);
		}
	}

	async function handleSave(name: string) {
		if (!org) return;

		try {
			await updateOrganizationMutation.mutateAsync({ id: org.id, name });
			pushSuccess(m.settings_org_updated());
			subView = 'main';
		} catch {
			pushError(m.settings_org_updateFailed());
		}
	}

	function handleCancel() {
		if (subView === 'edit') {
			subView = 'main';
			if (org) {
				form.setFieldValue('name', org.name);
			}
		} else {
			onClose();
		}
	}

	async function handleReset() {
		if (!org) return;

		if (!confirm(m.settings_org_resetConfirm())) {
			return;
		}

		try {
			await resetOrganizationDataMutation.mutateAsync(org.id);
			pushSuccess(m.settings_org_resetSuccess());
		} catch {
			pushError(m.settings_org_resetFailed());
		}
	}

	async function handlePopulateDemo() {
		if (!org) return;

		if (!confirm(m.settings_org_populateConfirm())) {
			return;
		}

		try {
			await populateDemoDataMutation.mutateAsync(org.id);
			pushSuccess(m.settings_org_populateSuccess());
		} catch {
			pushError(m.settings_org_populateFailed());
		}
	}

	let showSave = $derived(subView === 'edit');
	let cancelLabel = $derived(subView === 'main' ? m.common_close() : m.common_back());
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if org}
		{#if subView === 'main'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<!-- Organization Info -->
					<InfoCard title={m.settings_org_info()}>
						<InfoRow label={m.common_name()}>{org.name}</InfoRow>
						{#if org.plan}
							<InfoRow label={m.settings_org_plan()}>{org.plan.type}</InfoRow>
						{/if}
						<InfoRow label={m.settings_org_created()}>
							{formatTimestamp(org.created_at)}
						</InfoRow>
						<InfoRow label={m.settings_org_id()} mono={true}>{org.id}</InfoRow>
					</InfoCard>

					<!-- Actions -->
					<InfoCard>
						<div class="flex items-center justify-between">
							<div>
								<p class="text-primary text-sm font-medium">{m.settings_org_nameLabel()}</p>
								<p class="text-secondary text-xs">{m.settings_org_updateName()}</p>
							</div>
							<button
								onclick={() => {
									subView = 'edit';
									form.setFieldValue('name', org.name);
								}}
								class="btn-primary"
							>
								{m.common_edit()}
							</button>
						</div>
					</InfoCard>

					{#if isOwner}
						<!-- Reset Organization Data (available to all org owners) -->
						<InfoCard>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-primary text-sm font-medium">{m.settings_org_resetData()}</p>
									<p class="text-secondary text-xs">
										{m.settings_org_resetDataHelp()}
									</p>
								</div>
								<button onclick={handleReset} disabled={resetting} class="btn-danger">
									{resetting ? m.common_loading() : m.common_reset()}
								</button>
							</div>
						</InfoCard>

						{#if isDemoOrg}
							<!-- Populate Demo Data (only for Demo orgs) -->
							<InfoCard>
								<div class="flex items-center justify-between">
									<div>
										<p class="text-primary text-sm font-medium">{m.settings_org_populateDemo()}</p>
										<p class="text-secondary text-xs">
											{m.settings_org_populateDemoHelp()}
										</p>
									</div>
									<button onclick={handlePopulateDemo} disabled={populating} class="btn-primary">
										{populating ? m.settings_org_populating() : m.settings_org_populate()}
									</button>
								</div>
							</InfoCard>
						{/if}
					{/if}
				</div>
			</div>
		{:else if subView === 'edit'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<p class="text-secondary text-sm">{m.settings_org_updateName()}</p>
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<TextInput
								label={m.settings_org_nameLabel()}
								id="name"
								placeholder={m.settings_org_namePlaceholder()}
								required={true}
								{field}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		{/if}
	{:else}
		<div class="flex-1 overflow-auto p-6">
			<div class="text-secondary py-8 text-center">
				<p>{m.settings_org_unableToLoad()}</p>
				<p class="text-tertiary mt-2 text-sm">{m.settings_org_tryAgainLater()}</p>
			</div>
		</div>
	{/if}

	<!-- Footer -->
	<div class="modal-footer">
		<div class="flex items-center justify-end gap-3">
			<button type="button" onclick={handleCancel} class="btn-secondary">
				{cancelLabel}
			</button>
			{#if showSave}
				<button
					type="button"
					onclick={() => form.handleSubmit()}
					disabled={saving}
					class="btn-primary"
				>
					{saving ? m.settings_account_saving() : m.settings_account_saveChanges()}
				</button>
			{/if}
		</div>
	</div>
</div>
