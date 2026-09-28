<!-- Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import type {
		LibrarySearchResult,
		PrivateImportResponse,
		PrivateImportRight,
		PrivateDocumentSearchResult,
		PrivateImportSummary
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let q = $state('');
	let jurisdiction = $state('');
	let asOf = $state('');
	let lastSearchJurisdiction = $state('');
	let lastSearchAsOf = $state('');
	let results = $state<LibrarySearchResult[] | null>(null);
	let searched = $state(false);
	let searchError = $state('');
	let privateSearchResults = $state<PrivateDocumentSearchResult[]>([]);
	let busy = $state(false);
	let importRights = $state<PrivateImportRight[]>([]);
	let privateImports = $state<PrivateImportSummary[]>([]);
	let importsLoading = $state(true);
	let importsError = $state('');
	let importTitle = $state('');
	let importRightsRef = $state('');
	let selectedFile = $state<File | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let importing = $state(false);
	let importMessage = $state('');
	let importError = $state('');
	let readingId = $state('');
	let deletingId = $state('');
	let openedImport = $state<PrivateImportResponse | null>(null);

	function utcToday(): string {
		return new Date().toISOString().slice(0, 10);
	}

	function errorMessage(error: unknown): string {
		return error instanceof ApiError || error instanceof Error
			? error.message
			: 'The request could not be completed.';
	}

	async function search(e: Event) {
		e.preventDefault();
		busy = true;
		searchError = '';
		try {
			const selectedJurisdiction = jurisdiction.trim().toUpperCase();
			const selectedDate = asOf || utcToday();
			const searchResults = await Api.librarySearch(q, {
				jurisdiction: selectedJurisdiction || undefined,
				as_of: selectedDate
			});
			lastSearchJurisdiction = selectedJurisdiction;
			lastSearchAsOf = selectedDate;
			results = searchResults.results;
			privateSearchResults = searchResults.private_documents ?? [];
			searched = true;
		} catch (error) {
			searchError = errorMessage(error);
			results = [];
			privateSearchResults = [];
			searched = true;
		} finally {
			busy = false;
		}
	}

	async function loadPrivateImports() {
		importsLoading = true;
		importsError = '';
		try {
			const [rights, documents] = await Promise.all([
				Api.privateImportRights(),
				Api.listPrivateImports()
			]);
			importRights = rights.rights;
			privateImports = documents.documents;
		} catch (error) {
			importsError = errorMessage(error);
		} finally {
			importsLoading = false;
		}
	}

	function chooseFile(event: Event) {
		selectedFile = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
		importError = '';
		importMessage = '';
	}

	async function importDocument(event: Event) {
		event.preventDefault();
		importError = '';
		importMessage = '';
		if (!selectedFile || !importRightsRef) {
			importError = 'Choose a text file and an active rights reference.';
			return;
		}
		const name = selectedFile.name.toLowerCase();
		if (!name.endsWith('.txt') && !name.endsWith('.md')) {
			importError = 'Choose a .txt or .md file.';
			return;
		}
		if (selectedFile.size > 1024 * 1024) {
			importError = 'The file must be at most 1 MiB.';
			return;
		}
		importing = true;
		try {
			await Api.createPrivateImport({
				title: importTitle.trim() || selectedFile.name,
				media_type: name.endsWith('.md') ? 'text/markdown' : 'text/plain',
				content: await selectedFile.text(),
				rights_ref: importRightsRef
			});
			importMessage = 'Imported to your private Library.';
			importTitle = '';
			selectedFile = null;
			if (fileInput) fileInput.value = '';
			await loadPrivateImports();
		} catch (error) {
			importError = errorMessage(error);
		} finally {
			importing = false;
		}
	}

	async function readImport(documentId: string) {
		readingId = documentId;
		importError = '';
		try {
			openedImport = await Api.getPrivateImport(documentId);
		} catch (error) {
			openedImport = null;
			importError = errorMessage(error);
		} finally {
			readingId = '';
		}
	}

	async function deleteImport(documentId: string) {
		deletingId = documentId;
		importError = '';
		try {
			await Api.deletePrivateImport(documentId);
			if (openedImport?.document_id === documentId) openedImport = null;
			await loadPrivateImports();
		} catch (error) {
			importError = errorMessage(error);
		} finally {
			deletingId = '';
		}
	}

	function articleHref(result: LibrarySearchResult): string {
		const params = new URLSearchParams();
		if (lastSearchJurisdiction) params.set('jurisdiction', lastSearchJurisdiction);
		params.set('as_of', lastSearchAsOf || utcToday());
		return (
			base +
			'/library/articles/' +
			encodeURIComponent(result.slug) +
			'?' +
			params.toString()
		);
	}

	onMount(() => {
		asOf = utcToday();
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
		} else {
			void loadPrivateImports();
		}
	});
</script>

<svelte:head>
	<title>Library | Medical Learning OS</title>
</svelte:head>

<h1>Library</h1>

<section aria-labelledby="reviewed-heading">
	<h2 id="reviewed-heading">Reviewed articles</h2>
	<form onsubmit={search}>
		<label class="field" for="lib-q">
			<span>Search reviewed articles</span>
			<input id="lib-q" bind:value={q} placeholder="e.g. glorbin" data-testid="lib-q" />
		</label>
		<div class="scope-fields">
			<label class="field" for="lib-jurisdiction">
				<span>Country code</span>
				<input
					id="lib-jurisdiction"
					bind:value={jurisdiction}
					maxlength="2"
					autocomplete="off"
					placeholder="Blank shows global articles"
					data-testid="lib-jurisdiction"
				/>
			</label>
			<label class="field" for="lib-as-of">
				<span>Guidance date</span>
				<input id="lib-as-of" type="date" bind:value={asOf} data-testid="lib-as-of" />
			</label>
		</div>
		<button class="btn" type="submit" disabled={busy} data-loading={busy}>
			{busy ? 'Searching…' : 'Search'}
		</button>
	</form>
	{#if searchError}<p class="error-text" role="alert">{searchError}</p>{/if}

	{#if searched && results !== null}
		{#if results.length === 0 && privateSearchResults.length === 0}
			<p class="muted">Nothing found. Try another term.</p>
		{:else}
			{#each results as r (r.article_id)}
				<article class="card library-result" data-testid={'library-result-' + r.slug}>
					<a class="library-result-link" href={articleHref(r)} data-testid={'library-reader-' + r.slug}>
						<strong>{r.title}</strong>
						<span class="btn">Read article · v{r.version}</span>
					</a>
					<div class="result-meta">
						<span class="chip">Published</span>
						<span class="chip">{r.jurisdiction ?? 'Global guidance'}</span>
						<span class="muted">As of {r.as_of}</span>
					</div>
					<p>{r.excerpt}…</p>
					<p class="muted">Source: {r.source_ref}</p>
				</article>
			{/each}
		{/if}
	{/if}

	{#if searched && privateSearchResults.length > 0}
		<section aria-labelledby="private-search-heading">
			<h3 id="private-search-heading">Your searchable private imports</h3>
			{#each privateSearchResults as document (document.document_id)}
				<div class="card" data-testid={'private-search-' + document.document_id}>
					<strong>{document.title}</strong>
					<span class="chip">Private document</span>
					<p>{document.excerpt}…</p>
					<p class="muted">
						Rights: {document.rights_ref} · SHA-256:
						<code>{document.sha256}</code>
					</p>
					<button
						class="btn"
						type="button"
						onclick={() => readImport(document.document_id)}
						data-testid={'private-search-open-' + document.document_id}
					>
						Open private document
					</button>
				</div>
			{/each}
		</section>
	{/if}
</section>

<section class="card private-imports" aria-labelledby="private-import-heading">
	<h2 id="private-import-heading">Private imports</h2>
	<p class="muted">
		Import a permitted text or Markdown document. It stays private to your account and is readable only
		while its rights record is active.
	</p>
	<p class="muted">Use personal study materials only; do not upload patient-identifiable information.</p>
	<p class="muted">Private imports appear in search only when their rights record also permits search.</p>

	{#if importsLoading}
		<p class="muted is-loading" role="status">Loading your import rights and documents…</p>
	{:else if importsError}
		<p class="error-text" role="alert">{importsError}</p>
		<button class="btn" type="button" onclick={loadPrivateImports}>Retry</button>
	{:else}
		{#if importRights.length === 0}
			<p class="muted" data-testid="private-import-no-rights">
				No current rights record permits private import and display.
			</p>
		{:else}
			<form onsubmit={importDocument}>
				<label class="field" for="private-import-title">
					<span>Document title</span>
					<input id="private-import-title" bind:value={importTitle} maxlength="200" />
				</label>
				<label class="field" for="private-import-rights">
					<span>Rights reference</span>
					<select id="private-import-rights" bind:value={importRightsRef}>
						<option value="">Choose an active rights record</option>
						{#each importRights as right (right.rights_id)}
							<option value={right.ref_code}>
								{right.ref_code} · {right.licensor}
								{#if right.valid_to} · through {right.valid_to}{/if}
								· {right.search_allowed ? 'search permitted' : 'import only'}
							</option>
						{/each}
					</select>
				</label>
				<p class="muted">Up to 1 MiB per document, 25 documents and 10 MiB total per account. Search is available only when the selected rights record permits it.</p>
				<label class="field" for="private-import-file">
					<span>Document file (.txt or .md, up to 1 MiB)</span>
					<input
						bind:this={fileInput}
						id="private-import-file"
						type="file"
						accept=".txt,.md,text/plain,text/markdown"
						onchange={chooseFile}
					/>
				</label>
				<button
					class="btn"
					type="submit"
					disabled={importing || !selectedFile || !importRightsRef}
					data-loading={importing}
					data-testid="private-import-submit"
				>
					{importing ? 'Importing…' : 'Import document'}
				</button>
			</form>
		{/if}
	{/if}

	{#if importMessage}
		<p class="success-text" role="status" data-testid="private-import-message">{importMessage}</p>
	{/if}
	{#if importError}
		<p class="error-text" role="alert" data-testid="private-import-message">{importError}</p>
	{/if}

	<h3>Your documents</h3>
	{#if importsLoading}
		<p class="muted is-loading">Import list is loading.</p>
	{:else if privateImports.length === 0}
		<p class="muted">You have no private imports yet.</p>
	{:else}
		<ul class="import-list">
			{#each privateImports as document (document.document_id)}
				<li class="import-item" data-testid={`private-import-${document.document_id}`}>
					<strong>{document.title}</strong>
					<div class="import-meta">
						<span class="chip">{document.media_type}</span>
						<span class="chip">{document.rights_ref}</span>
					</div>
					<span class="muted">SHA-256: <code>{document.sha256}</code></span>
					{#if document.available}
						<button
							class="btn"
							type="button"
							disabled={readingId === document.document_id || deletingId === document.document_id}
							onclick={() => readImport(document.document_id)}
							data-testid={`private-import-read-${document.document_id}`}
						>
							{readingId === document.document_id ? 'Opening…' : 'Read'}
						</button>
					{:else}
						<p class="muted">Unavailable: its rights record expired or was revoked.</p>
					{/if}
					<button
						class="btn danger-text"
						type="button"
						disabled={deletingId === document.document_id || readingId === document.document_id}
						onclick={() => deleteImport(document.document_id)}
					>
						{deletingId === document.document_id ? 'Deleting…' : 'Delete'}
					</button>
				</li>
			{/each}
		</ul>
	{/if}

	{#if openedImport}
		<article class="opened-import" aria-labelledby="opened-import-title">
			<h3 id="opened-import-title">{openedImport.title}</h3>
			<p class="muted">Rights reference: {openedImport.rights_ref}</p>
			<p class="muted">SHA-256: <code>{openedImport.sha256}</code></p>
			<pre data-testid="private-import-content">{openedImport.content}</pre>
		</article>
	{/if}
</section>

<style>
	/* Hallmark · macrostructure: App Shell · tone: focused and utilitarian · anchor hue: violet
	 * theme: Midnight-equivalent (owner-locked) · variation: country/date controls and linked full-text reader
	 * motion: cut · contrast: pass (40–41) · mobile: pending final E2E (34, 49, 50–57)
	 */
	.scope-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 13rem), 1fr));
		gap: var(--space-md);
	}

	.library-result {
		min-width: 0;
		margin: var(--space-md) 0;
	}

	.library-result-link {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		color: var(--color-text-primary);
		text-decoration: none;
	}

	.library-result-link strong {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.library-result-link:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.library-result-link:active {
		opacity: 0.85;
	}

	.result-meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
		margin-top: var(--space-md);
	}

	.library-result > p:last-child {
		margin-bottom: 0;
	}

	.private-imports > p:first-of-type {
		margin-top: 0;
	}

	.import-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-md);
	}

	.import-item {
		min-width: 0;
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		display: grid;
		justify-items: start;
		gap: var(--space-sm);
	}

	.import-item code {
		overflow-wrap: anywhere;
	}

	.import-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-xs);
	}

	.opened-import {
		margin-top: var(--space-xl);
		min-width: 0;
	}

	.opened-import pre {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-family: var(--font-body);
		background: var(--color-canvas);
		border-radius: var(--radius-control);
		padding: var(--space-lg);
	}

	.private-imports :global(.success-text) {
		color: var(--color-success);
	}
</style>
