<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		placeholder?: string;
		required?: boolean;
		helpText?: string;
		type?: 'text' | 'email' | 'password' | 'number';
		disabled?: boolean;
	}

	let {
		label,
		field,
		id,
		placeholder = '',
		required = false,
		helpText = '',
		type = 'text',
		disabled = false
	}: Props = $props();

	let hasErrors = $derived(field.state.meta.isTouched && field.state.meta.errors.length > 0);
</script>

<FormField {label} {field} {required} {helpText} {id}>
	{#snippet children()}
		<input
			{id}
			{type}
			value={field.state.value ?? ''}
			onblur={() => field.handleBlur()}
			oninput={(e) => {
				const value = e.currentTarget.value;
				// Convert to number for number inputs, otherwise keep as string
				field.handleChange(type === 'number' ? (value === '' ? '' : Number(value)) : value);
			}}
			{placeholder}
			{disabled}
			class="input-field"
			class:input-field-error={hasErrors}
		/>
	{/snippet}
</FormField>
