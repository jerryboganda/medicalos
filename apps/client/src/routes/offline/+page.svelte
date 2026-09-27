<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 */
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, type PackQuestionResource } from '$lib/api';
	import {
		downloadPack,
		getOfflinePack,
		listOfflinePacks,
		MAX_PACK_QUESTIONS,
		removeOfflinePack,
		requestPersistentStorage,
		type OfflinePackContent,
		type OfflinePackSummary,
		type PackDownloadProgress
	} from '$lib/offline-packs';

	type Chapter = {
		chapter_id: string;
		chapter_name: string;
		system: string;
		subject: string;
		exam_id: string;
		exam: string;
		published_questions: number;
	};

	let chapters = $state<Chapter[]>([]);
	let packs = $state<OfflinePackSummary[]>([]);
	let selectedExam = $state('');
	let selectedChapters = $state<string[]>([]);
	let activePack = $state<OfflinePackContent | null>(null);
	let activeIndex = $state(0);
	let startingSession = $state(false);
	let error = $state('');
	let notice = $state('');
	let loading = $state(true);
	let downloading = $state(false);
	let removing = $state('');
	let online = $state(true);
	let persistentStorage = $state<boolean | null>(null);
	let progress = $state<PackDownloadProgress | null>(null);

	let examOptions = $derived(
		[...new Map(chapters.map((chapter) => [chapter.exam_id, chapter.exam])).entries()].map(
			([exam_id, exam]) => ({ exam_id, exam })
		)
	);
	let examChapters = $derived(chapters.filter((chapter) => chapter.exam_id === selectedExam));
	let activeQuestion: PackQuestionResource | null = $derived(
		activePack?.resources[activeIndex] ?? null
	);
	function restoreChapterSelection() {
		if (!selectedExam) return;
		selectedChapters = packs.find((pack) => pack.exam_id === selectedExam)?.chapters ?? [];
	}

	async function refreshPacks() {
		packs = await listOfflinePacks();
	}

	async function load() {
		loading = true;
		error = '';
		try {
			await refreshPacks();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not open browser storage.';
		}
		online = navigator.onLine;
		if (online) {
			try {
				chapters = (await Api.myCurriculum()).chapters;
				selectedExam = chapters[0]?.exam_id ?? packs[0]?.exam_id ?? '';
				restoreChapterSelection();
			} catch (cause) {
				if (!packs.length) {
					error = cause instanceof Error ? cause.message : 'Could not load chapters.';
				}
			}
		} else {
			selectedExam = packs[0]?.exam_id ?? '';
		}
		loading = false;
	}

	function changeExam(event: Event) {
		selectedExam = (event.currentTarget as HTMLSelectElement).value;
		restoreChapterSelection();
		activePack = null;
	}

	function toggleChapter(chapterId: string) {
		selectedChapters = selectedChapters.includes(chapterId)
			? selectedChapters.filter((id) => id !== chapterId)
			: [...selectedChapters, chapterId];
	}

	async function startDownload() {
		if (!online || !selectedExam || !selectedChapters.length || downloading) return;
		error = '';
		notice = '';
		progress = null;
		downloading = true;
		try {
			persistentStorage = await requestPersistentStorage();
			const pack = await downloadPack(selectedExam, selectedChapters, (value) => (progress = value));
			await refreshPacks();
			selectedChapters = pack.chapters;
			notice = pack.downloaded_count === pack.item_count
				? 'Pack downloaded and checked on this device.'
				: 'The download paused. Choose Resume to fetch the remaining questions.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The pack download could not finish.';
			try {
				await refreshPacks();
			} catch {
				// Keep the download error visible if the browser storage is unavailable.
			}
		} finally {
			downloading = false;
		}
	}

	async function openPack(summary: OfflinePackSummary) {
		error = '';
		notice = '';
		try {
			const pack = await getOfflinePack(summary.exam_id);
			if (!pack) throw new Error('This pack is no longer saved in this browser.');
			if (navigator.onLine) {
				const serverLeases = await Api.listPackLeases();
				const lease = serverLeases.leases.find((candidate) => candidate.lease_id === pack.lease_id);
				if (
					!lease ||
					lease.exam_id !== pack.exam_id ||
					lease.device_id !== pack.device_id ||
					Date.now() >= Date.parse(lease.expires_at) ||
					pack.chapters.some((chapter) => !lease.chapters.includes(chapter))
				) {
					await removeOfflinePack(summary.exam_id, pack.device_id);
					await refreshPacks();
					throw new Error('This pack lease expired or was revoked. The downloaded copy was removed.');
				}
			}
			activePack = pack;
			selectedExam = pack.exam_id;
			selectedChapters = pack.chapters;
			activeIndex = 0;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not open this offline pack.';
		}
	}

	async function startSyncedSession() {
		if (!activePack || !online || startingSession) return;
		startingSession = true;
		error = '';
		try {
			const { session_id } = await Api.createSession({
				preset: 'tutor',
				chapter_ids: activePack.chapters,
				source: 'any',
				question_count: Math.min(10, activePack.item_count)
			});
			await goto(`${base}/session/${session_id}`);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not start a synced tutor session.';
			startingSession = false;
		}
	}

	function moveQuestion(amount: number) {
		if (!activePack) return;
		activeIndex = Math.min(Math.max(0, activeIndex + amount), activePack.resources.length - 1);
	}

	async function deletePack(summary: OfflinePackSummary) {
		removing = summary.exam_id;
		error = '';
		try {
			const leaseId = await removeOfflinePack(summary.exam_id);
			if (activePack?.exam_id === summary.exam_id) activePack = null;
			await refreshPacks();
			if (navigator.onLine && leaseId) {
				try {
					await Api.revokePackLease(leaseId);
				} catch {
					notice = 'Removed from this browser. The server lease may remain active until it expires.';
				}
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not remove this pack.';
		} finally {
			removing = '';
		}
	}

	onMount(() => {
		void load();
		const updateNetwork = () => (online = navigator.onLine);
		window.addEventListener('online', updateNetwork);
		window.addEventListener('offline', updateNetwork);
		return () => {
			window.removeEventListener('online', updateNetwork);
			window.removeEventListener('offline', updateNetwork);
		};
	});
</script>

<svelte:head>
	<title>Offline packs · Medical Learning OS</title>
	<meta name="description" content="Download and review a small, device-bound practice pack offline." />
</svelte:head>

<h1>Offline packs</h1>
<p class="muted" data-testid="offline-best-effort">
	Browser packs are best effort. Storage can be evicted by the browser, and the configured lease must be
	renewed online. High-stakes and institutional assessments stay online.
</p>

{#if loading}
	<p class="muted is-loading" aria-live="polite">Opening this device's saved packs…</p>
{:else}
	{#if error}
		<p class="danger-text" role="alert" data-testid="pack-error">{error}</p>
	{/if}
	{#if notice}
		<p class="feedback" role="status">{notice}</p>
	{/if}

	<section class="card" aria-labelledby="pack-builder-heading">
		<h2 id="pack-builder-heading">Download questions</h2>
		<p class="muted">
			Choose chapters from one exam. Packs are limited to {MAX_PACK_QUESTIONS} questions in this browser.
			Starting or submitting a scored practice session still requires a connection.
			Updating a pack can add chapters; remove it first if you want to choose a smaller set.
		</p>
		{#if !online}
			<p class="muted" role="status">Reconnect to change or renew a pack. Saved questions below remain available until the lease expires.</p>
		{:else if examOptions.length === 0}
			<p class="muted">No published curriculum is available to download.</p>
		{:else}
			<label class="field" for="pack-exam">
				<span>Exam</span>
				<select id="pack-exam" value={selectedExam} onchange={changeExam} data-testid="pack-exam">
					{#each examOptions as exam (exam.exam_id)}
						<option value={exam.exam_id}>{exam.exam}</option>
					{/each}
				</select>
			</label>
			<fieldset class="chapter-list">
				<legend>Chapters ({selectedChapters.length} selected)</legend>
				{#each examChapters as chapter (chapter.chapter_id)}
					<label class="chapter-option">
						<input
							type="checkbox"
							checked={selectedChapters.includes(chapter.chapter_id)}
							disabled={packs.find((pack) => pack.exam_id === selectedExam)?.chapters.includes(chapter.chapter_id) ?? false}
							onchange={() => toggleChapter(chapter.chapter_id)}
							data-testid={`pack-chapter-${chapter.chapter_id}`}
						/>
						<span>
							<strong>{chapter.chapter_name}</strong>
							<small>{chapter.subject} · {chapter.system} · {chapter.published_questions} questions</small>
						</span>
					</label>
				{/each}
			</fieldset>
			<button
				class="btn primary"
				type="button"
				disabled={downloading || !selectedChapters.length}
				onclick={startDownload}
				data-testid="pack-download"
			>
				{downloading ? 'Downloading…' : packs.some((pack) => pack.exam_id === selectedExam) ? 'Resume or update pack' : 'Download selected chapters'}
			</button>
			{#if progress}
				<p class="muted" aria-live="polite" data-testid="pack-progress">
					{progress.done} of {progress.total} questions saved
				</p>
			{/if}
			{#if persistentStorage === false}
				<p class="muted" role="status">The browser did not grant persistent storage. It may remove this pack when device storage is low.</p>
			{/if}
		{/if}
	</section>

	<section class="card" aria-labelledby="saved-packs-heading">
		<h2 id="saved-packs-heading">Saved on this device</h2>
		{#if packs.length === 0}
			<p class="muted" data-testid="pack-empty">No offline packs are saved in this browser.</p>
		{:else}
			{#each packs as pack (pack.pack_id)}
				{@const expired = Date.now() >= Date.parse(pack.expires_at)}
				<article class="saved-pack" data-testid={pack.downloaded_count === pack.item_count ? 'pack-ready' : 'pack-partial'}>
					<div class="pack-copy">
						<strong>{chapters.find((chapter) => chapter.exam_id === pack.exam_id)?.exam ?? 'Saved exam pack'}</strong>
						<span>{pack.downloaded_count} of {pack.item_count} questions · {(pack.byte_count / 1_048_576).toFixed(1)} MB</span>
						<span class="muted">Lease renewed {new Date(pack.content_as_of).toLocaleDateString()} · saved {new Date(pack.saved_at).toLocaleDateString()} · lease expires {new Date(pack.expires_at).toLocaleDateString()}</span>
					</div>
					<div class="pack-actions">
						<button
							class="btn"
							type="button"
							disabled={expired || pack.downloaded_count === 0}
							onclick={() => openPack(pack)}
							data-testid="pack-open"
						>
							{expired ? 'Lease expired' : 'Open pack'}
						</button>
						<button
							class="btn remove"
							type="button"
							disabled={removing === pack.exam_id}
							onclick={() => deletePack(pack)}
							data-testid="pack-remove"
						>
							{removing === pack.exam_id ? 'Removing…' : 'Remove'}
						</button>
					</div>
				</article>
			{/each}
		{/if}
	</section>

	{#if activePack}
		<section class="card reader" aria-labelledby="offline-question-heading">
			<div class="reader-topline">
				<h2 id="offline-question-heading">Saved practice questions</h2>
				<span class="chip">Question {activePack.resources.length ? activeIndex + 1 : 0} of {activePack.resources.length}</span>
			</div>
			{#if activePack.resources.length === 0}
				<p class="muted">No question resources were saved. Resume the download while online.</p>
			{:else if activeQuestion}
				<div data-testid="pack-question">
					<p>{activeQuestion.vignette}</p>
					<p><strong>{activeQuestion.lead_in}</strong></p>
					<ol class="option-list">
						{#each activeQuestion.options as option}
							<li>{option.text}</li>
						{/each}
					</ol>
					<details data-testid="pack-answer-reveal">
						<summary>Reveal answer and explanations</summary>
						<p><strong>Answer:</strong> {activeQuestion.options[activeQuestion.correct_index]?.text}</p>
						{#each activeQuestion.options as option, index}
							<p class="rationale"><strong>{index === activeQuestion.correct_index ? 'Why this fits' : 'Why this option does not fit'}:</strong> {option.rationale}</p>
						{/each}
						<p>{activeQuestion.key_learning_point}</p>
						{#if activeQuestion.exam_tip}<p class="muted">{activeQuestion.exam_tip}</p>{/if}
						{#each activeQuestion.tutoring_cards as card (card.prompt_type)}
							<details>
								<summary>{card.prompt_type.replaceAll('_', ' ')}</summary>
								<p>{card.content}</p>
								<small>Source: {card.source_ref}</small>
							</details>
						{/each}
					</details>
				</div>
			{/if}
			<div class="reader-controls">
				<button class="btn" type="button" disabled={activeIndex === 0} onclick={() => moveQuestion(-1)}>Previous</button>
				<button class="btn" type="button" disabled={activeIndex >= activePack.resources.length - 1} onclick={() => moveQuestion(1)}>Next</button>
			</div>
			<p class="muted local-progress">This downloaded reader is for review. It does not record attempts or change learning progress.</p>
			{#if online && activePack.item_count > 0}
				<button class="btn primary" type="button" disabled={startingSession} onclick={startSyncedSession} data-testid="pack-start-session">
					{startingSession ? 'Starting…' : 'Start synced tutor session'}
				</button>
			{:else if !online}
				<p class="muted">Reconnect to start a synced tutor session.</p>
			{/if}
		</section>
	{/if}
{/if}

<style>
	/*
	 * Hallmark · App Shell (offline pack manager) + Long Document (question reader)
	 * theme: Midnight-equivalent (owner-locked §7.1 tokens)
	 * tone: utilitarian · audience: medical learners · job: save and review practice material
	 * variation: bounded download controls, resumable states, and a focused question reader
	 * token-only · keyboard labels and focus remain native · mobile: CI viewport gate pending
	 */
	.field {
		display: grid;
		gap: var(--space-xs);
		max-width: 100%;
		margin-block: var(--space-md);
	}

	.field select {
		min-height: 44px;
		max-width: 100%;
		padding: 0 var(--space-sm);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: inherit;
	}

	.chapter-list {
		min-width: 0;
		margin: var(--space-lg) 0;
		padding: 0;
		border: 0;
	}

	.chapter-list legend {
		margin-bottom: var(--space-sm);
		color: var(--color-text-secondary);
	}

	.chapter-option {
		display: flex;
		align-items: flex-start;
		gap: var(--space-sm);
		min-width: 0;
		margin-block: var(--space-sm);
		padding: var(--space-sm);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		cursor: pointer;
	}

	.chapter-option input {
		flex: 0 0 auto;
		width: 20px;
		height: 20px;
		margin-top: 2px;
		accent-color: var(--color-action-primary);
	}

	.option-list,
	.chapter-option span,
	.pack-copy {
		display: grid;
		min-width: 0;
		gap: var(--space-xs);
	}

	.chapter-option small,
	.pack-copy span,
	.rationale {
		color: var(--color-text-secondary);
		overflow-wrap: anywhere;
	}

	.saved-pack {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-md);
		min-width: 0;
		padding-block: var(--space-md);
		border-top: 1px solid var(--color-surface-elevated);
	}

	.pack-copy {
		flex: 1 1 14rem;
	}

	.pack-actions,
	.reader-controls,
	.reader-topline {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
	}

	.remove {
		border-color: var(--color-error);
	}

	.reader-topline {
		justify-content: space-between;
	}

	.reader-topline h2 {
		margin-bottom: var(--space-sm);
	}

	.reader p,
	.reader label,
	.reader summary {
		overflow-wrap: anywhere;
	}

	.reader-controls {
		justify-content: space-between;
		margin-top: var(--space-lg);
	}

	.local-progress {
		margin-bottom: 0;
		font-size: var(--text-sm);
	}

	@media (max-width: 360px) {
		.pack-actions {
			width: 100%;
		}

		.pack-actions .btn {
			flex: 1 1 auto;
			padding-inline: var(--space-md);
		}
	}
</style>
