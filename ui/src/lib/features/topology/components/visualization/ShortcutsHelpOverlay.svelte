<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		topology_shortcutsTitle,
		topology_shortcutSearch,
		topology_shortcutFitView,
		topology_shortcutZoomSelection,
		topology_shortcutToggleLock,
		topology_shortcutDeselect,
		topology_shortcutHelp,
		topology_shortcutSelectAll,
		topology_shortcutBoxSelect,
		topology_shortcutToggleSelect,
		topology_shortcutStepExpand,
		topology_shortcutStepCollapse
	} from '$lib/paraglide/messages';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';

	let { isOpen = $bindable(false), readonly = false }: { isOpen: boolean; readonly?: boolean } =
		$props();

	const allShortcuts = [
		{ keys: ['Ctrl/Cmd', 'F'], description: () => topology_shortcutSearch() },
		{ keys: ['/'], description: () => topology_shortcutSearch() },
		{ keys: ['F'], description: () => topology_shortcutFitView() },
		{ keys: ['Z'], description: () => topology_shortcutZoomSelection() },
		{ keys: ['L'], description: () => topology_shortcutToggleLock(), editOnly: true },
		{ keys: ['Escape'], description: () => topology_shortcutDeselect() },
		{ keys: ['?'], description: () => topology_shortcutHelp() },
		{ keys: ['Ctrl/Cmd', 'A'], description: () => topology_shortcutSelectAll() },
		{ keys: ['Shift', 'Drag'], description: () => topology_shortcutBoxSelect() },
		{ keys: ['Shift', 'Click'], description: () => topology_shortcutToggleSelect() },
		{ keys: [']'], description: () => topology_shortcutStepExpand() },
		{ keys: ['['], description: () => topology_shortcutStepCollapse() }
	];

	let shortcuts = $derived(readonly ? allShortcuts.filter((s) => !s.editOnly) : allShortcuts);
</script>

<GenericModal title={topology_shortcutsTitle()} {isOpen} onClose={() => (isOpen = false)} size="sm">
	<div class="space-y-1 p-4">
		{#each shortcuts as shortcut (shortcut.keys.join('+'))}
			<div class="flex items-center justify-between py-1.5">
				<span class="text-secondary text-sm">{shortcut.description()}</span>
				<div class="flex items-center gap-1">
					{#each shortcut.keys as key (key)}
						<KbdKey {key} size="md" />
					{/each}
				</div>
			</div>
		{/each}
	</div>
</GenericModal>
