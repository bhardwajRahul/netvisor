<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { useRetryDaemonConnectionMutation } from '$lib/features/daemons/queries';
	import { entities } from '$lib/shared/stores/metadata';
	import { ArrowBigUp, Edit, RefreshCw, Trash2 } from 'lucide-svelte';
	import {
		common_delete,
		common_edit,
		common_update,
		daemons_retryConnection
	} from '$lib/paraglide/messages';
	import { getDaemonStatusTag } from '$lib/features/daemons/utils';
	import type { TagProps } from '$lib/shared/components/data/types';
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import DaemonUpgradeModal from './DaemonUpgradeModal.svelte';
	import { modalState, openModal, closeModal } from '$lib/shared/stores/modal-registry';

	// Modal state — supports deep linking via ?modal=upgrade-daemon&id=<daemon-id>
	let upgradeModalOpen = $state(false);

	// Auto-open when deep-linked
	$effect(() => {
		if (
			$modalState.name === 'upgrade-daemon' &&
			$modalState.id === daemon.id &&
			!upgradeModalOpen
		) {
			upgradeModalOpen = true;
		}
	});

	function handleOpenUpgrade() {
		upgradeModalOpen = true;
		openModal('upgrade-daemon', { id: daemon.id });
	}

	function handleCloseUpgrade() {
		upgradeModalOpen = false;
		closeModal();
	}

	const retryConnectionMutation = useRetryDaemonConnectionMutation();

	let {
		daemon,
		columns,
		onDelete,
		onEdit,
		selected,
		onSelectionChange = () => {}
	}: {
		daemon: Daemon;
		/**
		 * The shared field definition, from the tab. The card no longer builds its
		 * own list: the tab declares each field once and both views render it, so
		 * a field cannot appear here and be missing from the table.
		 */
		columns: EntityColumn<Daemon>[];
		onDelete?: (daemon: Daemon) => void;
		onEdit?: (daemon: Daemon) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let status: TagProps = $derived(getDaemonStatusTag(daemon));

	let hasUpdateAvailable = $derived(
		daemon.version_status.status === 'Outdated' ||
			daemon.version_status.status === 'Deprecated' ||
			daemon.version_status.status === 'Unsupported'
	);

	// The scheduled sunset date for this daemon's version, if any (Deprecated /
	// Unsupported). Server-published — the UI never computes it.
	let sunsetDate = $derived(daemon.version_status.sunset_date ?? null);

	let retryPending = $derived(retryConnectionMutation.isPending);

	// Escalate the upgrade affordance by lifecycle stage: a scheduled/active
	// sunset is a warning/danger action, a plain newer release is neutral-info.
	let upgradeButtonClass = $derived.by(() => {
		switch (daemon.version_status.status) {
			case 'Unsupported':
				return 'btn-icon-danger';
			case 'Deprecated':
				return 'btn-icon-warning';
			case 'Outdated':
				return 'btn-icon-info';
			default:
				return 'btn-icon';
		}
	});

	// Build card data
	let cardData = $derived({
		title: daemon.name,
		iconColor: entities.getColorHelper('Daemon').icon,
		Icon: entities.getIconComponent('Daemon'),
		status,
		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(daemon)
						}
					]
				: []),
			...(hasUpdateAvailable && (daemon.is_unreachable !== true || sunsetDate !== null)
				? [
						{
							label: common_update(),
							icon: ArrowBigUp,
							class: upgradeButtonClass,
							onClick: handleOpenUpgrade,
							disabled: false,
							forceLabel: true
						}
					]
				: []),
			// Show retry button for unreachable ServerPoll daemons
			...(daemon.is_unreachable === true && daemon.mode === 'server_poll'
				? [
						{
							label: daemons_retryConnection(),
							icon: RefreshCw,
							class: 'btn-icon-info',
							onClick: () => retryConnectionMutation.mutate(daemon.id),
							disabled: retryPending,
							forceLabel: true
						}
					]
				: []),
			// Edit sits last (rightmost) per the convention shared by every other entity card.
			// It opens the daemon management modal, which is also where the daemon's 1:1 key
			// is managed — and where a legacy daemon can be given one.
			...(onEdit
				? [
						{
							label: common_edit(),
							icon: Edit,
							class: 'btn-icon',
							onClick: () => onEdit(daemon)
						}
					]
				: [])
		]
	});
</script>

<GenericCard {...cardData} {columns} item={daemon} {selected} {onSelectionChange} />

<DaemonUpgradeModal isOpen={upgradeModalOpen} onClose={handleCloseUpgrade} {daemon} />
