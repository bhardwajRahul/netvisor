import type { components } from '$lib/api/schema';
import { openModal } from '$lib/shared/stores/modal-registry';
import { trackEvent } from '$lib/shared/utils/analytics';

type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

export interface ChecklistStep {
	id: string;
	milestone: OnboardingOperation;
	prerequisite: OnboardingOperation | null;
	label: string;
	description: string;
	actionTab: string;
	actionModal?: string;
}

export const CHECKLIST_STEPS: ChecklistStep[] = [
	{
		id: 'account',
		milestone: 'OrgCreated',
		prerequisite: null,
		label: 'Account created',
		description: 'Your organization is set up and ready to go.',
		actionTab: 'home'
	},
	{
		id: 'daemon',
		milestone: 'FirstDaemonRegistered',
		prerequisite: 'OrgCreated',
		label: 'Install a daemon',
		description: 'Install a daemon to start discovering your network.',
		actionTab: 'daemons',
		actionModal: 'create-daemon'
	},
	{
		id: 'discovery',
		milestone: 'FirstDiscoveryCompleted',
		prerequisite: 'FirstDaemonRegistered',
		label: 'Run a discovery',
		description: 'See live results as your daemon discovers hosts and services.',
		actionTab: 'discovery-scans'
	},
	{
		id: 'topology',
		milestone: 'FirstTopologyRebuild',
		prerequisite: 'FirstDiscoveryCompleted',
		label: 'View your topology',
		description: 'See your network visualized as an interactive map.',
		actionTab: 'topology'
	}
];

export function isStepComplete(step: ChecklistStep, onboarding: OnboardingOperation[]): boolean {
	return onboarding.includes(step.milestone);
}

export function isStepEnabled(step: ChecklistStep, onboarding: OnboardingOperation[]): boolean {
	if (step.prerequisite === null) return true;
	return onboarding.includes(step.prerequisite);
}

export function getCompletedCount(onboarding: OnboardingOperation[]): number {
	return CHECKLIST_STEPS.filter((s) => onboarding.includes(s.milestone)).length;
}

export function isAllComplete(onboarding: OnboardingOperation[]): boolean {
	return CHECKLIST_STEPS.every((s) => onboarding.includes(s.milestone));
}

export function hasDaemon(onboarding: OnboardingOperation[]): boolean {
	return onboarding.includes('FirstDaemonRegistered');
}

export function executeStepAction(step: ChecklistStep, navigate: (tab: string) => void): void {
	navigate(step.actionTab);
	if (step.actionModal) {
		openModal(step.actionModal);
	}
}

export function trackChecklistStepClicked(stepId: string, source: 'home' | 'sidebar'): void {
	trackEvent('checklist_step_clicked', { step_id: stepId, source });
}
