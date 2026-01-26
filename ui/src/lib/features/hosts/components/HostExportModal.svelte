<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { FileSpreadsheet, FileArchive } from 'lucide-svelte';
	import { downloadCsv, downloadHostsZip, type CsvExportParams } from '$lib/shared/utils/csvExport';

	let {
		isOpen = false,
		onClose,
		exportParams = {}
	}: {
		isOpen?: boolean;
		onClose: () => void;
		exportParams?: CsvExportParams;
	} = $props();

	let isExporting = $state(false);

	async function handleExportCsv() {
		isExporting = true;
		try {
			await downloadCsv('Host', exportParams);
			onClose();
		} finally {
			isExporting = false;
		}
	}

	async function handleExportZip() {
		isExporting = true;
		try {
			await downloadHostsZip(exportParams);
			onClose();
		} finally {
			isExporting = false;
		}
	}
</script>

<GenericModal title="Export Hosts" {isOpen} {onClose} size="sm">
	<div class="p-6">
		<p class="text-secondary mb-4 text-sm">Choose an export format:</p>

		<div class="space-y-3">
			<button
				onclick={handleExportCsv}
				disabled={isExporting}
				class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
			>
				<FileSpreadsheet class="text-tertiary h-6 w-6 shrink-0" />
				<div>
					<div class="text-primary font-medium">Hosts Only (CSV)</div>
					<div class="text-tertiary text-sm">Export host data to a single CSV file</div>
				</div>
			</button>

			<button
				onclick={handleExportZip}
				disabled={isExporting}
				class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
			>
				<FileArchive class="text-tertiary h-6 w-6 shrink-0" />
				<div>
					<div class="text-primary font-medium">Hosts with Children (ZIP)</div>
					<div class="text-tertiary text-sm">
						Export hosts, interfaces, ports, services, and SNMP entries as separate CSVs in a ZIP
						file
					</div>
				</div>
			</button>
		</div>
	</div>
</GenericModal>
