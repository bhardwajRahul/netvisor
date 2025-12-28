<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';

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
	<h3 class="text-primary text-lg font-medium">Discovery Details</h3>

	<form.Field
		name="name"
		validators={{
			onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextInput
				label="Discovery Name"
				id="name"
				placeholder="e.g., Daily Network Scan, Docker Services Check"
				required={true}
				{field}
				disabled={readOnly}
			/>
		{/snippet}
	</form.Field>

	<!-- Daemon Selection -->
	<div class="space-y-2">
		<RichSelect
			label="Daemon"
			required={true}
			placeholder="Select daemon to run discovery..."
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
		<p class="text-tertiary text-xs">The daemon that will execute this discovery</p>
	</div>

	<TagPicker bind:selectedTagIds={formData.tags} />
</div>
