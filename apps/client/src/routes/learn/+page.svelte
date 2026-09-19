<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let queue = $state(null);
	let library = $state([]);
	let selectedLibrary = $state(null);
	let libraryLoading = $state(false);
	let libraryError = $state('');
	let loading = $state(true);
	let error = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			const [reviewQueue, libraryResponse] = await Promise.all([Api.reviewQueue(), Api.library()]);
			queue = reviewQueue;
			library = libraryResponse.items;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load your learning queue.';
		} finally {
			loading = false;
		}
	}

	async function openLibrary(item) {
		libraryLoading = true;
		libraryError = '';
		try {
			selectedLibrary = await Api.libraryVersion(item.item_id, item.version);
		} catch (err) {
			libraryError = err instanceof ApiError ? err.message : 'Could not open this library item.';
		} finally {
			libraryLoading = false;
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

{#if !loading && !error}
	<section class="card" aria-labelledby="library-heading" aria-busy={libraryLoading} data-testid="learn-library">
		<h2 id="library-heading">Library</h2>
		<p class="muted">Reviewed articles and references, with source date and jurisdiction shown explicitly.</p>

		{#if library.length === 0}
			<p class="muted">No published library items are available yet.</p>
		{:else}
			{#each library as item}
				<article>
					<h3>{item.title}</h3>
					<p class="muted">
						{item.kind === 'article' ? 'Article' : 'Reference'} · Version {item.version} ·
						{item.effective_date} · {item.jurisdiction}
					</p>
					<p class="muted">Source: {item.source_label} · {item.provenance_class}</p>
					<button class="btn" type="button" onclick={() => openLibrary(item)} disabled={libraryLoading}>
						Read
					</button>
				</article>
			{/each}
		{/if}
		{#if libraryLoading}
			<p class="muted" aria-live="polite">Opening library item…</p>
		{/if}
		{#if libraryError}
			<p class="error-text" role="alert">{libraryError}</p>
		{/if}

		{#if selectedLibrary}
			<article aria-live="polite">
				<h3>{selectedLibrary.title}</h3>
				<p>{selectedLibrary.body}</p>
				<p class="muted">
					Effective {selectedLibrary.effective_date} · {selectedLibrary.jurisdiction} ·
					{selectedLibrary.source_label}
				</p>
				{#if selectedLibrary.source_url}
					<a href={selectedLibrary.source_url} target="_blank" rel="noreferrer">Open source</a>
				{/if}
			</article>
		{/if}
	</section>
{/if}
