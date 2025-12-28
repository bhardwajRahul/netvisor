<script lang="ts">
	import { useNetworksQuery } from '$lib/features/networks/queries';

	/**
	 * SelectNetwork supports two usage patterns:
	 *
	 * 1. Binding (for svelte-forms): Use when parent prop is bindable
	 *    <SelectNetwork bind:selectedNetworkId />
	 *
	 * 2. Callback (for TanStack Form): Use when you need to handle changes manually
	 *    <SelectNetwork selectedNetworkId={value} onNetworkChange={(id) => handleChange(id)} />
	 */
	interface Props {
		selectedNetworkId?: string | null;
		disabled?: boolean;
		onNetworkChange?: (networkId: string) => void;
	}

	let { selectedNetworkId = $bindable(null), disabled = false, onNetworkChange }: Props = $props();

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Auto-select first network if none selected
	$effect(() => {
		if (!selectedNetworkId && networksData.length > 0) {
			const defaultId = networksData[0].id;
			// When using callback mode (TanStack Form), only call the callback
			// When using binding mode (svelte-forms), only set the bindable
			if (onNetworkChange) {
				onNetworkChange(defaultId);
			} else {
				selectedNetworkId = defaultId;
			}
		}
	});

	function handleChange(event: Event) {
		const value = (event.target as HTMLSelectElement).value;
		if (onNetworkChange) {
			onNetworkChange(value);
		} else {
			selectedNetworkId = value;
		}
	}
</script>

<div>
	<label for="network" class="text-secondary mb-2 block text-sm font-medium"> Network</label>
	<select id="network" {disabled} value={selectedNetworkId} onchange={handleChange} class="input-field">
		{#each networksData as network (network.id)}
			<option class="select-option" value={network.id}>{network.name}</option>
		{/each}
	</select>
	<p class="text-tertiary mt-2 text-xs">Select network</p>
</div>
