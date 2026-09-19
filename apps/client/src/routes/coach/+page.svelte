<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let questions = $state<
		{ question_version_id: string; vignette: string; chapter: string }[]
	>([]);
	let selected = $state('');
	let message = $state('');
	let promptType = $state('explain');
	let thread = $state<{ who: string; text: string }[]>([]);
	let busy = $state(false);
	let error = $state('');
	let loading = $state(true);

	async function ask() {
		if (!selected || !message.trim() || busy) return;
		busy = true;
		error = '';
		const key = `coach-${selected}-${Date.now()}-${crypto.randomUUID()}`;
		thread = [...thread, { who: 'you', text: message.trim() }];
		const shown = message.trim();
		message = '';
		try {
			const res = await Api.coachTurn(selected, promptType, shown, key);
			thread = [
				...thread,
				{
					who: 'coach',
					text: `${res.answer}

— grounded in ${res.adapter}`
				}
			];
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'The Coach could not answer right now.';
		} finally {
			busy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		try {
			questions = (await Api.answerableQuestions()).questions;
			if (questions.length > 0) {
				selected = questions[0].question_version_id;
			}
		} catch {
			error = 'Could not load your answered questions.';
		} finally {
			loading = false;
		}
	});
</script>

<h1>Coach</h1>
<p class="muted" data-testid="coach-page">
	Daily AI allowance applies to every account; study continues when it runs
	out.
</p>

{#if loading}
	<p class="muted">Loading…</p>
{:else if questions.length === 0}
	<div class="card">
		<p class="muted">
			The Coach explains material you have already worked through. Answer a
			question in Practice and it becomes Coach-ready here.
		</p>
		<a class="btn" href={`${base}/today`}>Go to Today</a>
	</div>
{:else}
	<div class="card">
		<label class="field" for="coach-q">
			<span>Pick a question you have answered</span>
			<select id="coach-q" bind:value={selected} data-testid="coach-q">
				{#each questions as q (q.question_version_id)}
					<option value={q.question_version_id}>
						{q.chapter} — {q.vignette.slice(0, 80)}
					</option>
				{/each}
			</select>
		</label>

		{#if thread.length > 0}
			{#each thread as turn, i (turn.who + turn.text)}
				<p style="margin: var(--space-sm) 0;">
					<strong>{turn.who === 'you' ? 'You' : 'Coach'}:</strong>
					<span data-testid={turn.who === 'coach' ? 'coach-answer' : `you-${i}`}>
						{turn.text}
					</span>
				</p>
			{/each}
		{/if}

		{#if error}
			<p class="error-text" role="alert">{error}</p>
		{/if}

		<label class="field" for="coach-msg">
			<span>Ask about this question</span>
			<textarea
				id="coach-msg"
				bind:value={message}
				rows="3"
				placeholder="e.g. Explain this simply — why is the right answer right?"
				data-testid="coach-msg"
			></textarea>
		</label>
		<label class="field" for="coach-type">
			<span>What kind of help</span>
			<select id="coach-type" bind:value={promptType}>
				<option value="explain">Explain simply</option>
				<option value="why_wrong">Why is my answer wrong?</option>
				<option value="free">Ask something else about it</option>
			</select>
		</label>
		<button
			class="btn primary"
			type="button"
			disabled={busy || !selected || !message.trim()}
			data-loading={busy}
			data-testid="coach-ask"
			onclick={ask}
		>
			{busy ? 'Thinking…' : 'Ask the Coach'}
		</button>
		<p class="muted" style="font-size: var(--text-sm); margin-top: var(--space-md);">
			The Coach answers only from this question's reviewed material and your
			own attempt. Daily AI allowance applies; everything else keeps working
			when it runs out.
		</p>
	</div>
{/if}
