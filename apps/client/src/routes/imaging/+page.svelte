<!-- Hallmark · pre-emit critique: P4 H4 E4 S4 R4 V3 -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, apiErrorMessage, type ImageCaseSummary, type ImageFinding } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let cases = $state<ImageCaseSummary[]>([]);
	let selectedCaseId = $state('');
	let currentIndex = $state(0);
	let zoom = $state(1);
	let showFindings = $state(false);
	let selectedAnnotationId = $state('');
	let loading = $state(true);
	let error = $state('');
	let imageError = $state(false);
	let findingsByCase = $state<Record<string, ImageFinding[]>>({});
	let findingsError = $state('');
	let loadingFindings = $state(false);

	let selectedCase = $derived(cases.find((study) => study.case_id === selectedCaseId) ?? null);
	let currentImage = $derived(selectedCase?.images[currentIndex] ?? null);
	let currentAnnotations = $derived(
		selectedCase?.annotations.filter((annotation) => annotation.image_index === currentIndex) ?? []
	);

	async function loadCases() {
		loading = true;
		error = '';
		try {
			const response = await Api.imageCases();
			cases = response.cases;
			selectedCaseId = response.cases[0]?.case_id ?? '';
		} catch (value) {
			error = apiErrorMessage(value, 'The image studies could not be loaded.');
		} finally {
			loading = false;
		}
	}

	function selectCase(caseId: string) {
		selectedCaseId = caseId;
		currentIndex = 0;
		zoom = 1;
		showFindings = false;
		findingsError = '';
		loadingFindings = false;
		selectedAnnotationId = '';
		imageError = false;
	}

	function moveTo(index: number) {
		if (!selectedCase) return;
		currentIndex = Math.max(0, Math.min(selectedCase.images.length - 1, index));
		selectedAnnotationId = '';
		imageError = false;
	}

	function changeIndex(event: Event) {
		moveTo(Number((event.currentTarget as HTMLInputElement).value) - 1);
	}

	function scrollStack(event: WheelEvent) {
		if (!selectedCase || selectedCase.kind !== 'stack' || event.deltaY === 0) return;
		const nextIndex = Math.max(0, Math.min(selectedCase.images.length - 1, currentIndex + Math.sign(event.deltaY)));
		if (nextIndex === currentIndex) return;
		event.preventDefault();
		moveTo(nextIndex);
	}

	function zoomIn() {
		zoom = Math.min(3, Math.round((zoom + 0.25) * 100) / 100);
	}

	function zoomOut() {
		zoom = Math.max(1, Math.round((zoom - 0.25) * 100) / 100);
	}

	async function toggleFindings(event: Event) {
		const enabled = (event.currentTarget as HTMLInputElement).checked;
		showFindings = enabled;
		findingsError = '';
		if (!enabled || !selectedCase || Object.hasOwn(findingsByCase, selectedCase.case_id) || loadingFindings) return;
		const caseId = selectedCase.case_id;
		loadingFindings = true;
		try {
			const detail = await Api.imageCase(caseId);
			findingsByCase = { ...findingsByCase, [caseId]: detail.findings };
		} catch (value) {
			if (selectedCaseId === caseId) {
				findingsError = apiErrorMessage(value, 'The image findings could not be loaded.');
			}
		} finally {
			if (selectedCaseId === caseId) loadingFindings = false;
		}
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) goto(`${base}/login`);
		else void loadCases();
	});
</script>

<svelte:head>
	<title>Image studies | Medical Learning OS</title>
</svelte:head>

<h1>Image studies</h1>
<p class="intro muted">
	Licensed educational images are shown in their original colors. These fictional teaching cases are
	for study only and do not support real-patient decisions.
</p>

{#if loading}
	<p class="card muted" role="status">Loading licensed image studies…</p>
{:else if error}
	<p class="card error-text" role="alert">{error}</p>
{:else if cases.length === 0}
	<p class="card muted" data-testid="image-studies-empty">
		No image studies are currently available under an active display license.
	</p>
{:else if selectedCase}
	<div class="study-layout">
		<aside class="case-list" aria-label="Image studies">
			<h2>Studies</h2>
			{#each cases as study (study.case_id)}
				<button
					class="case-option"
					class:selected={study.case_id === selectedCaseId}
					type="button"
					aria-pressed={study.case_id === selectedCaseId}
					data-testid={`image-case-${study.case_id}`}
					onclick={() => selectCase(study.case_id)}
				>
					<strong>{study.title}</strong>
					<span>{study.kind === 'stack' ? 'Image stack' : 'Still image'}</span>
					{#if study.modality}<span class="muted">{study.modality}</span>{/if}
				</button>
			{/each}
		</aside>

		<section class="viewer card" aria-labelledby="study-title">
			<div class="viewer-heading">
				<div>
					<h2 id="study-title">{selectedCase.title}</h2>
					<p class="muted">
						{selectedCase.kind === 'stack' ? 'Ordered image stack' : 'Single still image'}
						{#if selectedCase.modality} · {selectedCase.modality}{/if}
					</p>
				</div>
				<span class="chip">Education mode</span>
			</div>
			{#if selectedCase.concepts.length > 0}
				<section class="concept-links" aria-labelledby="study-concepts-heading" data-testid="image-case-concepts">
					<h3 id="study-concepts-heading">Study concepts</h3>
					<ul>
						{#each selectedCase.concepts as concept (concept.concept_id)}
							<li>
								<strong>{concept.display_name}</strong>
								<span>{concept.canonical_key} · v{concept.version}</span>
								<p>{concept.definition}</p>
							</li>
						{/each}
					</ul>
					<p class="muted">These editorial links support study; they are not diagnostic conclusions.</p>
				</section>
			{/if}

			<div class="image-controls" aria-label="Image controls">
				<button
					class="btn"
					type="button"
					disabled={currentIndex === 0}
					onclick={() => moveTo(currentIndex - 1)}
					aria-label="Previous image"
				>
					Previous
				</button>
				<output data-testid="image-position">Image {currentIndex + 1} of {selectedCase.images.length}</output>
				<button
					class="btn"
					type="button"
					disabled={currentIndex >= selectedCase.images.length - 1}
					onclick={() => moveTo(currentIndex + 1)}
					aria-label="Next image"
				>
					Next
				</button>
			</div>

			{#if selectedCase.images.length > 1}
				<label class="sequence-control" for="image-sequence">
					<span>Image in sequence</span>
					<input
						id="image-sequence"
						aria-label="Image in sequence"
						type="range"
						min="1"
						max={selectedCase.images.length}
						value={currentIndex + 1}
						oninput={changeIndex}
					/>
				</label>
			{/if}
			{#if selectedCase.kind === 'stack'}
				<p class="muted stack-hint">Use the slider or scroll over the image to move through the stack.</p>
			{/if}

			<div class="zoom-controls" aria-label="Zoom controls">
				<button class="btn" type="button" aria-label="Zoom out" disabled={zoom <= 1} onclick={zoomOut}>−</button>
				<output>{Math.round(zoom * 100)}%</output>
				<button class="btn" type="button" aria-label="Zoom in" disabled={zoom >= 3} onclick={zoomIn}>+</button>
				<button class="btn" type="button" onclick={() => (zoom = 1)}>Reset zoom</button>
			</div>

			<div
				class="image-stage"
				data-testid="image-stage"
				aria-label={selectedCase.kind === 'stack' ? 'Image stack. Scroll here to move between images.' : 'Image display'}
				aria-live="polite"
				onwheel={scrollStack}
			>
				{#if currentImage}
					<div
						class="image-frame"
						data-testid="image-frame"
						data-zoom={zoom}
						style={`transform: scale(${zoom})`}
					>
						<img
							class="study-image"
							data-testid="study-image"
							src={currentImage.url}
							alt={`Educational image ${currentIndex + 1} of ${selectedCase.images.length} in ${selectedCase.title}`}
							draggable="false"
							onload={() => (imageError = false)}
							onerror={() => (imageError = true)}
						/>
						{#each currentAnnotations as annotation, index (annotation.annotation_id)}
							<button
								class="annotation-marker"
								class:active={selectedAnnotationId === annotation.annotation_id}
								type="button"
								aria-label={`Show reviewed annotation ${index + 1}`}
								aria-pressed={selectedAnnotationId === annotation.annotation_id}
								data-testid={`image-annotation-${annotation.annotation_id}`}
								style={`left:${annotation.x_percent}%;top:${annotation.y_percent}%`}
								onclick={() => (selectedAnnotationId = annotation.annotation_id)}
							>
								{index + 1}
							</button>
						{/each}
					</div>
				{:else}
					<p class="muted">No image is available for this position.</p>
				{/if}
			</div>
			{#if imageError}
				<p class="muted" role="status">This licensed image could not be loaded from its source.</p>
			{/if}

			{#if currentAnnotations.length > 0}
				<section class="annotations" aria-labelledby="reviewed-notes-heading">
					<h3 id="reviewed-notes-heading">Approved teaching annotations</h3>
					<ol>
						{#each currentAnnotations as annotation, index (annotation.annotation_id)}
							<li>
								<button
									class="annotation-text"
									type="button"
									aria-pressed={selectedAnnotationId === annotation.annotation_id}
									onclick={() => (selectedAnnotationId = annotation.annotation_id)}
								>
									<span class="annotation-number">{index + 1}</span>
									{annotation.body}
								</button>
							</li>
						{/each}
					</ol>
					<p class="muted">Editorial review does not establish clinical validity.</p>
				</section>
			{/if}

			<label class="reveal-control">
				<input
					type="checkbox"
					aria-label="Reveal findings"
					checked={showFindings}
					onchange={toggleFindings}
				/>
				<span>Reveal findings</span>
			</label>
			{#if showFindings}
				<div class="findings" data-testid="image-findings">
					<h3>Teaching findings</h3>
					{#if loadingFindings}
						<p class="muted is-loading" role="status">Loading findings…</p>
					{:else if findingsError}
						<p class="error-text" role="alert">{findingsError}</p>
					{:else if Object.hasOwn(findingsByCase, selectedCase.case_id)}
						{#each findingsByCase[selectedCase.case_id] as finding, index (index)}
							<section>
								<h4>{finding.section}</h4>
								<p>{finding.text}</p>
							</section>
						{/each}
					{/if}
					<p class="muted">Fictional teaching material. Not for diagnosis or patient care.</p>
				</div>
			{/if}
		</section>
	</div>
{/if}

<style>
	.intro {
		max-width: 72ch;
	}
	.concept-links {
		margin-block: var(--space-lg);
		padding: var(--space-md);
		border-left: 2px solid var(--color-accent);
		background: var(--color-surface-elevated);
	}
	.concept-links h3,
	.concept-links p {
		margin-block: 0 var(--space-sm);
	}
	.concept-links ul {
		display: grid;
		gap: var(--space-md);
		margin: 0;
		padding-left: var(--space-lg);
	}
	.concept-links li {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.concept-links li span {
		display: block;
		color: var(--color-text-secondary);
	}
	.concept-links li p {
		margin-top: var(--space-xs);
	}
	.study-layout {
		display: grid;
		grid-template-columns: minmax(13rem, 17rem) minmax(0, 1fr);
		align-items: start;
		gap: var(--space-xl);
	}
	.case-list,
	.viewer {
		min-width: 0;
	}
	.case-list {
		display: grid;
		gap: var(--space-sm);
	}
	.case-list h2,
	.viewer h2,
	.viewer h3 {
		margin-top: 0;
	}
	.case-option {
		display: grid;
		gap: var(--space-xs);
		width: 100%;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-surface);
		color: var(--color-text-primary);
		text-align: left;
		cursor: pointer;
	}
	.case-option.selected {
		border-color: var(--color-accent);
		box-shadow: inset 3px 0 0 var(--color-accent);
	}
	.case-option span,
	.viewer-heading p {
		font-size: var(--text-sm);
	}
	.viewer-heading {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-md);
	}
	.viewer-heading h2 {
		margin-bottom: var(--space-xs);
		overflow-wrap: anywhere;
	}
	.image-controls,
	.zoom-controls {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-wrap: wrap;
		gap: var(--space-sm);
		margin-block: var(--space-md);
	}
	.image-controls output,
	.zoom-controls output {
		min-width: 6rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}
	.sequence-control {
		display: grid;
		grid-template-columns: auto minmax(4rem, 1fr);
		align-items: center;
		gap: var(--space-md);
		max-width: 44rem;
		margin: var(--space-md) auto;
	}
	.sequence-control input {
		width: 100%;
	}
	.stack-hint {
		margin: calc(-1 * var(--space-xs)) 0 var(--space-sm);
	}
	.image-stage {
		display: grid;
		place-items: start center;
		min-width: 0;
		min-height: min(50vh, 24rem);
		max-height: min(62vh, 36rem);
		padding: var(--space-sm);
		overflow: auto;
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
	}
	.image-frame {
		position: relative;
		width: fit-content;
		max-width: 100%;
		transform-origin: top center;
	}
	.study-image {
		display: block;
		width: auto;
		height: auto;
		max-width: 100%;
		max-height: min(58vh, 33rem);
		object-fit: contain;
	}
	.annotation-marker {
		position: absolute;
		z-index: 1;
		width: 2.75rem;
		height: 2.75rem;
		transform: translate(-50%, -50%);
		border: 2px solid var(--color-text-primary);
		border-radius: 50%;
		background: var(--color-action-primary);
		color: var(--color-text-primary);
		font-weight: 700;
		cursor: pointer;
	}
	.annotation-marker.active {
		outline: 3px solid var(--color-focus);
	}
	.annotations,
	.findings {
		margin-top: var(--space-xl);
		padding: var(--space-md);
		border-left: 3px solid var(--color-accent);
		background: var(--color-surface);
	}
	.findings section + section {
		margin-top: var(--space-md);
	}
	.findings h4 {
		margin: 0 0 var(--space-xs);
	}
	.annotations ol {
		display: grid;
		gap: var(--space-sm);
		padding-left: 1.5rem;
	}
	.annotation-text {
		min-height: 44px;
		padding: var(--space-xs) var(--space-sm);
		border: 0;
		background: transparent;
		color: var(--color-text-primary);
		text-align: left;
		cursor: pointer;
	}
	.annotation-number {
		display: inline-grid;
		place-items: center;
		width: 1.5rem;
		height: 1.5rem;
		margin-right: var(--space-xs);
		border-radius: 50%;
		background: var(--color-action-primary);
		color: var(--color-text-primary);
		font-size: var(--text-sm);
	}
	.reveal-control {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		margin-top: var(--space-xl);
		font-weight: 700;
	}
	.reveal-control input {
		width: 1.1rem;
		height: 1.1rem;
	}
	@media (max-width: 48rem) {
		.study-layout {
			grid-template-columns: minmax(0, 1fr);
		}
		.case-list {
			grid-template-columns: repeat(auto-fit, minmax(min(100%, 12rem), 1fr));
		}
		.viewer-heading {
			flex-direction: column;
		}
		.image-stage {
			min-height: 15rem;
		}
	}
</style>
