<script lang="ts">
	import { AlertTriangle } from 'lucide-svelte';
	import { useResendVerificationMutation } from '$lib/features/auth/queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import AppBanner from './AppBanner.svelte';
	import {
		auth_verificationEmailSent,
		auth_verifyEmailBannerBody,
		common_resend,
		common_sending
	} from '$lib/paraglide/messages';

	let { email }: { email: string } = $props();

	const resendMutation = useResendVerificationMutation();

	async function handleResend() {
		try {
			await resendMutation.mutateAsync({ email });
			pushSuccess(auth_verificationEmailSent());
		} catch {
			// Error handled by mutation
		}
	}
</script>

<AppBanner variant="warning" icon={AlertTriangle} body={auth_verifyEmailBannerBody()}>
	{#snippet actions()}
		<button
			onclick={handleResend}
			disabled={resendMutation.isPending}
			class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
		>
			{resendMutation.isPending ? common_sending() : common_resend()}
		</button>
	{/snippet}
</AppBanner>
