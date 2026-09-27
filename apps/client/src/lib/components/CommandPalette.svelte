<script module lang="ts">
	import type { IconName } from './icons';

	export type PaletteEntry = {
		label: string;
		hint: string;
		icon: IconName;
		keywords?: string;
		run: () => void;
	};
</script>

<script lang="ts">
	// ⌘K / Ctrl+K "Quick jump" (§6: a global command control opens anything
	// without adding primary navigation). Navigation and shell actions only —
	// it never pretends to search content. Native <dialog> gives focus trap,
	// Escape and the backdrop for free.
	import Icon from './Icon.svelte';

	let { open = $bindable(false), entries }: { open: boolean; entries: PaletteEntry[] } = $props();

	let dialog: HTMLDialogElement;
	let input: HTMLInputElement;
	let query = $state('');
	let active = $state(0);

	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return entries;
		return entries.filter((entry) =>
			`${entry.label} ${entry.hint} ${entry.keywords ?? ''}`.toLowerCase().includes(q)
		);
	});

	$effect(() => {
		if (open && !dialog.open) {
			query = '';
			active = 0;
			dialog.showModal();
			input.focus();
		} else if (!open && dialog.open) {
			dialog.close();
		}
	});

	function choose(entry: PaletteEntry) {
		open = false;
		entry.run();
	}

	function move(step: number) {
		if (results.length === 0) return;
		active = (active + step + results.length) % results.length;
		document.getElementById(`palette-option-${active}`)?.scrollIntoView({ block: 'nearest' });
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			move(1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			move(-1);
		} else if (event.key === 'Enter' && results[active]) {
			event.preventDefault();
			choose(results[active]);
		}
	}
</script>

<!-- Keyboard lives on the combobox input; clicks here are pointer conveniences. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	class="palette"
	aria-label="Quick jump"
	data-testid="command-palette"
	onclose={() => (open = false)}
	onclick={(event) => {
		if (event.target === dialog) open = false;
	}}
>
	<div class="palette-field">
		<Icon name="jump" />
		<input
			bind:this={input}
			bind:value={query}
			oninput={() => (active = 0)}
			{onkeydown}
			type="text"
			placeholder="Jump to a page or action"
			aria-label="Jump to"
			role="combobox"
			aria-expanded="true"
			aria-controls="palette-list"
			aria-autocomplete="list"
			aria-activedescendant={results[active] ? `palette-option-${active}` : undefined}
			autocomplete="off"
			spellcheck="false"
			data-testid="command-input"
		/>
		<kbd class="kbd">Esc</kbd>
	</div>

	{#if results.length > 0}
		<ul id="palette-list" class="palette-list" role="listbox" aria-label="Destinations" style:--active={active}>
			{#each results as entry, i (entry.label)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<li
					id="palette-option-{i}"
					role="option"
					aria-selected={i === active}
					onpointermove={() => (active = i)}
					onclick={() => choose(entry)}
				>
					<Icon name={entry.icon} />
					<span class="palette-label">{entry.label}</span>
					<span class="palette-hint">{entry.hint}</span>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="palette-empty" role="status">Nothing matches “{query}”.</p>
	{/if}

	<p class="palette-foot" aria-hidden="true">
		<span><kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> move</span>
		<span><kbd class="kbd">Enter</kbd> open</span>
	</p>
</dialog>

<style>
	.palette {
		width: min(560px, calc(100vw - 2 * var(--space-lg)));
		max-height: min(560px, calc(100dvh - 20vh));
		margin: 12vh auto auto;
		padding: 0;
		border: 1px solid var(--color-border-strong);
		border-radius: var(--radius-card);
		background: color-mix(in oklab, var(--color-surface) 92%, transparent);
		backdrop-filter: blur(24px) saturate(1.4);
		box-shadow: var(--shadow-3);
		color: var(--color-text-primary);
		overflow: hidden;
		flex-direction: column;
		opacity: 1;
		transform: none;
		transition:
			opacity var(--dur-base) var(--ease-in),
			transform var(--dur-base) var(--ease-in),
			overlay var(--dur-base) allow-discrete,
			display var(--dur-base) allow-discrete;
	}

	.palette[open] {
		display: flex;
		transition-duration: var(--dur-slow);
		transition-timing-function: var(--ease-spring);
	}

	.palette:not([open]) {
		opacity: 0;
		transform: translateY(-8px) scale(0.97);
	}

	@starting-style {
		.palette[open] {
			opacity: 0;
			transform: translateY(-8px) scale(0.96);
		}
	}

	.palette::backdrop {
		background: var(--color-scrim);
		backdrop-filter: blur(2px);
		opacity: 1;
		transition:
			opacity var(--dur-slow) var(--ease-out),
			overlay var(--dur-slow) allow-discrete,
			display var(--dur-slow) allow-discrete;
	}

	@starting-style {
		.palette[open]::backdrop {
			opacity: 0;
		}
	}

	.palette-field {
		display: flex;
		align-items: center;
		gap: var(--space-md);
		padding: var(--space-md) var(--space-lg);
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text-secondary);
	}

	.palette-field input {
		flex: 1;
		min-height: 44px;
		border: 0;
		padding: 0;
		background: transparent;
		color: var(--color-text-primary);
		font-size: var(--text-body-lg);
	}

	.palette-field input:focus-visible {
		outline: none;
	}

	.palette-list {
		position: relative;
		list-style: none;
		margin: 0;
		padding: var(--space-sm);
		overflow-y: auto;
	}

	/* The highlight slides between rows; the rows themselves never move. */
	.palette-list::before {
		content: '';
		position: absolute;
		inset: var(--space-sm) var(--space-sm) auto;
		height: 44px;
		border-radius: var(--radius-control);
		background: var(--color-action-wash);
		box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--color-accent) 35%, transparent);
		transform: translateY(calc(var(--active) * 44px));
		transition: transform var(--dur-fast) var(--ease-out);
		pointer-events: none;
	}

	.palette-list li {
		position: relative;
		display: flex;
		align-items: center;
		gap: var(--space-md);
		height: 44px;
		padding: 0 var(--space-md);
		border-radius: var(--radius-control);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: color var(--dur-fast) var(--ease-out);
	}

	.palette-list li[aria-selected='true'] {
		color: var(--color-text-primary);
	}

	.palette-label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 600;
	}

	.palette-hint {
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
	}

	.palette-empty {
		margin: 0;
		padding: var(--space-xl) var(--space-lg);
		color: var(--color-text-secondary);
		text-align: center;
	}

	.palette-foot {
		display: flex;
		gap: var(--space-lg);
		margin: 0;
		padding: var(--space-sm) var(--space-lg);
		border-top: 1px solid var(--color-border);
		color: var(--color-text-secondary);
		font-size: var(--text-xs);
	}

	.palette-foot span {
		display: inline-flex;
		align-items: center;
		gap: var(--space-xs);
	}

	@media (pointer: coarse) {
		.palette-foot {
			display: none;
		}
	}
</style>
