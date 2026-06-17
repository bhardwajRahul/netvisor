<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import MiniTopologyPreview from './MiniTopologyPreview.svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		daemons_promptTitle,
		daemons_promptBody,
		daemons_promptSkip,
		daemons_promptGetStarted
	} from '$lib/paraglide/messages';

	let {
		isOpen,
		onInstall,
		onSkip
	}: {
		isOpen: boolean;
		onInstall: () => void;
		onSkip: () => void;
	} = $props();

	let tracked = $state(false);

	$effect(() => {
		if (isOpen && !tracked) {
			trackEvent('daemon_prompt_viewed');
			tracked = true;
		}
	});
</script>

<GenericModal
	{isOpen}
	title={daemons_promptTitle()}
	size="md"
	onClose={onSkip}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<MiniTopologyPreview active={isOpen} />

				<p class="text-secondary text-sm">{daemons_promptBody()}</p>
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" class="btn-secondary" onclick={() => onSkip()}>
					{daemons_promptSkip()}
				</button>
				<button type="button" class="btn-primary" onclick={() => onInstall()}>
					{daemons_promptGetStarted()}
				</button>
			</div>
		</div>
	</div>
</GenericModal>
