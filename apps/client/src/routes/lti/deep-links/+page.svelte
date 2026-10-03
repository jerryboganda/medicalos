<!-- Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 -->
<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError, type LibrarySearchResult, type LtiDeepLinkResponse, type LtiResourceLink } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	type DeepLinkSettings = {
		deep_link_return_url?: string;
		accept_types?: string[];
		accept_presentation_document_targets?: string[];
		accept_multiple?: boolean;
	};
	type LaunchContext = {
		message_type?: string;
		deep_linking_settings?: DeepLinkSettings;
	};

	let context = $state<LaunchContext | null>(null);
	let settings = $state<DeepLinkSettings | null>(null);
	let query = $state('');
	let jurisdiction = $state('');
	let results = $state<LibrarySearchResult[]>([]);
	let selected = $state<LibrarySearchResult[]>([]);
	let searched = $state(false);
	let searching = $state(false);
	let preparing = $state(false);
	let searchError = $state('');
	let error = $state('');
	let submitError = $state('');
	let prepared = $state<LtiDeepLinkResponse | null>(null);
	let preparedItemCount = $state(0);
	let returnForm: HTMLFormElement | null = null;

	const maxSelected = $derived(settings?.accept_multiple === true ? 50 : 1);
	const returnHost = $derived(settings?.deep_link_return_url ? hostOf(settings.deep_link_return_url) : 'your LMS');

	onMount(() => {
		loadAuth();
		try {
			const saved = sessionStorage.getItem('mlos_lti_launch');
			sessionStorage.removeItem('mlos_lti_launch');
			context = saved ? (JSON.parse(saved) as LaunchContext) : null;
		} catch {
			context = null;
		}
		settings = context?.deep_linking_settings ?? null;

		if (!auth.token) {
			error = 'The LMS session could not be restored. Allow site storage, then launch the picker again from your LMS.';
		} else if (context?.message_type !== 'LtiDeepLinkingRequest' || !settings) {
			error = 'This picker launch is missing its platform settings. Return to your LMS and try again.';
		} else if (!settings.accept_types?.includes('ltiResourceLink')) {
			error = 'This LMS request does not accept Medical OS resource links.';
		} else if (!settings.deep_link_return_url || !isAllowedReturnUrl(settings.deep_link_return_url)) {
			error = 'The LMS return address is missing or is not a secure address.';
		}
	});

	function hostOf(value: string): string {
		try {
			return new URL(value).host;
		} catch {
			return 'your LMS';
		}
	}

	function isAllowedReturnUrl(value: string): boolean {
		try {
			const url = new URL(value);
			return (
				!url.username &&
				!url.password &&
				!url.hash &&
				(url.protocol === 'https:' || ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname))
			);
		} catch {
			return false;
		}
	}

	function errorMessage(reason: unknown, fallback: string): string {
		return reason instanceof ApiError || reason instanceof Error ? reason.message : fallback;
	}

	async function search(event: SubmitEvent) {
		event.preventDefault();
		searchError = '';
		if (query.trim().length < 2) {
			searchError = 'Enter at least two characters to search published articles.';
			return;
		}
		searching = true;
		try {
			const response = await Api.librarySearch(query.trim(), {
				jurisdiction: jurisdiction.trim().toUpperCase() || undefined,
				as_of: new Date().toISOString().slice(0, 10)
			});
			results = response.results.filter((item) => item.content_type === 'editorial_article');
			searched = true;
		} catch (reason) {
			results = [];
			searched = true;
			searchError = errorMessage(reason, 'Published articles could not be searched.');
		} finally {
			searching = false;
		}
	}

	function isSelected(article: LibrarySearchResult): boolean {
		return selected.some((item) => item.article_id === article.article_id);
	}

	function toggleArticle(article: LibrarySearchResult) {
		if (isSelected(article)) {
			selected = selected.filter((item) => item.article_id !== article.article_id);
		} else if (selected.length < maxSelected) {
			selected = [...selected, article];
		}
	}

	function toResourceLink(article: LibrarySearchResult, url: string): LtiResourceLink {
		const custom: Record<string, string> = {
			medicalos_content_type: 'article',
			medicalos_article_slug: article.slug
		};
		if (article.jurisdiction && /^[A-Z]{2,3}$/.test(article.jurisdiction)) {
			custom.medicalos_jurisdiction = article.jurisdiction;
		}
		return { type: 'ltiResourceLink', title: article.title, url, custom };
	}

	async function prepareResponse(articles: LibrarySearchResult[]) {
		if (!settings?.deep_link_return_url || !isAllowedReturnUrl(settings.deep_link_return_url)) return;
		preparing = true;
		error = '';
		try {
			const launchUrl = Api.ltiLaunchUrl(window.location.origin);
			const response = await Api.createLtiDeepLinkResponse(
				articles.map((article) => toResourceLink(article, launchUrl))
			);
			if (new URL(response.post_url).href !== new URL(settings.deep_link_return_url).href) {
				throw new Error('The server return address does not match the verified LMS launch.');
			}
			preparedItemCount = articles.length;
			prepared = response;
			await tick();
			if (returnForm) {
				returnForm.requestSubmit();
			} else {
				submitError = 'The signed response is ready, but the LMS form could not be submitted. Use the button below.';
			}
		} catch (reason) {
			error = errorMessage(reason, 'The LMS response could not be prepared.');
		} finally {
			preparing = false;
		}
	}
</script>

<svelte:head>
	<title>Choose Medical OS content · Medical OS</title>
</svelte:head>

<main class="picker">
	<section class="card" aria-labelledby="picker-heading">
		<p class="eyebrow">Institutional content</p>
		<h1 id="picker-heading">Choose Medical OS content</h1>
		<p class="lede">Search reviewed library articles, select what belongs in your course, then return the signed links to {returnHost}.</p>

		{#if error}
			<p class="notice" role="alert">{error}</p>
		{:else if prepared}
			<div class="prepared" role="status">
				<p class="flush"><strong>Returning to {returnHost}…</strong></p>
				<p class="muted">Sending {preparedItemCount} item{preparedItemCount === 1 ? '' : 's'} to your LMS.</p>
			</div>
			{#if submitError}
				<p class="notice" role="alert">{submitError}</p>
			{/if}
			<form bind:this={returnForm} method="post" action={prepared.post_url}>
				<input type="hidden" name="JWT" value={prepared.jwt} />
				<button class="btn primary" type="submit">Return to LMS</button>
			</form>
		{:else if settings}
			<form class="search-form" onsubmit={search}>
				<label class="field" for="lti-article-search">
					<span>Search published articles</span>
					<input id="lti-article-search" type="search" bind:value={query} autocomplete="off" />
				</label>
				<label class="field" for="lti-article-jurisdiction">
					<span>Country code <span class="muted">(optional)</span></span>
					<input id="lti-article-jurisdiction" bind:value={jurisdiction} maxlength="3" autocomplete="country" />
				</label>
				<button class="btn primary" type="submit" disabled={searching}>
					{searching ? 'Searching…' : 'Search'}
				</button>
			</form>

			{#if searchError}
				<p class="notice" role="alert">{searchError}</p>
			{/if}

			{#if searched}
				<fieldset class="results">
					<legend>Articles · {selected.length} of {maxSelected} selected</legend>
					{#if results.length === 0}
						<p class="muted">No published articles match this search.</p>
					{:else}
						{#each results as article (article.article_id)}
							<label class="result">
								<input
									type="checkbox"
								checked={isSelected(article)}
								disabled={!isSelected(article) && selected.length >= maxSelected}
								onchange={() => toggleArticle(article)}
							/>
								<span class="result-copy">
									<strong>{article.title}</strong>
									<span class="muted">{article.source_ref}{article.jurisdiction ? ` · ${article.jurisdiction}` : ''} · v{article.version}</span>
									<span>{article.excerpt}</span>
								</span>
							</label>
						{/each}
					{/if}
				</fieldset>
			{/if}

			<div class="actions">
				<button class="btn primary" type="button" disabled={preparing || selected.length === 0} onclick={() => prepareResponse(selected)}>
					{preparing ? 'Preparing…' : 'Add selected articles'}
				</button>
				<button class="btn ghost" type="button" disabled={preparing} onclick={() => prepareResponse([])}>
					Return to LMS without adding content
				</button>
			</div>
		{:else}
			<p class="muted" role="status">Loading the verified LMS request…</p>
		{/if}
	</section>
</main>

<style>
	.picker {
		box-sizing: border-box;
		width: min(100%, 58rem);
		margin: clamp(var(--space-xl), 6vh, 4rem) auto;
		padding-inline: var(--space-lg);
	}

	.eyebrow {
		color: var(--color-accent);
		font-size: var(--text-sm);
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	h1 {
		min-width: 0;
		overflow-wrap: anywhere;
		font-family: var(--font-display);
		font-size: clamp(var(--text-xl), 5vw, var(--text-2xl));
		font-style: normal;
	}

	.card {
		margin-bottom: 0;
	}

	.search-form {
		display: grid;
		grid-template-columns: minmax(0, 2fr) minmax(9rem, 1fr) auto;
		align-items: end;
		gap: var(--space-md);
		margin-block: var(--space-xl);
	}

	.search-form .field {
		min-width: 0;
		margin: 0;
	}

	.results {
		min-width: 0;
		border: 0;
		padding: 0;
		margin: var(--space-xl) 0;
	}

	.results legend {
		margin-bottom: var(--space-md);
		font-weight: 700;
	}

	.result {
		display: flex;
		align-items: flex-start;
		gap: var(--space-md);
		padding: var(--space-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-control);
		margin-bottom: var(--space-sm);
		cursor: pointer;
	}

	.result input {
		flex: none;
		margin-top: 0.25rem;
	}

	.result-copy {
		display: grid;
		min-width: 0;
		gap: var(--space-xs);
	}

	.result-copy > * {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.actions,
	.prepared {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-md);
		margin-top: var(--space-lg);
	}

	.notice {
		padding: var(--space-md);
		border: 1px solid var(--color-error);
		border-radius: var(--radius-control);
		color: var(--color-error);
	}

	@media (max-width: 42rem) {
		.search-form {
			grid-template-columns: minmax(0, 1fr);
		}

		.search-form .btn {
			width: 100%;
		}
	}
</style>
