import { writable, get } from 'svelte/store';
import type { UseCase, BlockerType, NetworkSetup } from '../types/base';

export interface DaemonSetupState {
	name: string;
	installNow: boolean;
	apiKey?: string;
}

export interface OnboardingState {
	useCase: UseCase | null;
	readyToScan: boolean | null;
	organizationName: string;
	networks: NetworkSetup[];
	daemonSetups: Map<string, DaemonSetupState>; // keyed by network id
	populateSeedData: boolean;
	currentBlocker: BlockerType | null;
	// CRM qualification data (company/msp only, not persisted to DB)
	jobTitle: string | null;
	companySize: string | null;
	// Referral source (Cloud only, not persisted to DB)
	referralSource: string | null;
	referralSourceOther: string | null;
}

const STORAGE_KEY = 'scanopy_onboarding';

// Fields to persist to localStorage (for billing page autofill)
interface PersistedState {
	useCase: UseCase | null;
	companySize: string | null;
}

function loadPersistedState(): PersistedState {
	if (typeof window === 'undefined') {
		return { useCase: null, companySize: null };
	}
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			const parsed = JSON.parse(stored);
			return {
				useCase: parsed.useCase ?? null,
				companySize: parsed.companySize ?? null
			};
		}
	} catch {
		// Ignore localStorage errors
	}
	return { useCase: null, companySize: null };
}

function savePersistedState(state: PersistedState): void {
	if (typeof window === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
	} catch {
		// Ignore localStorage errors
	}
}

const persisted = loadPersistedState();

const initialState: OnboardingState = {
	useCase: persisted.useCase,
	readyToScan: null,
	organizationName: '',
	networks: [{ name: '' }],
	daemonSetups: new Map(),
	populateSeedData: true,
	currentBlocker: null,
	jobTitle: null,
	companySize: persisted.companySize,
	referralSource: null,
	referralSourceOther: null
};

function createOnboardingStore() {
	const { subscribe, update } = writable<OnboardingState>({ ...initialState });

	// Helper to update and persist
	function updateAndPersist(updater: (state: OnboardingState) => OnboardingState): void {
		update((state) => {
			const newState = updater(state);
			savePersistedState({
				useCase: newState.useCase,
				companySize: newState.companySize
			});
			return newState;
		});
	}

	return {
		subscribe,
		// Reset clears most state but preserves useCase and companySize for billing page
		reset: () =>
			updateAndPersist((state) => ({
				...initialState,
				networks: [{ name: '' }],
				daemonSetups: new Map(),
				useCase: state.useCase, // Preserve for billing page
				companySize: state.companySize, // Preserve for billing page
				jobTitle: null
			})),

		setUseCase: (useCase: UseCase) =>
			updateAndPersist((state) => ({
				...state,
				useCase
			})),

		setReadyToScan: (ready: boolean) =>
			update((state) => ({
				...state,
				readyToScan: ready
			})),

		setOrganizationName: (name: string) =>
			update((state) => ({
				...state,
				organizationName: name
			})),

		setNetworks: (networks: NetworkSetup[]) =>
			update((state) => ({
				...state,
				networks
			})),

		addNetwork: () =>
			update((state) => ({
				...state,
				networks: [...state.networks, { name: '' }]
			})),

		removeNetwork: (index: number) =>
			update((state) => ({
				...state,
				networks: state.networks.filter((_, i) => i !== index)
			})),

		updateNetworkName: (index: number, name: string) =>
			update((state) => ({
				...state,
				networks: state.networks.map((n, i) => (i === index ? { ...n, name } : n))
			})),

		setNetworkIds: (networkIds: string[]) =>
			update((state) => ({
				...state,
				networks: state.networks.map((n, i) => ({
					...n,
					id: networkIds[i]
				}))
			})),

		setPopulateSeedData: (populate: boolean) =>
			update((state) => ({
				...state,
				populateSeedData: populate
			})),

		setDaemonSetup: (networkId: string, setup: DaemonSetupState) =>
			update((state) => {
				const newDaemonSetups = new Map(state.daemonSetups);
				newDaemonSetups.set(networkId, setup);
				return {
					...state,
					daemonSetups: newDaemonSetups
				};
			}),

		clearDaemonSetup: (networkId: string) =>
			update((state) => {
				const newDaemonSetups = new Map(state.daemonSetups);
				newDaemonSetups.delete(networkId);
				return {
					...state,
					daemonSetups: newDaemonSetups
				};
			}),

		setCurrentBlocker: (blocker: BlockerType | null) =>
			update((state) => ({
				...state,
				currentBlocker: blocker
			})),

		setJobTitle: (jobTitle: string | null) =>
			update((state) => ({
				...state,
				jobTitle
			})),

		setCompanySize: (companySize: string | null) =>
			updateAndPersist((state) => ({
				...state,
				companySize
			})),

		setReferralSource: (referralSource: string | null, referralSourceOther: string | null) =>
			update((state) => ({
				...state,
				referralSource,
				referralSourceOther
			})),

		// Get the current state synchronously
		getState: () => get({ subscribe })
	};
}

export const onboardingStore = createOnboardingStore();
