<script lang="ts">
	/**
	 * Give a legacy daemon (one with no 1:1 bound key) a dedicated key.
	 *
	 * The server cannot hand a key to a running daemon — the daemon reads its config once at
	 * boot and there is no server→daemon control channel — so associating a key also has to
	 * produce a reconfigure command for the operator to run on the machine.
	 *
	 * What that means differs by mode, so the flow does too:
	 *
	 * - DaemonPoll: the daemon dials the server and authenticates with the network-shared key
	 *   it already has, which this does not touch. It keeps running on that key until the
	 *   command is run, so minting is safe to do immediately.
	 * - ServerPoll: the server dials the daemon and will present the NEW key from the next
	 *   poll onwards, while the daemon still only accepts its old one. That is a hard cutover
	 *   — the daemon is unreachable until reinstalled — so it is gated behind a confirmation.
	 */
	import { KeyRound } from 'lucide-svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { useProvisionDaemonMutation } from '$lib/features/daemons/queries';
	import type { ProvisionDaemonResponse } from '$lib/features/daemons/types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import OsSelector from './OsSelector.svelte';
	import type { DaemonOS } from '$lib/features/daemons/utils';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		common_associating,
		common_cancel,
		common_confirm,
		daemons_associateKey,
		daemons_associateKeyConfirmTitle,
		daemons_associateKeyCta,
		daemons_associateKeyDaemonPollHelp,
		daemons_associateKeyFailed,
		daemons_associateKeyNoDowntimeTitle,
		daemons_associateKeyOfflineTitle,
		daemons_associateKeyServerPollConfirm,
		daemons_associateKeyServerPollWarning,
		daemons_reconfigureCommandDaemonPoll,
		daemons_reconfigureCommandServerPoll,
		daemons_reconfigureTitle
	} from '$lib/paraglide/messages';

	let { daemon }: { daemon: Daemon } = $props();

	const provisionMutation = useProvisionDaemonMutation();

	let isServerPoll = $derived(daemon.mode === 'server_poll');
	let confirming = $state(false);
	let associating = $state(false);
	let result = $state<ProvisionDaemonResponse | null>(null);
	let selectedOS = $state<DaemonOS>('linux');

	let command = $derived(
		result?.install_artifacts.commands.find((c) => c.platform === selectedOS)?.command ?? ''
	);

	async function associate() {
		associating = true;
		try {
			result = await provisionMutation.mutateAsync({ daemon_id: daemon.id });
			confirming = false;
		} catch {
			pushError(daemons_associateKeyFailed());
		} finally {
			associating = false;
		}
	}

	function handleClick() {
		// ServerPoll takes the daemon offline the moment the key is minted, so make that an
		// explicit decision rather than a surprise.
		if (isServerPoll) {
			confirming = true;
		} else {
			associate();
		}
	}
</script>

{#if result}
	<div class="space-y-4">
		<InlineInfo
			title={daemons_reconfigureTitle()}
			body={isServerPoll
				? daemons_reconfigureCommandServerPoll({ name: daemon.name })
				: daemons_reconfigureCommandDaemonPoll({ name: daemon.name })}
		/>

		<OsSelector {selectedOS} onOsSelect={(os) => (selectedOS = os)}>
			<CodeContainer
				language={selectedOS === 'windows' ? 'powershell' : 'bash'}
				expandable={false}
				maxHeight=""
				code={command}
				preventSelect={true}
			/>
		</OsSelector>
	</div>
{:else}
	<div class="space-y-4 py-4 text-center">
		<KeyRound class="text-tertiary mx-auto h-10 w-10" />
		<h3 class="text-primary text-lg font-medium">{daemons_associateKey()}</h3>

		{#if confirming}
			<InlineWarning
				title={daemons_associateKeyConfirmTitle()}
				body={daemons_associateKeyServerPollConfirm({ name: daemon.name })}
			/>
			<div class="flex items-center justify-center gap-3">
				<button type="button" class="btn-secondary" onclick={() => (confirming = false)}>
					{common_cancel()}
				</button>
				<button type="button" class="btn-danger" disabled={associating} onclick={associate}>
					{associating ? common_associating() : common_confirm()}
				</button>
			</div>
		{:else}
			{#if isServerPoll}
				<InlineWarning
					title={daemons_associateKeyOfflineTitle()}
					body={daemons_associateKeyServerPollWarning()}
				/>
			{:else}
				<InlineInfo
					title={daemons_associateKeyNoDowntimeTitle()}
					body={daemons_associateKeyDaemonPollHelp()}
				/>
			{/if}
			<button type="button" class="btn-primary" disabled={associating} onclick={handleClick}>
				{associating ? common_associating() : daemons_associateKeyCta()}
			</button>
		{/if}
	</div>
{/if}
