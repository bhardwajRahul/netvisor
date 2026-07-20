<script lang="ts">
	/**
	 * Give a legacy daemon (one with no 1:1 bound key) a dedicated key.
	 *
	 * The server cannot hand a key to a running daemon — the daemon reads its config once at
	 * boot and there is no server→daemon control channel — so associating a key also has to
	 * produce a reconfigure command for the operator to run on the machine.
	 *
	 * What that means differs by mode, so the copy does too:
	 *
	 * - DaemonPoll: the daemon dials the server and authenticates with the key it already has,
	 *   which this does not touch. It keeps working until the command is run.
	 * - ServerPoll: the server dials the daemon and will present the NEW key from the next poll
	 *   onwards, while the daemon still only accepts its old one — so it stops being reachable
	 *   until it is reconfigured.
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
		daemons_associateKey,
		daemons_associateKeyCta,
		daemons_associateKeyDaemonPollHelp,
		daemons_associateKeyFailed,
		daemons_associateKeyNoDowntimeTitle,
		daemons_associateKeyOfflineTitle,
		daemons_associateKeyServerPollWarning,
		daemons_reconfigureCommandDaemonPoll,
		daemons_reconfigureCommandServerPoll,
		daemons_reconfigureTitle
	} from '$lib/paraglide/messages';

	let { daemon }: { daemon: Daemon } = $props();

	const provisionMutation = useProvisionDaemonMutation();

	let isServerPoll = $derived(daemon.mode === 'server_poll');
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
		} catch {
			pushError(daemons_associateKeyFailed());
		} finally {
			associating = false;
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
	<!-- Only the icon, heading and button are centred; the explanatory bodies stay
	     left-aligned, since centred prose reads badly. -->
	<div class="space-y-4 py-4">
		<KeyRound class="text-tertiary mx-auto h-10 w-10" />
		<h3 class="text-primary text-center text-lg font-medium">{daemons_associateKey()}</h3>

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

		<div class="text-center">
			<button type="button" class="btn-primary" disabled={associating} onclick={associate}>
				{associating ? common_associating() : daemons_associateKeyCta()}
			</button>
		</div>
	</div>
{/if}
