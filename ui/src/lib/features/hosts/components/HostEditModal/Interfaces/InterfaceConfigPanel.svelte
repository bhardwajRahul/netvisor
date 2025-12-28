<script lang="ts">
	import type { Interface } from '$lib/features/hosts/types/base';
	import {
		required,
		ipAddressFormat,
		ipAddressInCidrFormat,
		macFormat,
		max
	} from '$lib/shared/components/forms/validators';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		iface: Interface;
		subnet: Subnet;
		index: number;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (iface: Interface) => void;
	}

	let { iface, subnet, index, form, onChange = () => {} }: Props = $props();

	// Field names for this interface in the form array
	let ipFieldName = $derived(`interfaces[${index}].ip_address`);
	let macFieldName = $derived(`interfaces[${index}].mac_address`);
	let nameFieldName = $derived(`interfaces[${index}].name`);

	// Notify parent of changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...iface, name: value || null });
	}

	function handleIpChange(value: string) {
		onChange({ ...iface, ip_address: value });
	}

	function handleMacChange(value: string) {
		onChange({ ...iface, mac_address: value || null });
	}
</script>

{#if subnet}
	<div class="space-y-6">
		<ConfigHeader
			title={'Interface with subnet "' + (subnet?.name ? subnet.name : subnet.cidr) + '"'}
			subtitle={subnet?.description}
		/>

		<div class="space-y-4">
			<form.Field
				name={nameFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => max(100)(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleNameChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput label="Name" id="interface_{iface.id}" placeholder="en0" {field} />
				{/snippet}
			</form.Field>

			<form.Field
				name={ipFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) =>
						required(value) || ipAddressFormat(value) || ipAddressInCidrFormat(subnet.cidr)(value),
					onChange: ({ value }: { value: string }) =>
						required(value) || ipAddressFormat(value) || ipAddressInCidrFormat(subnet.cidr)(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleIpChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label="IP Address"
						id="interface_ip_{iface.id}"
						placeholder="192.168.1.100"
						required={true}
						helpText="Must be within {subnet.cidr}"
						{field}
					/>
				{/snippet}
			</form.Field>

			<form.Field
				name={macFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => macFormat(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleMacChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label="MAC Address"
						id="interface_mac_{iface.id}"
						placeholder="00:1B:44:11:3A:B7"
						helpText="Format: XX:XX:XX:XX:XX:XX or XX-XX-XX-XX-XX-XX"
						{field}
					/>
				{/snippet}
			</form.Field>
		</div>
	</div>
{/if}
