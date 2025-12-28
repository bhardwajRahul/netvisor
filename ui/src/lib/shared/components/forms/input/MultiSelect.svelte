<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

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
	}

	let { label, field, id, options, helpText = '' }: Props = $props();

	function handleChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		const selectedValues = Array.from(select.selectedOptions).map((opt) => opt.value);
		field.handleChange(selectedValues);
	}
</script>

<FormField {label} {field} {helpText} {id}>
	<select
		{id}
		multiple
		value={field.state.value ?? []}
		onchange={handleChange}
		class="text-primary w-full rounded-md border border-gray-600 bg-gray-700 px-2 py-1.5 text-xs focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
	>
		{#each options as option (option?.id ?? option.value)}
			<option value={option.value} selected={(field.state.value ?? []).includes(option.value)}>
				{option.label}
			</option>
		{/each}
	</select>
</FormField>

<style>
	/* Style multi-select options */
	select[multiple] option {
		padding: 0.25rem 0.5rem;
		cursor: pointer;
	}

	select[multiple] option:checked {
		background-color: rgb(37, 99, 235);
		color: white;
	}

	/* Remove default select styling for multi-select */
	select[multiple] {
		scrollbar-width: thin;
		scrollbar-color: #4b5563 #1f2937;
	}

	select[multiple]::-webkit-scrollbar {
		width: 8px;
	}

	select[multiple]::-webkit-scrollbar-track {
		background: #1f2937;
	}

	select[multiple]::-webkit-scrollbar-thumb {
		background-color: #4b5563;
		border-radius: 4px;
	}
</style>
