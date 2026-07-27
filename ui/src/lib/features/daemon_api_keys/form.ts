/**
 * Shared TanStack form for editing an API key.
 *
 * Both the standalone key modal and the daemon management modal's key tab edit the same
 * shape, and both hand the form to `ApiKeyFormFields`. Creating it through one factory keeps
 * the default values in a single place and gives the shared fields component a real type to
 * accept, rather than an untyped form.
 */
import { createForm } from '@tanstack/svelte-form';
import type { ApiKey } from './types/base';
import { createEmptyApiKeyFormData } from './queries';

export function createApiKeyForm(onSubmit: (value: ApiKey) => Promise<void>) {
	return createForm(() => ({
		defaultValues: createEmptyApiKeyFormData(''),
		onSubmit: async ({ value }: { value: ApiKey }) => onSubmit(value)
	}));
}

export type ApiKeyForm = ReturnType<typeof createApiKeyForm>;
