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
	import { Check, Edit, Merge } from 'lucide-svelte';

	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { entities } from '$lib/shared/stores/metadata';
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { Subnet } from '../types/base';
	import {
		subnets_confirmRange,
		subnets_confirmRangeDetail,
		subnets_correctRange,
		subnets_correctRangeDetail,
		subnets_deployDaemonHere,
		subnets_deployDaemonHereDetail,
		subnets_mergeInto,
		subnets_mergeIntoDetail,
		subnets_resolveRange,
		subnets_resolveRangeIntro
	} from '$lib/paraglide/messages';

	interface Props {
		subnet: Subnet | null;
		/**
		 * A live subnet with a settled range that already covers this one, if there is one. Offered
		 * as a merge: containment means every address fits, so folding them loses nothing.
		 */
		coveredBy?: Subnet | null;
		isOpen?: boolean;
		/** Marks the range confirmed. Owned by the tab, which holds the mutation. */
		onConfirm: (subnet: Subnet) => Promise<void> | void;
		/** Opens the subnet editor on this subnet. */
		onCorrect: (subnet: Subnet) => void;
		/** Folds this subnet into the one that covers it. */
		onMerge: (subnet: Subnet, into: Subnet) => Promise<void> | void;
		onClose: () => void;
	}

	let {
		subnet,
		coveredBy = null,
		isOpen = false,
		onConfirm,
		onCorrect,
		onMerge,
		onClose
	}: Props = $props();

	/**
	 * From the entity metadata rather than picked here, so a daemon is drawn with the same icon in
	 * this modal as everywhere else it appears and a change to it reaches all of them at once.
	 */
	const DaemonIcon = entities.getIconComponent('Daemon');

	type Option = {
		id: string;
		title: string;
		description: string;
		icon: IconComponent;
		color: Color;
		run: (subnet: Subnet) => void;
	};

	let options = $derived.by<Option[]>(() => {
		const options: Option[] = [
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
			}
		];

		// Only where a measured range already covers this one. Without that there is nothing to fold
		// into, and the question is still how to reach the segment at all.
		if (coveredBy) {
			const into = coveredBy;
			options.push({
				id: 'merge',
				title: subnets_mergeInto({ cidr: into.cidr }),
				description: subnets_mergeIntoDetail(),
				icon: Merge,
				color: 'Violet',
				run: (subnet) => void onMerge(subnet, into)
			});
		}

		options.push({
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
		});

		return options;
	});
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

					<div class="flex flex-col gap-3">
						{#each options as option (option.id)}
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
