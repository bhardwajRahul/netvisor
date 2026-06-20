<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from 'lucide-svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import { useUpdateSubnetMutation } from '$lib/features/subnets/queries';
	import { inspector_thisSubnet, topology_focusNode } from '$lib/paraglide/messages';

	let {
		node,
		topology,
		editState
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
	} = $props();

	const { fitView } = useSvelteFlow();
	const updateSubnetMutation = useUpdateSubnetMutation();

	let isReadonly = $derived(editState.isReadonly);
	let subnet = $derived(topology.subnets.find((s) => s.id === node.id) ?? null);

	function handleFocus() {
		fitView({ nodes: [{ id: node.id }], padding: 0.5, duration: 300 });
	}

	let subnetContext = $derived({
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: subnet?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (subnet) {
				updateSubnetMutation.mutate({ ...subnet, description: desc });
			}
		},
		compact: true
	});
</script>

{#if subnet}
	<div>
		<div class="mb-2 flex items-center gap-2">
			<span class="text-secondary text-sm font-medium">{inspector_thisSubnet()}</span>
			<button class="btn-icon p-0.5" onclick={handleFocus} title={topology_focusNode()}>
				<Crosshair class="h-3.5 w-3.5" />
			</button>
		</div>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={subnetContext}
				item={subnet}
				displayComponent={SubnetDisplay}
			/>
		</div>
	</div>
{/if}
