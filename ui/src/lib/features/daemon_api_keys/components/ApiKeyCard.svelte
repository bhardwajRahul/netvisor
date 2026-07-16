<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { toColor } from '$lib/shared/utils/styling';
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { ApiKey } from '../types/base';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		common_created,
		common_delete,
		common_edit,
		common_enabled,
		common_expired,
		common_expires,
		common_lastUsed,
		common_never,
		common_no,
		common_tags,
		common_yes,
		common_legacy,
		daemons_legacyKeyHelp
	} from '$lib/paraglide/messages';

	let {
		apiKey,
		isInUse = false,
		onDelete = () => {},
		onEdit = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		apiKey: ApiKey;
		isInUse?: boolean;
		onDelete?: (apiKey: ApiKey) => void;
		onEdit?: (apiKey: ApiKey) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Build card data
	// Keys not bound 1:1 to a daemon predate server-provisioned identity. Since new
	// keys are only minted through provisioning, an unbound key is a legacy one.
	let isLegacy = $derived(!apiKey.daemon_id);

	let cardData = $derived({
		title: apiKey.name,
		// Mark legacy (unbound) keys with a tag + hover helptext, mirroring the daemon
		// card's status tags. Bound 1:1 keys get no tag.
		status: isLegacy
			? { label: common_legacy(), color: toColor('gray'), title: daemons_legacyKeyHelp() }
			: undefined,
		iconColor: entities.getColorHelper('DaemonApiKey').icon,
		Icon: entities.getIconComponent('DaemonApiKey'),
		fields: [
			{
				label: common_created(),
				value: formatTimestamp(apiKey.created_at)
			},
			{
				label: common_lastUsed(),
				value: apiKey.last_used ? formatTimestamp(apiKey.last_used) : common_never()
			},
			{
				label: common_expires(),
				value: apiKey.expires_at
					? new Date(apiKey.expires_at) < new Date()
						? common_expired()
						: formatTimestamp(apiKey.expires_at)
					: common_never()
			},
			{
				label: common_enabled(),
				value: apiKey.is_enabled ? common_yes() : common_no()
			},
			{ label: common_tags(), snippet: tagsSnippet }
		],
		actions: [
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				disabled: isInUse,
				onClick: () => onDelete(apiKey)
			},
			{
				label: common_edit(),
				icon: Edit,
				class: 'btn-icon',
				onClick: () => onEdit(apiKey)
			}
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={apiKey.tags} entityId={apiKey.id} entityType="DaemonApiKey" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
