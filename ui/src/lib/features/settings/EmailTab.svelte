<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUpdateSelfMutation } from '$lib/features/users/queries';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import {
		common_save,
		common_saving,
		settings_email_digestDescription,
		settings_email_digestLabel,
		settings_email_intro,
		settings_email_updateFailed,
		settings_email_updated
	} from '$lib/paraglide/messages';

	const currentUserQuery = useCurrentUserQuery();
	const updateSelfMutation = useUpdateSelfMutation();

	let user = $derived(currentUserQuery.data);
	let saving = $derived(updateSelfMutation.isPending);

	// Stable literal defaults — do NOT read reactive state inside the
	// createForm options getter. Reading $state there makes TanStack Form
	// recreate options on every change and loops against the $effect that
	// writes back.
	const form = createForm(() => ({
		defaultValues: { discovery_digest: true },
		onSubmit: async ({ value }) => {
			if (!user) return;
			try {
				await updateSelfMutation.mutateAsync({
					...user,
					email_settings: { discovery_digest: value.discovery_digest }
				});
				pushSuccess(settings_email_updated());
			} catch {
				pushError(settings_email_updateFailed());
			}
		}
	}));

	// Hydrate the form from server data exactly once when currentUser lands.
	// The `hydrated` flag guards against the effect re-firing on form state
	// changes that follow the reset.
	let hydrated = $state(false);
	$effect(() => {
		if (user && !hydrated) {
			form.reset({ discovery_digest: user.email_settings?.discovery_digest ?? true });
			hydrated = true;
		}
	});
</script>

<div class="flex flex-1 flex-col">
	<div class="flex-1 overflow-y-auto p-6">
		<div class="space-y-6">
			<p class="text-sm text-gray-600">{settings_email_intro()}</p>

			<form
				onsubmit={(e) => {
					e.preventDefault();
					e.stopPropagation();
					form.handleSubmit();
				}}
			>
				<InfoCard>
					<form.Field name="discovery_digest">
						{#snippet children(field)}
							<Checkbox
								id="discovery-digest"
								label={settings_email_digestLabel()}
								helpText={settings_email_digestDescription()}
								{field}
							/>
						{/snippet}
					</form.Field>

					<div class="mt-4 flex justify-end">
						<button
							type="submit"
							class="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
							disabled={saving || !user}
						>
							{saving ? common_saving() : common_save()}
						</button>
					</div>
				</InfoCard>
			</form>
		</div>
	</div>
</div>
