/**
 * TanStack Form utilities for Svelte
 */
import { createFormCreator, createFormCreatorContexts } from '@tanstack/svelte-form';
import type { FormApi } from '@tanstack/form-core';
import { pushError } from '$lib/shared/stores/feedback';

// Create context accessors for child components
export const { useFieldContext, useFormContext } = createFormCreatorContexts();

// Create the form factory (for context-based field access if needed)
export const { createAppForm } = createFormCreator({
	fieldComponents: {},
	formComponents: {}
});

/**
 * Validate a form and show user-friendly error feedback.
 * Returns true if form is valid, false otherwise.
 *
 * Usage:
 * ```svelte
 * const isValid = await validateForm(form);
 * if (isValid) { nextStep(); }
 * ```
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function validateForm(form: FormApi<any, any, any, any, any, any, any, any, any, any, any, any>): Promise<boolean> {
	// Validate all fields first
	await form.validateAllFields('submit');

	// Check for validation errors
	const errorFields = Object.entries(form.state.fieldMeta)
		.filter(([_, meta]) => meta?.errors && meta.errors.length > 0)
		.map(([name]) => name);

	if (errorFields.length > 0) {
		const fieldNames = errorFields.map((f) => f.replace(/_/g, ' ')).join(', ');
		pushError(`Please fix the following fields: ${fieldNames}`);
		return false;
	}

	return true;
}

/**
 * Submit a form with user-friendly validation feedback.
 * Shows a pushError notification if there are validation errors.
 *
 * Usage:
 * ```svelte
 * <form onsubmit={(e) => { e.preventDefault(); e.stopPropagation(); submitForm(form); }}>
 * ```
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function submitForm(form: FormApi<any, any, any, any, any, any, any, any, any, any, any, any>): Promise<void> {
	const isValid = await validateForm(form);
	if (!isValid) {
		return;
	}

	// Submit the form
	await form.handleSubmit();
}
