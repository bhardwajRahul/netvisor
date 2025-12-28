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
		disabled?: boolean;
		min?: string;
		max?: string;
	}

	let {
		label,
		field,
		id,
		placeholder = '',
		required = false,
		helpText = '',
		disabled = false,
		min = undefined,
		max = undefined
	}: Props = $props();

	// Convert ISO 8601 string to datetime-local format (YYYY-MM-DDTHH:00)
	function toDateTimeLocal(isoString: string): string {
		if (!isoString) return '';
		const date = new Date(isoString);
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		const hours = String(date.getHours()).padStart(2, '0');
		const minutes = String(date.getMinutes()).padStart(2, '0');
		return `${year}-${month}-${day}T${hours}:${minutes}`;
	}

	// Convert datetime-local string to ISO 8601 with Z suffix
	function toISO8601(localString: string): string {
		if (!localString) return '';
		const date = new Date(localString);
		return date.toISOString();
	}

	// Local value for display (datetime-local format)
	let localValue = $derived(toDateTimeLocal(field.state.value ?? ''));
	let hasErrors = $derived(field.state.meta.isTouched && field.state.meta.errors.length > 0);

	// Sync changes back to field in ISO format
	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		field.handleChange(toISO8601(target.value));
	}
</script>

<FormField {label} {field} {required} {helpText} {id}>
	<input
		{id}
		type="datetime-local"
		value={localValue}
		onblur={() => field.handleBlur()}
		oninput={handleInput}
		{placeholder}
		{disabled}
		{min}
		{max}
		class="input-field datetime-picker"
		class:input-field-error={hasErrors}
	/>
</FormField>

<style>
	/* Style the datetime picker to match app theme */
	:global(.datetime-picker) {
		color-scheme: dark;
	}

	/* Style the calendar icon to use text-secondary color */
	:global(.datetime-picker::-webkit-calendar-picker-indicator) {
		cursor: pointer;
	}

	:global(.datetime-picker::-webkit-calendar-picker-indicator:hover) {
		filter: invert(1) opacity(1);
	}
</style>
