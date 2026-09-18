<script>
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
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

	const item = $derived(session?.items?.[current] ?? null);
	const allAnswered = $derived(
		session?.items ? session.items.every((i) => i.answered) : false
	);

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
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not submit the session.';
		} finally {
			submitting = false;
		}
	}

	function onKeydown(event) {
		if (result || !session || !item) return;
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
{:else if result}
	<div class="card" data-testid="results">
		<h1>Session submitted</h1>
		<div class="stat-row">
			<span>Score<strong data-testid="score">{result.score}%</strong></span>
			<span>Correct<strong>{result.correct}</strong></span>
			<span>Incorrect<strong>{result.incorrect}</strong></span>
			<span>Skipped<strong>{result.skipped}</strong></span>
		</div>
		<p class="muted">This result is about this form — it is not a prediction of anything.</p>
		<a class="btn primary" href={`${base}/today`} data-testid="back-today">Back to Today</a>
	</div>
{:else if session}
	<p class="muted" style="margin-bottom: var(--space-sm);">
		Question {current + 1} of {session.items.length}
		· {session.preset === 'revision' ? 'Re-practice' : session.preset === 'timed' ? 'Timed' : 'Tutor mode'}
		{#if remainingMs !== null}
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

	{#if item}
		<div class="card">
			<p>{item.vignette}</p>
			<p><strong>{item.lead_in}</strong></p>

			<div class="options">
				{#each item.options as option, i (i)}
					<button
						type="button"
						class="option {item.answered && i === item.correct_index ? 'correct' : ''}
							{item.answered && item.chosen_index === i && item.correct === false ? 'incorrect' : ''}"
						aria-pressed={!item.answered && selected === i}
						disabled={item.answered || busy}
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

			{#if !item.answered}
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
						{item.correct === true ? 'Correct.' : item.correct === false ? 'Not quite.' : 'Skipped.'}
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
				</div>

				{#if current < session.items.length - 1}
					<button
						class="btn primary"
						type="button"
						data-testid="next"
						onclick={() => (current += 1)}
					>
						Next
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
