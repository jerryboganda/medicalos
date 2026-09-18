<script>
	import { onMount } from 'svelte';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';

	let queue = $state(null);
	let loading = $state(true);
	let error = $state('');
	let revealed = $state(false);
	let current = $state(null);
	let busy = $state(false);
	let done = $state(false);

	const RATINGS = [
		{ key: 'again', label: 'Again', cls: 'danger-text' },
		{ key: 'hard', label: 'Hard', cls: '' },
		{ key: 'good', label: 'Good', cls: 'primary' },
		{ key: 'easy', label: 'Easy', cls: 'success' }
	];

	function nextCard() {
		revealed = false;
		if (queue.due.length > 0) {
			current = { ...queue.due[0], kind: 'due' };
			queue.due.shift();
		} else if (queue.new.length > 0) {
			current = { ...queue.new[0], kind: 'new' };
			queue.new.shift();
		} else {
			current = null;
			done = true;
		}
	}

	async function load() {
		loading = true;
		error = '';
		try {
			queue = await Api.reviewQueue();
			done = false;
			nextCard();
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Could not load your review queue.';
		} finally {
			loading = false;
		}
	}

	async function rate(key) {
		if (!current || busy) return;
		busy = true;
		error = '';
		try {
			await Api.reviewEvent(
				current.card_id,
				key,
				`${current.card_id}:${Date.now()}:${crypto.randomUUID()}`
			);
			nextCard();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not record the review.';
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
		await load();
	});
</script>

<h1>Review</h1>

{#if loading}
	<p class="muted">Loading your queue…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if done || !current}
	<div class="card">
		<p class="muted" data-testid="all-caught-up">
			All caught up. Cards return when they are due — that is the schedule
			working, not a punishment.
		</p>
		{#if queue && queue.backlog_remaining > 0}
			<p class="muted">{queue.backlog_remaining} overdue cards wait behind today's cap.</p>
		{/if}
		<a class="btn" href={`${base}/today`}>Back to Today</a>
	</div>
{:else}
	<div class="card" data-testid="review-card">
		<span class="chip">{current.kind === 'due' ? 'Due' : 'New'}</span>
		<p style="font-size: var(--text-body-lg);">{current.front}</p>
		{#if revealed}
			<hr style="border: none; border-top: 1px solid var(--color-surface-elevated);" />
			<p data-testid="back">{current.back}</p>
			<p class="muted" style="font-size: var(--text-sm);">
				Rate honestly — the schedule needs real recall, not kindness.
			</p>
			<div style="display:flex; gap:12px; flex-wrap:wrap;">
				{#each RATINGS as r (r.key)}
					<button
						class="btn {r.cls}"
						type="button"
						disabled={busy}
						data-testid={`rate-${r.key}`}
						onclick={() => rate(r.key)}
					>
						{r.label}
					</button>
				{/each}
			</div>
		{:else}
			<button class="btn primary" type="button" data-testid="reveal" onclick={() => (revealed = true)}>
				Show answer
			</button>
		{/if}
	</div>
{/if}
