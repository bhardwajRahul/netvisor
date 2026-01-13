<script lang="ts">
	import * as m from '$lib/paraglide/messages';

	export let message: string = m.common_loading();
</script>

<div class="py-8 text-center">
	<div class="mx-auto h-8 w-8 animate-spin rounded-full border-b-2 border-blue-500"></div>
	<p class="text-secondary mt-2">{message}</p>
</div>
