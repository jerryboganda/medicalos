<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let today = $state(null);
	let loading = $state(true);
	let error = $state('');
	let startingTask = $state('');
	let pendingTakeover = $state(null);
	let undoing = $state('');
	let goals = $state(null);
	let goalsLoading = $state(true);
	let goalsError = $state('');
	let goalsEditorOpen = $state(false);
	let goalsDailyMinutes = $state('');
	let goalsExamDate = $state('');
	let goalCommitments = $state([]);
	let goalsStatus = $state('');
	let savingGoals = $state(false);
	let undoingGoals = $state(false);
	let quickLoading = $state(false);

	function syncGoalEditor(profile) {
		goalsDailyMinutes = profile.daily_minutes ?? '';
		goalsExamDate = profile.exam_date ?? '';
		goalCommitments = profile.protected_commitments.map((commitment) => ({ ...commitment }));
	}

	async function loadGoals() {
		goalsLoading = true;
		goalsError = '';
		try {
			goals = await Api.goals();
			syncGoalEditor(goals);
		} catch (err) {
			goalsError = err instanceof ApiError ? err.message : 'Could not load your goals.';
		} finally {
			goalsLoading = false;
		}
	}

	function addCommitment() {
		goalsStatus = '';
		goalCommitments = [...goalCommitments, { title: '', date: '' }];
	}

	function removeCommitment(index) {
		goalsStatus = '';
		goalCommitments = goalCommitments.filter((_, current) => current !== index);
	}

	function markGoalsDirty() {
		if (!savingGoals && !undoingGoals) goalsStatus = '';
	}

	async function saveGoals() {
		if (!goals || savingGoals) return;
		savingGoals = true;
		goalsStatus = 'Saving…';
		goalsError = '';
		try {
			goals = await Api.updateGoals({
				expected_version: goals.version,
				daily_minutes: goalsDailyMinutes === '' ? null : Number(goalsDailyMinutes),
				exam_date: goalsExamDate || null,
				protected_commitments: goalCommitments.map((commitment) => ({
					title: commitment.title,
					date: commitment.date
				}))
			});
			syncGoalEditor(goals);
			goalsStatus = 'Saved';
		} catch (err) {
			goalsError = err instanceof ApiError ? err.message : 'Could not save your goals.';
			goalsStatus = '';
		} finally {
			savingGoals = false;
		}
	}

	async function undoGoals() {
		if (!goals?.can_undo || undoingGoals) return;
		undoingGoals = true;
		goalsStatus = 'Undoing…';
		goalsError = '';
		try {
			goals = await Api.undoGoals(goals.version);
			syncGoalEditor(goals);
			goalsStatus = 'Saved';
		} catch (err) {
			goalsError = err instanceof ApiError ? err.message : 'Could not undo your last goal change.';
			goalsStatus = '';
		} finally {
			undoingGoals = false;
		}
	}

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

	async function openSession(body, busyKey) {
		if (startingTask) return;
		startingTask = busyKey;
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
			startingTask = '';
		}
	}

	async function confirmTakeover() {
		if (!pendingTakeover || startingTask) return;
		const pending = pendingTakeover;
		startingTask = pending.busyKey;
		error = '';
		try {
			const { session_id } = await Api.createSession({ ...pending.body, takeover: true });
			pendingTakeover = null;
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not take over the study session.';
			startingTask = '';
		}
	}

	async function startTask(task) {
		const body =
			task.kind === 'revision'
				? { preset: 'revision', source_session_id: task.source_session_id }
				: { preset: 'tutor', chapter_id: task.chapter_id, question_count: 10 };
		await openSession(body, task.id);
	}

	async function startTimed(task) {
		await openSession(
			{
				preset: 'timed',
				chapter_id: task.chapter_id,
				question_count: 10,
				time_limit_seconds: 300
			},
			`timed-${task.id}`
		);
	}

	async function startQuick10() {
		if (startingTask || quickLoading) return;
		quickLoading = true;
		error = '';
		try {
			const builder = await Api.builder();
			const chapterIds = builder.nodes.filter((node) => node.kind === 'chapter').map((node) => node.id);
			if (chapterIds.length === 0) {
				error = 'No question-bank chapters are available yet.';
				return;
			}
			quickLoading = false;
			await openSession(
				{
					preset: 'tutor',
					chapter_ids: chapterIds,
					question_count: 10,
					pool: 'all'
				},
				'quick-10'
			);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not start Quick 10.';
		} finally {
			quickLoading = false;
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
			goto(`${base}/login`);
			return;
		}
		await Promise.all([load(), loadGoals()]);
	});
</script>

<h1>Today</h1>

{#if loading}
	<p class="muted">Loading your plan…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	{#if pendingTakeover}
		<div class="takeover-actions">
			<button
				class="btn primary"
				type="button"
				disabled={startingTask !== ''}
				onclick={confirmTakeover}
				data-testid="confirm-takeover"
			>
				{startingTask ? 'Starting…' : 'Take over study session'}
			</button>
			<button
				class="btn"
				type="button"
				disabled={startingTask !== ''}
				onclick={() => {
					pendingTakeover = null;
					error = '';
				}}
			>
				Cancel
			</button>
		</div>
	{:else}
		<button class="btn" type="button" onclick={load}>Retry</button>
	{/if}
{:else if today}
	<section class="card" aria-labelledby="quick-practice-heading">
		<h2 id="quick-practice-heading">Quick practice</h2>
		<p class="muted">Start up to 10 available questions across the current question bank.</p>
		<button
			class="btn primary"
			type="button"
			disabled={startingTask !== '' || quickLoading}
			data-loading={quickLoading || startingTask === 'quick-10'}
			data-testid="quick-10"
			onclick={startQuick10}
		>
			{quickLoading || startingTask === 'quick-10' ? 'Starting…' : 'Quick 10'}
		</button>
	</section>

	<section class="card goals-card" aria-labelledby="goals-heading">
		<div class="goals-heading-row">
			<div>
				<h2 id="goals-heading">Study goals</h2>
				{#if goalsLoading}
					<p class="muted" data-testid="goal-summary">Loading goals…</p>
				{:else if goals}
					<div class="goal-summary" data-testid="goal-summary">
						<strong>{goals.daily_minutes === null ? 'No daily target' : `${goals.daily_minutes} min/day`}</strong>
						{#if goals.exam_date}
							<span class="muted">Exam date {goals.exam_date}</span>
						{/if}
						{#if goals.protected_commitments.length > 0}
							<ul>
								{#each goals.protected_commitments as commitment}
									<li>{commitment.title} · {commitment.date}</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/if}
			</div>
			<button
				class="btn"
				type="button"
				data-testid="goals-editor-toggle"
				aria-expanded={goalsEditorOpen}
				disabled={goalsLoading || !goals}
				onclick={() => (goalsEditorOpen = !goalsEditorOpen)}
			>
				{goalsEditorOpen ? 'Close' : 'Edit goals'}
			</button>
		</div>

		{#if goalsError}
			<p class="error-text" role="alert">{goalsError}</p>
		{/if}

		{#if goalsEditorOpen && goals}
			<div class="goals-editor">
				<label class="field">
					<span>Daily study target (minutes)</span>
					<input
						type="number"
						inputmode="numeric"
						bind:value={goalsDailyMinutes}
						oninput={markGoalsDirty}
						data-testid="goals-daily-minutes"
					/>
				</label>
				<label class="field">
					<span>Exam date (optional)</span>
					<input
						type="date"
						bind:value={goalsExamDate}
						oninput={markGoalsDirty}
						data-testid="goals-exam-date"
					/>
				</label>

				<div class="commitments">
					<div class="commitments-heading">
						<strong>Protected commitments</strong>
						<button class="btn" type="button" data-testid="add-commitment" onclick={addCommitment}>
							Add commitment
						</button>
					</div>
					{#each goalCommitments as commitment, index}
						<div class="commitment-row">
							<label class="field">
								<span>Commitment {index + 1}</span>
								<input
									type="text"
									bind:value={commitment.title}
									oninput={markGoalsDirty}
									data-testid={`commitment-title-${index}`}
								/>
							</label>
							<label class="field">
								<span>Date</span>
								<input
									type="date"
									bind:value={commitment.date}
									oninput={markGoalsDirty}
									data-testid={`commitment-date-${index}`}
								/>
							</label>
							<button class="btn danger-text" type="button" onclick={() => removeCommitment(index)}>
								Remove
							</button>
						</div>
					{/each}
				</div>

				<div class="goals-actions">
					<button
						class="btn primary"
						type="button"
						disabled={savingGoals || undoingGoals}
						data-loading={savingGoals}
						data-testid="save-goals"
						onclick={saveGoals}
					>
						{savingGoals ? 'Saving…' : 'Save goals'}
					</button>
					{#if goals.can_undo}
						<button
							class="btn"
							type="button"
							disabled={savingGoals || undoingGoals}
							data-testid="undo-goals"
							onclick={undoGoals}
						>
							{undoingGoals ? 'Undoing…' : 'Undo last change'}
						</button>
					{/if}
					<span class="muted" role="status" aria-live="polite" data-testid="goals-status">
						{goalsStatus}
					</span>
				</div>
			</div>
		{/if}
	</section>

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
				<p style="margin-bottom:0; display:flex; gap:12px; flex-wrap:wrap;">
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
					{#if task.kind === 'practice' && task.chapter_id}
						<button
							class="btn"
							type="button"
							disabled={startingTask !== ''}
							data-testid="task-timed"
							onclick={() => startTimed(task)}
						>
							{startingTask === `timed-${task.id}` ? 'Starting…' : 'Start timed (5 min)'}
						</button>
					{/if}
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

<style>
	/* Hallmark · macrostructure: App Shell · tone: calm utilitarian · anchor hue: violet */
	.goals-card {
		display: grid;
		gap: var(--space-md);
	}

	.takeover-actions {
		display: flex;
		gap: var(--space-md);
		flex-wrap: wrap;
	}

	.goals-heading-row,
	.commitments-heading,
	.goals-actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		flex-wrap: wrap;
	}

	.goals-heading-row h2,
	.goal-summary,
	.goal-summary ul,
	.goals-actions {
		margin: 0;
	}

	.goal-summary {
		display: grid;
		gap: var(--space-xs);
		margin-top: var(--space-xs);
	}

	.goal-summary ul {
		padding-left: var(--space-lg);
	}

	.goals-editor,
	.commitments {
		display: grid;
		gap: var(--space-md);
	}

	.commitment-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 180px) auto;
		gap: var(--space-md);
		align-items: end;
	}

	@media (max-width: 640px) {
		.commitment-row {
			grid-template-columns: 1fr;
		}
	}
</style>
