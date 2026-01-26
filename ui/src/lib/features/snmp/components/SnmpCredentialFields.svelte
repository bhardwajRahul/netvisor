<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import {
		common_version,
		snmp_communityString,
		snmp_communityStringHelp,
		snmp_communityStringPlaceholder,
		snmp_versionV2c,
		snmp_versionV3ComingSoon
	} from '$lib/paraglide/messages';

	interface Props {
		versionField: AnyFieldApi;
		communityField: AnyFieldApi;
		disabled?: boolean;
		showLabels?: boolean;
		showHelpText?: boolean;
	}

	let {
		versionField,
		communityField,
		disabled = false,
		showLabels = true,
		showHelpText = true
	}: Props = $props();
</script>

<div class="space-y-4">
	<div class="space-y-2">
		{#if showLabels}
			<label for="version" class="text-secondary block text-sm font-medium">
				{common_version()}
			</label>
		{/if}
		<SelectInput
			label={showLabels ? '' : common_version()}
			id="version"
			field={versionField}
			{disabled}
			options={[
				{ value: 'V2c', label: snmp_versionV2c() },
				{ value: 'V3', label: snmp_versionV3ComingSoon(), disabled: true }
			]}
		/>
	</div>

	<TextInput
		label={showLabels ? snmp_communityString() : ''}
		id="community"
		type="password"
		field={communityField}
		placeholder={snmp_communityStringPlaceholder()}
		required
		helpText={showHelpText ? snmp_communityStringHelp() : ''}
		{disabled}
	/>
</div>
