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
		rows?: number;
		disabled?: boolean;
	}

	let {
		label,
		field,
		id,
		placeholder = '',
		required = false,
		helpText = '',
		rows = 3,
		disabled = false
	}: Props = $props();

	let hasErrors = $derived(field.state.meta.isTouched && field.state.meta.errors.length > 0);
</script>

<FormField {label} {field} {required} {helpText} {id}>
	<textarea
		{id}
		value={field.state.value ?? ''}
		onblur={() => field.handleBlur()}
		oninput={(e) => field.handleChange(e.currentTarget.value)}
		{placeholder}
		{rows}
		{disabled}
		class="input-field"
		class:input-field-error={hasErrors}
	></textarea>
</FormField>
