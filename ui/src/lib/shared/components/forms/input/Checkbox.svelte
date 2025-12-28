<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		label: string;
		field: AnyFieldApi;
		id: string;
		helpText?: string;
		required?: boolean;
		disabled?: boolean;
	}

	let { label, field, id, helpText = '', required = false, disabled = false }: Props = $props();
</script>

<div class:disabled>
	<FormField {label} {field} {helpText} {id} inline={true} {required}>
		{#snippet children()}
			<input
				type="checkbox"
				{id}
				checked={field.state.value}
				{disabled}
				onchange={(e) => field.handleChange(e.currentTarget.checked)}
				class="h-4 w-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
			/>
		{/snippet}
	</FormField>
</div>

<style>
	input[type='checkbox']:checked {
		background-color: rgb(37, 99, 235);
		border-color: rgb(37, 99, 235);
	}

	.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.disabled :global(label) {
		cursor: not-allowed;
	}
</style>
