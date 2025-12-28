<script lang="ts">
	import type { Port } from '$lib/features/hosts/types/base';
	import { port as portValidator, required } from '$lib/shared/components/forms/validators';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		port: Port;
		index: number;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (port: Port) => void;
	}

	let { port, index, form, onChange = () => {} }: Props = $props();

	// Field names for this port in the form array
	let numberFieldName = $derived(`ports[${index}].number`);
	let protocolFieldName = $derived(`ports[${index}].protocol`);

	let protocolOptions = [
		{ value: 'Tcp', label: 'TCP' },
		{ value: 'Udp', label: 'UDP' }
	];

	// Notify parent of changes for real-time sync
	function handleNumberChange(value: number) {
		onChange({ ...port, number: value });
	}

	function handleProtocolChange(value: string) {
		onChange({ ...port, protocol: value as 'Tcp' | 'Udp' });
	}
</script>

<div class="space-y-6">
	<ConfigHeader title="Port Configuration" subtitle="Configure the port number and protocol" />

	<div class="space-y-4">
		<form.Field
			name={numberFieldName}
			validators={{
				onBlur: ({ value }: { value: number | string }) =>
					required(String(value)) || portValidator(value),
				onChange: ({ value }: { value: number | string }) =>
					required(String(value)) || portValidator(value)
			}}
			listeners={{
				onChange: ({ value }: { value: number }) => handleNumberChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<TextInput
					label="Port Number"
					id="port_{port.id}_number"
					placeholder="80"
					type="number"
					required={true}
					helpText="Port must be between 1 and 65535"
					{field}
				/>
			{/snippet}
		</form.Field>

		<form.Field
			name={protocolFieldName}
			listeners={{
				onChange: ({ value }: { value: string }) => handleProtocolChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<SelectInput
					label="Protocol"
					id="port_{port.id}_protocol"
					options={protocolOptions}
					{field}
				/>
			{/snippet}
		</form.Field>
	</div>
</div>
