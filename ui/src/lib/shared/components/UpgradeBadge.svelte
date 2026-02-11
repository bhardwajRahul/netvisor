<script lang="ts">
	import { ArrowUpCircle } from 'lucide-svelte';
	import Tag from './data/Tag.svelte';

	let {
		feature
	}: {
		feature: string;
	} = $props();
</script>

<span title="Upgrade your plan to access {feature}">
	<Tag label="Upgrade" icon={ArrowUpCircle} color="Yellow" />
</span>
