<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let today = $state(null);
	let loading = $state(true);
	let error = $state('');
	let startingTask = $state('');
	let undoing = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			today = await Api.today();
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Could not load your plan. Check your connection and retry.';
		} finally {
			loading = false;
		}
	}

	async function startTask(task) {
		if (startingTask) return;
		startingTask = task.id;
		error = '';
		try {
			const body =
				task.kind === 'revision'
					? { preset: 'revision', source_session_id: task.source_session_id }
					: { preset: 'tutor', chapter_id: task.chapter_id, question_count: 10 };
			const { session_id } = await Api.createSession(body);
			goto(`/session/${session_id}`);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Could not start the session. Try again.';
			startingTask = '';
		}
	}

	async function undo(revision) {
		if (undoing) return;
		undoing = revision.id;
		error = '';
		try {
			await Api.undo(today.plan_id, revision.id);
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not undo. Try again.';
		} finally {
			undoing = '';
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto('/login');
			return;
		}
		await load();
	});
</script>

<h1>Today</h1>

{#if loading}
	<p class="muted">Loading your plan…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if today}
	{#if today.tasks.length === 0}
		<div class="card">
			<p class="muted">Nothing scheduled for today yet. Your plan appears as you study.</p>
		</div>
	{:else}
		{#each today.tasks as task (task.id)}
			<div class="card">
				<div style="display:flex; justify-content:space-between; gap:12px; align-items:center;">
					<h2 style="margin:0; font-size:var(--text-body-lg);">{task.title}</h2>
					<span class="chip {task.status === 'done' ? 'done' : ''}">
						{task.status === 'done' ? 'Done' : task.kind === 'revision' ? 'Re-practice' : 'Practice'}
					</span>
				</div>
				{#if task.status !== 'done'}
					<p style="margin-bottom:0;">
						<button
							class="btn primary"
							type="button"
							disabled={startingTask !== ''}
							data-loading={startingTask === task.id}
							data-testid={task.kind === 'revision' ? 'revision-start' : 'task-start'}
							onclick={() => startTask(task)}
						>
							{startingTask === task.id ? 'Starting…' : task.kind === 'revision' ? 'Start re-practice' : 'Start'}
						</button>
					</p>
				{/if}
			</div>
		{/each}
	{/if}

	{#if today.revisions.length > 0}
		<h2>Plan changes</h2>
		{#each today.revisions as revision (revision.id)}
			<div class="card" data-testid="revision-card">
				<span class="chip {revision.undone ? 'undone' : ''}">
					{revision.undone ? 'Undone' : revision.automatic ? 'Applied — automatic' : 'Applied'}
				</span>
				<p style="margin: var(--space-md) 0 0;">{revision.explanation}</p>
				{#if !revision.undone}
					<button
						class="btn danger-text"
						type="button"
						disabled={undoing !== ''}
						data-testid="undo"
						onclick={() => undo(revision)}
					>
						{undoing === revision.id ? 'Undoing…' : 'Undo'}
					</button>
				{/if}
			</div>
		{/each}
	{/if}

	<h2>Progress</h2>
	{#if today.learner.length === 0}
		<div class="card">
			<p class="muted" data-testid="no-evidence">
				No evidence yet — answer questions and your chapters appear here.
			</p>
		</div>
	{:else}
		{#each today.learner as chapter (chapter.chapter_id)}
			<div class="card">
				<strong>{chapter.chapter_name}</strong>
				<p class="muted" style="margin: 4px 0 0;">
					{#if chapter.mastery_index === null}
						Not enough evidence yet ({chapter.independent_count} answered) — no
						mastery shown until it means something.
					{:else}
						Accuracy {chapter.mastery_index}% · evidence: {chapter.evidence_level.replace('_', ' ')}
					{/if}
				</p>
			</div>
		{/each}
	{/if}
{/if}
