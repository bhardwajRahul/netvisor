<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUpdateSelfMutation } from '$lib/features/users/queries';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		settings_email_digestDescription,
		settings_email_digestLabel,
		settings_email_intro,
		settings_email_updateFailed
	} from '$lib/paraglide/messages';

	const currentUserQuery = useCurrentUserQuery();
	const updateSelfMutation = useUpdateSelfMutation();

	let user = $derived(currentUserQuery.data);

	// Stable literal defaults — do NOT read reactive state inside the createForm
	// options getter (see TanStack + Svelte 5 reactivity notes).
	const form = createForm(() => ({
		defaultValues: { discovery_digest: true },
		onSubmit: async ({ value }) => {
			if (!user) return;
			try {
				await updateSelfMutation.mutateAsync({
					...user,
					email_settings: { discovery_digest: value.discovery_digest }
				});
			} catch {
				// Persist failed — revert the toggle to the last server value (form.reset is
				// the same path the hydrate uses, so the checkbox re-renders) and surface a
				// toast. Never leave the UI claiming a state the server rejected.
				resetForm(user.email_settings?.discovery_digest ?? true);
				pushError(settings_email_updateFailed());
			}
		}
	}));

	// Reset the form without triggering an auto-save — used for both hydration and
	// error-revert, neither of which should fire a write back to the server.
	let suppressSave = false;
	function resetForm(discovery_digest: boolean) {
		suppressSave = true;
		form.reset({ discovery_digest });
		suppressSave = false;
	}

	// Hydrate the form from server data exactly once when currentUser lands.
	let hydrated = $state(false);
	$effect(() => {
		if (user && !hydrated) {
			resetForm(user.email_settings?.discovery_digest ?? true);
			hydrated = true;
		}
	});

	// Auto-save on each change, debounced so rapid toggles collapse into one request.
	// The checkbox reflects the new value immediately (optimistic) via field.handleChange.
	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function scheduleSave() {
		if (suppressSave || !hydrated) return;
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => void form.handleSubmit(), 400);
	}
</script>

<div class="flex flex-1 flex-col">
	<div class="flex-1 overflow-y-auto p-6">
		<div class="space-y-6">
			<p class="text-sm text-gray-600">{settings_email_intro()}</p>

			<InfoCard>
				<form.Field name="discovery_digest" listeners={{ onChange: scheduleSave }}>
					{#snippet children(field)}
						<Checkbox
							id="discovery-digest"
							label={settings_email_digestLabel()}
							helpText={settings_email_digestDescription()}
							{field}
						/>
					{/snippet}
				</form.Field>
			</InfoCard>
		</div>
	</div>
</div>
