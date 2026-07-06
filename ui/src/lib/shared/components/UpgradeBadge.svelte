<script lang="ts">
	import { ArrowUpCircle } from 'lucide-svelte';
	import Tag from './data/Tag.svelte';
	import { common_upgrade } from '$lib/paraglide/messages';

	let {
		feature
	}: {
		feature: string;
	} = $props();
</script>

<span title="Upgrade your plan to access {feature}">
	<Tag label={common_upgrade()} icon={ArrowUpCircle} color="Yellow" />
</span>
