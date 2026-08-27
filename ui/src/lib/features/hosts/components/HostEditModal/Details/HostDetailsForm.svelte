<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { hostnameFormat, max, required } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		common_contact,
		common_description,
		common_hostname,
		common_location,
		common_name,
		common_placeholderHostname,
		hosts_details_descriptionPlaceholder,
		hosts_details_namePlaceholder,
		common_manufacturer,
		common_model,
		common_serialNumber,
		common_firmwareRevision,
		hosts_hardwareInfo,
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysName,
		hosts_snmp_sysObjectId,
		hosts_snmp_systemInfo
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing?: boolean;
	}

	let { form, formData = $bindable(), isEditing = false }: Props = $props();

	// network_id is read/written directly against formData — no local
	// snapshot. A prior `$state(formData.network_id)` mirror captured the
	// value once at mount, went stale when HostEditor reassigned formData
	// via resetForm(host), and then got clobbered by SelectNetwork's
	// auto-default (first network) on the falsy initial capture.

	// Check if host has any SNMP system info
	let hasSnmpInfo = $derived(
		!!(
			formData.sys_descr ||
			formData.sys_object_id ||
			formData.sys_location ||
			formData.sys_contact ||
			formData.chassis_id ||
			formData.management_url ||
			formData.sys_name
		)
	);

	// Hardware identity gets its own card: manufacturer/model/serial also arrive from controller
	// integrations (UniFi, HPE Instant On), so they must not sit under an SNMP heading.
	let hasHardwareInfo = $derived(
		!!(formData.manufacturer || formData.model || formData.serial_number)
	);

	// The side column exists if either card has something to show.
	let hasDeviceInfo = $derived(hasSnmpInfo || hasHardwareInfo);
</script>

<div class="space-y-6 p-6">
	<div class="flex gap-6" class:flex-col={!isEditing || !hasDeviceInfo}>
		<!-- Form fields column -->
		<div class="min-w-0 space-y-6" class:flex-[3]={isEditing && hasDeviceInfo}>
			<div class="grid grid-cols-2 gap-6">
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_name()}
							id="name"
							placeholder={hosts_details_namePlaceholder()}
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<form.Field
					name="hostname"
					validators={{
						onBlur: ({ value }: { value: string }) => hostnameFormat(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_hostname()}
							id="hostname"
							placeholder={common_placeholderHostname()}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>

			<SelectNetwork
				selectedNetworkId={formData.network_id}
				onNetworkChange={(id) => (formData.network_id = id)}
			/>

			<form.Field
				name="description"
				validators={{
					onBlur: ({ value }: { value: string }) => max(500)(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextArea
						label={common_description()}
						id="description"
						placeholder={hosts_details_descriptionPlaceholder()}
						{field}
					/>
				{/snippet}
			</form.Field>

			<TagPicker bind:selectedTagIds={formData.tags} />
		</div>

		<!-- Device info column (only when editing and has data) -->
		{#if isEditing && hasDeviceInfo}
			<div class="flex-[2] space-y-6">
				{#if hasHardwareInfo}
					<InfoCard title={hosts_hardwareInfo()}>
						<InfoRow label={common_manufacturer()}>{formData.manufacturer || '-'}</InfoRow>
						<InfoRow label={common_model()} mono>{formData.model || '-'}</InfoRow>
						<InfoRow label={common_serialNumber()} mono>{formData.serial_number || '-'}</InfoRow>
						<InfoRow label={common_firmwareRevision()} mono
							>{formData.firmware_revision || '-'}</InfoRow
						>
					</InfoCard>
				{/if}
				{#if hasSnmpInfo}
					<InfoCard title={hosts_snmp_systemInfo()}>
						<InfoRow label={hosts_snmp_sysName()}>{formData.sys_name || '-'}</InfoRow>
						<InfoRow label={hosts_snmp_sysDescr()}>{formData.sys_descr || '-'}</InfoRow>
						<InfoRow label={hosts_snmp_sysObjectId()} mono>{formData.sys_object_id || '-'}</InfoRow>
						<InfoRow label={common_location()}>{formData.sys_location || '-'}</InfoRow>
						<InfoRow label={common_contact()}>{formData.sys_contact || '-'}</InfoRow>
						<InfoRow label={hosts_snmp_chassisId()} mono>{formData.chassis_id || '-'}</InfoRow>
						<InfoRow label={hosts_snmp_managementUrl()}>
							{#if formData.management_url}
								<!-- eslint-disable svelte/no-navigation-without-resolve -->
								<a
									href={formData.management_url}
									target="_blank"
									rel="external noopener noreferrer"
									class="break-all text-blue-400 hover:text-blue-300"
								>
									{formData.management_url}
								</a>
								<!-- eslint-enable svelte/no-navigation-without-resolve -->
							{:else}
								-
							{/if}
						</InfoRow>
					</InfoCard>
				{/if}
			</div>
		{/if}
	</div>
</div>
