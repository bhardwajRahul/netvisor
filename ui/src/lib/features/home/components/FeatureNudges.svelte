<script lang="ts">
	import type { components } from '$lib/api/schema';
	import FeatureNudge from './FeatureNudge.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import {
		activeView,
		showViewSwitcherHint,
		showDependencyTutorial
	} from '$lib/features/topology/queries';
	import { entities, billingPlans } from '$lib/shared/stores/metadata';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { onMount } from 'svelte';
	import {
		common_getStarted,
		home_nudges_apiKeysAction,
		home_nudges_apiKeysDescription,
		home_nudges_apiKeysTitle,
		home_nudges_applicationTagsDescription,
		home_nudges_applicationTagsTitle,
		home_nudges_daemonAttentionAction,
		home_nudges_daemonAttentionDescription,
		home_nudges_daemonAttentionTitle,
		home_nudges_dependenciesAction,
		home_nudges_dependenciesDescription,
		home_nudges_dependenciesTitle,
		home_nudges_explorePerspectivesAction,
		home_nudges_explorePerspectivesDescription,
		home_nudges_explorePerspectivesTitle,
		home_nudges_inviteTeamAction,
		home_nudges_inviteTeamDescription,
		home_nudges_inviteTeamTitle,
		home_nudges_multiNetworkAction,
		home_nudges_multiNetworkDescription,
		home_nudges_multiNetworkTitle,
		home_nudges_shareAction,
		home_nudges_shareDescription,
		home_nudges_shareTitle,
		home_nudges_snmpAction,
		home_nudges_snmpDescription,
		home_nudges_snmpTitle,
		home_nudges_tagsAction,
		home_nudges_tagsDescription,
		home_nudges_tagsTitle
	} from '$lib/paraglide/messages';

	type Organization = components['schemas']['Organization'];
	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DashboardSummary = components['schemas']['DashboardSummary'];

	let {
		organization,
		dashboard,
		onNavigate
	}: {
		organization: Organization;
		dashboard: DashboardSummary;
		onNavigate: (tab: string) => void;
	} = $props();

	const daemonsQuery = useDaemonsQuery();
	let hasUnreachableDaemons = $derived(
		(daemonsQuery.data ?? []).some((d) => d.standby === true || d.is_unreachable === true)
	);

	let mounted = $state(false);
	let dismissCount = $state(0);
	onMount(() => {
		mounted = true;
	});

	function onDismiss() {
		dismissCount++;
	}

	const planType = $derived(organization.plan?.type ?? null);
	const onboarding = $derived(organization.onboarding ?? []);
	const has = (op: OnboardingOperation) => onboarding.includes(op);

	const planMetadata = $derived(billingPlans.getMetadata(planType));
	const features = $derived(planMetadata?.features ?? {});
	interface Nudge {
		id: string;
		title: string;
		description: string;
		actionLabel: string;
		action: () => void;
		visible: boolean;
		icon: IconComponent;
		iconColor: string;
	}

	let nudges = $derived.by((): Nudge[] => {
		const all: Nudge[] = [
			{
				id: 'daemon-attention',
				title: home_nudges_daemonAttentionTitle(),
				description: home_nudges_daemonAttentionDescription(),
				actionLabel: home_nudges_daemonAttentionAction(),
				action: () => {
					onNavigate('daemons');
				},
				visible: hasUnreachableDaemons,
				icon: entities.getIconComponent('Daemon'),
				iconColor: entities.getColorHelper('Daemon').icon
			},
			{
				id: 'explore-perspectives',
				title: home_nudges_explorePerspectivesTitle(),
				description: home_nudges_explorePerspectivesDescription(),
				actionLabel: home_nudges_explorePerspectivesAction(),
				action: () => {
					showViewSwitcherHint.set(true);
					onNavigate('topology');
				},
				visible: has('FirstTopologyRebuild'),
				icon: entities.getIconComponent('Topology'),
				iconColor: entities.getColorHelper('Topology').icon
			},
			{
				id: 'application-tags',
				title: home_nudges_applicationTagsTitle(),
				description: home_nudges_applicationTagsDescription(),
				actionLabel: common_getStarted(),
				action: () => {
					activeView.set('Application');
					onNavigate('topology');
				},
				visible: has('FirstTopologyRebuild') && !has('FirstApplicationTagCreated'),
				icon: entities.getIconComponent('Tag'),
				iconColor: entities.getColorHelper('Tag').icon
			},
			{
				id: 'dependencies',
				title: home_nudges_dependenciesTitle(),
				description: home_nudges_dependenciesDescription(),
				actionLabel: home_nudges_dependenciesAction(),
				action: () => {
					activeView.set(has('FirstApplicationTagCreated') ? 'Application' : 'Workloads');
					showDependencyTutorial.set(true);
					onNavigate('topology');
				},
				visible: has('FirstTopologyRebuild') && !has('FirstDependencyCreated'),
				icon: entities.getIconComponent('Dependency'),
				iconColor: entities.getColorHelper('Dependency').icon
			},
			{
				id: 'snmp',
				title: home_nudges_snmpTitle(),
				description: home_nudges_snmpDescription(),
				actionLabel: home_nudges_snmpAction(),
				action: () => {
					onNavigate('credentials');
					openModal('credential-editor');
				},
				visible: has('FirstDiscoveryCompleted') && !has('FirstSnmpCredentialCreated'),
				icon: entities.getIconComponent('Credential'),
				iconColor: entities.getColorHelper('Credential').icon
			},
			{
				id: 'tags',
				title: home_nudges_tagsTitle(),
				description: home_nudges_tagsDescription(),
				actionLabel: home_nudges_tagsAction(),
				action: () => {
					onNavigate('tags');
					openModal('tag-editor');
				},
				visible: has('FirstDiscoveryCompleted') && !has('FirstTagCreated'),
				icon: entities.getIconComponent('Tag'),
				iconColor: entities.getColorHelper('Tag').icon
			},
			{
				id: 'invite-team',
				title: home_nudges_inviteTeamTitle(),
				description: home_nudges_inviteTeamDescription(),
				actionLabel: home_nudges_inviteTeamAction(),
				action: () => {
					onNavigate('users');
					openModal('invite-user');
				},
				visible:
					((organization.plan?.included_seats ?? 0) > 1 ||
						(organization.plan?.seat_cents ?? 0) > 0) &&
					!has('InviteSent'),
				icon: entities.getIconComponent('User'),
				iconColor: entities.getColorHelper('User').icon
			},
			{
				id: 'multi-network',
				title: home_nudges_multiNetworkTitle(),
				description: home_nudges_multiNetworkDescription(),
				actionLabel: home_nudges_multiNetworkAction(),
				action: () => {
					onNavigate('networks');
					openModal('network-editor');
				},
				visible:
					(organization.plan?.included_networks === null ||
						(organization.plan?.included_networks ?? 0) > 1 ||
						(organization.plan?.network_cents ?? 0) > 0) &&
					dashboard.networks.length === 1,
				icon: entities.getIconComponent('Network'),
				iconColor: entities.getColorHelper('Network').icon
			},
			{
				id: 'api-keys',
				title: home_nudges_apiKeysTitle(),
				description: home_nudges_apiKeysDescription(),
				actionLabel: home_nudges_apiKeysAction(),
				action: () => {
					onNavigate('api-keys');
					openModal('user-api-key');
				},
				visible:
					features.api_access && has('FirstDiscoveryCompleted') && !has('FirstUserApiKeyCreated'),
				icon: entities.getIconComponent('UserApiKey'),
				iconColor: entities.getColorHelper('UserApiKey').icon
			},
			{
				id: 'share',
				title: home_nudges_shareTitle(),
				description: home_nudges_shareDescription(),
				actionLabel: home_nudges_shareAction(),
				action: () => {
					onNavigate('topology');
					openModal('topology-share');
				},
				visible: features.share_views && has('FirstTopologyRebuild'),
				icon: entities.getIconComponent('Share'),
				iconColor: entities.getColorHelper('Share').icon
			}
		];

		return all.filter((n) => n.visible);
	});

	// Check localStorage for dismissed nudges and limit to 2
	// dismissCount is a reactive dependency so this recomputes when a nudge is dismissed
	let visibleNudgeIds = $derived.by((): string[] => {
		void dismissCount;
		if (!mounted) return [];
		const visible: string[] = [];
		for (const nudge of nudges) {
			if (localStorage.getItem(`nudge-dismissed:${nudge.id}`) !== 'true') {
				visible.push(nudge.id);
			}
			if (visible.length >= 2) break;
		}
		return visible;
	});
</script>

{#if visibleNudgeIds.length > 0}
	<section>
		<h3 class="text-primary mb-3 text-base font-semibold">Suggestions</h3>
		<div class="grid gap-4 sm:grid-cols-2">
			{#each nudges.filter((n) => visibleNudgeIds.includes(n.id)) as nudge (nudge.id)}
				<FeatureNudge
					id={nudge.id}
					title={nudge.title}
					description={nudge.description}
					actionLabel={nudge.actionLabel}
					onAction={nudge.action}
					Icon={nudge.icon}
					iconColor={nudge.iconColor}
					{onDismiss}
				/>
			{/each}
		</div>
	</section>
{/if}
