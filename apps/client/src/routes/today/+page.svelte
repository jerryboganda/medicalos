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
	let undoing = $state('');
	let mocks = $state(null);
	let startingMock = $state('');
	let engagement = $state(null);
	let answeringQotd = $state(false);
	let qotdError = $state('');
	let qotdResult = $state(null);

	async function loadMocks() {
		try {
			mocks = (await Api.listMocks()).mocks;
		} catch {
			mocks = [];
		}
	}

	async function loadEngagement() {
		try {
			engagement = await Api.engagement();
		} catch {
			engagement = null;
		}
	}

	async function setEngagement(patch) {
		qotdError = '';
		try {
			await Api.updateEngagementSettings(patch);
			engagement = await Api.engagement();
		} catch (err) {
			qotdError = err instanceof ApiError ? err.message : 'Could not update settings.';
		}
	}

	async function answerQotd(index) {
		if (answeringQotd || !engagement?.qotd?.question_version_id) return;
		answeringQotd = true;
		qotdError = '';
		try {
			qotdResult = await Api.answerQotd(engagement.qotd.question_version_id, index);
			engagement = await Api.engagement();
		} catch (err) {
			qotdError = err instanceof ApiError ? err.message : 'Could not save your answer.';
		} finally {
			answeringQotd = false;
		}
	}

	async function startMock(mock) {
		if (startingMock) return;
		startingMock = mock.mock_id;
		error = '';
		try {
			const { session_id } = await Api.startMock(mock.mock_id);
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not start the mock.';
			startingMock = '';
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
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Could not start the session. Try again.';
			startingTask = '';
		}
	}

	async function startTimed(task) {
		if (startingTask) return;
		startingTask = `timed-${task.id}`;
		error = '';
		try {
			const { session_id } = await Api.createSession({
				preset: 'timed',
				chapter_id: task.chapter_id,
				question_count: 10,
				time_limit_seconds: 300
			});
			goto(`${base}/session/${session_id}`);
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
			goto(`${base}/login`);
			return;
		}
		await Promise.all([load(), loadMocks(), loadEngagement()]);
	});
</script>

<h1>Today</h1>

{#if engagement?.enabled}
	<div class="card" data-testid="engagement-card">
		{#if engagement.daily_goal.enabled}
			<div style="display:flex; justify-content:space-between; gap:12px; align-items:center;">
				<strong data-testid="daily-goal">
					Daily goal {engagement.daily_goal.answered_today}/{engagement.daily_goal.target}
				</strong>
				{#if engagement.daily_goal.met}
					<span class="chip done" data-testid="goal-met">Met</span>
				{/if}
			</div>
		{/if}
		{#if engagement.streak.enabled}
			<p class="muted" style="margin: var(--space-sm) 0;" data-testid="streak">
				{engagement.streak.count}-day streak · {engagement.streak.freezes} freezes held
			</p>
		{/if}
		{#if engagement.qotd.enabled}
			{#if engagement.qotd.answered}
				<div data-testid="qotd-answered">
					{#if qotdResult}
						<p style="margin: var(--space-sm) 0;" data-testid="qotd-verdict">
							{qotdResult.correct ? 'Correct' : 'Not correct'} — correct answer was
							option {qotdResult.correct_index + 1}.
						</p>
					{:else}
						<p class="muted" style="margin: var(--space-sm) 0;">Question of the day — answered.</p>
					{/if}
					{#if engagement.qotd.community_split?.length}
						<p class="muted" style="margin: 0;" data-testid="qotd-community">
							{#each engagement.qotd.community_split as part (part.chosen_index)}
								<span class="chip">Option {part.chosen_index + 1}: {part.count}</span>
							{/each}
							<span class="muted">of {engagement.qotd.community_total} answered</span>
						</p>
					{/if}
				</div>
			{:else if engagement.qotd.available}
				<p class="muted" style="margin: var(--space-sm) 0 4px;">Question of the day</p>
				<p style="margin: 0 0 var(--space-sm);" data-testid="qotd-vignette">{engagement.qotd.vignette}</p>
				<div style="display:flex; flex-direction:column; gap:8px;">
					{#each engagement.qotd.options ?? [] as option, i (i)}
						<button
							class="btn"
							type="button"
							disabled={answeringQotd}
							data-testid={`qotd-option-${i}`}
							onclick={() => answerQotd(i)}
						>
							{answeringQotd ? 'Saving…' : option.text}
						</button>
					{/each}
				</div>
			{/if}
		{/if}
		{#if qotdError}
			<p class="error-text" role="alert" data-testid="qotd-error">{qotdError}</p>
		{/if}
		<div style="display:flex; gap:8px; flex-wrap:wrap; margin-top:var(--space-sm);">
			<button
				class="btn"
				type="button"
				data-testid="toggle-goal"
				onclick={() => setEngagement({ daily_goal_enabled: !engagement.daily_goal.enabled })}
			>
				{engagement.daily_goal.enabled ? 'Turn off daily goal' : 'Turn on daily goal'}
			</button>
			<button
				class="btn"
				type="button"
				data-testid="toggle-streak"
				onclick={() => setEngagement({ streak_enabled: !engagement.streak.enabled })}
			>
				{engagement.streak.enabled ? 'Turn off streak' : 'Turn on streak'}
			</button>
			<button
				class="btn"
				type="button"
				data-testid="toggle-qotd"
				onclick={() => setEngagement({ qotd_enabled: !engagement.qotd.enabled })}
			>
				{engagement.qotd.enabled ? 'Turn off question of the day' : 'Turn on question of the day'}
			</button>
		</div>
	</div>
{/if}

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

	<h2>Mock tests</h2>
	{#if mocks === null || mocks.length === 0}
		<div class="card">
			<p class="muted">No mock tests are available yet.</p>
		</div>
	{:else}
		{#each mocks as mock (mock.mock_id)}
			<div class="card" data-testid="mock-card">
				<div style="display:flex; justify-content:space-between; gap:12px; align-items:center;">
					<strong>{mock.title}</strong>
					<span class="chip">Pass mark {mock.pass_mark_percent}%</span>
				</div>
				<p class="muted" style="margin: var(--space-sm) 0;">
					{mock.attempts_used} of {mock.attempts_allowed} attempts used
					{#if mock.time_limit_seconds}· {Math.round(mock.time_limit_seconds / 60)} min{/if}
					— results are observations about the form, not predictions.
				</p>
				{#if mock.attempts_used < mock.attempts_allowed}
					<button
						class="btn primary"
						type="button"
						disabled={startingMock !== ''}
						data-testid="mock-start"
						onclick={() => startMock(mock)}
					>
						{startingMock === mock.mock_id ? 'Preparing…' : 'Start mock'}
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
