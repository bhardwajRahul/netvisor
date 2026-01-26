<script lang="ts">
	import { createColorHelper, AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import type { Group, EdgeStyle } from '../../types/base';
	import { Edit } from 'lucide-svelte';
	import {
		common_bezier,
		common_done,
		common_step,
		common_straight,
		groups_edgeColor,
		groups_edgeColorHelp,
		groups_edgeStyleHelp,
		groups_edgeStyleLabel,
		groups_editEdgeStyle,
		groups_simpleBezier,
		groups_smoothStep
	} from '$lib/paraglide/messages';

	let {
		formData = $bindable(),
		collapsed = $bindable(false),
		editable = true,
		showCollapseToggle = true,
		onColorChange,
		onEdgeStyleChange
	}: {
		formData: Group;
		collapsed?: boolean;
		editable?: boolean;
		showCollapseToggle?: boolean;
		onColorChange?: (color: Color) => void;
		onEdgeStyleChange?: (style: EdgeStyle) => void;
	} = $props();

	function handleColorChange(color: Color) {
		if (onColorChange) {
			onColorChange(color);
		} else {
			formData.color = color;
		}
	}

	function handleEdgeStyleChange(style: EdgeStyle) {
		if (onEdgeStyleChange) {
			onEdgeStyleChange(style);
		} else {
			formData.edge_style = style;
		}
	}

	let edgeStyleOptions = $derived([
		{ value: 'Straight' as const, label: common_straight() },
		{ value: 'SmoothStep' as const, label: groups_smoothStep() },
		{ value: 'Step' as const, label: common_step() },
		{ value: 'Bezier' as const, label: common_bezier() },
		{ value: 'SimpleBezier' as const, label: groups_simpleBezier() }
	]);

	// Ensure formData has default values if not set
	$effect(() => {
		if (!formData.color) {
			formData.color = 'Blue';
		}
		if (!formData.edge_style) {
			formData.edge_style = 'SmoothStep';
		}
	});

	let selectedColorHelper = $derived(createColorHelper(formData.color));
	let selectedEdgeStyleLabel = $derived(
		edgeStyleOptions.find((opt) => opt.value === formData.edge_style)?.label || groups_smoothStep()
	);
</script>

{#if showCollapseToggle && collapsed}
	<!-- Collapsed view -->
	<div class="flex items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<!-- Color indicator -->
			<div
				class="h-8 w-8 rounded-lg border-2 border-white ring-2 ring-white ring-offset-2 ring-offset-gray-800"
				style="background-color: {selectedColorHelper.rgb};"
				aria-label="Selected color: {formData.color}"
			></div>

			<!-- Edge style label -->
			<div class="flex flex-col">
				<span class="text-sm font-medium text-gray-200">{selectedEdgeStyleLabel}</span>
				<span class="text-xs capitalize text-gray-400">{formData.color}</span>
			</div>
		</div>

		<!-- Edit button -->
		<button
			type="button"
			onclick={() => (editable ? (collapsed = false) : {})}
			class="btn-icon"
			disabled={!editable}
			aria-label={groups_editEdgeStyle()}
		>
			<Edit size={16} />
		</button>
	</div>
{:else}
	<!-- Expanded view -->
	<div class="space-y-6">
		{#if showCollapseToggle}
			<!-- Header with collapse button -->
			<div class="flex items-center justify-between">
				<div class="block text-sm font-medium text-gray-200">{groups_editEdgeStyle()}</div>
				<button type="button" onclick={() => (collapsed = true)} class="btn-secondary text-xs">
					{common_done()}
				</button>
			</div>
		{/if}

		<!-- Edge Color Section -->
		<div class="space-y-3">
			<div class="block text-sm font-medium text-gray-200">{groups_edgeColor()}</div>
			<p class="text-xs text-gray-400">{groups_edgeColorHelp()}</p>

			<div class="grid grid-cols-7 gap-2">
				{#each AVAILABLE_COLORS as color (color)}
					{@const colorHelper = createColorHelper(color)}
					<button
						type="button"
						onclick={() => handleColorChange(color)}
						class="group relative aspect-square w-full rounded-lg border-2 transition-all hover:scale-110"
						class:border-gray-500={formData.color !== color}
						class:border-white={formData.color === color}
						class:ring-2={formData.color === color}
						class:ring-white={formData.color === color}
						class:ring-offset-2={formData.color === color}
						class:ring-offset-gray-800={formData.color === color}
						style="background-color: {colorHelper.rgb};"
						aria-label={`Select ${color} color`}
					>
						{#if formData.color === color}
							<div class="absolute inset-0 flex items-center justify-center">
								<svg
									class="h-5 w-5 text-white drop-shadow-lg"
									fill="currentColor"
									viewBox="0 0 20 20"
								>
									<path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/>
								</svg>
							</div>
						{/if}
					</button>
				{/each}
			</div>
		</div>

		<!-- Edge Style Section -->
		<div class="space-y-3">
			<div class="block text-sm font-medium text-gray-200">{groups_edgeStyleLabel()}</div>
			<p class="text-xs text-gray-400">{groups_edgeStyleHelp()}</p>

			<div class="space-y-2">
				{#each edgeStyleOptions as option (option.value)}
					<button
						type="button"
						onclick={() => handleEdgeStyleChange(option.value)}
						class="flex w-full items-center gap-3 rounded-lg border px-4 py-3 text-left transition-all hover:bg-gray-700/30"
						class:border-gray-600={formData.edge_style !== option.value}
						class:bg-gray-700-20={formData.edge_style !== option.value}
						class:border-blue-500={formData.edge_style === option.value}
						class:bg-blue-900-20={formData.edge_style === option.value}
						class:ring-1={formData.edge_style === option.value}
						class:ring-blue-500={formData.edge_style === option.value}
					>
						<div
							class="flex h-5 w-5 items-center justify-center rounded-full border-2 transition-all"
							class:border-gray-500={formData.edge_style !== option.value}
							class:border-blue-500={formData.edge_style === option.value}
							class:bg-blue-500={formData.edge_style === option.value}
						>
							{#if formData.edge_style === option.value}
								<div class="h-2 w-2 rounded-full bg-white"></div>
							{/if}
						</div>
						<span
							class="text-sm font-medium transition-colors"
							class:text-gray-300={formData.edge_style !== option.value}
							class:text-blue-400={formData.edge_style === option.value}
						>
							{option.label}
						</span>
					</button>
				{/each}
			</div>
		</div>
	</div>
{/if}
