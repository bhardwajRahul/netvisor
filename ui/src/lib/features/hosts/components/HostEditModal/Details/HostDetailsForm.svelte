<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { Host, HostFormData } from '$lib/features/hosts/types/base';
	import { hostnameFormat, max, required } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';

	interface Props {
		host?: Host | null;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing: boolean;
	}

	let { host = null, form, formData = $bindable(), isEditing }: Props = $props();

	// Track network_id separately (not a form field, so needs manual sync)
	let selectedNetworkId = $state(formData.network_id);
	$effect(() => {
		formData.network_id = selectedNetworkId;
	});
</script>

<div class="space-y-6 p-6">
	<div class="grid grid-cols-2 gap-6">
		<form.Field
			name="name"
			validators={{
				onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<TextInput
					label="Name"
					id="name"
					placeholder="Enter a name for this host..."
					required={true}
					{field}
				/>
			{/snippet}
		</form.Field>

		<form.Field
			name="hostname"
			validators={{
				onBlur: ({ value }: { value: string }) => hostnameFormat(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<TextInput label="Hostname" id="hostname" placeholder="api.example.com" {field} />
			{/snippet}
		</form.Field>
	</div>

	<SelectNetwork bind:selectedNetworkId />

	<form.Field
		name="description"
		validators={{
			onBlur: ({ value }: { value: string }) => max(500)(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextArea
				label="Description"
				id="description"
				placeholder="Describe what this host does or its role in your infrastructure"
				{field}
			/>
		{/snippet}
	</form.Field>

	<TagPicker bind:selectedTagIds={formData.tags} />

	{#if isEditing}
		<EntityMetadataSection entities={[host]} />
	{/if}
</div>
