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
			error = err instanceof ApiError ? err.message : 'Could not load recorded plan changes.';
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

<h1>Coach</h1>

{#if loading}
	<p class="muted">Loading recorded plan changes…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if today.revisions.length === 0}
	<div class="card" data-testid="coach-empty">
		<p class="muted">No plan changes are recorded yet.</p>
	</div>
{:else}
	{#each today.revisions as revision (revision.id)}
		<div class="card" data-testid="coach-revision">
			<span class="chip {revision.undone ? 'undone' : ''}">
				{revision.undone ? 'Undone' : revision.automatic ? 'Applied — automatic' : 'Applied'}
			</span>
			<p>{revision.explanation}</p>
		</div>
	{/each}
{/if}
