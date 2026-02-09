<script lang="ts">
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { DockerDiscovery, NetworkDiscovery, SelfReportDiscovery } from '../../types/api';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { billingPlans, discoveryTypes, subnetTypes } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { showBillingPlanModal } from '$lib/features/billing/stores';
	import { ArrowUpCircle } from 'lucide-svelte';
	import type { Component } from 'svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { generateCronSchedule } from '../../queries';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_days,
		common_hours,
		common_ipAddress,
		discovery_adHoc,
		discovery_adHocDescription,
		discovery_allSubnetsScanned,
		discovery_bestService,
		discovery_configuration,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_discoveryType,
		discovery_dockerScan,
		discovery_hostNameFallback,
		discovery_hostNameFallbackHelp,
		discovery_manualDiscovery,
		discovery_manualDiscoveryHelp,
		discovery_networkScan,
		discovery_noDays,
		discovery_noHours,
		discovery_nonInterfacedSubnet,
		discovery_nonInterfacedSubnetWarning,
		discovery_oneDay,
		discovery_oneHour,
		discovery_runType,
		discovery_scheduleConfiguration,
		discovery_scheduleHelp,
		discovery_scheduled,
		discovery_scheduledDescription,
		discovery_selectSubnet,
		discovery_selfReport,
		discovery_targetSubnets,
		discovery_targetSubnetsHelp,
		discovery_xDays,
		discovery_xHours
	} from '$lib/paraglide/messages';

	// Props
	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; state: { values: Record<string, any> } };
		formData: Discovery;
		readOnly?: boolean;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { form, formData = $bindable(), readOnly = false, daemonHostId, daemon }: Props = $props();

	// Queries
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let hasScheduledDiscovery = $derived.by(() => {
		if (!org?.plan?.type) return true;
		return billingPlans.getMetadata(org.plan.type).features.scheduled_discovery;
	});

	const subnetsQuery = useSubnetsQuery();

	// Derived data
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Discovery type options
	let discoveryTypeOptions = $derived([
		{ value: 'Network', label: discovery_networkScan(), disabled: false },
		{
			value: 'Docker',
			label: discovery_dockerScan(),
			disabled: daemonHostId == null || !daemon.capabilities.has_docker_socket
		},
		{ value: 'SelfReport', label: discovery_selfReport(), disabled: daemonHostId == null }
	]);

	let hostNameFallbackOptions = $derived([
		{ value: 'Ip', label: common_ipAddress() },
		{ value: 'BestService', label: discovery_bestService() }
	]);

	let runTypeOptions: SimpleOption[] = $derived([
		{ value: 'AdHoc', label: discovery_adHoc() },
		{
			value: 'Scheduled',
			label: discovery_scheduled(),
			disabled: !hasScheduledDiscovery,
			tags: !hasScheduledDiscovery
				? [
						{
							label: 'Upgrade',
							color: 'Yellow',
							icon: ArrowUpCircle as unknown as Component
						}
					]
				: []
		}
	]);

	// Handle run type changes - update formData when form field changes
	function handleRunTypeChange(value: string) {
		if (value === 'AdHoc' && formData.run_type.type !== 'AdHoc') {
			formData.run_type = {
				type: 'AdHoc',
				last_run: null
			};
		} else if (value === 'Scheduled' && formData.run_type.type !== 'Scheduled') {
			formData.run_type = {
				type: 'Scheduled',
				cron_schedule: '0 0 */1 * * *', // Default: every hour
				last_run: null,
				enabled: true
			};
		}
	}

	// Handle discovery type changes - update formData when form field changes
	function handleDiscoveryTypeChange(value: string) {
		if (value === 'Network' && formData.discovery_type.type !== 'Network') {
			formData.discovery_type = {
				type: 'Network',
				subnet_ids: daemon.capabilities.interfaced_subnet_ids,
				host_naming_fallback: 'BestService',
				probe_raw_socket_ports: false
			} as NetworkDiscovery;
		} else if (value === 'Docker' && formData.discovery_type.type !== 'Docker') {
			formData.discovery_type = {
				type: 'Docker',
				host_id: daemonHostId,
				host_naming_fallback: 'BestService'
			} as DockerDiscovery;
		} else if (value === 'SelfReport' && formData.discovery_type.type !== 'SelfReport') {
			formData.discovery_type = {
				type: 'SelfReport',
				host_id: daemonHostId
			} as SelfReportDiscovery;
		}
	}

	// Handle host naming fallback changes
	function handleHostNameFallbackChange(value: string) {
		if (formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network') {
			if (formData.discovery_type.host_naming_fallback !== value) {
				formData.discovery_type = {
					...formData.discovery_type,
					host_naming_fallback: value as 'BestService' | 'Ip'
				};
			}
		}
	}

	// Handle schedule changes - update cron from days/hours
	function handleScheduleChange(days: number, hours: number) {
		if (formData.run_type.type === 'Scheduled') {
			const totalHours = days * 24 + hours;
			formData.run_type = {
				...formData.run_type,
				cron_schedule: generateCronSchedule(totalHours)
			};
		}
	}

	// Subnet management for Network
	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				formData.discovery_type.type === 'Network' &&
				s.network_id == formData.network_id &&
				!formData.discovery_type.subnet_ids?.includes(s.id) &&
				subnetTypes.getMetadata(s.subnet_type).network_scan_discovery_eligible
		)
	);

	let selectedSubnets = $derived(
		formData.discovery_type.type === 'Network' && formData.discovery_type.subnet_ids
			? formData.discovery_type.subnet_ids
					.map((id) => subnetsData.find((s) => s.id === id))
					.filter(Boolean)
			: []
	);

	let nonInterfacedSubnets = $derived(
		formData.discovery_type.type == 'Network' &&
			formData.discovery_type.subnet_ids &&
			formData.discovery_type.subnet_ids.length > 0
			? formData.discovery_type.subnet_ids
					.filter((s) => !daemon.capabilities.interfaced_subnet_ids.includes(s))
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
					.map((s) => s.name + ` (${s.cidr})`)
			: []
	);

	function handleAddSubnet(subnetId: string) {
		if (formData.discovery_type.type === 'Network') {
			const currentIds = formData.discovery_type.subnet_ids || [];
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: [...currentIds, subnetId]
			};
		}
	}

	function handleRemoveSubnet(index: number) {
		if (formData.discovery_type.type === 'Network' && formData.discovery_type.subnet_ids) {
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: formData.discovery_type.subnet_ids.filter((_, i) => i !== index)
			};
		}
	}

	// Services affected by raw socket port filtering
	let rawSocketServiceNames = $derived(
		(serviceDefinitions.getItems() ?? [])
			.filter((s) => s.metadata?.has_raw_socket_endpoint)
			.map((s) => s.name)
			.join(', ')
	);

	// Handle probe_raw_socket_ports toggle
	function handleProbeRawSocketPortsChange(value: boolean) {
		if (formData.discovery_type.type === 'Network') {
			formData.discovery_type = {
				...formData.discovery_type,
				probe_raw_socket_ports: value
			};
		}
	}

	// Day and hour options for schedule
	let dayOptions = $derived(
		Array.from({ length: 31 }, (_, i) => ({
			value: String(i),
			label:
				i === 0 ? discovery_noDays() : i === 1 ? discovery_oneDay() : discovery_xDays({ count: i })
		}))
	);

	let hourOptions = $derived(
		Array.from({ length: 24 }, (_, i) => ({
			value: String(i),
			label:
				i === 0
					? discovery_noHours()
					: i === 1
						? discovery_oneHour()
						: discovery_xHours({ count: i })
		}))
	);
</script>

<div class="space-y-6">
	<div class="border-t border-gray-700 pt-6">
		<h3 class="text-primary mb-4 text-lg font-medium">{discovery_configuration()}</h3>

		<div class="space-y-4">
			<!-- Run Type Selection -->
			<form.Field
				name="run_type_type"
				listeners={{
					onChange: ({ value }: { value: string }) => handleRunTypeChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<RichSelect
						label={discovery_runType()}
						selectedValue={field.state.value}
						options={runTypeOptions}
						onSelect={(value) => field.handleChange(value)}
						onDisabledClick={() => showBillingPlanModal.set(true)}
						displayComponent={SimpleOptionDisplay}
						disabled={readOnly}
					/>
					<p class="text-tertiary mt-1 text-xs">
						{field.state.value === 'AdHoc'
							? discovery_adHocDescription()
							: discovery_scheduledDescription()}
					</p>
				{/snippet}
			</form.Field>

			<!-- Discovery Type Selection -->
			<form.Field
				name="discovery_type_type"
				listeners={{
					onChange: ({ value }: { value: string }) => handleDiscoveryTypeChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<SelectInput
						label={discovery_discoveryType()}
						id="discovery_type"
						options={discoveryTypeOptions}
						{field}
						disabled={readOnly}
					/>
					<p class="text-tertiary mt-1 text-xs">
						{discoveryTypes.getDescription(field.state.value)}
					</p>
				{/snippet}
			</form.Field>

			{#if daemonHostId == null}
				<InlineWarning
					title={discovery_daemonHostMissing()}
					body={discovery_daemonHostMissingHelp()}
				/>
			{/if}

			<!-- Type-specific configuration -->
			{#if formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network'}
				<form.Field
					name="host_naming_fallback"
					listeners={{
						onChange: ({ value }: { value: string }) => handleHostNameFallbackChange(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<SelectInput
							label={discovery_hostNameFallback()}
							id="host_name_fallback"
							options={hostNameFallbackOptions}
							{field}
							disabled={readOnly}
							helpText={discovery_hostNameFallbackHelp()}
						/>
					{/snippet}
				</form.Field>
			{/if}

			{#if formData.discovery_type.type === 'Network'}
				<div class="rounded-lg bg-gray-800/50 p-4">
					<ListManager
						label={discovery_targetSubnets()}
						helpText={discovery_targetSubnetsHelp()}
						placeholder={discovery_selectSubnet()}
						emptyMessage={discovery_allSubnetsScanned()}
						allowReorder={false}
						allowItemEdit={() => false}
						showSearch={true}
						options={availableSubnets}
						items={selectedSubnets}
						optionDisplayComponent={SubnetDisplay}
						itemDisplayComponent={SubnetDisplay}
						onAdd={handleAddSubnet}
						onRemove={handleRemoveSubnet}
					/>
				</div>
				{#if nonInterfacedSubnets.length > 0}
					<InlineWarning
						title={discovery_nonInterfacedSubnet()}
						body={discovery_nonInterfacedSubnetWarning({
							subnets: nonInterfacedSubnets.join('\n')
						})}
					/>
				{/if}
				<form.Field
					name="probe_raw_socket_ports"
					listeners={{
						onChange: ({ value }: { value: boolean }) => handleProbeRawSocketPortsChange(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<Checkbox
							label="Probe raw socket ports (9100-9107)"
							id="probe_raw_socket_ports"
							{field}
							disabled={readOnly}
							helpText={rawSocketServiceNames
								? `May cause ghost printing on JetDirect printers. Required to detect: ${rawSocketServiceNames}`
								: 'May cause ghost printing on JetDirect printers'}
						/>
					{/snippet}
				</form.Field>
			{/if}
		</div>
	</div>

	<!-- Frequency Configuration (only for scheduled runs) -->
	{#if formData.run_type.type === 'Scheduled'}
		<div class="border-t border-gray-700 pt-6">
			<h3 class="text-primary mb-4 text-lg font-medium">{discovery_scheduleConfiguration()}</h3>

			<div class="space-y-4">
				<p class="text-tertiary text-sm">
					{discovery_scheduleHelp()}
				</p>

				<div class="grid grid-cols-2 gap-4">
					<form.Field
						name="schedule_days"
						listeners={{
							onChange: ({ value }: { value: string }) => {
								const hours = form.state.values.schedule_hours || '0';
								handleScheduleChange(parseInt(value), parseInt(hours));
							}
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<SelectInput
								label={common_days()}
								id="frequency_days"
								options={dayOptions}
								{field}
								disabled={readOnly}
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="schedule_hours"
						listeners={{
							onChange: ({ value }: { value: string }) => {
								const days = form.state.values.schedule_days || '1';
								handleScheduleChange(parseInt(days), parseInt(value));
							}
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<SelectInput
								label={common_hours()}
								id="frequency_hours"
								options={hourOptions}
								{field}
								disabled={readOnly}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>
	{:else if formData.run_type.type === 'AdHoc'}
		<div class="border-t border-gray-700 pt-6">
			<div class="rounded-lg bg-gray-800/50 p-4">
				<div class="flex items-start gap-3">
					<svg
						class="mt-0.5 h-5 w-5 flex-shrink-0 text-gray-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<div>
						<h4 class="mb-1 text-sm font-medium text-gray-300">{discovery_manualDiscovery()}</h4>
						<p class="text-sm text-gray-400">
							{discovery_manualDiscoveryHelp()}
						</p>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
