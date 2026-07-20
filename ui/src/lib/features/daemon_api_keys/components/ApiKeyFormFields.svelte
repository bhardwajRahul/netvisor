<script lang="ts">
	/**
	 * The editable fields of an API key, plus its generate/rotate control.
	 *
	 * Extracted from ApiKeyModal so the daemon management modal can offer the same key
	 * editing inside a tab — the modal shell, footer and form lifecycle can't nest, but
	 * the fields can. The owning component supplies the TanStack form.
	 */
	import type { ApiKeyForm } from '../form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import ApiKeyGenerator from '$lib/shared/components/api-keys/ApiKeyGenerator.svelte';
	import {
		common_apiKeyNameHelp,
		common_enableApiKey,
		common_expirationDateOptional,
		common_expirationNeverHelp,
		common_keyDetails,
		common_name,
		daemonApiKeys_enableApiKeyHelp,
		daemonApiKeys_namePlaceholder
	} from '$lib/paraglide/messages';

	interface Props {
		form: ApiKeyForm;
		isEditing: boolean;
		generatedKey: string | null;
		loading: boolean;
		onGenerate: () => void;
		onRotate: () => void;
		/** Hide the network selector where the network is fixed by the owning entity. */
		showNetwork?: boolean;
	}

	let {
		form,
		isEditing,
		generatedKey,
		loading,
		onGenerate,
		onRotate,
		showNetwork = true
	}: Props = $props();

	// Minimum selectable expiry (now), in the local format datetime-local expects.
	function getLocalDateTimeMin(): string {
		const now = new Date();
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`;
	}
	const today = getLocalDateTimeMin();
</script>

<div class="space-y-6">
	<div class="space-y-4">
		<h3 class="text-primary text-lg font-medium">{common_keyDetails()}</h3>

		<form.Field
			name="name"
			validators={{
				onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
			}}
		>
			{#snippet children(field)}
				<TextInput
					label={common_name()}
					id="name"
					{field}
					placeholder={daemonApiKeys_namePlaceholder()}
					helpText={common_apiKeyNameHelp()}
					required
				/>
			{/snippet}
		</form.Field>

		{#if showNetwork}
			<form.Field name="network_id">
				{#snippet children(field)}
					<SelectNetwork
						selectedNetworkId={field.state.value}
						onNetworkChange={(id) => field.handleChange(id)}
						disabled={isEditing}
					/>
				{/snippet}
			</form.Field>
		{/if}

		<form.Field name="tags">
			{#snippet children(field)}
				<TagPicker
					selectedTagIds={field.state.value || []}
					onChange={(tags) => field.handleChange(tags)}
				/>
			{/snippet}
		</form.Field>

		<form.Field name="expires_at">
			{#snippet children(field)}
				<DateInput
					label={common_expirationDateOptional()}
					id="expires_at"
					{field}
					helpText={common_expirationNeverHelp()}
					min={today}
				/>
			{/snippet}
		</form.Field>

		<form.Field name="is_enabled">
			{#snippet children(field)}
				<Checkbox
					{field}
					label={common_enableApiKey()}
					helpText={daemonApiKeys_enableApiKeyHelp()}
					id="enableApiKey"
				/>
			{/snippet}
		</form.Field>
	</div>

	<ApiKeyGenerator {generatedKey} {isEditing} {loading} {onGenerate} {onRotate} />
</div>
