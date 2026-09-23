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
	let dailyMinutes = $state(60);
	let activityPreference = $state('any');
	let studyTimeMultiplier = $state('1');
	let planBusy = $state(false);
	let protectionBusy = $state('');
	let planError = $state('');
	let planNotice = $state('');
	let nextAction = $state(null);
	let nextActionBusy = $state(false);
	let nextActionError = $state('');

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
		const taskId = task.id ?? task.task_id;
		if (!taskId) return;
		startingTask = taskId;
		error = '';
		try {
			const body =
				task.kind === 'revision'
					? {
						preset: 'revision',
						source_session_id: task.source_session_id,
						plan_task_key: task.task_key
					}
					: {
						preset: 'tutor',
						chapter_id: task.chapter_id,
						question_count: task.question_count,
						plan_task_key: task.task_key
					};
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
				question_count: task.question_count,
				plan_task_key: task.task_key,
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
		planError = '';
		try {
			await Api.undo(today.plan_id, revision.id);
			await load();
		} catch (err) {
			if (err instanceof ApiError && err.code === 'stale_plan_version') {
				const message = err.message;
				await load();
				planError = message;
			} else {
				planError = err instanceof ApiError ? err.message : 'Could not undo. Try again.';
			}
		} finally {
			undoing = '';
		}
	}

	async function protectTask(task) {
		if (!today || protectionBusy) return;
		protectionBusy = task.id;
		planError = '';
		planNotice = '';
		try {
			const result = await Api.protectPlanTask(today.plan_id, task.id, !task.protected);
			planNotice = result.protected ? 'Task protected from replanning.' : 'Task can be moved during replanning.';
			await load();
		} catch (err) {
			if (err instanceof ApiError && err.code === 'stale_plan_version') {
				const message = err.message;
				await load();
				planError = message;
			} else {
				planError = err instanceof ApiError ? err.message : 'Could not update task protection.';
			}
		} finally {
			protectionBusy = '';
		}
	}

	async function replan(event) {
		event.preventDefault();
		if (!today || planBusy) return;
		const budget = Number(dailyMinutes);
		if (!Number.isInteger(budget) || budget < 5 || budget > 480) {
			planError = 'Enter a daily budget from 5 to 480 minutes.';
			planNotice = '';
			return;
		}
		planBusy = true;
		planError = '';
		planNotice = '';
		try {
			const result = await Api.replan(budget, today.version);
			planNotice = result.replanned
				? `Plan updated. ${result.deferred_tasks ?? 0} task(s) deferred.`
				: 'Your pending tasks already fit this budget.';
			await load();
		} catch (err) {
			if (err instanceof ApiError && err.code === 'stale_plan_version') {
				const message = err.message;
				await load();
				planError = message;
			} else {
				planError = err instanceof ApiError ? err.message : 'Could not replan your day.';
			}
		} finally {
			planBusy = false;
		}
	}

	async function recommendNextAction() {
		const budget = Number(dailyMinutes);
		nextAction = null;
		nextActionError = '';
		if (!Number.isInteger(budget) || budget < 5 || budget > 480) {
			nextActionError = 'Enter a daily budget from 5 to 480 minutes.';
			return;
		}
		nextActionBusy = true;
		try {
			nextAction = await Api.recommendNextAction(
				budget,
				activityPreference,
				Number(studyTimeMultiplier)
			);
		} catch (err) {
			nextActionError = err instanceof ApiError ? err.message : 'Could not find a next task.';
		} finally {
			nextActionBusy = false;
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
	<section class="card" aria-labelledby="plan-controls-title" data-testid="plan-controls">
		<h2 id="plan-controls-title">Plan controls</h2>
		<p class="muted" data-testid="plan-version">Plan version {today.version}</p>
		<p class="muted" data-testid="automatic-revision-budget">
			Automatic changes {today.revision_budget.automatic_used}/{today.revision_budget.automatic_limit} today.
			{#if today.revision_budget.automatic_used >= today.revision_budget.automatic_limit}
				Further automatic changes pause until tomorrow.
			{:else}
				They stay within a daily limit.
			{/if}
		</p>
		<p class="muted">
			Plan changes {today.revision_budget.total_used}/{today.revision_budget.total_limit} today.
			Protect a pending task to keep it when you replan.
		</p>
		<form onsubmit={replan}>
			<label class="field" for="daily-minutes">
				<span>Daily available minutes</span>
				<input
					id="daily-minutes"
					name="daily_minutes"
					type="number"
					min="5"
					max="480"
					step="1"
					bind:value={dailyMinutes}
					aria-describedby="daily-minutes-help"
				/>
				<span id="daily-minutes-help">Use 5–480 minutes. Completed tasks do not use today's remaining budget.</span>
			</label>
			<button
				class="btn primary"
				type="submit"
				disabled={planBusy}
				data-loading={planBusy}
				data-testid="replan-submit"
			>
				{planBusy ? 'Replanning…' : 'Replan my day'}
			</button>
		</form>
		<label class="field" for="next-action-activity">
			<span>Preferred activity</span>
			<select id="next-action-activity" bind:value={activityPreference}>
				<option value="any">Any planned activity</option>
				<option value="practice">Practice</option>
				<option value="revision">Revision</option>
			</select>
		</label>
		<label class="field" for="next-action-time-multiplier">
			<span>Study-time adjustment</span>
			<select id="next-action-time-multiplier" bind:value={studyTimeMultiplier}>
				<option value="1">Use the plan estimate</option>
				<option value="1.25">1.25× the plan estimate</option>
				<option value="1.5">1.5× the plan estimate</option>
				<option value="2">2× the plan estimate</option>
				<option value="3">3× the plan estimate</option>
				<option value="4">4× the plan estimate</option>
			</select>
			<span>Use extra time for reading or interaction needs; a task is offered only when its adjusted estimate fits.</span>
		</label>
		<button
			class="btn"
			type="button"
			disabled={nextActionBusy}
			data-testid="recommend-next-action"
			onclick={recommendNextAction}
		>
			{nextActionBusy ? 'Finding a task…' : 'Find a next task within this time'}
		</button>
		{#if nextActionError}
			<p class="error-text" role="alert">{nextActionError}</p>
		{:else if nextAction?.recommended_action}
			<p class="muted" role="status" data-testid="next-action-result">
				Recommended next: {nextAction.recommended_action.title}. Est.
				{nextAction.recommended_action.adjusted_estimated_minutes} min
				{#if nextAction.time_multiplier > 1}(adjusted for study time){/if} ·
				{nextAction.recommended_action.question_count} questions.
				{#if nextAction.recommended_action.reason_code === 'protected_task'}
					This task is protected in your plan.
				{:else if nextAction.recommended_action.reason_code === 'missed_question_revision'}
					Follow-up from missed questions.
				{:else if nextAction.recommended_action.reason_code === 'lower_observed_accuracy'}
					Chosen from lower observed accuracy with {nextAction.recommended_action.independent_count} independent attempts.
				{:else}
					Next task in your current plan.
				{/if}
			</p>
			<button
				class="btn primary"
				type="button"
				disabled={startingTask !== ''}
				onclick={() => startTask(nextAction.recommended_action)}
			>
				Start recommended task
			</button>
		{:else if nextAction?.reason_code === 'free_allowance_reached'}
			<p class="muted" role="status" data-testid="next-action-empty">
				Your daily question allowance is used, so practice cannot be recommended right now.
			</p>
		{:else if nextAction?.reason_code === 'free_allowance_insufficient'}
			<p class="muted" role="status" data-testid="next-action-empty">
				Only {nextAction.allowance?.remaining ?? 0} of {nextAction.allowance?.limit ?? 0} free questions remain today; this planned task needs {nextAction.allowance?.required ?? 0}.
			</p>
		{:else if nextAction?.reason_code === 'exam_deadline_passed'}
			<p class="muted" role="status" data-testid="next-action-empty">
				Your saved exam date has passed. Update your goal to get an exam-aligned next action.
			</p>
		{:else if nextAction?.reason_code === 'no_current_plan'}
			<p class="muted" role="status" data-testid="next-action-empty">
				There is no plan for today yet. Refresh Today, then request a next action.
			</p>
		{:else if nextAction?.reason_code === 'activity_preference_unavailable'}
			<p class="muted" role="status" data-testid="next-action-empty">
				No pending {activityPreference} task fits {nextAction.available_minutes} minutes.
			</p>
		{:else if nextAction?.reason_code === 'content_unavailable'}
			<p class="muted" role="status" data-testid="next-action-empty">
				No fitting task currently has enough available questions to start.
			</p>
		{:else if nextAction}
			<p class="muted" role="status" data-testid="next-action-empty">
				No pending task fits {nextAction.available_minutes} minutes.
			</p>
		{/if}
		{#if planError}
			<p class="error-text" role="alert" data-testid="plan-error">{planError}</p>
		{/if}
		{#if planNotice}
			<p role="status" data-testid="plan-notice">{planNotice}</p>
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
				<p
					class="muted"
					style="margin: var(--space-sm) 0 0;"
					data-testid={`task-estimate-${task.id}`}
				>
					Est. {task.estimated_minutes} min · {task.question_count} questions
				</p>
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
						<button
							class="btn"
							type="button"
							aria-pressed={task.protected}
							disabled={protectionBusy !== ''}
							data-loading={protectionBusy === task.id}
							data-testid={`task-protection-${task.id}`}
							onclick={() => protectTask(task)}
						>
							{protectionBusy === task.id ? 'Saving…' : task.protected ? 'Unprotect task' : 'Protect task'}
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
				{#if revision.deferred_tasks.length > 0}
					<p class="muted" data-testid="deferred-tasks">
						Deferred: {revision.deferred_tasks.join(', ')}
					</p>
				{/if}
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
