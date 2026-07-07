<script lang="ts">
	import { Copy } from 'lucide-svelte';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { common_clickToCopy, common_copied, common_failedToCopy } from '$lib/paraglide/messages';

	let { command }: { command: string } = $props();

	async function copy() {
		try {
			await navigator.clipboard.writeText(command);
			pushSuccess(common_copied());
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}
</script>

<button
	type="button"
	class="group inline-flex max-w-full items-center gap-2 rounded bg-gray-900 px-3 py-2 text-left font-mono text-xs text-gray-300 transition-colors hover:bg-gray-800 dark:bg-gray-900 dark:hover:bg-gray-800"
	onclick={copy}
	title={common_clickToCopy()}
>
	<span class="min-w-0 truncate">{command}</span>
	<Copy class="h-3 w-3 flex-shrink-0 opacity-0 transition-opacity group-hover:opacity-60" />
</button>
