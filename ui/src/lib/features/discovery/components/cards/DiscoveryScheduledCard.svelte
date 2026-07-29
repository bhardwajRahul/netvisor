<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import AnimatedProgressBar from './AnimatedProgressBar.svelte';
	import DiscoveryEstimation from '../DiscoveryEstimation.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { Edit, Loader2, Play, Power, Trash2, X } from 'lucide-svelte';
	import type { Discovery } from '../../types/base';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { cancellingSessions } from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { Host } from '$lib/features/hosts/types/base';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { formatScheduleDisplay } from '../../queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import {
		common_delete,
		common_disable,
		common_enable,
		common_legacy,
		common_tags,
		discovery_alreadyRunning,
		discovery_cannotDeleteWhileRunning,
		discovery_cannotToggleWhileRunning,
		discovery_disableScheduleTooltip,
		discovery_enableScheduleTooltip,
		discovery_schedulePausedFreePlan
	} from '$lib/paraglide/messages';

	// Queries
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();
	const subnetsQuery = useSubnetsQuery();
	const credentialsQuery = useCredentialsQuery();
	const organizationQuery = useOrganizationQuery();

	// Derived data
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let org = $derived(organizationQuery.data);
	let schedulePaused = $derived.by(() => {
		if (!org?.plan?.type) return false;
		return !billingPlans.getMetadata(org.plan.type).features.scheduled_discovery;
	});

	let {
		viewMode,
		discovery,
		hosts = [],
		activeSession = null,
		onEdit,
		onDelete,
		onRun,
		onCancel,
		onToggleEnabled,
		selected,
		onSelectionChange = () => {}
	}: {
		viewMode: 'card' | 'list';
		discovery: Discovery;
		/**
		 * Hosts the daemons in this list run on. Passed in rather than fetched
		 * here: each card needs one host name for a popover, and fetching per card
		 * meant every card subscribing to an unpaginated org-wide hosts query.
		 */
		hosts?: Host[];
		activeSession?: DiscoveryUpdatePayload | null;
		onEdit?: (discovery: Discovery) => void;
		onDelete?: (discovery: Discovery) => void;
		onRun?: (discovery: Discovery) => void;
		onCancel?: (sessionId: string) => void;
		onToggleEnabled?: (discovery: Discovery) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let hostsData = $derived(hosts);

	let isEnabled = $derived(discovery.run_type.type === 'Scheduled' && discovery.run_type.enabled);
	let hasActiveSession = $derived(!!activeSession);
	let isCancelling = $derived(
		activeSession?.session_id ? $cancellingSessions.get(activeSession.session_id) === true : false
	);

	let legacyStatus: TagProps | null = $derived(
		discovery.discovery_type.type !== 'Unified' ? { label: common_legacy(), color: 'Yellow' } : null
	);

	let cardData = $derived({
		title: discovery.name,
		status: legacyStatus,
		iconColor: entities.getColorHelper('Discovery').icon,
		Icon: entities.getIconComponent('Discovery'),
		fields: [
			{
				label: 'Daemon',
				value: (() => {
					const daemon = daemonsData.find((d) => d.id == discovery.daemon_id);
					if (!daemon) return 'Unknown Daemon';
					return [
						{
							id: daemon.id,
							label: daemon.name,
							color: entities.getColorHelper('Daemon').color,
							entityRef: entityRef('Daemon', daemon.id, daemon, {
								hosts: hostsData,
								subnets: subnetsData
							})
						}
					];
				})()
			},
			{
				label: 'Network',
				value: (() => {
					const network = networksData.find((n) => n.id == discovery.network_id);
					if (!network) return 'Unknown Network';
					return [
						{
							id: network.id,
							label: network.name,
							color: entities.getColorHelper('Network').color,
							entityRef: entityRef('Network', network.id, network, {
								credentials: credentialsData
							})
						}
					];
				})()
			},
			{
				label: 'Schedule',
				value:
					discovery.run_type.type == 'Scheduled'
						? schedulePaused
							? discovery_schedulePausedFreePlan()
							: formatScheduleDisplay(discovery.run_type.cron_schedule, discovery.run_type.timezone)
						: 'Manual'
			},
			{
				label: 'Last Run',
				value:
					discovery.run_type.type != 'Historical' && discovery.run_type.last_run
						? formatTimestamp(discovery.run_type.last_run)
						: 'Never'
			},
			{ label: 'Tags', snippet: tagsSnippet },
			...(hasActiveSession ? [{ label: '', snippet: progressSnippet }] : [])
		],
		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: `btn-icon`,
							onClick: () => onDelete(discovery),
							disabled: hasActiveSession,
							tooltip: hasActiveSession ? discovery_cannotDeleteWhileRunning() : undefined
						}
					]
				: []),
			...(onToggleEnabled && discovery.run_type.type === 'Scheduled'
				? [
						{
							label: isEnabled ? common_disable() : common_enable(),
							icon: Power,
							class: isEnabled ? `btn-icon-success` : `btn-icon`,
							onClick: () => onToggleEnabled(discovery),
							disabled: hasActiveSession,
							tooltip: hasActiveSession
								? discovery_cannotToggleWhileRunning()
								: isEnabled
									? discovery_disableScheduleTooltip()
									: discovery_enableScheduleTooltip()
						}
					]
				: []),
			...(onRun
				? [
						{
							label: 'Run',
							icon: Play,
							class: `btn-icon`,
							onClick: () => onRun(discovery),
							disabled: hasActiveSession,
							tooltip: hasActiveSession ? discovery_alreadyRunning() : undefined
						}
					]
				: []),
			...(onEdit
				? [{ label: 'Edit', icon: Edit, class: `btn-icon`, onClick: () => onEdit(discovery) }]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline
			selectedTagIds={discovery.tags}
			entityId={discovery.id}
			entityType="Discovery"
		/>
	</div>
{/snippet}

{#snippet progressSnippet()}
	<div class="flex items-center justify-between gap-3">
		<div class="flex-1 space-y-2">
			<div class="flex items-center gap-3">
				<span class={`text-secondary ${viewMode == 'list' ? 'text-xs' : 'text-sm'} font-medium`}
					>Phase:
				</span>
				<span class={`text-accent ${viewMode == 'list' ? 'text-xs' : 'text-sm'} font-medium`}
					>{isCancelling ? 'Cancelling' : activeSession!.phase}</span
				>
			</div>

			<DiscoveryEstimation
				phase={isCancelling ? 'Cancelling' : activeSession!.phase}
				hosts_discovered={activeSession!.hosts_discovered}
				estimated_remaining_secs={activeSession!.estimated_remaining_secs}
				class="mb-1"
			/>

			<div class="flex items-center gap-2">
				{#if onCancel}
					<button
						class="btn-icon-danger"
						disabled={isCancelling}
						onclick={isCancelling ? undefined : () => onCancel(activeSession!.session_id)}
					>
						{#if isCancelling}
							<Loader2 size={16} class="animate-spin" />
						{:else}
							<X size={16} />
						{/if}
					</button>
				{/if}
				<ProgressTrack class="flex-1">
					<AnimatedProgressBar progress={activeSession!.progress} />
				</ProgressTrack>
				<span class="text-secondary text-xs">{activeSession!.progress}%</span>
			</div>
		</div>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
