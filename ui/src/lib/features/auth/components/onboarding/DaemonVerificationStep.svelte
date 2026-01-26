<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Loader2, SatelliteDish, CheckCircle } from 'lucide-svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { useActiveSessionsQuery, discoverySSEManager } from '$lib/features/discovery/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import {
		onboarding_daemonConnected,
		onboarding_skipVerification,
		onboarding_verifyingDaemon,
		onboarding_waitingForDaemon
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen: boolean;
		onComplete: () => void;
		onSkip: () => void;
	}

	let { isOpen, onComplete, onSkip }: Props = $props();

	// Query for active sessions to detect when daemon connects and starts scanning
	let queryEnabled = $state(false);
	const sessionsQuery = useActiveSessionsQuery(() => queryEnabled);
	let sessionsData = $derived(sessionsQuery.data ?? []);

	// Query for daemons to detect when daemon registers
	const daemonsQuery = useDaemonsQuery();
	let daemonsData = $derived(daemonsQuery.data ?? []);

	// Check if we have an active session (daemon connected and started scanning)
	let hasActiveSession = $derived(sessionsData.length > 0);

	// Check if any daemon has connected (last_seen is set)
	let hasDaemonConnected = $derived(daemonsData.some((d) => d.last_seen !== null));

	// Track if daemon was connected (to show success state before auto-advancing)
	let daemonVerified = $state(false);

	// Rotating status text for waiting state
	let currentMessageIndex = $state(0);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		// Enable queries and connect to SSE
		const enableQuery = () => {
			queryEnabled = true;
			discoverySSEManager.connect();
		};

		if ('requestIdleCallback' in window) {
			requestIdleCallback(enableQuery);
		} else {
			setTimeout(enableQuery, 0);
		}

		// Toggle messages every 4 seconds
		intervalId = setInterval(() => {
			currentMessageIndex = currentMessageIndex === 0 ? 1 : 0;
		}, 4000);

		// Also poll daemon status more frequently
		const daemonPollId = setInterval(() => {
			daemonsQuery.refetch();
		}, 5000);

		return () => {
			if (intervalId) clearInterval(intervalId);
			clearInterval(daemonPollId);
		};
	});

	onDestroy(() => {
		discoverySSEManager.disconnect();
		if (intervalId) clearInterval(intervalId);
	});

	// Auto-advance when daemon connects or session starts
	$effect(() => {
		if (hasActiveSession || hasDaemonConnected) {
			// Mark as verified and wait a moment before advancing
			daemonVerified = true;

			// Clear pending daemon setup flag
			if (typeof localStorage !== 'undefined') {
				localStorage.removeItem('pendingDaemonSetup');
			}

			// Wait 1.5 seconds to show success state, then advance
			setTimeout(() => {
				onComplete();
			}, 1500);
		}
	});

	function handleSkip() {
		// Clear pending daemon setup flag when skipping
		if (typeof localStorage !== 'undefined') {
			localStorage.removeItem('pendingDaemonSetup');
		}
		onSkip();
	}
</script>

<GenericModal
	{isOpen}
	title={onboarding_verifyingDaemon()}
	onClose={handleSkip}
	size="md"
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	<div class="flex flex-col items-center justify-center space-y-6 p-8">
		{#if daemonVerified}
			<!-- Success state -->
			<div class="flex h-16 w-16 items-center justify-center rounded-full bg-success/20">
				<CheckCircle class="h-10 w-10 text-success" />
			</div>
			<div class="text-center">
				<p class="text-primary text-lg font-medium">{onboarding_daemonConnected()}</p>
				<p class="text-secondary mt-1 text-sm">Redirecting...</p>
			</div>
		{:else}
			<!-- Waiting state -->
			<div class="relative flex h-16 w-16 items-center justify-center">
				<div class="absolute inset-0 flex items-center justify-center">
					<div
						class="h-16 w-16 animate-spin rounded-full border-4 border-gray-700 border-t-primary-500"
					></div>
				</div>
				<SatelliteDish class="text-primary-400 h-8 w-8" />
			</div>

			<div class="h-12 text-center">
				<!-- Rotating messages -->
				<div class="relative h-6 overflow-hidden">
					<p
						class="text-primary absolute inset-0 flex items-center justify-center text-lg font-medium transition-transform duration-300"
						style="transform: translateY({currentMessageIndex === 0 ? '0' : '-100%'})"
					>
						{onboarding_waitingForDaemon()}
					</p>
					<p
						class="text-primary absolute inset-0 flex items-center justify-center text-lg font-medium transition-transform duration-300"
						style="transform: translateY({currentMessageIndex === 1 ? '0' : '100%'})"
					>
						Starting network scan...
					</p>
				</div>
				<p class="text-secondary mt-2 text-sm">
					This usually takes less than a minute after installing the daemon.
				</p>
			</div>

			<div class="flex items-center gap-2 text-gray-400">
				<Loader2 class="h-4 w-4 animate-spin" />
				<span class="text-sm">Checking connection...</span>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		{#if !daemonVerified}
			<div class="modal-footer">
				<div class="flex justify-center">
					<button
						type="button"
						class="text-secondary hover:text-primary text-sm underline"
						onclick={handleSkip}
					>
						{onboarding_skipVerification()}
					</button>
				</div>
			</div>
		{/if}
	{/snippet}
</GenericModal>
