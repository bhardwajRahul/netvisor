<!--
	What to do about a range Scanopy assumed.

	The badge said a range was a guess and stopped there, which left the operator with a question and
	no answer to it. These are the four things that actually resolve one, and they are offered
	together because choosing between them is the decision: two settle the range itself, and two
	settle the reason it had to be guessed — that nothing can reach the segment.

	Confirm is the only one that acts here. The rest hand off to the flow that already owns them
	rather than growing a second copy of it: the subnet editor for a correction, the daemon modal for
	a deployment, and the docs for pointing an existing daemon at a segment it can route to.
-->
<script lang="ts">
	import { Check, Crosshair, Edit } from 'lucide-svelte';

	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { entities } from '$lib/shared/stores/metadata';
	import { docsUrl } from '$lib/shared/utils/docs';
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { Subnet } from '../types/base';
	import {
		subnets_confirmRange,
		subnets_confirmRangeDetail,
		subnets_correctRange,
		subnets_correctRangeDetail,
		subnets_deployDaemonHere,
		subnets_deployDaemonHereDetail,
		subnets_makeRoutable,
		subnets_makeRoutableDetail,
		subnets_resolveRange,
		subnets_resolveRangeIntro
	} from '$lib/paraglide/messages';

	interface Props {
		subnet: Subnet | null;
		isOpen?: boolean;
		/** Marks the range confirmed. Owned by the tab, which holds the mutation. */
		onConfirm: (subnet: Subnet) => Promise<void> | void;
		/** Opens the subnet editor on this subnet. */
		onCorrect: (subnet: Subnet) => void;
		onClose: () => void;
	}

	let { subnet, isOpen = false, onConfirm, onCorrect, onClose }: Props = $props();

	/**
	 * From the entity metadata rather than picked here, so a daemon is drawn with the same icon in
	 * this modal as everywhere else it appears and a change to it reaches all of them at once.
	 */
	const DaemonIcon = entities.getIconComponent('Daemon');

	/**
	 * The guide for pointing a daemon at a segment it can route to but has no interface on — which
	 * is this subnet's whole situation, and the step the "create it first" half of that guide has
	 * already done for the operator.
	 */
	const REMOTE_SUBNET_DOCS = '/docs/guides/scanning-remote-subnets/';

	type Option = {
		id: string;
		title: string;
		description: string;
		icon: IconComponent;
		color: Color;
		run: (subnet: Subnet) => void;
	};

	const OPTIONS: Option[] = [
		{
			id: 'confirm',
			title: subnets_confirmRange(),
			description: subnets_confirmRangeDetail(),
			icon: Check,
			color: 'Green',
			run: (subnet) => void onConfirm(subnet)
		},
		{
			id: 'correct',
			title: subnets_correctRange(),
			description: subnets_correctRangeDetail(),
			icon: Edit,
			color: 'Blue',
			run: (subnet) => onCorrect(subnet)
		},
		{
			id: 'deploy-daemon',
			title: subnets_deployDaemonHere(),
			description: subnets_deployDaemonHereDetail(),
			icon: DaemonIcon,
			color: 'Indigo',
			// The daemon modal lives in the daemons tab, so the hash has to move with it.
			run: () => {
				onClose();
				window.location.hash = 'daemons';
				openModal('create-daemon');
			}
		},
		{
			id: 'make-routable',
			title: subnets_makeRoutable(),
			description: subnets_makeRoutableDetail(),
			icon: Crosshair,
			color: 'Gray',
			run: () => {
				onClose();
				window.open(docsUrl(REMOTE_SUBNET_DOCS), '_blank', 'noopener,noreferrer');
			}
		}
	];
</script>

<GenericModal
	name="provisional-range"
	entityId={subnet?.id}
	title={subnets_resolveRange()}
	{isOpen}
	{onClose}
	size="md"
>
	{#if subnet}
		<!-- `modal-content` supplies no padding of its own, so a body has to bring the same
		     scrollable, padded container every other modal wraps its content in. -->
		<div class="flex min-h-0 flex-1 flex-col">
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-4">
					<p class="text-secondary text-sm">
						{subnets_resolveRangeIntro({ cidr: subnet.cidr })}
					</p>

					<div class="grid gap-3 sm:grid-cols-2">
						{#each OPTIONS as option (option.id)}
							{@const colors = createColorHelper(option.color)}
							<button onclick={() => option.run(subnet)} class="card w-full text-left">
								<div class="flex items-center gap-3">
									<div
										class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {colors.bg}"
									>
										<option.icon class="h-5 w-5 {colors.icon}" />
									</div>
									<div class="min-w-0 flex-1">
										<p class="text-primary text-sm font-medium">{option.title}</p>
										<p class="text-secondary text-xs">{option.description}</p>
									</div>
								</div>
							</button>
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</GenericModal>
