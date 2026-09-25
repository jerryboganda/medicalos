<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 */
	import { onMount, tick } from 'svelte';
	import { base } from '$app/paths';
	import {
		Api,
		ApiError,
		type CompetitionAttemptStep,
		type CompetitionLeaderboard,
		type CompetitionLeagueState,
		type CompetitionSummary
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let chapters = $state<
		{
			chapter_id: string;
			chapter_name: string;
			system: string;
			subject: string;
			exam_id: string;
			exam: string;
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
	let competitions = $state<CompetitionSummary[]>([]);
	let phaseClock = $state(Date.now());
	let communityHandle = $state<string | null>(null);
	let competitionError = $state('');
	let competitionBusy = $state(false);
	let startingCompetitionId = $state('');
	let activeCompetition = $state<{
		competition: CompetitionSummary;
		step: CompetitionAttemptStep;
	} | null>(null);
	let selectedCompetitionChoice = $state<number | null>(null);
	let pendingCompetitionAnswerKey = $state('');
	let competitionLeaderboard = $state<CompetitionLeaderboard | null>(null);
	let leaderboardCompetition = $state<CompetitionSummary | null>(null);
	let competitionQuestionHeading = $state<HTMLHeadingElement>();
	let competitionErrorSummary = $state<HTMLDivElement>();
	let leaderboardHeading = $state<HTMLHeadingElement>();
	let leagueExams = $state<{ exam_id: string; exam: string }[]>([]);
	let selectedLeagueExam = $state('');
	let leagueState = $state<CompetitionLeagueState | null>(null);
	let leagueLoading = $state(false);
	let leagueBusy = $state(false);
	let leagueError = $state('');
	let leagueRequestId = 0;

	$effect(() => {
		const timer = window.setInterval(() => (phaseClock = Date.now()), 30_000);
		return () => window.clearInterval(timer);
	});

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

	function competitionPhase(competition: CompetitionSummary) {
		const now = phaseClock;
		if (competition.attempt_status === 'submitted' || competition.entered) return 'Submitted';
		if (competition.attempt_status === 'in_progress') {
			return competition.status === 'closed' || now >= Date.parse(competition.ends_at)
				? 'Closed'
				: 'In progress';
		}
		if (now < Date.parse(competition.starts_at)) return 'Upcoming';
		if (competition.status === 'closed' || now >= Date.parse(competition.ends_at)) {
			return 'Closed';
		}
		return 'Open';
	}

	function competitionDate(value: string) {
		return `${new Date(value).toLocaleString('en-US', {
			dateStyle: 'medium',
			timeStyle: 'short',
			timeZone: 'UTC'
		})} UTC`;
	}

	function cadenceLabel(cadence: CompetitionSummary['cadence']) {
		return ({ one_off: 'One-off', daily: 'Daily', weekly: 'Weekly', monthly: 'Monthly', live: 'Live' })[
			cadence
		] ?? 'One-off';
	}

	function leagueWeek(value: string) {
		return new Date(`${value}T00:00:00Z`).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			timeZone: 'UTC'
		});
	}

	async function loadCompetitionLeague() {
		if (!selectedLeagueExam) return;
		const requestId = ++leagueRequestId;
		leagueLoading = true;
		leagueError = '';
		leagueState = null;
		try {
			const result = await Api.competitionLeague(selectedLeagueExam);
			if (requestId === leagueRequestId) leagueState = result;
		} catch (err) {
			if (requestId === leagueRequestId) {
				leagueState = null;
				leagueError = err instanceof Error ? err.message : 'Could not load league standings.';
			}
		} finally {
			if (requestId === leagueRequestId) leagueLoading = false;
		}
	}

	async function joinCompetitionLeague() {
		if (leagueBusy || !selectedLeagueExam || !communityHandle) return;
		leagueBusy = true;
		leagueError = '';
		try {
			leagueState = await Api.joinCompetitionLeague(selectedLeagueExam);
		} catch (err) {
			leagueError = err instanceof Error ? err.message : 'Could not join this league.';
		} finally {
			leagueBusy = false;
		}
	}

	async function leaveCompetitionLeague() {
		if (leagueBusy || !selectedLeagueExam) return;
		leagueBusy = true;
		leagueError = '';
		try {
			await Api.leaveCompetitionLeague(selectedLeagueExam);
			leagueState = { joined: false };
		} catch (err) {
			leagueError = err instanceof Error ? err.message : 'Could not leave this league.';
		} finally {
			leagueBusy = false;
		}
	}

	async function focusCompetitionQuestion() {
		await tick();
		competitionQuestionHeading?.focus();
	}

	async function showCompetitionError(message: string) {
		competitionError = message;
		await tick();
		competitionErrorSummary?.focus();
	}

	async function startCompetition(competition: CompetitionSummary) {
		if (competitionBusy || !communityHandle) return;
		competitionBusy = true;
		startingCompetitionId = competition.competition_id;
		competitionError = '';
		competitionLeaderboard = null;
		leaderboardCompetition = null;
		try {
			const step = await Api.startCompetitionEntry(competition.competition_id, communityHandle);
			activeCompetition = { competition, step };
			selectedCompetitionChoice = null;
			pendingCompetitionAnswerKey = '';
			competitions = competitions.map((item) =>
				item.competition_id === competition.competition_id
					? { ...item, attempt_status: 'in_progress' }
					: item
			);
			await focusCompetitionQuestion();
		} catch (err) {
			await showCompetitionError(
				err instanceof Error ? err.message : 'Could not open this competition.'
			);
		} finally {
			competitionBusy = false;
			startingCompetitionId = '';
		}
	}

	async function answerCompetitionQuestion() {
		if (
			competitionBusy ||
			!activeCompetition ||
			activeCompetition.step.submitted ||
			selectedCompetitionChoice === null
		)
			return;
		competitionBusy = true;
		competitionError = '';
		const { competition, step } = activeCompetition;
		if (!pendingCompetitionAnswerKey) pendingCompetitionAnswerKey = crypto.randomUUID();
		try {
			const next = await Api.answerCompetitionQuestion(
				competition.competition_id,
				step.question.question_version_id,
				selectedCompetitionChoice,
				pendingCompetitionAnswerKey
			);
			activeCompetition = { competition, step: next };
			selectedCompetitionChoice = null;
			pendingCompetitionAnswerKey = '';
			competitions = competitions.map((item) =>
				item.competition_id === competition.competition_id
					? { ...item, entered: next.submitted, attempt_status: next.submitted ? 'submitted' : 'in_progress' }
					: item
			);
			if (next.submitted) {
				try {
					competitionLeaderboard = await Api.competitionLeaderboard(competition.competition_id);
					leaderboardCompetition = competition;
				} catch (err) {
					competitionError =
						err instanceof Error ? err.message : 'The score was saved, but standings did not load.';
				}
				await tick();
				competitionQuestionHeading?.focus();
			} else {
				await focusCompetitionQuestion();
			}
		} catch (err) {
			await showCompetitionError(
				err instanceof Error ? err.message : 'Could not record this answer. Retry to continue.'
			);
		} finally {
			competitionBusy = false;
		}
	}

	async function showLeaderboard(competition: CompetitionSummary) {
		competitionError = '';
		try {
			competitionLeaderboard = await Api.competitionLeaderboard(competition.competition_id);
			leaderboardCompetition = competition;
			await tick();
			leaderboardHeading?.focus();
		} catch (err) {
			await showCompetitionError(
				err instanceof Error ? err.message : 'Could not load the competition standings.'
			);
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
		}
		try {
			competitions = (await Api.listCompetitions()).competitions;
		} catch (err) {
			competitionError = err instanceof Error ? err.message : 'Could not load competitions.';
		}
		try {
			const profile = await Api.communityProfile();
			communityHandle = profile.opted_in ? profile.handle ?? null : null;
		} catch {
			communityHandle = null;
		}
		const examById = new Map<string, string>();
		for (const item of [...chapters, ...competitions]) {
			if (item.exam_id && item.exam) examById.set(item.exam_id, item.exam);
		}
		leagueExams = [...examById].map(([exam_id, exam]) => ({ exam_id, exam }));
		selectedLeagueExam = leagueExams[0]?.exam_id ?? '';
		if (selectedLeagueExam) await loadCompetitionLeague();
		loading = false;
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

	<div class="card" data-testid="competition-section">
		<h2>Competitions</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Opt-in events use one timed attempt. Questions appear one at a time.
		</p>
		{#if competitionError}
			<div
				class="danger-text"
				role="alert"
				tabindex="-1"
				bind:this={competitionErrorSummary}
				data-testid="competition-error"
			>
				{competitionError}
			</div>
		{/if}
		{#if competitions.length === 0}
			<p class="muted" data-testid="competition-empty">No competitions are available yet.</p>
		{:else}
			{#each competitions as competition (competition.competition_id)}
				{@const phase = competitionPhase(competition)}
				<div class="card" style="margin-bottom: var(--space-md);" data-testid="competition-card">
					<div style="display:flex; justify-content:space-between; gap:12px; align-items:center; flex-wrap:wrap;">
						<strong style="min-width:0; overflow-wrap:anywhere;">{competition.title}</strong>
						<div style="display:flex; gap:var(--space-xs); align-items:center;">
							<span class="chip" data-testid="competition-cadence">{cadenceLabel(competition.cadence)}</span>
							<span class="chip" data-testid="competition-status">{phase}</span>
						</div>
					</div>
					<p class="muted" style="margin: var(--space-xs) 0; font-size: var(--text-sm);">
						Starts {competitionDate(competition.starts_at)} · Ends {competitionDate(competition.ends_at)}
					</p>
					{#if !communityHandle && (phase === 'Open' || phase === 'In progress')}
						<p class="muted">Joining is optional. Set up a community handle to enter.</p>
						<a class="btn" href={`${base}/community`}>Set up in Community</a>
					{:else if phase === 'Open' || phase === 'In progress'}
						<button
							class="btn primary"
							type="button"
							disabled={
								competitionBusy ||
								(!!activeCompetition &&
									!activeCompetition.step.submitted &&
									activeCompetition.competition.competition_id !== competition.competition_id)
							}
							onclick={() => startCompetition(competition)}
							data-testid="competition-enter"
						>
							{startingCompetitionId === competition.competition_id
								? 'Opening…'
								: phase === 'In progress' ? 'Resume attempt' : 'Enter competition'}
						</button>
					{:else if phase === 'Submitted' || phase === 'Closed'}
						<button
							class="btn"
							type="button"
							disabled={competitionBusy}
							onclick={() => showLeaderboard(competition)}
							data-testid="competition-standings"
						>
							View standings
						</button>
					{/if}
				</div>
			{/each}
		{/if}

		{#if activeCompetition}
			{@const step = activeCompetition.step}
			<div class="card" style="margin-top: var(--space-md);" data-testid="competition-attempt">
				<h3 tabindex="-1" bind:this={competitionQuestionHeading}>
					{step.submitted ? 'Competition result' : `Question ${step.question.question_number} of ${step.question.total_questions}`}
				</h3>
				{#if step.submitted}
					<p data-testid="competition-result">
						Your score: <strong>{step.score}</strong> · {step.questions} questions · {Math.round(step.total_time_ms / 1000)} seconds
					</p>
				{:else}
					<p style="overflow-wrap:anywhere;">{step.question.vignette}</p>
					<p style="overflow-wrap:anywhere;"><strong>{step.question.lead_in}</strong></p>
					<fieldset style="border:0; padding:0; margin:0 0 var(--space-md);">
						<legend class="muted" style="font-size: var(--text-sm);">Choose one answer</legend>
						{#each step.question.options as option, index}
							<label style="display:flex; gap:8px; align-items:flex-start; margin: var(--space-sm) 0;">
								<input
									type="radio"
									name="competition-answer"
									value={index}
									bind:group={selectedCompetitionChoice}
								/>
								<span>{option.text}</span>
							</label>
						{/each}
					</fieldset>
					<button
						class="btn primary"
						type="button"
						disabled={competitionBusy || selectedCompetitionChoice === null}
						onclick={answerCompetitionQuestion}
						data-testid="competition-answer"
					>
						{competitionBusy ? 'Recording…' : 'Submit answer'}
					</button>
					<p class="muted" aria-live="polite">Your response time is measured by the server.</p>
				{/if}
			</div>
		{/if}

		{#if competitionLeaderboard && leaderboardCompetition}
			<div class="card" style="margin-top: var(--space-md);" data-testid="competition-leaderboard">
				<h3 tabindex="-1" bind:this={leaderboardHeading}>{leaderboardCompetition.title} standings</h3>
				{#if competitionLeaderboard.entries.length === 0}
					<p class="muted">No eligible leaderboard entries are available.</p>
				{:else}
					<ol>
						{#each competitionLeaderboard.entries as entry (`${entry.rank}-${entry.handle}`)}
							<li style="margin: var(--space-sm) 0;">
								<strong style="overflow-wrap:anywhere;">#{entry.rank} {entry.handle}{entry.is_me ? ' · you' : ''}</strong>
								<div class="muted" style="font-size: var(--text-sm);">
									{entry.score} points · {Math.round(entry.accuracy * 100)}% accuracy ·
									{entry.questions_attempted} questions · {Math.round(entry.average_response_time_ms / 100) / 10}s average
								</div>
							</li>
						{/each}
					</ol>
				{/if}
			</div>
		{/if}
	</div>

	<div class="card" data-testid="competition-league">
		<h2>Weekly leagues</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Opt in to a private cohort for your exam. Weekly standings use completed competition scores.
		</p>
		{#if leagueExams.length === 0}
			<p class="muted">League cohorts will appear when an exam is available.</p>
		{:else}
			<label class="field">
				<span>Exam</span>
				<select
					bind:value={selectedLeagueExam}
					onchange={loadCompetitionLeague}
					disabled={leagueBusy}
					data-testid="league-exam"
				>
					{#each leagueExams as exam (exam.exam_id)}
						<option value={exam.exam_id}>{exam.exam}</option>
					{/each}
				</select>
			</label>
			{#if leagueError}
				<p class="danger-text" role="alert" data-testid="league-error">{leagueError}</p>
			{/if}
			{#if !communityHandle}
				<p class="muted">League participation is optional. Set up a community handle to join.</p>
				<a class="btn" href={`${base}/community`}>Create a community profile</a>
			{:else if leagueLoading}
				<p class="muted" aria-live="polite">Loading league…</p>
			{:else if leagueState && leagueState.joined}
				<div data-testid="league-standings">
					<p>
						<strong>Division {leagueState.division} · Cohort {leagueState.cohort_number}</strong>
						<span class="muted"> · Week {leagueWeek(leagueState.week_start)}–{leagueWeek(leagueState.week_end)}</span>
					</p>
					{#if leagueState.standings.length === 0}
						<p class="muted">Complete a competition this week to appear in standings.</p>
					{:else}
						<ol>
							{#each leagueState.standings as entry (`${entry.rank}-${entry.handle}`)}
								<li style="margin: var(--space-sm) 0;">
									<strong>#{entry.rank} {entry.handle}{entry.is_me ? ' · you' : ''}</strong>
									<div class="muted" style="font-size: var(--text-sm);">
										{entry.points} points · {Math.round(entry.accuracy * 100)}% accuracy · {Math.round(entry.total_time_ms / 1000)}s
									</div>
								</li>
							{/each}
						</ol>
					{/if}
					<button class="btn" type="button" disabled={leagueBusy} onclick={leaveCompetitionLeague} data-testid="league-leave">
						{leagueBusy ? 'Saving…' : 'Leave league'}
					</button>
				</div>
			{:else}
				<p class="muted">Join a cohort of up to 30 learners. Cohorts of 10 or more can earn promotion or relegation each week.</p>
				<button class="btn primary" type="button" disabled={leagueBusy} onclick={joinCompetitionLeague} data-testid="league-join">
					{leagueBusy ? 'Joining…' : 'Join weekly league'}
				</button>
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
