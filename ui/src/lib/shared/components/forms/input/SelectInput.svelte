<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import InlineInfo from '../../feedback/InlineInfo.svelte';

	interface SelectOption {
		value: string;
		label: string;
		id?: string;
		disabled?: boolean;
		description?: string;
	}

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		options: SelectOption[];
		helpText?: string;
		disabled?: boolean;
	}

	let { label, field, id, options, helpText = '', disabled = false }: Props = $props();

	let selectedOption = $derived(options.find((f) => f.value == field.state.value));
	let description = $derived(selectedOption?.description ?? '');
</script>

<FormField {label} {field} {helpText} {id}>
	{#snippet children()}
		<select
			{id}
			value={field.state.value}
			onchange={(e) => field.handleChange(e.currentTarget.value)}
			{disabled}
			class="input-field"
			onclick={(e) => e.stopPropagation()}
		>
			{#each options as option (option.id ?? option.value)}
				<option disabled={option.disabled} value={option.value}>{option.label}</option>
			{/each}
		</select>
	{/snippet}
</FormField>
{#if selectedOption && description && description.length > 0}
	<InlineInfo title={label} body={description} />
{/if}
