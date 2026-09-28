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
	let curriculum = $state([]);
	let qotdCurriculumLoading = $state(true);
	let qotdCurriculumError = $state('');
	let qotdExamSelection = $state('');
	let changingQotdExam = $state(false);
	let qotdStartedAt = $state(0);
	let qotdTimerQuestionId = $state('');
	let answeringQotd = $state(false);
	let qotdError = $state('');
	let qotdResult = $state(null);
	let dailyMinutes = $state(60);
	let dailyGoalMode = $state('questions');
	let savingAvailableMinutes = $state(false);
	let availableMinutesError = $state('');
	let updatingEngagement = $state(false);
	let dailyGoalError = $state('');
	let activityPreference = $state('any');
	let studyTimeMultiplier = $state('1');
	let planBusy = $state(false);
	let protectionBusy = $state('');
	let planError = $state('');
	let planNotice = $state('');
	let nextAction = $state(null);
	let nextActionBusy = $state(false);
	let nextActionError = $state('');
	let qotdExams = $derived.by(() => {
		const exams = new Map();
		for (const chapter of curriculum) {
			const exam = exams.get(chapter.exam_id) ?? {
				exam_id: chapter.exam_id,
				name: chapter.exam
			};
			exams.set(chapter.exam_id, exam);
		}
		return [...exams.values()].sort((a, b) => a.name.localeCompare(b.name));
	});

	// §7.2 greeting + date context — from the device clock, nothing inferred.
	const now = new Date();
	const greeting = now.getHours() < 12 ? 'Good morning' : now.getHours() < 18 ? 'Good afternoon' : 'Good evening';
	const dateLabel = now.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' });
	const goalRatio = $derived.by(() => {
		const goal = engagement?.daily_goal;
		if (!goal?.target) return 0;
		const done = goal.mode === 'minutes' ? goal.minutes_today : goal.answered_today;
		return Math.min(1, done / goal.target);
	});
	// Presentation only: the server orders the plan; the first pending task in
	// that order carries the one primary button (§7.1 one dominant action).
	const nextTaskId = $derived(today?.tasks.find((task) => task.status !== 'done')?.id ?? null);

	async function loadMocks() {
		try {
			mocks = (await Api.listMocks()).mocks;
		} catch {
			mocks = [];
		}
	}

	async function loadEngagement() {
		try {
			const next = await Api.engagement();
			const questionId = next.qotd?.question_version_id ?? '';
			if (questionId && questionId !== qotdTimerQuestionId) {
				qotdTimerQuestionId = questionId;
				qotdStartedAt = Date.now();
			} else if (!questionId) {
				qotdTimerQuestionId = '';
				qotdStartedAt = 0;
			}
			engagement = next;
			qotdExamSelection = engagement.qotd.exam_id ?? '';
			dailyMinutes = engagement.available_minutes;
			dailyGoalMode = engagement.daily_goal.mode;
		} catch {
			engagement = null;
		}
	}

	async function loadQotdCurriculum() {
		try {
			curriculum = (await Api.myCurriculum()).chapters;
		} catch {
			qotdCurriculumError = 'Could not load exam choices.';
		} finally {
			qotdCurriculumLoading = false;
		}
	}

	async function setQotdExam(examId) {
		if (changingQotdExam) return;
		const previous = engagement?.qotd?.exam_id ?? '';
		qotdExamSelection = examId;
		changingQotdExam = true;
		qotdError = '';
		try {
			await Api.updateEngagementSettings({ qotd_exam_id: examId || null });
			await loadEngagement();
		} catch (err) {
			qotdExamSelection = previous;
			qotdError = err instanceof ApiError ? err.message : 'Could not update the QOTD exam.';
		} finally {
			changingQotdExam = false;
		}
	}

	async function setEngagement(patch) {
		if (updatingEngagement) return;
		updatingEngagement = true;
		if (patch.daily_goal_mode !== undefined) dailyGoalMode = patch.daily_goal_mode;
		qotdError = '';
		dailyGoalError = '';
		try {
			await Api.updateEngagementSettings(patch);
			engagement = await Api.engagement();
			qotdExamSelection = engagement.qotd.exam_id ?? '';
			dailyMinutes = engagement.available_minutes;
			dailyGoalMode = engagement.daily_goal.mode;
			if (patch.qotd_enabled === true && curriculum.length === 0) {
				qotdCurriculumLoading = true;
				qotdCurriculumError = '';
				await loadQotdCurriculum();
			}
		} catch (err) {
			const message = err instanceof ApiError ? err.message : 'Could not update settings.';
			if (patch.daily_goal_mode !== undefined) {
				dailyGoalError = message;
				dailyGoalMode = engagement?.daily_goal?.mode ?? 'questions';
			}
			else qotdError = message;
		} finally {
			updatingEngagement = false;
		}
	}

	async function saveAvailableMinutes() {
		const value = Number(dailyMinutes);
		if (!Number.isInteger(value) || value < 5 || value > 480) {
			availableMinutesError = 'Enter a daily budget from 5 to 480 minutes.';
			return;
		}
		if (savingAvailableMinutes) return;
		savingAvailableMinutes = true;
		availableMinutesError = '';
		try {
			await Api.updateEngagementSettings({ available_minutes: value });
			await loadEngagement();
		} catch (err) {
			availableMinutesError =
				err instanceof ApiError ? err.message : 'Could not save daily available minutes.';
		} finally {
			savingAvailableMinutes = false;
		}
	}

	async function answerQotd(index) {
		if (answeringQotd || !engagement?.qotd?.question_version_id) return;
		answeringQotd = true;
		qotdError = '';
		try {
			const elapsedMs = qotdStartedAt
				? Math.max(0, Math.min(Date.now() - qotdStartedAt, 3_600_000))
				: 0;
			qotdResult = await Api.answerQotd(
				engagement.qotd.question_version_id,
				index,
				elapsedMs
			);
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
		if (engagement?.qotd?.enabled) {
			await loadQotdCurriculum();
		} else {
			qotdCurriculumLoading = false;
		}
	});
</script>

<svelte:head>
	<title>Today | Medical Learning OS</title>
</svelte:head>

<header class="today-head">
	<p class="today-date">{greeting} · {dateLabel}</p>
	<h1>Today</h1>
</header>

{#if engagement?.enabled}
	<div class="card goal-card" data-testid="engagement-card">
		{#if engagement.daily_goal.enabled}
			<div class="goal">
				<div class="row">
					<strong data-testid="daily-goal">
						{#if engagement.daily_goal.mode === 'minutes'}
							Daily goal {engagement.daily_goal.minutes_today}/{engagement.daily_goal.target} minutes
						{:else}
							Daily goal {engagement.daily_goal.answered_today}/{engagement.daily_goal.target} questions
						{/if}
					</strong>
					{#if engagement.daily_goal.met}
						<span class="chip done" data-testid="goal-met">Met</span>
					{/if}
				</div>
				<div class="meter" class:done={engagement.daily_goal.met} style:--value={goalRatio}><span></span></div>
			</div>
		{/if}
		<div class="form-row">
			<label class="field" for="daily-goal-mode">
				<span>Daily goal measured in</span>
				<select
					id="daily-goal-mode"
					data-testid="daily-goal-mode"
					bind:value={dailyGoalMode}
					disabled={updatingEngagement}
					onchange={(event) => setEngagement({ daily_goal_mode: event.currentTarget.value })}
				>
					<option value="questions">Questions answered</option>
					<option value="minutes">Study minutes</option>
				</select>
			</label>
			{#if engagement.qotd.enabled}
				<label class="field" for="qotd-exam">
					<span>Question of the day exam</span>
					<select
						id="qotd-exam"
						data-testid="qotd-exam"
						bind:value={qotdExamSelection}
						disabled={changingQotdExam || answeringQotd || engagement.qotd.answered}
						onchange={(event) => setQotdExam(event.currentTarget.value)}
					>
						<option value="">Choose an exam</option>
						{#each qotdExams as exam (exam.exam_id)}
							<option value={exam.exam_id}>{exam.name}</option>
						{/each}
					</select>
				</label>
			{/if}
		</div>
		{#if engagement.daily_goal.mode === 'minutes'}
			<p class="muted small" data-testid="daily-goal-time-help">
				Counts time recorded while answering completed practice questions and today's question.
			</p>
		{/if}
		{#if dailyGoalError}
			<p class="error-text" role="alert" data-testid="daily-goal-error">{dailyGoalError}</p>
		{/if}
		{#if engagement.streak.enabled}
			<p class="streak" data-testid="streak">
				{engagement.streak.count}-day streak · {engagement.streak.freezes} freezes held
			</p>
		{/if}
		{#if engagement.qotd.enabled}
			<div class="qotd">
				{#if qotdCurriculumLoading}
					<p class="muted is-loading" data-testid="qotd-exams-loading">Loading exam choices…</p>
				{:else if qotdCurriculumError}
					<p class="error-text" role="alert">{qotdCurriculumError}</p>
				{:else if qotdExams.length === 0}
					<p class="muted" data-testid="qotd-no-exams">No exams are available in your curriculum.</p>
				{/if}
				{#if engagement.qotd.answered}
					<div data-testid="qotd-answered">
						<p class="muted tight" data-testid="qotd-exam-lock">
							Exam selection unlocks when today's question resets.
						</p>
						{#if qotdResult}
							<p class="qotd-verdict" class:good={qotdResult.correct} data-testid="qotd-verdict">
								{qotdResult.correct ? 'Correct' : 'Not correct'} — correct answer was
								option {qotdResult.correct_index + 1}.
							</p>
						{:else}
							<p class="muted tight">Question of the day — answered.</p>
						{/if}
						{#if engagement.qotd.community_split?.length}
							<p class="muted cluster flush" data-testid="qotd-community">
								{#each engagement.qotd.community_split as part (part.chosen_index)}
									<span class="chip">Option {part.chosen_index + 1}: {part.count}</span>
								{/each}
								<span class="muted">of {engagement.qotd.community_total} answered</span>
							</p>
						{/if}
					</div>
				{:else if engagement.qotd.needs_exam_selection}
					<p class="muted" data-testid="qotd-needs-exam">Choose an exam to see today's question.</p>
				{:else if engagement.qotd.available}
					<p class="qotd-label">Question of the day</p>
					<p class="qotd-vignette" data-testid="qotd-vignette">{engagement.qotd.vignette}</p>
					<div class="options">
						{#each engagement.qotd.options ?? [] as option, i (i)}
							<button
								class="option"
								type="button"
								disabled={answeringQotd}
								data-testid={`qotd-option-${i}`}
								onclick={() => answerQotd(i)}
							>
								{answeringQotd ? 'Saving…' : option.text}
							</button>
						{/each}
					</div>
				{:else}
					<p class="muted" data-testid="qotd-unavailable">
						No published questions are available for this exam today.
					</p>
				{/if}
			</div>
		{/if}
		{#if qotdError}
			<p class="error-text" role="alert" data-testid="qotd-error">{qotdError}</p>
		{/if}
		<div class="cluster goal-toggles">
			<button
				class="btn small ghost"
				type="button"
				data-testid="toggle-goal"
				onclick={() => setEngagement({ daily_goal_enabled: !engagement.daily_goal.enabled })}
			>
				{engagement.daily_goal.enabled ? 'Turn off daily goal' : 'Turn on daily goal'}
			</button>
			<button
				class="btn small ghost"
				type="button"
				data-testid="toggle-streak"
				onclick={() => setEngagement({ streak_enabled: !engagement.streak.enabled })}
			>
				{engagement.streak.enabled ? 'Turn off streak' : 'Turn on streak'}
			</button>
			<button
				class="btn small ghost"
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
	<div class="card" aria-busy="true">
		<p class="muted is-loading">Loading your plan…</p>
	</div>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if today}
	<section class="plan" aria-labelledby="plan-title">
		<h2 id="plan-title">Your plan</h2>
		{#if today.tasks.length === 0}
			<div class="card">
				<p class="muted">Nothing scheduled for today yet. Your plan appears as you study.</p>
			</div>
		{:else}
			<ol class="bare-list enter-stagger">
				{#each today.tasks as task (task.id)}
					<li class="card task" class:done={task.status === 'done'} class:next={task.id === nextTaskId}>
						<div class="row">
							<h3 class="task-title">{task.title}</h3>
							<span class="chip {task.status === 'done' ? 'done' : task.id === nextTaskId ? 'info' : ''}">
								{task.status === 'done' ? 'Done' : task.kind === 'revision' ? 'Re-practice' : 'Practice'}
							</span>
						</div>
						<p class="muted small task-meta" data-testid={`task-estimate-${task.id}`}>
							Est. {task.estimated_minutes} min · {task.question_count} questions
						</p>
						{#if task.status !== 'done'}
							<div class="cluster">
								<button
									class="btn {task.id === nextTaskId ? 'primary' : ''}"
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
									class="btn ghost"
									type="button"
									aria-pressed={task.protected}
									disabled={protectionBusy !== ''}
									data-loading={protectionBusy === task.id}
									data-testid={`task-protection-${task.id}`}
									onclick={() => protectTask(task)}
								>
									{protectionBusy === task.id ? 'Saving…' : task.protected ? 'Unprotect task' : 'Protect task'}
								</button>
							</div>
						{/if}
					</li>
				{/each}
			</ol>
		{/if}
	</section>

	{#if today.revisions.length > 0}
		<section aria-labelledby="changes-title">
			<h2 id="changes-title">Plan changes</h2>
			{#each today.revisions as revision (revision.id)}
				<div class="card revision" class:undone={revision.undone} data-testid="revision-card">
					<span class="chip {revision.undone ? 'undone' : 'info'}">
						{revision.undone ? 'Undone' : revision.automatic ? 'Applied — automatic' : 'Applied'}
					</span>
					<p>{revision.explanation}</p>
					{#if revision.deferred_tasks.length > 0}
						<p class="muted small" data-testid="deferred-tasks">
							Deferred: {revision.deferred_tasks.join(', ')}
						</p>
					{/if}
					{#if !revision.undone}
						<button
							class="btn danger-text small"
							type="button"
							disabled={undoing !== ''}
							data-loading={undoing === revision.id}
							data-testid="undo"
							onclick={() => undo(revision)}
						>
							{undoing === revision.id ? 'Undoing…' : 'Undo'}
						</button>
					{/if}
				</div>
			{/each}
		</section>
	{/if}

	<section class="card" aria-labelledby="plan-controls-title" data-testid="plan-controls">
		<h2 id="plan-controls-title">Plan controls</h2>
		<div class="plan-budget">
			<p class="muted small" data-testid="plan-version">Plan version {today.version}</p>
			<p class="muted small" data-testid="automatic-revision-budget">
				Automatic changes {today.revision_budget.automatic_used}/{today.revision_budget.automatic_limit} today.
				{#if today.revision_budget.automatic_used >= today.revision_budget.automatic_limit}
					Further automatic changes pause until tomorrow.
				{:else}
					They stay within a daily limit.
				{/if}
			</p>
			<p class="muted small">
				Plan changes {today.revision_budget.total_used}/{today.revision_budget.total_limit} today.
				Protect a pending task to keep it when you replan.
			</p>
		</div>
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
					onchange={saveAvailableMinutes}
					disabled={savingAvailableMinutes}
				/>
				<span id="daily-minutes-help">Use 5–480 minutes. This is also your goal target in minutes mode.</span>
				{#if availableMinutesError}
					<span class="error-text" role="alert" data-testid="available-minutes-error">{availableMinutesError}</span>
				{/if}
			</label>
			<button
				class="btn"
				type="submit"
				disabled={planBusy}
				data-loading={planBusy}
				data-testid="replan-submit"
			>
				{planBusy ? 'Replanning…' : 'Replan my day'}
			</button>
		</form>
		<hr />
		<div class="form-row">
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
			</label>
		</div>
		<p class="muted small">Use extra time for reading or interaction needs; a task is offered only when its adjusted estimate fits.</p>
		<button
			class="btn"
			type="button"
			disabled={nextActionBusy}
			data-loading={nextActionBusy}
			data-testid="recommend-next-action"
			onclick={recommendNextAction}
		>
			{nextActionBusy ? 'Finding a task…' : 'Find a next task within this time'}
		</button>
		{#if nextActionError}
			<p class="error-text" role="alert">{nextActionError}</p>
		{:else if nextAction?.recommended_action}
			<div class="feedback next-action">
				<p role="status" data-testid="next-action-result">
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
			</div>
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
			<p class="success-text" role="status" data-testid="plan-notice">{planNotice}</p>
		{/if}
	</section>

	<section aria-labelledby="mocks-title">
		<h2 id="mocks-title">Mock tests</h2>
		{#if mocks === null || mocks.length === 0}
			<div class="card">
				<p class="muted">No mock tests are available yet.</p>
			</div>
		{:else}
			{#each mocks as mock (mock.mock_id)}
				<div class="card" data-testid="mock-card">
					<div class="row">
						<strong>{mock.title}</strong>
						<span class="chip">Pass mark {mock.pass_mark_percent}%</span>
					</div>
					<p class="muted small">
						{mock.attempts_used} of {mock.attempts_allowed} attempts used
						{#if mock.time_limit_seconds}· {Math.round(mock.time_limit_seconds / 60)} min{/if}
						— results are observations about the form, not predictions.
					</p>
					{#if mock.attempts_used < mock.attempts_allowed}
						<button
							class="btn"
							type="button"
							disabled={startingMock !== ''}
							data-loading={startingMock === mock.mock_id}
							data-testid="mock-start"
							onclick={() => startMock(mock)}
						>
							{startingMock === mock.mock_id ? 'Preparing…' : 'Start mock'}
						</button>
					{/if}
				</div>
			{/each}
		{/if}
	</section>

	<section aria-labelledby="evidence-title">
		<h2 id="evidence-title">Progress</h2>
		{#if today.learner.length === 0}
			<div class="card">
				<p class="muted" data-testid="no-evidence">
					No evidence yet — answer questions and your chapters appear here.
				</p>
			</div>
		{:else}
			<div class="card evidence">
				{#each today.learner as chapter (chapter.chapter_id)}
					<div class="evidence-row">
						<strong>{chapter.chapter_name}</strong>
						<p class="muted small flush">
							{#if chapter.mastery_index === null}
								Not enough evidence yet ({chapter.independent_count} answered) — no
								mastery shown until it means something.
							{:else}
								Accuracy {chapter.mastery_index}% · evidence: {chapter.evidence_level.replace('_', ' ')}
							{/if}
						</p>
						{#if chapter.mastery_index !== null}
							<div class="meter" style:--value={chapter.mastery_index / 100}><span></span></div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</section>
{/if}

<style>
	.today-head h1 {
		margin-bottom: var(--space-xl);
	}

	.today-date {
		margin: 0 0 var(--space-xs);
		color: var(--color-text-secondary);
		font-weight: 600;
	}

	.goal {
		display: grid;
		gap: var(--space-sm);
		margin-bottom: var(--space-lg);
	}

	.streak {
		display: inline-flex;
		align-items: center;
		gap: var(--space-sm);
		margin: 0 0 var(--space-md);
		color: var(--color-text-secondary);
		font-weight: 600;
	}

	.streak::before {
		content: '';
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-warning);
		box-shadow: 0 0 10px var(--color-warning);
	}

	.qotd {
		margin: var(--space-md) 0;
		padding-top: var(--space-md);
		border-top: 1px solid var(--color-border);
	}

	.qotd-label {
		margin: 0 0 var(--space-xs);
		color: var(--color-accent);
		font-size: var(--text-sm);
		font-weight: 700;
	}

	.qotd-vignette {
		margin: 0;
		font-size: var(--text-body-lg);
	}

	.qotd-verdict {
		margin: var(--space-sm) 0;
		font-weight: 600;
		color: var(--color-error);
		animation: mlos-rise var(--dur-slow) var(--ease-out) backwards;
	}

	.qotd-verdict.good {
		color: var(--color-success);
	}

	.goal-toggles {
		margin-top: var(--space-md);
		padding-top: var(--space-md);
		border-top: 1px solid var(--color-border);
	}

	.plan > h2 {
		margin-top: 0;
	}

	.task-title {
		margin: 0;
		font-size: var(--text-body-lg);
	}

	.task-meta {
		margin: var(--space-xs) 0 var(--space-md);
	}

	.task.next {
		border-color: color-mix(in oklab, var(--color-accent) 45%, transparent);
		background:
			radial-gradient(120% 90% at 0% 0%, var(--color-action-wash), transparent 60%),
			var(--color-surface);
	}

	.task.done {
		opacity: 0.72;
	}

	.task.done .task-title {
		text-decoration: line-through;
		text-decoration-color: color-mix(in oklab, currentColor 40%, transparent);
	}

	.revision.undone {
		opacity: 0.7;
	}

	.revision > p {
		margin: var(--space-md) 0;
	}

	.plan-budget {
		margin-bottom: var(--space-lg);
	}

	.plan-budget p {
		margin: 0 0 var(--space-xs);
	}

	.next-action {
		margin-top: var(--space-md);
		display: grid;
		gap: var(--space-md);
		justify-items: start;
	}

	.evidence {
		display: grid;
		gap: var(--space-lg);
	}

	.evidence-row {
		display: grid;
		gap: var(--space-xs);
	}
</style>
