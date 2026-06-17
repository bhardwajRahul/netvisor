<script lang="ts">
	import { Rocket } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { auth_createAccount, demo_bannerBody } from '$lib/paraglide/messages';
</script>

<AppBanner variant="info" icon={Rocket} body={demo_bannerBody()}>
	{#snippet actions()}
		<a
			href="https://app.scanopy.net/onboarding"
			target="_blank"
			rel="noopener noreferrer"
			class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline"
		>
			{auth_createAccount()}
		</a>
	{/snippet}
</AppBanner>
