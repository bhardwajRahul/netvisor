<script lang="ts">
	import {
		matchConfidenceColor,
		matchConfidenceLabel,
		matchReasonLabel,
		type MatchDetails
	} from '$lib/shared/types';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import * as m from '$lib/paraglide/messages';

	let { details }: { details: MatchDetails } = $props();

	let confidenceColor = $derived(matchConfidenceColor(details.confidence));
</script>

<div class="flex items-center justify-between">
	<div class="text-secondary flex items-center gap-2">
		<h3 class="text-sm font-semibold">{m.hosts_services_matchDetails()}</h3>
		<span class="text-tertiary text-xs">({matchReasonLabel(details.reason)})</span>
	</div>
	<Tag label={matchConfidenceLabel(details.confidence)} color={confidenceColor} />
</div>
