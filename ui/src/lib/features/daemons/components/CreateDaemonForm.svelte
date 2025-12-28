<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { fieldDefs } from '../config';
	import type { Daemon } from '../types/base';

	interface Props {
		daemon?: Daemon | null;
		networkId: string;
		apiKey?: string | null;
		showAdvanced?: boolean;
		initialName?: string;
		showModeSelect?: boolean;
	}

	let {
		daemon = null,
		networkId,
		apiKey = null,
		showAdvanced = true,
		initialName = '',
		showModeSelect = true
	}: Props = $props();

	const configQuery = useConfigQuery();

	// Separate field defs - conditionally exclude mode if showModeSelect is false
	let basicFieldDefs = $derived(
		fieldDefs.filter((d) => !d.section && (d.id !== 'mode' || showModeSelect))
	);
	const advancedFieldDefs = fieldDefs.filter((d) => d.section);

	// Get unique section names in order of appearance
	const sectionNames = [...new Set(advancedFieldDefs.map((d) => d.section!))];

	// Group advanced fields by section
	const advancedSections = sectionNames.map((name) => ({
		name,
		fields: advancedFieldDefs.filter((d) => d.section === name)
	}));

	// Track which sections are expanded
	let advancedExpanded = $state(false);

	// Build default values from field definitions
	function buildDefaultValues(): Record<string, string | number | boolean> {
		const defaults: Record<string, string | number | boolean> = {};
		for (const def of fieldDefs) {
			if (def.id === 'name' && initialName) {
				defaults[def.id] = initialName;
			} else if (daemon) {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				defaults[def.id] = (daemon as any)[def.id] ?? def.defaultValue ?? '';
			} else {
				defaults[def.id] = def.defaultValue ?? '';
			}
		}
		return defaults;
	}

	// Create TanStack Form
	const form = createForm(() => ({
		defaultValues: buildDefaultValues(),
		onSubmit: async () => {
			// Form submission is handled by parent component
		}
	}));

	// Get validators for a field
	function getValidators(fieldId: string) {
		const def = fieldDefs.find((d) => d.id === fieldId);
		if (!def?.validators || def.validators.length === 0) return {};

		return {
			onBlur: ({ value }: { value: FormValue }) => {
				for (const validator of def.validators!) {
					const error = validator(value);
					if (error) return error;
				}
				return undefined;
			}
		};
	}

	let isNewDaemon = $derived(daemon === null);
	let serverUrl = $derived(configQuery.data?.public_url ?? '');

	const installScript = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	// Local state for form values to enable Svelte 5 reactivity
	// (form.state.values is NOT tracked by $derived)
	let formValues = $state<Record<string, string | number | boolean>>(buildDefaultValues());

	// Subscribe to form store changes to keep formValues in sync
	$effect(() => {
		return form.store.subscribe(() => {
			formValues = { ...form.state.values } as Record<string, string | number | boolean>;
		});
	});

	let runCommand = $derived(buildRunCommand(serverUrl, networkId, apiKey, formValues, daemon));
	let dockerCompose = $derived(
		apiKey ? buildDockerCompose(serverUrl, networkId, apiKey, formValues) : ''
	);

	// Check if a field value passes all its validators
	function fieldPassesValidation(def: (typeof fieldDefs)[0], value: FormValue): boolean {
		if (!def.validators || def.validators.length === 0) return true;
		for (const validator of def.validators) {
			const error = validator(value);
			if (error) return false;
		}
		return true;
	}

	function buildRunCommand(
		serverUrl: string,
		networkId: string,
		key: string | null,
		values: Record<string, string | number | boolean>,
		daemon: Daemon | null
	): string {
		let cmd = `sudo scanopy-daemon --server-url ${serverUrl}`;

		if (!daemon && networkId) {
			cmd += ` --network-id ${networkId}`;
		}

		if (key) {
			cmd += ` --daemon-api-key ${key}`;
		}

		for (const def of fieldDefs) {
			const value = values[def.id];

			if (value === '' || value === null || value === undefined) {
				continue;
			}

			// Skip fields that don't pass validation
			if (!fieldPassesValidation(def, value)) {
				continue;
			}

			// Skip advanced fields (those with a section) that match their default value
			if (def.section && value === def.defaultValue) {
				continue;
			}

			if (def.id === 'mode') {
				cmd += ` ${def.cliFlag} ${String(value).toLowerCase()}`;
			} else if (def.type === 'boolean') {
				if (value) cmd += ` ${def.cliFlag} true`;
			} else {
				cmd += ` ${def.cliFlag} ${value}`;
			}
		}

		return cmd;
	}

	function buildDockerCompose(
		serverUrl: string,
		networkId: string,
		key: string,
		values: Record<string, string | number | boolean>
	): string {
		const envVars: string[] = [`SCANOPY_SERVER_URL=${serverUrl}`, `SCANOPY_DAEMON_API_KEY=${key}`];

		if (networkId) {
			envVars.splice(1, 0, `SCANOPY_NETWORK_ID=${networkId}`);
		}

		for (const def of fieldDefs) {
			const value = values[def.id];

			if (value === '' || value === null || value === undefined) {
				continue;
			}

			// Skip fields that don't pass validation
			if (!fieldPassesValidation(def, value)) {
				continue;
			}

			// Skip advanced fields (those with a section) that match their default value
			if (def.section && value === def.defaultValue) {
				continue;
			}

			if (def.type === 'boolean') {
				if (value) envVars.push(`${def.envVar}=true`);
			} else {
				envVars.push(`${def.envVar}=${value}`);
			}
		}

		const dockerProxyDef = fieldDefs.find((d) => d.id === 'dockerProxy');
		const hasDockerProxy =
			values.dockerProxy &&
			values.dockerProxy !== '' &&
			(!dockerProxyDef || fieldPassesValidation(dockerProxyDef, values.dockerProxy));
		const volumeMounts = ['daemon-config:/root/.config/daemon'];
		if (!hasDockerProxy) {
			volumeMounts.push('/var/run/docker.sock:/var/run/docker.sock:ro');
		}

		const lines = [
			'services:',
			'  daemon:',
			'    image: ghcr.io/scanopy/scanopy/daemon:latest',
			'    container_name: scanopy-daemon',
			'    network_mode: host',
			'    privileged: true',
			'    restart: unless-stopped',
			'    environment:',
			...envVars.map((v) => `      - ${v}`),
			'    volumes:',
			...volumeMounts.map((v) => `      - ${v}`),
			'',
			'volumes:',
			'  daemon-config:'
		];

		return lines.join('\n');
	}

	// Export validate function for parent components - uses shared validateForm
	export async function validate(): Promise<boolean> {
		return await validateForm(form);
	}

	// Export the daemon name value for parent components
	export function getDaemonName(): string {
		return form.state.values['name'] as string;
	}

	// Export form for parent access
	export function getForm() {
		return form;
	}

	// Check if form has validation errors (after fields have been validated)
	let hasErrors = $derived.by(() => {
		const fieldMeta = form.state.fieldMeta;
		for (const key of Object.keys(fieldMeta)) {
			const meta = fieldMeta[key];
			if (meta?.errors && meta.errors.length > 0) {
				return true;
			}
		}
		return false;
	});
</script>

<div class="space-y-4">
	<!-- Basic Fields -->
	{#each basicFieldDefs as def (def.id)}
		{#if def.type === 'string'}
			<form.Field name={def.id} validators={getValidators(def.id)}>
				{#snippet children(field)}
					<TextInput
						label={def.label}
						{field}
						id={def.id}
						placeholder={String(def.placeholder ?? '')}
						required={def.required ?? false}
						helpText={def.helpText}
					/>
				{/snippet}
			</form.Field>
		{:else if def.type === 'select'}
			<form.Field name={def.id}>
				{#snippet children(field)}
					<SelectInput
						label={def.label}
						{field}
						id={def.id}
						options={def.options ?? []}
						helpText={def.helpText}
						disabled={def.disabled?.(isNewDaemon) ?? false}
					/>
				{/snippet}
			</form.Field>
		{/if}
	{/each}

	<!-- Advanced Configuration -->
	{#if showAdvanced}
		<div class="border-tertiary border-t pt-4">
			<button
				type="button"
				class="text-secondary hover:text-primary flex w-full items-center gap-2 text-sm font-medium"
				onclick={() => (advancedExpanded = !advancedExpanded)}
			>
				{#if advancedExpanded}
					<ChevronDown class="h-4 w-4" />
				{:else}
					<ChevronRight class="h-4 w-4" />
				{/if}
				Advanced Configuration
			</button>

			{#if advancedExpanded}
				<div class="mt-4 space-y-6">
					{#each advancedSections as section (section.name)}
						<div class="card card-static">
							<div class="text-secondary text-m mb-3 font-medium">{section.name}</div>
							<div class="grid grid-cols-2 gap-4">
								{#each section.fields as def (def.id)}
									{#if def.type === 'string'}
										<form.Field name={def.id} validators={getValidators(def.id)}>
											{#snippet children(field)}
												<TextInput
													label={def.label}
													{field}
													id={def.id}
													placeholder={String(def.placeholder ?? '')}
													helpText={def.helpText}
												/>
											{/snippet}
										</form.Field>
									{:else if def.type === 'number'}
										<form.Field name={def.id} validators={getValidators(def.id)}>
											{#snippet children(field)}
												<TextInput
													label={def.label}
													{field}
													id={def.id}
													type="number"
													placeholder={String(def.placeholder ?? '')}
													helpText={def.helpText}
												/>
											{/snippet}
										</form.Field>
									{:else if def.type === 'select'}
										<form.Field name={def.id}>
											{#snippet children(field)}
												<SelectInput
													label={def.label}
													{field}
													id={def.id}
													options={def.options ?? []}
													helpText={def.helpText}
												/>
											{/snippet}
										</form.Field>
									{:else if def.type === 'boolean'}
										<form.Field name={def.id}>
											{#snippet children(field)}
												<Checkbox label={def.label} {field} id={def.id} helpText={def.helpText} />
											{/snippet}
										</form.Field>
									{/if}
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Installation Instructions (shown when API key is available) -->
	{#if apiKey}
		{#if hasErrors}
			<InlineWarning
				title="Please fix validation errors"
				body="Correct the field validation issues above before using the installation commands."
			/>
		{:else}
			<div class="space-y-4">
				<div class="text-secondary">
					<b>Option 1.</b> Run the install script, then start the daemon
				</div>
				<CodeContainer language="bash" expandable={false} code={installScript} />
				<CodeContainer language="bash" expandable={false} code={runCommand} />

				<div class="text-secondary">
					<b>Option 2.</b> Run with Docker Compose
					<span class="text-tertiary">(Linux only)</span>
				</div>
				<CodeContainer language="yaml" expandable={false} code={dockerCompose} />
			</div>
		{/if}
	{/if}
</div>
