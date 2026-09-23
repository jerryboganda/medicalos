<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let chapters = $state<
		{
			chapter_id: string;
			chapter_name: string;
			system: string;
			subject: string;
			published_questions: number;
		}[]
	>([]);
	let selected = $state<string[]>([]);
	let pool = $state('any');
	let questionCount = $state(10);
	let startingBuilder = $state(false);
	let builderError = $state('');

	let mocks = $state<
		{
			mock_id: string;
			title: string;
			pass_mark_percent: number;
			attempts_allowed: number;
			attempts_used: number;
			time_limit_seconds: number | null;
		}[]
	>([]);
	let loading = $state(true);
	let startingMock = $state('');
	let error = $state('');

	async function startMock(mock: (typeof mocks)[0]) {
		if (startingMock) return;
		startingMock = mock.mock_id;
		error = '';
		try {
			const res = await Api.startMock(mock.mock_id);
			goto(`${base}/session/${res.session_id}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Could not start the mock.';
			startingMock = '';
		}
	}

	function toggleChapter(id: string) {
		selected = selected.includes(id)
			? selected.filter((c) => c !== id)
			: [...selected, id];
	}

	async function startBuilderSession() {
		if (startingBuilder || selected.length === 0) return;
		startingBuilder = true;
		builderError = '';
		try {
			const res = await Api.createSession({
				preset: 'tutor',
				chapter_ids: selected,
				source: pool,
				question_count: questionCount
			});
			goto(`${base}/session/${res.session_id}`);
		} catch (err) {
			builderError =
				err instanceof ApiError ? err.message : 'Could not start the session.';
			startingBuilder = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		try {
			mocks = (await Api.listMocks()).mocks;
		} catch {
			mocks = [];
		}
		try {
			chapters = (await Api.myCurriculum()).chapters;
		} catch {
			chapters = [];
		} finally {
			loading = false;
		}
	});
</script>

<h1>Practice</h1>

{#if loading}
	<p class="muted">Loading…</p>
{:else}
	<div class="card">
		<h2>Mock tests</h2>
		{#if mocks.length === 0}
			<p class="muted">No mock tests are available yet.</p>
		{:else}
			{#each mocks as mock (mock.mock_id)}
				<div class="card" style="margin-bottom: var(--space-md);" data-testid="mock-card">
					<div style="display:flex; justify-content:space-between; gap:12px; align-items:center;">
						<strong>{mock.title}</strong>
						<span class="chip">Pass {mock.pass_mark_percent}%</span>
					</div>
					<p class="muted" style="margin: var(--space-xs) 0;">
						{mock.attempts_used}/{mock.attempts_allowed} attempts
						{#if mock.time_limit_seconds}
							· {Math.round(mock.time_limit_seconds / 60)} min
						{/if}
					</p>
					{#if mock.attempts_used < mock.attempts_allowed}
						<button
							class="btn primary"
							type="button"
							disabled={startingMock !== ''}
							data-testid="mock-start"
							onclick={() => startMock(mock)}
						>
							{startingMock === mock.mock_id ? 'Starting…' : 'Start mock'}
						</button>
					{:else}
						<p class="muted" style="margin:0;">All attempts used.</p>
					{/if}
				</div>
			{/each}
		{/if}
	</div>

	<div class="card">
		<h2>Build a practice session</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Pick one or more chapters, choose which pool the questions come
			from, and set the length. An empty pool says so honestly — nothing
			is invented to fill it.
		</p>
		{#if chapters.length === 0}
			<p class="muted">No curriculum is available yet.</p>
		{:else}
			<fieldset style="border:0; padding:0; margin:0 0 var(--space-md);">
				<legend class="muted" style="font-size: var(--text-sm);">
					Chapters ({selected.length} selected)
				</legend>
				{#each chapters as ch (ch.chapter_id)}
					<label
						style="display:flex; gap:8px; align-items:center; margin: var(--space-xs) 0;"
					>
						<input
							type="checkbox"
							checked={selected.includes(ch.chapter_id)}
							onchange={() => toggleChapter(ch.chapter_id)}
							data-testid={`builder-ch-${ch.chapter_id}`}
						/>
						<span>
							{ch.chapter_name}
							<span class="muted" style="font-size: var(--text-sm);">
								· {ch.system} · {ch.published_questions} published
							</span>
						</span>
					</label>
				{/each}
			</fieldset>
			<div style="display:flex; gap:12px; flex-wrap:wrap; align-items:end;">
				<label class="field" for="builder-pool">
					<span>Pool</span>
					<select id="builder-pool" bind:value={pool}>
						<option value="any">All questions</option>
						<option value="unseen">Unseen</option>
						<option value="incorrect">Incorrect</option>
						<option value="marked">Marked</option>
					</select>
				</label>
				<label class="field" for="builder-count">
					<span>Questions</span>
					<input
						id="builder-count"
						type="number"
						min="1"
						max="50"
						bind:value={questionCount}
					/>
				</label>
				<button
					class="btn primary"
					type="button"
					disabled={startingBuilder || selected.length === 0}
					data-testid="builder-start"
					onclick={startBuilderSession}
				>
					{startingBuilder ? 'Starting…' : 'Start session'}
				</button>
			</div>
			{#if builderError}
				<p class="danger-text" data-testid="builder-error">{builderError}</p>
			{/if}
		{/if}
	</div>

	<div class="card">
		<h2>Study tools</h2>
		<p style="margin:0;">
			<a class="btn" href={`${base}/review`}>Review flashcards</a>
			<a class="btn" href={`${base}/coach`}>Ask the Coach</a>
			<a class="btn" href={`${base}/notes`}>My notes</a>
		</p>
	</div>
{/if}
