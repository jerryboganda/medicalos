<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	const PRESETS_KEY = 'medical-os:qbank-builder:presets:v1';
	const LAST_KEY = 'medical-os:qbank-builder:last:v1';
	const COUNTS = [5, 10, 20, 25, 40, 50, 100];
	const POOLS = [
		['all', 'All questions'],
		['incorrect_skipped', 'Incorrect + skipped'],
		['unattempted', 'Unattempted'],
		['marked', 'Marked']
	];
	const DIFFICULTIES = ['easy', 'medium', 'hard'];

	let builder = $state(null);
	let today = $state(null);
	let loading = $state(true);
	let availabilityLoading = $state(false);
	let error = $state('');
	let starting = $state('');
	let pendingTakeover = $state(null);
	let search = $state('');
	let selectedNodeIds = $state([]);
	let pool = $state('all');
	let difficulties = $state([]);
	let highYield = $state(false);
	let countChoice = $state(10);
	let customCount = $state('10');
	let preset = $state('tutor');
	let presetName = $state('');
	let savedPresets = $state([]);
	let lastSetup = $state(null);
	let storageMessage = $state('');
	let availabilityRequest = 0;

	function tasks() {
		return (
			today?.tasks.filter(
				(task) => task.kind === 'practice' && task.chapter_id && task.status !== 'done'
			) ?? []
		);
	}

	function nodeDepth(node) {
		let depth = 0;
		let parentId = node.parent_id;
		while (parentId && builder) {
			const parent = builder.nodes.find((candidate) => candidate.id === parentId);
			if (!parent) break;
			depth += 1;
			parentId = parent.parent_id;
		}
		return depth;
	}

	function visibleNodes() {
		const term = search.trim().toLowerCase();
		if (!builder || !term) return builder?.nodes ?? [];
		return builder.nodes.filter((node) => node.name.toLowerCase().includes(term));
	}

	function chaptersForNode(nodeId) {
		if (!builder) return [];
		const chapters = [];
		const visit = (id) => {
			const node = builder.nodes.find((candidate) => candidate.id === id);
			if (!node) return;
			if (node.kind === 'chapter') chapters.push(node.id);
			for (const child of builder.nodes.filter((candidate) => candidate.parent_id === id)) {
				visit(child.id);
			}
		};
		visit(nodeId);
		return chapters;
	}

	function selectedChapterIds() {
		if (!builder) return [];
		if (selectedNodeIds.length === 0) {
			return builder.nodes.filter((node) => node.kind === 'chapter').map((node) => node.id);
		}
		const ids = new Set();
		for (const nodeId of selectedNodeIds) {
			for (const chapterId of chaptersForNode(nodeId)) ids.add(chapterId);
		}
		return [...ids];
	}

	function currentFilters() {
		return {
			chapter_ids: selectedChapterIds(),
			pool,
			difficulties,
			high_yield: highYield
		};
	}

	function requestedCount() {
		if (countChoice === 'all') return null;
		const value = countChoice === 'custom' ? Number(customCount) : Number(countChoice);
		return Math.min(100, Math.max(1, Number.isFinite(value) ? Math.floor(value) : 10));
	}

	function availabilityText() {
		if (!builder) return '';
		const available = builder.selection.matching;
		if (availabilityLoading) return 'Checking availability…';
		if (available === 0) return 'No questions are available for this setup.';
		const requested = requestedCount();
		if (requested !== null && requested > available) {
			return `Only ${available} questions are available for this setup. Your session will use ${available}.`;
		}
		return `${available} question${available === 1 ? '' : 's'} available for this setup.`;
	}

	function setupSnapshot() {
		return {
			selectedNodeIds: [...selectedNodeIds],
			pool,
			difficulties: [...difficulties],
			highYield,
			countChoice,
			customCount,
			preset
		};
	}

	function readStorage(key, fallback) {
		try {
			const value = localStorage.getItem(key);
			return value ? JSON.parse(value) : fallback;
		} catch {
			storageMessage = 'Saved setups are unavailable in this browser session.';
			return fallback;
		}
	}

	function writeStorage(key, value) {
		try {
			localStorage.setItem(key, JSON.stringify(value));
			return true;
		} catch {
			storageMessage = 'Saved setups are unavailable in this browser session.';
			return false;
		}
	}

	function loadLocalSetups() {
		savedPresets = readStorage(PRESETS_KEY, []);
		lastSetup = readStorage(LAST_KEY, null);
	}

	async function applySetup(setup) {
		if (!setup) return;
		const validIds = new Set(builder?.nodes.map((node) => node.id) ?? []);
		selectedNodeIds = (setup.selectedNodeIds ?? []).filter((id) => validIds.has(id));
		pool = POOLS.some(([value]) => value === setup.pool) ? setup.pool : 'all';
		difficulties = (setup.difficulties ?? []).filter((value) => DIFFICULTIES.includes(value));
		highYield = Boolean(setup.highYield);
		countChoice = [...COUNTS, 'custom', 'all'].includes(setup.countChoice) ? setup.countChoice : 10;
		customCount = String(setup.customCount ?? '10');
		preset = setup.preset === 'timed' ? 'timed' : 'tutor';
		await refreshAvailability();
	}

	function savePreset() {
		const name = presetName.trim();
		if (!name) return;
		const next = [
			{ name, setup: setupSnapshot() },
			...savedPresets.filter((saved) => saved.name.toLowerCase() !== name.toLowerCase())
		].slice(0, 12);
		if (writeStorage(PRESETS_KEY, next)) {
			savedPresets = next;
			presetName = '';
		}
	}

	async function load() {
		loading = true;
		error = '';
		try {
			const [builderResult, todayResult] = await Promise.allSettled([Api.builder(), Api.today()]);
			if (builderResult.status === 'rejected') throw builderResult.reason;
			builder = builderResult.value;
			today = todayResult.status === 'fulfilled' ? todayResult.value : null;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load the question bank builder.';
		} finally {
			loading = false;
		}
	}

	async function refreshAvailability() {
		if (!builder) return;
		const request = ++availabilityRequest;
		availabilityLoading = true;
		error = '';
		try {
			const next = await Api.builder(currentFilters());
			if (request === availabilityRequest) builder = next;
		} catch (err) {
			if (request === availabilityRequest) {
				error = err instanceof ApiError ? err.message : 'Could not refresh question availability.';
			}
		} finally {
			if (request === availabilityRequest) availabilityLoading = false;
		}
	}

	async function toggleNode(nodeId) {
		selectedNodeIds = selectedNodeIds.includes(nodeId)
			? selectedNodeIds.filter((id) => id !== nodeId)
			: [...selectedNodeIds, nodeId];
		await refreshAvailability();
	}

	async function toggleDifficulty(value) {
		difficulties = difficulties.includes(value)
			? difficulties.filter((difficulty) => difficulty !== value)
			: [...difficulties, value];
		await refreshAvailability();
	}

	async function openSession(body, busyKey) {
		if (starting) return;
		starting = busyKey;
		error = '';
		pendingTakeover = null;
		try {
			const { session_id } = await Api.createSession(body);
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			if (err instanceof ApiError && err.code === 'active_study_session') {
				pendingTakeover = { body, busyKey };
				error =
					'Another study session is already active. Taking over will end that open session and start this one.';
			} else {
				error = err instanceof ApiError ? err.message : 'Could not start the session. Try again.';
			}
			starting = '';
		}
	}

	async function startBuilderSession() {
		const chapterIds = selectedChapterIds();
		if (chapterIds.length === 0 || !builder?.selection.matching) return;
		const body = {
			preset,
			chapter_ids: chapterIds,
			pool,
			difficulties,
			high_yield: highYield,
			all_available: countChoice === 'all'
		};
		if (countChoice !== 'all') body.question_count = requestedCount();
		if (preset === 'timed') body.time_limit_seconds = 300;
		const setup = setupSnapshot();
		if (writeStorage(LAST_KEY, setup)) lastSetup = setup;
		await openSession(body, 'builder');
	}

	async function start(task, taskPreset) {
		const body = {
			preset: taskPreset,
			chapter_id: task.chapter_id,
			question_count: task.question_count || 10
		};
		if (taskPreset === 'timed') body.time_limit_seconds = 300;
		await openSession(body, `${taskPreset}-${task.id}`);
	}

	async function confirmTakeover() {
		if (!pendingTakeover || starting) return;
		const pending = pendingTakeover;
		starting = pending.busyKey;
		error = '';
		try {
			const { session_id } = await Api.createSession({ ...pending.body, takeover: true });
			pendingTakeover = null;
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not take over the study session.';
			starting = '';
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		loadLocalSetups();
		await load();
	});
</script>

<h1>Practice</h1>

{#if loading}
	<p class="muted">Loading the question bank…</p>
{:else}
	{#if error}
		<p class="error-text" role="alert">{error}</p>
		{#if pendingTakeover}
			<div class="actions">
				<button class="btn primary" type="button" onclick={confirmTakeover} data-testid="confirm-takeover">
					{starting ? 'Starting…' : 'Take over study session'}
				</button>
				<button
					class="btn"
					type="button"
					onclick={() => {
						pendingTakeover = null;
						error = '';
					}}
				>
					Cancel
				</button>
			</div>
		{:else if !builder}
			<button class="btn" type="button" onclick={load}>Retry</button>
		{/if}
	{/if}

	{#if builder}
		<section class="card builder" data-testid="qbank-builder" aria-labelledby="builder-heading">
			<div class="builder-heading">
				<div>
					<h2 id="builder-heading">Build a question session</h2>
					<p class="muted">Choose curriculum, pool, filters, and session size on one screen.</p>
				</div>
				<div class="mode-switch" role="group" aria-label="Session mode">
					<button class="btn" class:primary={preset === 'tutor'} type="button" aria-pressed={preset === 'tutor'} onclick={() => (preset = 'tutor')}>
						Tutor
					</button>
					<button class="btn" class:primary={preset === 'timed'} type="button" aria-pressed={preset === 'timed'} onclick={() => (preset = 'timed')}>
						Timed · 5 min
					</button>
				</div>
			</div>

			<details open>
				<summary>Curriculum</summary>
				<label class="field search-field">
					<span>Search curriculum</span>
					<input data-testid="builder-search" type="search" bind:value={search} placeholder="Search subjects, systems, chapters" />
				</label>
				<div class="tree" data-testid="builder-tree">
					{#each visibleNodes() as node (node.id)}
						<button
							class="tree-row"
							class:selected={selectedNodeIds.includes(node.id)}
							type="button"
							aria-pressed={selectedNodeIds.includes(node.id)}
							data-testid="builder-node"
							style={`padding-left: calc(var(--space-md) + ${nodeDepth(node)} * var(--space-lg));`}
							onclick={() => toggleNode(node.id)}
						>
							<span><strong>{node.name}</strong><small>{node.kind}</small></span>
							<span class="node-counts">{node.available} available · {node.attempted} attempted · {node.unattempted} unattempted</span>
						</button>
					{/each}
					{#if visibleNodes().length === 0}
						<p class="muted">No curriculum matches that search.</p>
					{/if}
				</div>
				<p class="muted selection-summary">
					{selectedNodeIds.length === 0 ? 'All curriculum selected.' : `${selectedChapterIds().length} chapter${selectedChapterIds().length === 1 ? '' : 's'} selected.`}
				</p>
			</details>

			<details open>
				<summary>Question filters</summary>
				<fieldset class="choice-grid" data-testid="builder-pool">
					<legend>Pool</legend>
					{#each POOLS as [value, label]}
						<label>
							<input type="radio" name="pool" value={value} bind:group={pool} onchange={refreshAvailability} />
							<span>{label}</span>
						</label>
					{/each}
				</fieldset>

				<fieldset class="choice-grid">
					<legend>Difficulty</legend>
					{#each DIFFICULTIES as value}
						<label>
							<input
								type="checkbox"
								checked={difficulties.includes(value)}
								onchange={() => toggleDifficulty(value)}
							/>
							<span>{value[0].toUpperCase() + value.slice(1)}</span>
						</label>
					{/each}
					<label>
						<input type="checkbox" bind:checked={highYield} onchange={refreshAvailability} />
						<span>High-yield only</span>
					</label>
				</fieldset>
			</details>

			<details open>
				<summary>Session size</summary>
				<div class="count-grid">
					{#each COUNTS as count}
						<button
							class="btn"
							class:primary={countChoice === count}
							type="button"
							aria-pressed={countChoice === count}
							data-testid={`builder-count-${count}`}
							onclick={() => (countChoice = count)}
						>
							{count}
						</button>
					{/each}
					<button class="btn" class:primary={countChoice === 'custom'} type="button" aria-pressed={countChoice === 'custom'} onclick={() => (countChoice = 'custom')}>
						Custom
					</button>
					<button class="btn" class:primary={countChoice === 'all'} type="button" aria-pressed={countChoice === 'all'} onclick={() => (countChoice = 'all')}>
						All available
					</button>
				</div>
				{#if countChoice === 'custom'}
					<label class="field custom-count">
						<span>Custom count · 1–100</span>
						<input type="number" min="1" max="100" inputmode="numeric" bind:value={customCount} />
					</label>
				{/if}
			</details>

			<div class="availability" data-testid="builder-availability" role="status" aria-live="polite">
				<strong>{availabilityText()}</strong>
				<span class="muted">All {builder.selection.all} · Attempted {builder.selection.attempted} · Unattempted {builder.selection.unattempted} · Incorrect/skipped {builder.selection.incorrect_skipped} · Marked {builder.selection.marked}</span>
			</div>

			<details open>
				<summary>Saved setups</summary>
				<div class="preset-save">
					<label class="field">
						<span>Preset name</span>
						<input data-testid="builder-preset-name" type="text" maxlength="60" bind:value={presetName} placeholder="e.g. Easy focus" />
					</label>
					<button class="btn" type="button" data-testid="builder-save-preset" disabled={!presetName.trim()} onclick={savePreset}>
						Save preset
					</button>
				</div>
				<div class="saved-presets" data-testid="builder-saved-presets">
					{#if lastSetup}
						<button class="btn" type="button" data-testid="builder-repeat-last" onclick={() => applySetup(lastSetup)}>Repeat last setup</button>
					{/if}
					{#each savedPresets as saved (saved.name)}
						<button class="btn" type="button" onclick={() => applySetup(saved.setup)}>{saved.name}</button>
					{/each}
					{#if !lastSetup && savedPresets.length === 0}
						<span class="muted">No saved setups yet.</span>
					{/if}
				</div>
				{#if storageMessage}<p class="muted" role="status">{storageMessage}</p>{/if}
			</details>

			<div class="builder-actions">
				<button
					class="btn primary"
					type="button"
					disabled={starting !== '' || availabilityLoading || builder.selection.matching === 0}
					data-loading={starting === 'builder'}
					data-testid="builder-start"
					onclick={startBuilderSession}
				>
					{starting === 'builder' ? 'Starting…' : preset === 'timed' ? 'Start timed session' : 'Start tutor session'}
				</button>
			</div>
		</section>

		{#if tasks().length > 0}
			<h2>Today's planned practice</h2>
			{#each tasks() as task (task.id)}
				<div class="card" data-testid="practice-task">
					<h3>{task.title}</h3>
					<p class="muted">{task.question_count} planned questions</p>
					<div class="actions">
						<button class="btn primary" type="button" disabled={starting !== ''} data-testid="practice-tutor" onclick={() => start(task, 'tutor')}>
							{starting === `tutor-${task.id}` ? 'Starting…' : 'Start tutor'}
						</button>
						<button class="btn" type="button" disabled={starting !== ''} data-testid="practice-timed" onclick={() => start(task, 'timed')}>
							{starting === `timed-${task.id}` ? 'Starting…' : 'Start timed (5 min)'}
						</button>
					</div>
				</div>
			{/each}
		{/if}
	{/if}
{/if}

<style>
	/* Hallmark · pre-emit critique: P5 H4 E4 S5 R5 V4 · existing Medical OS tokens preserved. */
	.actions,
	.mode-switch,
	.builder-actions,
	.saved-presets,
	.count-grid {
		display: flex;
		gap: var(--space-sm);
		flex-wrap: wrap;
	}

	.builder-heading,
	.preset-save {
		display: flex;
		gap: var(--space-lg);
		align-items: end;
		justify-content: space-between;
	}

	.builder-heading h2,
	.builder-heading p {
		margin-top: 0;
	}

	details {
		border-top: 1px solid var(--color-surface-elevated);
		padding: var(--space-md) 0;
	}

	summary {
		cursor: pointer;
		font-weight: 700;
		margin-bottom: var(--space-md);
	}

	.search-field,
	.custom-count {
		max-width: 520px;
	}

	.tree {
		display: grid;
		gap: var(--space-xs);
		max-height: 360px;
		overflow: auto;
		padding-right: var(--space-xs);
	}

	.tree-row {
		display: flex;
		width: 100%;
		min-height: 48px;
		gap: var(--space-md);
		align-items: center;
		justify-content: space-between;
		text-align: left;
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: var(--text-body) var(--font-body);
		cursor: pointer;
	}

	.tree-row:hover {
		background: var(--color-surface-hover);
	}

	.tree-row:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.tree-row.selected {
		border-color: var(--color-action-primary);
		background: var(--color-action-wash);
	}

	.tree-row small {
		display: block;
		color: var(--color-text-secondary);
		margin-top: var(--space-xs);
	}

	.node-counts {
		color: var(--color-text-secondary);
		font-size: var(--text-sm);
		text-align: right;
	}

	.selection-summary {
		margin-bottom: 0;
	}

	.choice-grid {
		display: flex;
		gap: var(--space-sm) var(--space-lg);
		flex-wrap: wrap;
		border: 0;
		padding: 0;
		margin: 0 0 var(--space-lg);
	}

	.choice-grid legend {
		font-weight: 700;
		margin-bottom: var(--space-sm);
	}

	.choice-grid label {
		display: inline-flex;
		gap: var(--space-xs);
		align-items: center;
		min-height: 44px;
	}

	.availability {
		display: grid;
		gap: var(--space-xs);
		padding: var(--space-md);
		margin: var(--space-md) 0;
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-surface);
	}

	.preset-save .field {
		flex: 1 1 260px;
		margin-bottom: 0;
	}

	@media (max-width: 700px) {
		.builder-heading,
		.preset-save,
		.tree-row {
			align-items: stretch;
			flex-direction: column;
		}

		.node-counts {
			text-align: left;
		}
	}
</style>
