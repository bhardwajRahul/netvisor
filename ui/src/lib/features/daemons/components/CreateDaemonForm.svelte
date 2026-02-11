<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import { ArrowUpCircle, ChevronDown, ChevronRight, RotateCcwKey } from 'lucide-svelte';

	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { showBillingPlanModal } from '$lib/features/billing/stores';
	import { fieldDefs } from '../config';
	import type { Daemon } from '../types/base';
	import {
		common_apiKey,
		common_generateKey,
		common_pressGenerateKey,
		daemons_advancedConfiguration,
		daemons_apiKeyHelp,
		daemons_fixValidationErrors,
		daemons_fixValidationErrorsBody,
		daemons_generateNewKey,
		daemons_generateNewKeyHelp,
		daemons_linuxOnly,
		daemons_option1,
		daemons_option1Text,
		daemons_option2,
		daemons_option2Text,
		daemons_pasteApiKey,
		daemons_useExistingKey,
		daemons_useExistingKeyHelp,
		daemons_useKey
	} from '$lib/paraglide/messages';

	interface Props {
		daemon?: Daemon | null;
		networkId: string;
		apiKey?: string | null;
		showAdvanced?: boolean;
		initialName?: string;
		showModeSelect?: boolean;
		// API key source selection props
		allowExistingKey?: boolean;
		keySet?: boolean;
		onGenerateKey?: () => void;
		onUseExistingKey?: () => void;
	}

	let {
		daemon = null,
		networkId,
		apiKey = null,
		showAdvanced = true,
		initialName = '',
		showModeSelect = true,
		allowExistingKey = false,
		keySet = false,
		onGenerateKey,
		onUseExistingKey
	}: Props = $props();

	const configQuery = useConfigQuery();
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();

	// Get current user ID for user_id field
	let currentUserId = $derived(currentUserQuery.data?.id ?? null);

	let org = $derived(organizationQuery.data);
	let hasDaemonPoll = $derived.by(() => {
		if (!org?.plan?.type) return true;
		return billingPlans.getMetadata(org.plan.type).features.daemon_poll;
	});

	// Separate field defs - conditionally exclude mode and daemonUrl if showModeSelect is false
	// (daemonUrl depends on mode selection, so both should be hidden until Install Now)
	let basicFieldDefs = $derived(
		fieldDefs.filter((d) => !d.section && (!['mode', 'daemonUrl'].includes(d.id) || showModeSelect))
	);
	const advancedFieldDefs = fieldDefs.filter((d) => d.section);

	// Get unique section names in order of appearance (compare by return value, not function reference)
	const sectionNames = [...new Set(advancedFieldDefs.map((d) => d.section!()))];

	// Group advanced fields by section (compare by return value)
	const advancedSections = sectionNames.map((name) => ({
		name: () => name,
		fields: advancedFieldDefs.filter((d) => d.section!() === name)
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
		// Add UI state fields (not part of daemon config, just for form interaction)
		defaults.keySource = 'generate';
		defaults.existingKeyInput = '';
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

	let runCommand = $derived(
		buildRunCommand(serverUrl, networkId, apiKey, formValues, daemon, currentUserId)
	);
	let dockerCompose = $derived(
		apiKey ? buildDockerCompose(serverUrl, networkId, apiKey, formValues, currentUserId) : ''
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
		daemon: Daemon | null,
		userId: string | null
	): string {
		let cmd = `sudo scanopy-daemon --server-url ${serverUrl}`;

		if (!daemon && networkId) {
			cmd += ` --network-id ${networkId}`;
		}

		if (key) {
			cmd += ` --daemon-api-key ${key}`;
		}

		// Include user_id for new daemon registrations
		if (!daemon && userId) {
			cmd += ` --user-id ${userId}`;
		}

		const mode = values['mode'] as string;

		for (const def of fieldDefs) {
			const value = values[def.id];

			if (def.docsOnly) {
				continue;
			}

			// Skip daemonUrl - only used for provisioning, not in daemon config
			if (def.id === 'daemonUrl') {
				continue;
			}

			// Skip daemonPort for DaemonPoll mode (server never connects to daemon)
			if (def.id === 'daemonPort' && mode === 'daemon_poll') {
				continue;
			}

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
		values: Record<string, string | number | boolean>,
		userId: string | null
	): string {
		const envVars: string[] = [`SCANOPY_SERVER_URL=${serverUrl}`, `SCANOPY_DAEMON_API_KEY=${key}`];

		if (networkId) {
			envVars.splice(1, 0, `SCANOPY_NETWORK_ID=${networkId}`);
		}

		// Include user_id for new daemon registrations
		if (userId) {
			envVars.push(`SCANOPY_USER_ID=${userId}`);
		}

		const mode = values['mode'] as string;

		for (const def of fieldDefs) {
			const value = values[def.id];

			if (def.docsOnly) {
				continue;
			}

			// Skip daemonUrl - only used for provisioning, not in daemon config
			if (def.id === 'daemonUrl') {
				continue;
			}

			// Skip daemonPort for DaemonPoll mode (server never connects to daemon)
			if (def.id === 'daemonPort' && mode === 'daemon_poll') {
				continue;
			}

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

	// Export the existing key input value for parent components
	export function getExistingKeyInput(): string {
		return (form.state.values['existingKeyInput'] as string) ?? '';
	}

	// Export form for parent access
	export function getForm() {
		return form;
	}

	// Export the daemon port value for parent components
	export function getDaemonPort(): number {
		const port = form.state.values['daemonPort'];
		return typeof port === 'number' ? port : 60073;
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
		{#if !def.showWhen || def.showWhen(formValues)}
			{#if def.docsOnly}
				<div></div>
			{:else if def.type === 'string'}
				<form.Field name={def.id} validators={getValidators(def.id)}>
					{#snippet children(field)}
						<TextInput
							label={def.label()}
							{field}
							id={def.id}
							placeholder={String(
								typeof def.placeholder === 'function' ? def.placeholder() : (def.placeholder ?? '')
							)}
							required={def.required ?? false}
							helpText={def.helpText()}
						/>
					{/snippet}
				</form.Field>
			{:else if def.type === 'number'}
				<form.Field name={def.id} validators={getValidators(def.id)}>
					{#snippet children(field)}
						<TextInput
							label={def.label()}
							{field}
							id={def.id}
							type="number"
							placeholder={String(
								typeof def.placeholder === 'function' ? def.placeholder() : (def.placeholder ?? '')
							)}
							required={def.required ?? false}
							helpText={def.helpText()}
						/>
					{/snippet}
				</form.Field>
			{:else if def.type === 'select'}
				<form.Field name={def.id}>
					{#snippet children(field)}
						{#if def.id === 'mode'}
							<RichSelect
								label={def.label()}
								selectedValue={String(field.state.value ?? '')}
								options={(def.options ?? []).map((opt): SimpleOption => {
									const needsUpgrade = opt.value === 'daemon_poll' && !hasDaemonPoll;
									return {
										value: opt.value,
										label: opt.label(),
										description:
											opt.value === 'daemon_poll'
												? 'Daemon connects to server; works behind NAT/firewall without opening ports'
												: 'Server connects to daemon; requires providing Daemon URL',
										disabled: needsUpgrade,
										tags: needsUpgrade
											? [
													{
														label: 'Upgrade',
														color: 'Yellow',
														icon: ArrowUpCircle
													}
												]
											: []
									};
								})}
								onSelect={(value) => field.handleChange(value)}
								onDisabledClick={() => showBillingPlanModal.set(true)}
								displayComponent={SimpleOptionDisplay}
								disabled={def.disabled?.(isNewDaemon) ?? false}
							/>
						{:else}
							<SelectInput
								label={def.label()}
								{field}
								id={def.id}
								options={(def.options ?? []).map((opt) => ({
									value: opt.value,
									label: opt.label()
								}))}
								helpText={def.helpText()}
								disabled={def.disabled?.(isNewDaemon) ?? false}
							/>
						{/if}
					{/snippet}
				</form.Field>
			{/if}
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
				{daemons_advancedConfiguration()}
			</button>

			{#if advancedExpanded}
				<div class="mt-4 space-y-6">
					{#each advancedSections as section (section.name)}
						<div class="card card-static">
							<div class="text-secondary text-m mb-3 font-medium">{section.name()}</div>
							<div class="grid grid-cols-2 gap-4">
								{#each section.fields as def (def.id)}
									{#if !def.showWhen || def.showWhen(formValues)}
										{#if def.docsOnly}
											<div></div>
										{:else if def.type === 'string'}
											<form.Field name={def.id} validators={getValidators(def.id)}>
												{#snippet children(field)}
													<TextInput
														label={def.label()}
														{field}
														id={def.id}
														placeholder={String(
															typeof def.placeholder === 'function'
																? def.placeholder()
																: (def.placeholder ?? '')
														)}
														helpText={def.helpText()}
													/>
												{/snippet}
											</form.Field>
										{:else if def.type === 'number'}
											<form.Field name={def.id} validators={getValidators(def.id)}>
												{#snippet children(field)}
													<TextInput
														label={def.label()}
														{field}
														id={def.id}
														type="number"
														placeholder={String(
															typeof def.placeholder === 'function'
																? def.placeholder()
																: (def.placeholder ?? '')
														)}
														helpText={def.helpText()}
													/>
												{/snippet}
											</form.Field>
										{:else if def.type === 'select'}
											<form.Field name={def.id}>
												{#snippet children(field)}
													<SelectInput
														label={def.label()}
														{field}
														id={def.id}
														options={(def.options ?? []).map((opt) => ({
															value: opt.value,
															label: opt.label()
														}))}
														helpText={def.helpText()}
													/>
												{/snippet}
											</form.Field>
										{:else if def.type === 'boolean'}
											<form.Field name={def.id}>
												{#snippet children(field)}
													<Checkbox
														label={def.label()}
														{field}
														id={def.id}
														helpText={def.helpText()}
													/>
												{/snippet}
											</form.Field>
										{/if}
									{/if}
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- API Key Source Selection (only when allowExistingKey is true) -->
	{#if allowExistingKey}
		<div class="space-y-3 pb-2">
			<form.Field name="keySource">
				{#snippet children(field)}
					<RadioGroup
						label={common_apiKey()}
						id="key-source"
						{field}
						options={[
							{
								value: 'generate',
								label: daemons_generateNewKey(),
								helpText: daemons_generateNewKeyHelp()
							},
							{
								value: 'existing',
								label: daemons_useExistingKey(),
								helpText: daemons_useExistingKeyHelp()
							}
						]}
						disabled={keySet}
					/>
				{/snippet}
			</form.Field>

			{#if formValues.keySource === 'generate'}
				<!-- Generate new key flow -->
				<div class="flex items-start gap-2">
					<button
						class="btn-primary m-1 flex-shrink-0 self-stretch"
						disabled={keySet}
						type="button"
						onclick={() => onGenerateKey?.()}
					>
						<RotateCcwKey />
						<span>{common_generateKey()}</span>
					</button>

					<div class="flex-1">
						<CodeContainer
							language="bash"
							expandable={false}
							code={apiKey ? apiKey : common_pressGenerateKey()}
						/>
					</div>
				</div>
				{#if !apiKey}
					<div class="text-tertiary mt-1 text-xs">
						{daemons_apiKeyHelp()}
					</div>
				{/if}
			{:else}
				<!-- Use existing key flow -->
				<form.Field name="existingKeyInput">
					{#snippet children(field)}
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<TextInput
									label=""
									{field}
									id="existing-key-input"
									placeholder={daemons_pasteApiKey()}
									disabled={keySet}
								/>
							</div>
							<button
								class="btn-primary flex-shrink-0"
								disabled={keySet || !String(formValues.existingKeyInput ?? '').trim()}
								type="button"
								onclick={() => onUseExistingKey?.()}
							>
								<span>{daemons_useKey()}</span>
							</button>
						</div>
					{/snippet}
				</form.Field>
				{#if apiKey}
					<div class="mt-2">
						<CodeContainer language="bash" expandable={false} code={apiKey} />
					</div>
				{/if}
			{/if}
		</div>
	{/if}

	<!-- Installation Instructions (shown when API key is available) -->
	{#if apiKey}
		{#if hasErrors}
			<InlineWarning
				title={daemons_fixValidationErrors()}
				body={daemons_fixValidationErrorsBody()}
			/>
		{:else}
			<div class="space-y-4">
				<div class="text-secondary">
					<b>{daemons_option1()}</b>
					{daemons_option1Text()}
				</div>
				<CodeContainer language="bash" expandable={false} code={installScript} />
				<CodeContainer language="bash" expandable={false} code={runCommand} />

				<div class="text-secondary">
					<b>{daemons_option2()}</b>
					{daemons_option2Text()}
					<span class="text-tertiary">{daemons_linuxOnly()}</span>
				</div>
				<CodeContainer language="yaml" expandable={false} code={dockerCompose} />
			</div>
		{/if}
	{/if}
</div>
