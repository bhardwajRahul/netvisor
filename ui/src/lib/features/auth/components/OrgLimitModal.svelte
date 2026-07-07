<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		auth_orgLimitReachedBody,
		auth_orgLimitReachedContact,
		auth_orgLimitReachedTitle,
		auth_scanopyLogo,
		auth_signInInstead
	} from '$lib/paraglide/messages';

	let {
		adminContactEmail = null,
		onClose,
		onSwitchToLogin
	}: {
		adminContactEmail?: string | null;
		onClose: () => void;
		onSwitchToLogin?: () => void;
	} = $props();
</script>

<GenericModal
	isOpen={true}
	title={auth_orgLimitReachedTitle()}
	size="lg"
	{onClose}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="space-y-4 p-4 sm:p-6">
		<p class="text-secondary text-center text-sm">
			{auth_orgLimitReachedBody()}
		</p>
		{#if adminContactEmail}
			<p class="text-secondary text-center text-sm">
				{auth_orgLimitReachedContact()}
				<a href="mailto:{adminContactEmail}" class="text-accent underline hover:no-underline">
					{adminContactEmail}
				</a>
			</p>
		{/if}
		{#if onSwitchToLogin}
			<div class="text-center">
				<button type="button" onclick={onSwitchToLogin} class="text-link text-sm hover:underline">
					{auth_signInInstead()}
				</button>
			</div>
		{/if}
	</div>
</GenericModal>
