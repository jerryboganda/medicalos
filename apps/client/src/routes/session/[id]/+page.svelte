<script>
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError } from '$lib/api';

	let { data } = $props();

	const sid = data.id;
	let session = $state(null);
	let current = $state(0);
	let selected = $state(null);
	let busy = $state(false);
	let submitting = $state(false);
	let error = $state('');
	let result = $state(null);
	let loadFailed = $state('');
	let reviewing = $state(false);
	let actionBusy = $state(false);
	let actionError = $state('');
	let markBusy = $state(false);
	let markError = $state('');

	// QB-08: report-a-problem control on answered items.
	let reportOpen = $state(false);
	let reportCategory = $state('wrong_answer');
	let reportNote = $state('');
	let reportBusy = $state(false);
	let reportDone = $state('');
	let reportError = $state('');

	const REPORT_CATEGORIES = [
		['wrong_answer', 'Wrong answer'],
		['bad_explanation', 'Bad explanation'],
		['typo', 'Typo'],
		['duplicate', 'Duplicate'],
		['outdated', 'Outdated'],
		['broken_image', 'Broken image'],
		['other', 'Other']
	];

	const item = $derived(session?.items?.[current] ?? null);
	const allAnswered = $derived(
		session?.items ? session.items.every((i) => i.answered) : false
	);
	const missedCount = $derived(result ? result.incorrect + result.skipped : 0);

	// EX-08: countdown derives from the server-issued deadline and server_now,
	// so changing the device clock never extends the timer.
	let clockSkewMs = $state(0);
	let remainingMs = $state(null);
	let autoSubmitted = $state(false);

	function tick() {
		if (!session?.deadline) return;
		remainingMs = new Date(session.deadline).getTime() - (Date.now() + clockSkewMs);
		if (remainingMs <= 0 && !autoSubmitted && !result && session.status === 'open') {
			autoSubmitted = true;
			submitSession();
		}
	}

	function fmt(ms) {
		const total = Math.max(0, Math.floor(ms / 1000));
		const m = Math.floor(total / 60);
		const s = total % 60;
		return `${m}:${String(s).padStart(2, '0')}`;
	}

	function fmtDuration(seconds) {
		const total = Math.max(0, Math.floor(seconds ?? 0));
		const h = Math.floor(total / 3600);
		const m = Math.floor((total % 3600) / 60);
		const s = total % 60;
		if (h > 0) return `${h}h ${m}m ${s}s`;
		if (m > 0) return `${m}m ${s}s`;
		return `${s}s`;
	}

	function storageKey(index) {
		return `mlos_key_${sid}_${index}`;
	}

	function idempotencyKey(index) {
		let v = localStorage.getItem(storageKey(index));
		if (!v) {
			v = crypto.randomUUID();
			localStorage.setItem(storageKey(index), v);
		}
		return v;
	}

	async function load() {
		try {
			session = await Api.getSession(sid);
			if (session.status === 'submitted' && session.result) {
				result = session.result;
				current = 0;
				return;
			}
			const firstUnanswered = session.items.findIndex((i) => !i.answered);
			current = firstUnanswered === -1 ? session.items.length - 1 : firstUnanswered;
			if (session.deadline && session.server_now) {
				clockSkewMs = new Date(session.server_now).getTime() - Date.now();
				tick();
			}
		} catch (err) {
			loadFailed =
				err instanceof ApiError ? err.message : 'Could not load the session.';
		}
	}

	async function answer(chosen) {
		if (!item || item.answered || busy) return;
		busy = true;
		error = '';
		try {
			const res = await Api.answer(sid, {
				item_index: current,
				chosen_index: chosen,
				idempotency_key: idempotencyKey(current)
			});
			session.items[current] = {
				...session.items[current],
				answered: true,
				chosen_index: chosen,
				correct: res.correct,
				correct_index: res.correct_index,
				options: res.options,
				key_learning_point: res.key_learning_point,
				exam_tip: res.exam_tip
			};
			selected = null;
		} catch (err) {
			if (err instanceof ApiError && err.code === 'session_expired') {
				// Server deadline passed: submit what exists (auto-submit, §11.6).
				autoSubmitted = true;
				await submitSession();
				return;
			}
			error = err instanceof ApiError ? err.message : 'Could not record the answer.';
		} finally {
			busy = false;
		}
	}

	async function submitSession() {
		if (submitting) return;
		submitting = true;
		error = '';
		try {
			result = await Api.submit(sid);
			try {
				session = await Api.getSession(sid);
				if (session.result) result = session.result;
			} catch {
				if (session) session.status = 'submitted';
			}
			current = 0;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not submit the session.';
		} finally {
			submitting = false;
		}
	}

	function startReview() {
		current = 0;
		reviewing = true;
		actionError = '';
	}

	async function startResultSession(kind) {
		if (!session || actionBusy) return;
		actionBusy = true;
		actionError = '';
		try {
			let request;
			if (kind === 'missed') {
				request = { preset: 'revision', source_session_id: sid };
			} else {
				if (!session.chapter_id) throw new Error('This session cannot be retried by chapter.');
				request = {
					preset: session.preset === 'timed' ? 'timed' : 'tutor',
					chapter_id: session.chapter_id,
					question_count: session.items.length
				};
				if (request.preset === 'timed' && session.time_limit_seconds) {
					request.time_limit_seconds = session.time_limit_seconds;
				}
			}
			const { session_id } = await Api.createSession(request);
			await goto(`${base}/session/${session_id}`);
		} catch (err) {
			actionError =
				err instanceof ApiError || err instanceof Error
					? err.message
					: 'Could not start the next session.';
		} finally {
			actionBusy = false;
		}
	}

	async function submitReport() {
		if (reportBusy || !item) return;
		reportBusy = true;
		reportError = '';
		try {
			const res = await Api.reportQuestion(item.question_version_id, {
				category: reportCategory,
				note: reportNote.trim() ? reportNote.trim() : undefined
			});
			session.items[current] = {
				...session.items[current],
				report_status: res.quarantined ? 'quarantined' : 'open'
			};
			reportDone = res.quarantined
				? 'Thanks — enough learners flagged this, so it is out of rotation pending review.'
				: 'Thanks — your report is recorded for editorial review.';
			reportOpen = false;
			reportNote = '';
		} catch (err) {
			reportError =
				err instanceof ApiError ? err.message : 'Could not send the report.';
		} finally {
			reportBusy = false;
		}
	}

	async function toggleMark() {
		if (!item || markBusy) return;
		markBusy = true;
		markError = '';
		try {
			const next = await Api.setQuestionMark(item.question_version_id, !item.marked);
			session.items[current] = { ...session.items[current], marked: next.marked };
		} catch (err) {
			markError = err instanceof ApiError ? err.message : 'Could not update this question mark.';
		} finally {
			markBusy = false;
		}
	}

	// Fresh report panel per question — navigating never leaks state.
	$effect(() => {
		current;
		reportOpen = false;
		reportDone = '';
		reportError = '';
		reportNote = '';
		markError = '';
	});

	function onKeydown(event) {
		if ((result && !reviewing) || !session || !item) return;
		if (reviewing) {
			if (
				(event.key === 'Enter' || event.key === 'n' || event.key === 'N') &&
				current < session.items.length - 1
			) {
				current += 1;
			}
			return;
		}
		const letter = 'abcdefghij'.indexOf(event.key.toLowerCase());
		if (letter >= 0 && !item.answered && letter < item.options.length) {
			selected = letter;
		} else if (event.key === 'Enter' || event.key === 'n' || event.key === 'N') {
			if (!item.answered && selected !== null) {
				answer(selected);
			} else if (item.answered && current < session.items.length - 1) {
				current += 1;
			} else if (item.answered && allAnswered) {
				submitSession();
			}
		}
	}

	$effect(() => {
		if (!session?.deadline) return;
		const timer = setInterval(tick, 500);
		return () => clearInterval(timer);
	});

	function letterLabel(index) {
		return String.fromCharCode(65 + index);
	}

	onMount(load);
</script>

<svelte:window onkeydown={onKeydown} />

{#if loadFailed}
	<p class="error-text" role="alert">{loadFailed}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if result && !reviewing}
	<div class="card" data-testid="results">
		<h1>Session submitted</h1>
		<div class="stat-row">
			<span>Score<strong data-testid="score">{result.score}%</strong></span>
			<span>Correct<strong>{result.correct}</strong></span>
			<span>Incorrect<strong>{result.incorrect}</strong></span>
			<span>Skipped<strong>{result.skipped}</strong></span>
			<span>Total<strong data-testid="total">{result.total}</strong></span>
			<span
				>Time taken<strong data-testid="time-taken">{fmtDuration(result.time_taken_seconds)}</strong></span
			>
		</div>
		<p class="muted">This result is about this form — it is not a prediction of anything.</p>
		<div class="result-actions" aria-label="Result actions">
			{#if missedCount > 0}
				<button
					class="btn primary"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="practice-missed"
					onclick={() => startResultSession('missed')}
				>
					Practice missed questions
				</button>
			{:else if session?.chapter_id}
				<button
					class="btn primary"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="retry-session"
					onclick={() => startResultSession('retry')}
				>
					Retry this session
				</button>
			{:else}
				<a class="btn primary" href={`${base}/today`} data-testid="back-today">Back to Today</a>
			{/if}

			<button class="btn" type="button" data-testid="review-answers" onclick={startReview}>
				Review answers
			</button>

			{#if missedCount > 0 && session?.chapter_id}
				<button
					class="btn"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="retry-session"
					onclick={() => startResultSession('retry')}
				>
					Retry this session
				</button>
			{/if}

			{#if missedCount > 0 || session?.chapter_id}
				<a class="btn" href={`${base}/today`} data-testid="back-today">Back to Today</a>
			{/if}
		</div>
		{#if actionError}
			<p class="error-text" role="alert">{actionError}</p>
		{/if}
	</div>
{:else if session}
	<p class="muted" style="margin-bottom: var(--space-sm);">
		{reviewing
			? `Reviewing question ${current + 1} of ${session.items.length}`
			: `Question ${current + 1} of ${session.items.length}`}
		· {session.preset === 'revision' ? 'Re-practice' : session.preset === 'timed' ? 'Timed' : 'Tutor mode'}
		{#if !reviewing && remainingMs !== null}
			· <span
				class="timer"
				style:color={remainingMs < 60_000 ? 'var(--color-warning)' : 'inherit'}
				style:font-weight="700"
				data-testid="timer"
			>
				{fmt(remainingMs)} left
			</span>
		{/if}
	</p>
	{#if reviewing}
		<button
			class="linklike"
			type="button"
			data-testid="back-results"
			onclick={() => (reviewing = false)}
		>
			Back to results
		</button>
	{/if}

	{#if item}
		<div class="card">
			<div class="question-actions">
				<button
					class="btn"
					class:primary={item.marked}
					type="button"
					disabled={markBusy}
					data-loading={markBusy}
					data-testid="question-mark"
					aria-pressed={item.marked}
					onclick={toggleMark}
				>
					{markBusy ? 'Saving…' : item.marked ? 'Marked' : 'Mark question'}
				</button>
			</div>
			{#if markError}
				<p class="error-text" role="alert">{markError}</p>
			{/if}
			<p>{item.vignette}</p>
			<p><strong>{item.lead_in}</strong></p>

			<div class="options">
				{#each item.options as option, i (i)}
					<button
						type="button"
						class="option {(item.answered || reviewing || session.status === 'submitted') && i === item.correct_index ? 'correct' : ''}
							{item.answered && item.chosen_index === i && item.correct === false ? 'incorrect' : ''}"
						aria-pressed={!item.answered && selected === i}
						disabled={item.answered || busy || reviewing || session.status !== 'open'}
						data-testid={`option-${i}`}
						onclick={() => {
							if (!item.answered) selected = selected === i ? null : i;
						}}
					>
						<span class="key">{letterLabel(i)}</span>
						<span>{option.text}</span>
					</button>
				{/each}
			</div>

			{#if error}
				<p class="error-text" role="alert">{error}</p>
			{/if}

			{#if !item.answered && !reviewing && session.status === 'open'}
				<button
					class="btn primary"
					type="button"
					disabled={selected === null || busy || (remainingMs !== null && remainingMs <= 0)}
					data-loading={busy}
					data-testid="answer"
					onclick={() => answer(selected)}
				>
					{busy ? 'Recording…' : 'Answer'}
				</button>
				<button
					class="linklike"
					type="button"
					disabled={busy || (remainingMs !== null && remainingMs <= 0)}
					data-testid="skip"
					onclick={() => answer(null)}
				>
					Skip — I don't want to guess
				</button>
			{:else}
				<div
					class="feedback {item.correct === true ? 'good' : item.correct === false ? 'bad' : ''}"
					data-testid="feedback"
				>
					<p class="verdict">
						{item.correct === true
							? 'Correct.'
							: item.correct === false
								? 'Not quite.'
								: item.answered
									? 'Skipped.'
									: 'Not answered.'}
					</p>
					{#each item.options as option, i (i)}
						{#if option.rationale && (i === item.correct_index || i === item.chosen_index)}
							<p style="margin: 4px 0;">
								<strong>{letterLabel(i)}.</strong> {option.rationale}
							</p>
						{/if}
					{/each}
					<p style="margin: var(--space-sm) 0 0;">
						<strong>Key learning point:</strong> {item.key_learning_point}
					</p>
					{#if item.exam_tip}
						<p class="muted" style="margin: 4px 0 0;">Exam tip: {item.exam_tip}</p>
					{/if}
					{#if item.report_status === 'quarantined'}
						<p class="muted" style="margin: 4px 0 0;">
							Flagged by learners — out of rotation pending editorial review.
						</p>
					{:else if item.report_status === 'open'}
						<p class="muted" style="margin: 4px 0 0;">
							Flagged by a learner — under review.
						</p>
					{/if}
				</div>

				{#if reportDone}
					<p class="muted" data-testid="report-done">{reportDone}</p>
				{:else if !reportOpen}
					<button
						class="linklike"
						type="button"
						disabled={reportBusy}
						data-testid="report-open"
						onclick={() => {
							reportOpen = true;
							reportError = '';
						}}
					>
						Report a problem with this question
					</button>
				{:else}
					<div data-testid="report-form">
						<p class="field">
							<span>What's wrong?</span>
							<span style="display:flex; gap:8px; flex-wrap:wrap;">
								{#each REPORT_CATEGORIES as [value, label]}
									<button
										type="button"
										class="btn {reportCategory === value ? 'primary' : ''}"
										disabled={reportBusy}
										data-testid={`report-cat-${value}`}
										onclick={() => {
											reportCategory = value;
										}}
									>
										{label}
									</button>
								{/each}
							</span>
						</p>
						<label class="field">
							<span>Details (optional)</span>
							<input
								type="text"
								bind:value={reportNote}
								disabled={reportBusy}
								maxlength="2000"
								placeholder="What looks wrong?"
								data-testid="report-note"
							/>
						</label>
						{#if reportError}
							<p class="error-text" role="alert">{reportError}</p>
						{/if}
						<p style="display:flex; gap:12px;">
							<button
								class="btn primary"
								type="button"
								disabled={reportBusy}
								data-loading={reportBusy}
								data-testid="report-submit"
								onclick={submitReport}
							>
								{reportBusy ? 'Sending…' : 'Send report'}
							</button>
							<button
								class="linklike"
								type="button"
								disabled={reportBusy}
								onclick={() => {
									reportOpen = false;
									reportError = '';
								}}
							>
								Cancel
							</button>
						</p>
					</div>
				{/if}

				{#if current < session.items.length - 1}
					<button
						class="btn primary"
						type="button"
						data-testid="next"
						onclick={() => (current += 1)}
					>
						Next
					</button>
				{:else if reviewing}
					<button
						class="btn"
						type="button"
						data-testid="back-results-end"
						onclick={() => (reviewing = false)}
					>
						Back to results
					</button>
				{:else if allAnswered}
					<button
						class="btn primary"
						type="button"
						disabled={submitting}
						data-loading={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						{submitting ? 'Submitting…' : 'Submit session'}
					</button>
				{:else}
					<button
						class="btn"
						type="button"
						disabled={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						Submit with unanswered items (they count as skipped)
					</button>
				{/if}
			{/if}
		</div>
	{/if}
{:else}
	<p class="muted">Loading the session…</p>
{/if}
