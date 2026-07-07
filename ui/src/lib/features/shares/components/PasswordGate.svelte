<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required } from '$lib/shared/components/forms/validators';
	import { Lock } from 'lucide-svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import {
		common_errorOccurred,
		common_loading,
		common_password,
		common_viewTopology,
		errors_auth_password_invalid,
		errors_auth_password_required,
		shares_passwordPlaceholder
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		title?: string;
		onSubmit: (password: string) => Promise<boolean>;
		submitLabel?: string;
	}

	let { isOpen = true, title, onSubmit, submitLabel }: Props = $props();

	let displayTitle = $derived(title ?? errors_auth_password_required());
	let displaySubmitLabel = $derived(submitLabel ?? common_viewTopology());

	let loading = $state(false);
	let serverError = $state('');

	const form = createForm(() => ({
		defaultValues: {
			password: ''
		},
		onSubmit: async ({ value }) => {
			serverError = '';
			loading = true;

			try {
				const success = await onSubmit(value.password);
				if (!success) {
					serverError = errors_auth_password_invalid();
					form.setFieldValue('password', '');
				}
			} catch {
				serverError = common_errorOccurred();
			} finally {
				loading = false;
			}
		}
	}));

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleOpen() {
		form.reset();
		serverError = '';
	}
</script>

<GenericModal
	{isOpen}
	title={displayTitle}
	size="sm"
	onClose={() => {}}
	onOpen={handleOpen}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Lock} color="Blue" />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				<form.Field
					name="password"
					validators={{
						onBlur: ({ value }) => required(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={common_password()}
							id="password"
							type="password"
							{field}
							placeholder={shares_passwordPlaceholder()}
							required={true}
							disabled={loading}
						/>
					{/snippet}
				</form.Field>

				{#if serverError}
					<p class="text-danger text-sm">{serverError}</p>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex items-center justify-end">
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? common_loading() : displaySubmitLabel}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
