<script lang="ts">
	/* Hallmark · pre-emit critique: P5 H4 E5 S5 R5 V5
	 * macrostructure: Long Document · scope controls followed by one source-linked reading column
	 * theme: Midnight-equivalent (owner-locked) · variation: persistent country/date context above the article
	 * motion: cut · contrast: pass (40–41) · mobile: CI viewport gate authored; final run pending
	 */
	import { onDestroy, onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError, type ArticleMedia, type LibraryArticleResponse } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let slug = $state('');
	let jurisdiction = $state('');
	let asOf = $state('');
	let article = $state<LibraryArticleResponse | null>(null);
	let captionTrackUrls = $state<Record<string, string>>({});
	let mediaElements: Record<string, HTMLMediaElement | undefined> = {};
	let loading = $state(false);
	let error = $state('');

	function utcToday(): string {
		return new Date().toISOString().slice(0, 10);
	}

	function mediaTime(milliseconds: number): string {
		const totalSeconds = Math.floor(milliseconds / 1000);
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;
		return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
	}

	function captionVtt(media: ArticleMedia): string {
		const cues = media.captions.map((cue, index) => {
			const text = cue.text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replace(/\r?\n+/g, ' ');
			return `${index + 1}\n${mediaTime(cue.start_ms)}.${(cue.start_ms % 1000).toString().padStart(3, '0')} --> ${mediaTime(cue.end_ms)}.${(cue.end_ms % 1000).toString().padStart(3, '0')}\n${text}`;
		});
		return `WEBVTT\n\n${cues.join('\n\n')}\n`;
	}

	function updateCaptionTracks(media: ArticleMedia[]) {
		for (const objectUrl of Object.values(captionTrackUrls)) URL.revokeObjectURL(objectUrl);
		const next: Record<string, string> = {};
		for (const item of media) {
			if (item.captions.length) {
				next[item.media_id] = URL.createObjectURL(
					new Blob([captionVtt(item)], { type: 'text/vtt' })
				);
			}
		}
		captionTrackUrls = next;
	}

	function seekMedia(mediaId: string, atMs: number) {
		const player = mediaElements[mediaId];
		if (!player) return;
		player.focus();
		const seek = () => {
			player.currentTime = atMs / 1000;
		};
		if (player.readyState === HTMLMediaElement.HAVE_NOTHING) {
			player.addEventListener('loadedmetadata', seek, { once: true });
			player.load();
			return;
		}
		seek();
	}

	async function loadArticle() {
		if (!slug) return;
		loading = true;
		error = '';
		article = null;
		updateCaptionTracks([]);
		try {
			const selectedJurisdiction = jurisdiction.trim().toUpperCase();
			article = await Api.libraryArticle(slug, {
				jurisdiction: selectedJurisdiction || undefined,
				as_of: asOf || utcToday()
			});
			updateCaptionTracks(article.media);
		} catch (value) {
			error =
				value instanceof ApiError && value.status === 404
					? 'No published version is available for this country and date. Content from another country was not substituted.'
					: value instanceof Error
						? value.message
						: 'The article could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function applyScope(event: SubmitEvent) {
		event.preventDefault();
		const params = new URLSearchParams();
		const selectedJurisdiction = jurisdiction.trim().toUpperCase();
		if (selectedJurisdiction) params.set('jurisdiction', selectedJurisdiction);
		params.set('as_of', asOf || utcToday());
		const url =
			base +
			'/library/articles/' +
			encodeURIComponent(slug) +
			'?' +
			params.toString();
		await goto(url, { replaceState: true, keepFocus: true, noScroll: true });
		jurisdiction = selectedJurisdiction;
		asOf = asOf || utcToday();
		await loadArticle();
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) {
			goto(base + '/login');
			return;
		}
		const pathParts = window.location.pathname.split('/').filter(Boolean);
		slug = decodeURIComponent(pathParts[pathParts.length - 1] ?? '');
		const params = new URLSearchParams(window.location.search);
		jurisdiction = (params.get('jurisdiction') ?? '').toUpperCase();
		asOf = params.get('as_of') ?? utcToday();
		void loadArticle();
	});

	onDestroy(() => updateCaptionTracks([]));
</script>

<svelte:head>
	<title>{article?.title ?? 'Library article'} | Medical Learning OS</title>
</svelte:head>

<header class="reader-heading">
	<a class="btn" href={base + '/library'}>Back to Library</a>
	<p class="muted">Choose the country and date for the guidance you want to read.</p>
</header>

<section class="scope-card" aria-labelledby="reading-scope-heading">
	<h2 id="reading-scope-heading">Reading context</h2>
	<form class="scope-controls" onsubmit={applyScope} data-testid="article-reading-scope">
		<label class="field" for="reader-jurisdiction">
			<span>Country code</span>
			<input
				id="reader-jurisdiction"
				bind:value={jurisdiction}
				maxlength="2"
				autocomplete="off"
				placeholder="Blank shows global guidance only"
				data-testid="reader-jurisdiction"
			/>
		</label>
		<label class="field" for="reader-as-of">
			<span>Guidance date</span>
			<input id="reader-as-of" type="date" bind:value={asOf} data-testid="reader-as-of" />
		</label>
		<button class="btn" type="submit" disabled={loading} data-testid="reader-apply-scope">
			Apply context
		</button>
	</form>
</section>

{#if loading}
	<p role="status" class="muted">Loading the published version for this context…</p>
{:else if error}
	<section class="card reader-error" role="alert" data-testid="article-unavailable">
		<h1>Version unavailable</h1>
		<p>{error}</p>
		<p class="muted">Change the country or date above, or return to the Library search.</p>
	</section>
{:else if article}
	<article class="article-reader" data-testid="library-article">
		<header class="article-heading">
			<p class="eyebrow">Published · version {article.version}</p>
			<h1>{article.title}</h1>
			<p class="muted">
				Selected country: {article.selected_jurisdiction ?? 'Global only'} · As of {article.as_of}
			</p>
			<p class="scope-result">
				{#if article.jurisdiction}
					Country-specific guidance for {article.jurisdiction}
				{:else}
					Global guidance
				{/if}
				{#if article.effective_from || article.effective_to}
					· Effective {article.effective_from ?? 'without a start date'} through
					{article.effective_to ?? 'an open end date'}
				{/if}
			</p>
			{#if article.selected_jurisdiction && !article.jurisdiction}
				<p class="muted">
					No current country-specific version matched. Global guidance is shown; guidance from another country is never substituted.
				</p>
			{/if}
		</header>

		<p class="source-ref"><strong>Source:</strong> {article.source_ref}</p>
		<p class="review-status muted">
			Published content is not represented as clinically reviewed.
		</p>
		<pre class="article-body" data-testid="article-body">{article.body}</pre>

		{#if article.media.length > 0}
			<section class="article-media" aria-labelledby="article-media-heading" data-testid="article-media">
				<h2 id="article-media-heading">Audio and video</h2>
				{#each article.media as media (media.media_id)}
					<section class="media-item" data-testid={`article-media-${media.media_id}`}>
						<h3>{media.kind === 'video' ? 'Video' : 'Audio'}</h3>
						{#if media.kind === 'video'}
							<video
								bind:this={mediaElements[media.media_id]}
								class="media-player"
								controls
								preload="none"
								src={media.url}
								data-testid={`article-media-player-${media.media_id}`}
							>
								{#if captionTrackUrls[media.media_id]}
									<track
										kind="captions"
										srclang="und"
										label="Captions"
										src={captionTrackUrls[media.media_id]}
										default
										data-testid={`article-media-captions-${media.media_id}`}
									/>
								{/if}
								Your browser cannot play this video.
							</video>
						{:else if media.kind === 'audio'}
							<audio
								bind:this={mediaElements[media.media_id]}
								class="media-player"
								controls
								preload="none"
								src={media.url}
								data-testid={`article-media-player-${media.media_id}`}
							>
								{#if captionTrackUrls[media.media_id]}
									<track
										kind="captions"
										srclang="und"
										label="Captions"
										src={captionTrackUrls[media.media_id]}
										default
										data-testid={`article-media-captions-${media.media_id}`}
									/>
								{/if}
								Your browser cannot play this audio.
							</audio>
						{/if}
						{#if media.captions.length > 0}
							<div class="media-transcript" data-testid={`article-media-transcript-${media.media_id}`}>
								<h4>Transcript</h4>
								<ol>
									{#each media.captions as cue, index (index)}
										<li><time>{mediaTime(cue.start_ms)}</time> {cue.text}</li>
									{/each}
								</ol>
							</div>
						{/if}
						{#if media.chapters.length > 0}
							<nav aria-label={`${media.kind} chapters`}>
								<h4>Chapters</h4>
								<ol class="media-chapters">
									{#each media.chapters as chapter, index (index)}
										<li>
											<button
												class="linklike"
												type="button"
												aria-label={`Start ${chapter.title} at ${mediaTime(chapter.at_ms)}`}
												data-testid={`article-media-chapter-${media.media_id}-${chapter.at_ms}`}
												onclick={() => seekMedia(media.media_id, chapter.at_ms)}
											>
												<time>{mediaTime(chapter.at_ms)}</time> · {chapter.title}
											</button>
										</li>
									{/each}
								</ol>
							</nav>
						{/if}
						<p class="muted" data-testid={`article-media-rights-${media.media_id}`}>
							Rights reference: {media.rights_ref}
						</p>
					</section>
				{/each}
				<p class="muted">Media files are not saved in offline packs.</p>
			</section>
		{/if}

		{#if article.citations.length > 0}
			<section class="citation-section" aria-labelledby="article-citations-heading">
				<h2 id="article-citations-heading">Source locations</h2>
				<ol class="citation-list">
					{#each article.citations as citation, index (index)}
						<li data-testid="article-citation">
							<strong>{citation.anchor}</strong>
							<span>{citation.kind}</span>
							<span>{citation.target}</span>
						</li>
					{/each}
				</ol>
			</section>
		{/if}
	</article>
{/if}

<style>
	/* Hallmark · macrostructure: Long Document (source-linked reading column)
	 * tone: focused reading utility · anchor hue: violet · theme: Midnight-equivalent (owner-locked)
	 * variation: country/date context stays above one plain-text version · motion: cut
	 * contrast: pass (40–41) · mobile: pending final E2E (34, 49, 50–57)
	 */
	.reader-heading {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		margin-bottom: var(--space-lg);
	}

	.reader-heading p {
		margin: 0;
	}

	.scope-card {
		margin-bottom: var(--space-xl);
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card);
		background: var(--color-surface);
	}

	.scope-card h2 {
		margin-top: 0;
	}

	.scope-controls {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 13rem), 1fr));
		align-items: end;
		gap: var(--space-md);
	}

	.article-reader {
		max-width: 72ch;
		margin-inline: auto;
	}

	.article-heading {
		padding-bottom: var(--space-lg);
		border-bottom: 1px solid var(--color-surface-elevated);
	}

	.article-heading h1 {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.eyebrow {
		color: var(--color-accent);
		font-size: var(--text-sm);
		font-weight: 600;
	}

	.scope-result {
		font-weight: 600;
	}

	.source-ref,
	.review-status {
		margin: var(--space-md) 0;
		overflow-wrap: anywhere;
	}

	.article-body {
		max-width: 100%;
		margin: var(--space-xl) 0;
		padding: var(--space-lg);
		border-radius: var(--radius-control);
		background: var(--color-surface);
		color: var(--color-text-primary);
		font: inherit;
		line-height: 1.75;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.article-media {
		display: grid;
		gap: var(--space-lg);
		margin-block: var(--space-xl);
	}

	.article-media > h2,
	.media-item h3,
	.media-transcript h4,
	.media-item nav h4 {
		margin-block: 0 var(--space-sm);
	}

	.media-item {
		display: grid;
		gap: var(--space-md);
		min-width: 0;
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card);
		background: var(--color-surface);
	}

	.media-player {
		display: block;
		width: 100%;
		max-width: 100%;
		border-radius: var(--radius-control);
		background: var(--color-canvas);
	}

	.media-transcript,
	.media-chapters {
		min-width: 0;
		padding-left: var(--space-xl);
	}

	.media-transcript li,
	.media-chapters li {
		margin-block: var(--space-xs);
		overflow-wrap: anywhere;
	}

	.media-transcript time,
	.media-chapters time {
		color: var(--color-text-secondary);
		font-variant-numeric: tabular-nums;
	}

	.media-chapters button {
		min-height: 44px;
		text-align: left;
	}

	.citation-section {
		margin-top: var(--space-xl);
		padding-top: var(--space-lg);
		border-top: 1px solid var(--color-surface-elevated);
	}

	.citation-list {
		padding-left: var(--space-xl);
	}

	.citation-list li {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
		margin: var(--space-sm) 0;
		overflow-wrap: anywhere;
	}

	.citation-list li span {
		color: var(--color-text-secondary);
	}

	.reader-error {
		max-width: 48rem;
		margin-inline: auto;
	}

	.reader-error h1 {
		margin-top: 0;
	}
</style>
