<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

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
		} finally {
			loading = false;
		}
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
		<h2>Study tools</h2>
		<p style="margin:0;">
			<a class="btn" href={`${base}/review`}>Review flashcards</a>
			<a class="btn" href={`${base}/coach`}>Ask the Coach</a>
			<a class="btn" href={`${base}/notes`}>My notes</a>
		</p>
	</div>
{/if}
