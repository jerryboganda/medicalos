<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let today = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			today = await Api.today();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load your progress evidence.';
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

<h1>Progress</h1>

{#if loading}
	<p class="muted">Loading your progress evidence…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if today.learner.length === 0}
	<div class="card" data-testid="progress-empty">
		<p class="muted">No evidence yet — answer questions and your chapters appear here.</p>
	</div>
{:else}
	{#each today.learner as chapter (chapter.chapter_id)}
		<div class="card" data-testid="progress-chapter">
			<strong>{chapter.chapter_name}</strong>
			<p class="muted">
				{#if chapter.mastery_index === null}
					Not enough evidence yet ({chapter.independent_count} answered) — no mastery shown until it means something.
				{:else}
					Accuracy {chapter.mastery_index}% · evidence: {chapter.evidence_level.replace('_', ' ')}
				{/if}
			</p>
		</div>
	{/each}
{/if}
