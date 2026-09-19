<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let queue = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			queue = await Api.reviewQueue();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load your learning queue.';
		} finally {
			loading = false;
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

<h1>Learn</h1>

{#if loading}
	<p class="muted">Loading your learning queue…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if queue.due.length === 0 && queue.new.length === 0}
	<div class="card" data-testid="learn-empty">
		<p class="muted">All caught up. Cards return when they are due — that is the schedule working, not a punishment.</p>
		{#if queue.backlog_remaining > 0}
			<p class="muted">{queue.backlog_remaining} overdue cards wait behind today's cap.</p>
		{/if}
	</div>
{:else}
	<div class="card" data-testid="learn-queue">
		<h2>Spaced review</h2>
		<div class="stat-row">
			<div><strong>{queue.due.length}</strong><span class="muted">Due</span></div>
			<div><strong>{queue.new.length}</strong><span class="muted">New</span></div>
		</div>
		{#if queue.backlog_remaining > 0}
			<p class="muted">{queue.backlog_remaining} overdue cards wait behind today's cap.</p>
		{/if}
		<a class="btn primary" href={`${base}/review`} data-testid="learn-review">Start review</a>
	</div>
{/if}
