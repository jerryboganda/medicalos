<script lang="ts">
	// The brand emblem (§7.2 "medical/scientific emblem"): an ECG pulse on a
	// violet tile. Same artwork as packages/design-system/emblem.svg. The
	// pulse draws itself once when the shell first mounts.
	const id = $props.id();
</script>

<svg class="emblem" width="28" height="28" viewBox="0 0 32 32" aria-hidden="true" focusable="false">
	<defs>
		<linearGradient id="{id}-g" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" class="stop-a" />
			<stop offset="1" class="stop-b" />
		</linearGradient>
	</defs>
	<rect width="32" height="32" rx="8" fill="url(#{id}-g)" />
	<path
		class="pulse"
		d="M5 17h5.2l2.3-6.5 4.2 12 3-8.5 1.6 3H27"
		fill="none"
		stroke-width="2.4"
		stroke-linecap="round"
		stroke-linejoin="round"
		pathLength="100"
	/>
</svg>

<style>
	.emblem {
		flex: none;
		display: block;
		border-radius: 8px;
		box-shadow: 0 6px 18px -8px color-mix(in oklab, var(--color-action-primary) 80%, transparent);
	}

	.stop-a {
		stop-color: var(--color-action-hover);
	}

	.stop-b {
		stop-color: var(--color-action-primary);
	}

	.pulse {
		stroke: var(--color-on-action);
		--path-length: 100;
		stroke-dasharray: 100;
		animation: mlos-draw 1100ms var(--ease-out) 120ms backwards;
	}
</style>
