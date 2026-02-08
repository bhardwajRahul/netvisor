<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import type { Snippet } from 'svelte';
	import { queryClient, queryKeys } from '$lib/api/query-client';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		identifyUser,
		trackPlunkEvent,
		trackEvent,
		flushEventQueue
	} from '$lib/shared/utils/analytics';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import { resolve } from '$app/paths';
	import { resetTopologyOptions } from '$lib/features/topology/queries';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { getRoute } from '$lib/shared/utils/navigation';
	import type { PostHog } from 'posthog-js';
	import { browser } from '$app/environment';
	import CookieConsent, {
		hasAnalyticsConsent
	} from '$lib/shared/components/feedback/CookieConsent.svelte';
	import {
		billing_subscriptionActivated,
		billing_subscriptionDelayed
	} from '$lib/paraglide/messages';

	let { children }: { children: Snippet } = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let isAuthenticated = $derived(currentUser != null);
	let isCheckingAuth = $derived(currentUserQuery.isPending);
	let authCheckComplete = $derived(!currentUserQuery.isPending);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// TanStack Query for config
	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	// Track if we've done initial setup
	let hasInitialized = $state(false);
	let previouslyAuthenticated = $state<boolean | null>(null);

	// Effect to handle logout (clear data when user goes from authenticated to not)
	$effect(() => {
		if (authCheckComplete) {
			if (previouslyAuthenticated === true && !isAuthenticated) {
				// User logged out - clear data
				resetTopologyOptions();
				queryClient.clear();
			}
			previouslyAuthenticated = isAuthenticated;
		}
	});

	let posthogInstance = $state<PostHog | null>(null);
	let posthogInitStarted = false;

	$effect(() => {
		if (!configData) return;

		const posthogKey = configData.posthog_key;

		if (browser && posthogKey && !posthogInitStarted) {
			posthogInitStarted = true;
			// Lazy-load posthog-js to avoid blocking initial bundle
			import('posthog-js').then(({ default: posthog }) => {
				posthog.init(posthogKey, {
					api_host: 'https://ph.scanopy.net',
					ui_host: 'https://us.posthog.com',
					defaults: '2025-11-30',
					secure_cookie: true,
					persistence: 'localStorage+cookie',
					opt_out_capturing_by_default: !hasAnalyticsConsent(),
					opt_out_capturing_persistence_type: 'localStorage', // Respect opt-out choice
					capture_pageview: true,
					capture_pageleave: true,

					// Don't auto-identify until consent
					person_profiles: 'identified_only', // Only create person profiles after identify

					loaded: () => {
						posthogInstance = posthog;
						flushEventQueue();
					}
				});
			});
		}
	});

	// Identify user in PostHog when authenticated (skipped in demo mode by identifyUser)
	$effect(() => {
		if (posthogInstance && currentUser && organization !== undefined) {
			identifyUser(currentUser.id, currentUser.email, currentUser.organization_id);
		}
	});

	async function waitForBillingActivation(maxAttempts = 10) {
		for (let i = 0; i < maxAttempts; i++) {
			// Invalidate and refetch organization data
			await queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
			const orgData = queryClient.getQueryData<typeof organization>(
				queryKeys.organizations.current()
			);

			if (orgData && isBillingPlanActive(orgData)) {
				// Track billing completion for funnel analytics
				trackEvent('billing_completed', {
					plan: orgData.plan?.type ?? 'unknown',
					amount: orgData.plan?.base_cents ?? 0,
					plan_status: orgData.plan_status
				});

				pushSuccess(billing_subscriptionActivated());
				return true;
			}

			// Wait 2 seconds before next check
			await new Promise((r) => setTimeout(r, 2000));
		}

		pushError(billing_subscriptionDelayed());
		return false;
	}

	// Handle routing after auth check completes
	$effect(() => {
		if (!authCheckComplete || hasInitialized) return;
		if (!browser) return;

		hasInitialized = true;

		// Check for OIDC error in URL
		const error = $page.url.searchParams.get('error');
		if (error) {
			pushError(decodeURIComponent(error));
			const cleanUrl = new URL($page.url);
			cleanUrl.searchParams.delete('error');
			window.history.replaceState({}, '', cleanUrl.toString());
		}

		if (!isAuthenticated) {
			// Not authenticated - redirect to login/onboarding if not on public route
			const isPublicRoute =
				$page.url.pathname === '/auth' ||
				$page.url.pathname === '/login' ||
				$page.url.pathname === '/onboarding' ||
				$page.url.pathname === '/verify-email' ||
				$page.url.pathname.startsWith('/share/');

			if (!isPublicRoute) {
				const token = $page.url.searchParams.get('token');
				const isDemo = $page.url.hostname === 'demo.scanopy.net';
				if (token) {
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					goto(`${resolve('/login')}?token=${token}`);
				} else if (isDemo) {
					goto(resolve('/login'));
				} else if (typeof localStorage !== 'undefined' && localStorage.getItem('hasAccount')) {
					goto(resolve('/login'));
				} else {
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					goto(`${resolve('/onboarding')}${$page.url.search}`);
				}
			}
		} else {
			// Authenticated - check if email is verified
			if (currentUser && !currentUser.email_verified) {
				// Redirect unverified users to verification page
				const isVerifyPage = $page.url.pathname === '/verify-email';
				if (!isVerifyPage) {
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					goto(`${resolve('/verify-email')}?email=${encodeURIComponent(currentUser.email)}`);
				}
				return;
			}

			// Handle Plunk tracking
			const pendingPlunk = sessionStorage.getItem('pendingPlunkRegistration');
			if (pendingPlunk && currentUser) {
				sessionStorage.removeItem('pendingPlunkRegistration');
				trackPlunkEvent('register', currentUser.email, pendingPlunk === 'true');
			}
		}
	});

	// Handle organization-dependent routing (runs after org data loads)
	$effect(() => {
		if (!authCheckComplete || !isAuthenticated || !browser) return;
		if (!organization) return;

		const sessionId = $page.url.searchParams.get('session_id');

		// Handle Stripe session callback (billing activation)
		if (sessionId && !isBillingPlanActive(organization)) {
			const cleanUrl = new URL($page.url);
			cleanUrl.searchParams.delete('session_id');
			window.history.replaceState({}, '', cleanUrl.toString());

			waitForBillingActivation().then((activated) => {
				if (activated) {
					const correctRoute = getRoute();
					// eslint-disable-next-line svelte/no-navigation-without-resolve
					goto(correctRoute);
				}
			});
			return;
		} else if (sessionId && isBillingPlanActive(organization)) {
			// User returned from Stripe payment method setup while already on an active/trialing plan
			trackEvent('payment_method_setup_completed', {
				plan_type: organization.plan?.type,
				plan_status: organization.plan_status
			});

			const cleanUrl = new URL($page.url);
			cleanUrl.searchParams.delete('session_id');
			window.history.replaceState({}, '', cleanUrl.toString());

			// Refresh org data to update has_payment_method
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		}

		// Check if current page matches where user should be
		// Skip routing check for share pages and onboarding (which handles its own navigation)
		const isSharePage = $page.url.pathname.startsWith('/share/');
		const isOnboardingPage = $page.url.pathname === '/onboarding';
		if (!isSharePage && !isOnboardingPage) {
			const correctRoute = getRoute();
			if ($page.url.pathname !== correctRoute) {
				// eslint-disable-next-line svelte/no-navigation-without-resolve
				goto(correctRoute);
			}
		}
	});
</script>

{#if isCheckingAuth && !$page.url.pathname.startsWith('/onboarding')}
	<div class="flex min-h-screen items-center justify-center bg-gray-900">
		<Loading />
	</div>
{:else}
	{@render children()}
{/if}

{#if configData && configData.needs_cookie_consent}
	<CookieConsent />
{/if}
