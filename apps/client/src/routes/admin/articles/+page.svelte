<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E5 S5 R5 V5
	 * macrostructure: Workbench · index/detail editor with version history
	 * theme: Midnight-equivalent (owner-locked) · variation: source-first draft fields and compact history rail
	 * motion: cut · contrast: pass (40–41) · mobile: pending final E2E (34, 49, 50–57)
	 */
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import {
		Api,
		ApiError,
		adminToken,
		type AdminArticleSummary,
		type AdminArticleVersion,
		type ArticleCitation
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let token = $state('');
	let tokenRequired = $state(true);
	let articles = $state<AdminArticleSummary[]>([]);
	let selected = $state<AdminArticleVersion | null>(null);
	let creating = $state(false);
	let loading = $state(false);
	let busy = $state(false);
	let dirty = $state(false);
	let error = $state('');
	let message = $state('');
	let slug = $state('');
	let title = $state('');
	let body = $state('');
	let sourceRef = $state('');
	let jurisdiction = $state('');
	let effectiveFrom = $state('');
	let effectiveTo = $state('');
	let citations = $state<ArticleCitation[]>([]);

	function errorMessage(value: unknown): string {
		return value instanceof Error ? value.message : 'The request could not be completed.';
	}

	function setForm(version: AdminArticleVersion) {
		body = version.body;
		sourceRef = version.source_ref;
		jurisdiction = version.jurisdiction ?? '';
		effectiveFrom = version.effective_from ?? '';
		effectiveTo = version.effective_to ?? '';
		citations = version.citations.map((citation) => ({ ...citation }));
		dirty = false;
	}

	async function refreshArticles() {
		articles = (await Api.adminArticles()).articles;
	}

	async function loadWorkspace() {
		if (!auth.token) {
			goto(base + '/login');
			return;
		}
		if (!adminToken()) {
			tokenRequired = true;
			return;
		}

		loading = true;
		error = '';
		try {
			await refreshArticles();
			tokenRequired = false;
		} catch (value) {
			articles = [];
			tokenRequired = true;
			error = errorMessage(value);
		} finally {
			loading = false;
		}
	}

	async function submitToken(event: SubmitEvent) {
		event.preventDefault();
		const candidate = token.trim();
		if (!candidate || busy) return;
		try {
			localStorage.setItem('mlos_admin', candidate);
		} catch {
			error = 'Browser storage is unavailable. Allow local storage to use the article workspace.';
			return;
		}
		token = candidate;
		await loadWorkspace();
	}

	function changeAdminToken() {
		try {
			localStorage.removeItem('mlos_admin');
		} catch {
			error = 'Browser storage is unavailable. Clear the saved admin token in browser settings.';
			return;
		}
		token = '';
		tokenRequired = true;
		selected = null;
		creating = false;
		error = '';
	}

	function beginNewArticle() {
		selected = null;
		creating = true;
		slug = '';
		title = '';
		body = '';
		sourceRef = '';
		jurisdiction = '';
		effectiveFrom = '';
		effectiveTo = '';
		citations = [];
		dirty = false;
		error = '';
		message = '';
	}

	async function openVersion(summary: AdminArticleSummary) {
		busy = true;
		error = '';
		message = '';
		creating = false;
		try {
			const version = await Api.adminArticleVersion(summary.article_id, summary.version_id);
			selected = version;
			setForm(version);
		} catch (value) {
			error = errorMessage(value);
		} finally {
			busy = false;
		}
	}

	function requestFields() {
		return {
			body,
			source_ref: sourceRef,
			jurisdiction: jurisdiction.trim().toUpperCase() || null,
			effective_from: effectiveFrom || null,
			effective_to: effectiveTo || null,
			citations
		};
	}

	function addCitation() {
		citations = [...citations, { kind: 'page', anchor: '', target: '' }];
		dirty = true;
	}

	function removeCitation(index: number) {
		citations = citations.filter((_, citationIndex) => citationIndex !== index);
		dirty = true;
	}

	async function createArticle(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		message = '';
		try {
			const created = await Api.createAdminArticle({
				slug: slug.trim(),
				title: title.trim(),
				...requestFields()
			});
			creating = false;
			selected = created;
			setForm(created);
			await refreshArticles();
			message = 'Draft created. It is hidden from learners until you publish it.';
		} catch (value) {
			error = errorMessage(value);
		} finally {
			busy = false;
		}
	}

	async function saveDraft(event: SubmitEvent) {
		event.preventDefault();
		if (!selected || selected.status !== 'draft' || busy) return;
		busy = true;
		error = '';
		message = '';
		try {
			const updated = await Api.updateAdminArticleDraft(
				selected.article_id,
				selected.version_id,
				requestFields()
			);
			selected = updated;
			setForm(updated);
			await refreshArticles();
			message = 'Draft saved.';
		} catch (value) {
			error = errorMessage(value);
		} finally {
			busy = false;
		}
	}

	async function createRevision(articleId: string) {
		busy = true;
		error = '';
		message = '';
		try {
			const version = await Api.createAdminArticleVersion(articleId);
			selected = version;
			creating = false;
			setForm(version);
			await refreshArticles();
			message = 'Revision created from the latest published version.';
		} catch (value) {
			error = errorMessage(value);
		} finally {
			busy = false;
		}
	}

	async function publishDraft() {
		if (!selected || selected.status !== 'draft' || dirty || busy) return;
		busy = true;
		error = '';
		message = '';
		try {
			await Api.publishAdminArticleDraft(selected.article_id, selected.version_id);
			const version = await Api.adminArticleVersion(selected.article_id, selected.version_id);
			selected = version;
			setForm(version);
			await refreshArticles();
			message = 'Version published and available to learners.';
		} catch (value) {
			error = errorMessage(value);
		} finally {
			busy = false;
		}
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) {
			goto(base + '/login');
			return;
		}
		token = adminToken();
		tokenRequired = !token;
		if (token) void loadWorkspace();
	});
</script>

<svelte:head>
	<title>Article workspace | Medical Learning OS</title>
</svelte:head>

<header class="workspace-heading">
	<div>
		<h1>Article workspace</h1>
		<p class="muted">
			Prepare source-linked drafts and publish immutable versions. Publication does not certify clinical review or source rights.
		</p>
	</div>
	<nav class="workspace-links" aria-label="Admin navigation">
		<a class="btn" href={base + '/admin'}>Editorial console</a>
		<a class="btn" href={base + '/admin/dashboard'}>Owner dashboard</a>
		<button class="btn" type="button" onclick={changeAdminToken}>Change admin token</button>
	</nav>
</header>

{#if tokenRequired}
	<form class="card token-form" onsubmit={submitToken} data-testid="article-admin-token-gate">
		<h2>Administrator access</h2>
		<p class="muted">Use the administrator token for the editorial console.</p>
		<label class="field" for="article-admin-token">
			<span>Admin token</span>
			<input
				id="article-admin-token"
				type="password"
				autocomplete="current-password"
				bind:value={token}
				data-testid="article-admin-token"
			/>
		</label>
		{#if error}<p class="error-text" role="alert">{error}</p>{/if}
		<button class="btn" type="submit" disabled={busy || loading || !token.trim()}>
			Unlock article workspace
		</button>
	</form>
{:else}
	<div class="article-workspace">
		<section class="article-index" aria-labelledby="article-index-heading">
			<div class="index-heading">
				<div>
					<h2 id="article-index-heading">Versions</h2>
					<p class="muted">Drafts stay private. Published versions remain unchanged.</p>
				</div>
				<button class="btn" type="button" onclick={beginNewArticle} disabled={busy}>
					New article
				</button>
			</div>
			{#if loading}
				<p class="is-loading" role="status">Loading articles…</p>
			{:else if articles.length === 0}
				<p class="muted">No article versions yet. Create a draft to start the library.</p>
			{:else}
				<ul class="version-list">
					{#each articles as item (item.version_id)}
						<li class="version-item" data-testid={'admin-article-' + item.slug + '-v' + item.version}>
							<div>
								<h3>{item.title}</h3>
								<p class="muted">{item.slug} · v{item.version}</p>
								<div class="version-meta">
									<span class="chip">{item.status}</span>
									<span class="chip">{item.jurisdiction ?? 'Global'}</span>
									{#if item.effective_from || item.effective_to}
										<span class="muted">
											{item.effective_from ?? 'Any date'} – {item.effective_to ?? 'open-ended'}
										</span>
									{/if}
								</div>
							</div>
							<div class="version-actions">
								<button
									class="btn"
									type="button"
									disabled={busy}
									aria-label={'Open version ' + item.version + ' of ' + item.title}
									onclick={() => openVersion(item)}
								>
									Open v{item.version}
								</button>
								{#if item.is_latest && item.status === 'published'}
									<button
										class="btn"
										type="button"
										disabled={busy}
										onclick={() => createRevision(item.article_id)}
									>
										New revision
									</button>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<section class="article-editor" aria-labelledby="article-editor-heading">
			{#if error}<p class="error-text" role="alert" data-testid="article-editor-error">{error}</p>{/if}
			{#if message}<p class="success-text" role="status" data-testid="article-editor-message">{message}</p>{/if}
			{#if creating}
				<h2 id="article-editor-heading">New article draft</h2>
				<form onsubmit={createArticle} data-testid="article-create-form">
					<div class="field-grid">
						<label class="field" for="article-slug">
							<span>Permanent slug</span>
							<input id="article-slug" bind:value={slug} maxlength="120" required data-testid="article-slug" />
						</label>
						<label class="field" for="article-title">
							<span>Title</span>
							<input id="article-title" bind:value={title} maxlength="200" required data-testid="article-title" />
						</label>
					</div>
					{@render draftFields()}
					<div class="editor-actions">
						<button class="btn" type="submit" disabled={busy} data-testid="article-create">
							{busy ? 'Creating…' : 'Create draft'}
						</button>
						<button class="btn" type="button" onclick={() => (creating = false)}>Cancel</button>
					</div>
				</form>
			{:else if selected}
				<div class="editor-heading">
					<div>
						<h2 id="article-editor-heading">{selected.title}</h2>
						<p class="muted">{selected.slug} · version {selected.version}</p>
					</div>
					<span class="chip">{selected.status}</span>
				</div>
				{#if selected.status === 'draft'}
					<form onsubmit={saveDraft} data-testid="article-draft-form">
						{@render draftFields()}
						<div class="editor-actions">
							<button class="btn" type="submit" disabled={busy} data-testid="article-save">
								{busy ? 'Saving…' : 'Save draft'}
							</button>
							<button
								class="btn"
								type="button"
								disabled={busy || dirty}
								onclick={publishDraft}
								data-testid="article-publish"
							>
								Publish version
							</button>
							{#if dirty}<span class="muted">Save changes before publishing.</span>{/if}
						</div>
					</form>
				{:else}
					<p class="muted">
						This published version is immutable. Create a revision to prepare changes.
					</p>
					<dl class="published-meta">
						<div><dt>Source</dt><dd>{selected.source_ref}</dd></div>
						<div><dt>Country</dt><dd>{selected.jurisdiction ?? 'Global'}</dd></div>
						<div>
							<dt>Effective dates</dt>
							<dd>{selected.effective_from ?? 'Any date'} – {selected.effective_to ?? 'open-ended'}</dd>
						</div>
					</dl>
					<pre class="article-preview" data-testid="article-published-body">{selected.body}</pre>
					{#if selected.citations.length > 0}
						<h3>Citations</h3>
						<ul class="citation-list">
							{#each selected.citations as citation, index (index)}
								<li><strong>{citation.anchor}</strong> · {citation.kind}: {citation.target}</li>
							{/each}
						</ul>
					{/if}
				{/if}
			{:else}
				<div class="editor-empty">
					<h2 id="article-editor-heading">Select a version</h2>
					<p class="muted">Open a draft to edit it, or create a new article or revision.</p>
				</div>
			{/if}
		</section>
	</div>
{/if}

{#snippet draftFields()}
	<label class="field" for="article-body">
		<span>Article body</span>
		<textarea
			id="article-body"
			bind:value={body}
			oninput={() => (dirty = true)}
			maxlength="200000"
			rows="12"
			required
			placeholder="Plain text only. Markup is displayed as text, never executed."
			data-testid="article-body"
		></textarea>
	</label>
	<div class="field-grid">
		<label class="field" for="article-source">
			<span>Source reference</span>
			<input
				id="article-source"
				bind:value={sourceRef}
				oninput={() => (dirty = true)}
				maxlength="512"
				required
				data-testid="article-source"
			/>
		</label>
		<label class="field" for="article-jurisdiction">
			<span>Country code (blank means global)</span>
			<input
				id="article-jurisdiction"
				bind:value={jurisdiction}
				oninput={() => (dirty = true)}
				maxlength="2"
				autocomplete="off"
				placeholder="PK"
				data-testid="article-jurisdiction"
			/>
		</label>
		<label class="field" for="article-effective-from">
			<span>Effective from (inclusive)</span>
			<input
				id="article-effective-from"
				type="date"
				bind:value={effectiveFrom}
				onchange={() => (dirty = true)}
				data-testid="article-effective-from"
			/>
		</label>
		<label class="field" for="article-effective-to">
			<span>Effective to (inclusive)</span>
			<input
				id="article-effective-to"
				type="date"
				bind:value={effectiveTo}
				onchange={() => (dirty = true)}
				data-testid="article-effective-to"
			/>
		</label>
	</div>
	<fieldset class="citation-editor">
		<legend>Citations</legend>
		<p class="muted">Each anchor must appear in the body. Timestamp targets use MM:SS or HH:MM:SS.</p>
		{#each citations as citation, index (index)}
			<div class="citation-row" data-testid="article-citation">
				<label class="field" for={'citation-kind-' + index}>
					<span>Type</span>
					<select
						id={'citation-kind-' + index}
						bind:value={citation.kind}
						onchange={() => (dirty = true)}
					>
						<option value="source">Source</option>
						<option value="page">Page</option>
						<option value="figure">Figure</option>
						<option value="timestamp">Timestamp</option>
					</select>
				</label>
				<label class="field" for={'citation-anchor-' + index}>
					<span>Body anchor</span>
					<input
						id={'citation-anchor-' + index}
						bind:value={citation.anchor}
						oninput={() => (dirty = true)}
						maxlength="160"
					/>
				</label>
				<label class="field" for={'citation-target-' + index}>
					<span>Target</span>
					<input
						id={'citation-target-' + index}
						bind:value={citation.target}
						oninput={() => (dirty = true)}
						maxlength="512"
						placeholder="page 12, figure 2, or 00:42"
					/>
				</label>
				<button class="btn" type="button" onclick={() => removeCitation(index)}>Remove</button>
			</div>
		{/each}
		<button class="btn" type="button" onclick={addCitation} data-testid="article-add-citation">
			Add citation
		</button>
	</fieldset>
{/snippet}

<style>
	/* Hallmark · macrostructure: Workbench (version index + draft editor)
	 * tone: calm editorial utility · anchor hue: violet · theme: Midnight-equivalent (owner-locked)
	 * variation: history rail and source-first draft form · motion: cut
	 * contrast: pass (40–41) · mobile: pending final E2E (34, 49, 50–57)
	 */
	.workspace-heading,
	.editor-heading,
	.index-heading {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-md);
	}

	.workspace-heading {
		margin-bottom: var(--space-xl);
	}

	.workspace-heading > div {
		min-width: 0;
		flex: 1 1 20rem;
	}

	.workspace-heading p {
		max-width: 64ch;
		margin: 0;
	}

	.workspace-links,
	.version-actions,
	.editor-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
	}

	.article-workspace {
		display: grid;
		grid-template-columns: minmax(17rem, 0.8fr) minmax(0, 1.6fr);
		align-items: start;
		gap: var(--space-lg);
	}

	.article-index,
	.article-editor {
		min-width: 0;
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card);
		background: var(--color-surface);
	}

	.index-heading h2,
	.editor-heading h2 {
		margin-top: 0;
	}

	.index-heading p,
	.editor-heading p {
		margin-bottom: 0;
	}

	.version-list,
	.citation-list {
		list-style: none;
		margin: var(--space-lg) 0 0;
		padding: 0;
	}

	.version-item {
		display: grid;
		gap: var(--space-md);
		padding: var(--space-md) 0;
		border-top: 1px solid var(--color-surface-elevated);
	}

	.version-item h3 {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.version-item p {
		margin: var(--space-xs) 0;
		overflow-wrap: anywhere;
	}

	.version-meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-xs);
	}

	.field-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 13rem), 1fr));
		gap: var(--space-md);
	}

	.article-editor textarea {
		min-height: 12rem;
		resize: vertical;
	}

	.citation-editor {
		min-width: 0;
		margin: var(--space-lg) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.citation-editor legend {
		padding: 0 var(--space-xs);
		font-weight: 600;
	}

	.citation-row {
		display: grid;
		grid-template-columns: minmax(7rem, 0.6fr) minmax(8rem, 1fr) minmax(8rem, 1fr) auto;
		align-items: end;
		gap: var(--space-sm);
		padding: var(--space-md) 0;
		border-top: 1px solid var(--color-surface-elevated);
	}

	.published-meta {
		display: grid;
		gap: var(--space-sm);
	}

	.published-meta div {
		display: grid;
		grid-template-columns: minmax(7rem, 0.5fr) minmax(0, 1fr);
		gap: var(--space-sm);
	}

	.published-meta dt {
		color: var(--color-text-secondary);
	}

	.published-meta dd {
		min-width: 0;
		margin: 0;
		overflow-wrap: anywhere;
	}

	.article-preview {
		max-width: 100%;
		padding: var(--space-md);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		font: inherit;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.citation-list li {
		padding: var(--space-sm) 0;
		border-top: 1px solid var(--color-surface-elevated);
		overflow-wrap: anywhere;
	}

	.editor-empty {
		padding: var(--space-xl) 0;
	}

	.token-form {
		max-width: 32rem;
	}

	.token-form h2 {
		margin-top: 0;
	}

	@media (max-width: 54rem) {
		.article-workspace {
			grid-template-columns: minmax(0, 1fr);
		}

		.citation-row {
			grid-template-columns: repeat(auto-fit, minmax(min(100%, 12rem), 1fr));
			align-items: start;
		}
	}
</style>
