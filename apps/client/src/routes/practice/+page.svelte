<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let today = $state(null);
	let loading = $state(true);
	let error = $state('');
	let starting = $state('');
	let pendingTakeover = $state(null);

	function tasks() {
		return (
			today?.tasks.filter(
				(task) => task.kind === 'practice' && task.chapter_id && task.status !== 'done'
			) ?? []
		);
	}

	async function load() {
		loading = true;
		error = '';
		try {
			today = await Api.today();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load your practice options.';
		} finally {
			loading = false;
		}
	}

	async function openSession(body, busyKey) {
		if (starting) return;
		starting = busyKey;
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
			starting = '';
		}
	}

	async function start(task, preset) {
		const body = {
			preset,
			chapter_id: task.chapter_id,
			question_count: task.question_count || 10
		};
		if (preset === 'timed') body.time_limit_seconds = 300;
		await openSession(body, `${preset}-${task.id}`);
	}

	async function confirmTakeover() {
		if (!pendingTakeover || starting) return;
		const pending = pendingTakeover;
		starting = pending.busyKey;
		error = '';
		try {
			const { session_id } = await Api.createSession({ ...pending.body, takeover: true });
			pendingTakeover = null;
			goto(`${base}/session/${session_id}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not take over the study session.';
			starting = '';
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

<h1>Practice</h1>

{#if loading}
	<p class="muted">Loading your practice options…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	{#if pendingTakeover}
		<div class="actions">
			<button class="btn primary" type="button" onclick={confirmTakeover} data-testid="confirm-takeover">
				{starting ? 'Starting…' : 'Take over study session'}
			</button>
			<button
				class="btn"
				type="button"
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
{:else if tasks().length === 0}
	<div class="card" data-testid="practice-empty">
		<p class="muted">No practice task is available in today's plan yet.</p>
		<a class="btn" href={`${base}/today`}>Back to Today</a>
	</div>
{:else}
	{#each tasks() as task (task.id)}
		<div class="card" data-testid="practice-task">
			<h2>{task.title}</h2>
			<p class="muted">{task.question_count} planned questions</p>
			<div class="actions">
				<button
					class="btn primary"
					type="button"
					disabled={starting !== ''}
					data-loading={starting === `tutor-${task.id}`}
					data-testid="practice-tutor"
					onclick={() => start(task, 'tutor')}
				>
					{starting === `tutor-${task.id}` ? 'Starting…' : 'Start tutor'}
				</button>
				<button
					class="btn"
					type="button"
					disabled={starting !== ''}
					data-testid="practice-timed"
					onclick={() => start(task, 'timed')}
				>
					{starting === `timed-${task.id}` ? 'Starting…' : 'Start timed (5 min)'}
				</button>
			</div>
		</div>
	{/each}
{/if}

<style>
	/* Hallmark · preserves the existing Medical OS app-shell tokens and interaction states. */
	.actions {
		display: flex;
		gap: var(--space-md);
		flex-wrap: wrap;
	}
</style>
