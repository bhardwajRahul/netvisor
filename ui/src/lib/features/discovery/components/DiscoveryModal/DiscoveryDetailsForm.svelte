<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import * as m from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; state: { values: { name: string } } };
		formData: Discovery;
		daemons?: Daemon[];
		readOnly?: boolean;
	}

	let { form, formData = $bindable(), daemons = [], readOnly = false }: Props = $props();
</script>

<div class="space-y-4">
	<h3 class="text-primary text-lg font-medium">{m.discovery_details()}</h3>

	<form.Field
		name="name"
		validators={{
			onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextInput
				label={m.discovery_name()}
				id="name"
				placeholder={m.discovery_namePlaceholder()}
				required={true}
				{field}
				disabled={readOnly}
			/>
		{/snippet}
	</form.Field>

	<!-- Daemon Selection -->
	<div class="space-y-2">
		<RichSelect
			label={m.discovery_daemon()}
			required={true}
			placeholder={m.discovery_daemonSelect()}
			disabled={readOnly}
			selectedValue={formData.daemon_id}
			options={daemons}
			displayComponent={DaemonDisplay}
			onSelect={(value) => {
				const selectedDaemon = daemons.find((d) => d.id === value);
				if (selectedDaemon) {
					formData = { ...formData, daemon_id: value, network_id: selectedDaemon.network_id };
				}
			}}
		/>
		<p class="text-tertiary text-xs">{m.discovery_daemonHelp()}</p>
	</div>

	<TagPicker bind:selectedTagIds={formData.tags} />
</div>
