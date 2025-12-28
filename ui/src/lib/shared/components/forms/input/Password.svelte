<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { AlertCircle } from 'lucide-svelte';

	interface Props {
		passwordField: AnyFieldApi;
		confirmPasswordField?: AnyFieldApi;
		label?: string;
		confirmLabel?: string;
		required?: boolean;
	}

	let {
		passwordField,
		confirmPasswordField,
		label = 'Password',
		confirmLabel = 'Confirm Password',
		required = true
	}: Props = $props();

	// Password requirements derived from field value
	let value = $derived(passwordField.state.value as string);
	let hasUppercase = $derived(/[A-Z]/.test(value));
	let hasLowercase = $derived(/[a-z]/.test(value));
	let hasNumber = $derived(/[0-9]/.test(value));
	let passwordLongEnough = $derived(value.length >= 10);

	// Password field errors
	let passwordErrors = $derived(passwordField.state.meta.errors);
	let showPasswordErrors = $derived(
		passwordField.state.meta.isTouched && passwordErrors.length > 0
	);

	// Confirm field errors
	let confirmErrors = $derived(confirmPasswordField?.state.meta.errors ?? []);
	let showConfirmErrors = $derived(
		confirmPasswordField?.state.meta.isTouched && confirmErrors.length > 0
	);
</script>

<div class="space-y-4">
	<div class="space-y-2">
		<label for="password" class="text-secondary block text-sm font-medium">
			{label}
			{#if required}<span class="text-red-400">*</span>{/if}
		</label>
		<input
			id="password"
			type="password"
			value={passwordField.state.value}
			onblur={() => passwordField.handleBlur()}
			oninput={(e) => passwordField.handleChange(e.currentTarget.value)}
			placeholder="Create a strong password"
			class="input-field"
			class:input-field-error={showPasswordErrors}
		/>

		<!-- Password Requirements -->
		{#if value}
			<div class="space-y-1 rounded-md bg-gray-700 p-3">
				<p class="text-xs font-medium text-gray-300">Password Requirements:</p>
				<p class="text-xs {passwordLongEnough ? 'text-green-400' : 'text-gray-400'}">
					{passwordLongEnough ? '✓' : '○'} At least 10 characters
				</p>
				<p class="text-xs {hasUppercase ? 'text-green-400' : 'text-gray-400'}">
					{hasUppercase ? '✓' : '○'} Contains uppercase letter
				</p>
				<p class="text-xs {hasLowercase ? 'text-green-400' : 'text-gray-400'}">
					{hasLowercase ? '✓' : '○'} Contains lowercase letter
				</p>
				<p class="text-xs {hasNumber ? 'text-green-400' : 'text-gray-400'}">
					{hasNumber ? '✓' : '○'} Contains number
				</p>
			</div>
		{/if}

		{#if showPasswordErrors}
			<div class="text-danger flex items-center gap-2">
				<AlertCircle size={16} />
				<p class="text-xs">{passwordErrors[0]}</p>
			</div>
		{/if}
	</div>

	{#if confirmPasswordField}
		<div class="space-y-2">
			<label for="confirmPassword" class="text-secondary block text-sm font-medium">
				{confirmLabel}
				{#if required}<span class="text-red-400">*</span>{/if}
			</label>
			<input
				id="confirmPassword"
				type="password"
				value={confirmPasswordField.state.value}
				onblur={() => confirmPasswordField.handleBlur()}
				oninput={(e) => confirmPasswordField.handleChange(e.currentTarget.value)}
				placeholder="Re-enter your password"
				class="input-field"
				class:input-field-error={showConfirmErrors}
			/>
			{#if showConfirmErrors}
				<div class="text-danger flex items-center gap-2">
					<AlertCircle size={16} />
					<p class="text-xs">{confirmErrors[0]}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>
