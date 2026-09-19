<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	interface Result {
		article_id: string;
		slug: string;
		title: string;
		version: number;
		excerpt: string;
	}

	let q = $state('');
	let results = $state<Result[] | null>(null);
	let searched = $state(false);
	let busy = $state(false);

	async function search(e: Event) {
		e.preventDefault();
		busy = true;
		try {
			results = (await Api.librarySearch(q)).results as Result[];
			searched = true;
		} finally {
			busy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
		}
	});
</script>

<h1>Library</h1>

<form onsubmit={search}>
	<label class="field" for="lib-q">
		<span>Search reviewed articles</span>
		<input id="lib-q" bind:value={q} placeholder="e.g. glorbin" data-testid="lib-q" />
	</label>
	<button class="btn primary" type="submit" disabled={busy}>
		{busy ? 'Searching…' : 'Search'}
	</button>
</form>

{#if searched && results !== null}
	{#if results.length === 0}
		<p class="muted">Nothing found. Try another term.</p>
	{:else}
		{#each results as r (r.article_id)}
			<div class="card">
				<strong>{r.title}</strong>
				<span class="chip" style="margin-left: 8px;">v{r.version}</span>
				<p style="margin: var(--space-xs) 0 0;">{r.excerpt}…</p>
			</div>
		{/each}
	{/if}
{/if}
