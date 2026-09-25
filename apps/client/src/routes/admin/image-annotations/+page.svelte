<!-- Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V3 -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import {
		Api,
		apiErrorMessage,
		adminToken,
		type AdminImageAnnotation,
		type ImageCaseSummary
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let cases = $state<ImageCaseSummary[]>([]);
	let annotations = $state<AdminImageAnnotation[]>([]);
	let selectedCaseId = $state('');
	let imageNumber = $state('1');
	let xPercent = $state('50');
	let yPercent = $state('50');
	let annotationBody = $state('');
	let reviewNotes = $state<Record<string, string>>({});
	let loading = $state(true);
	let saving = $state(false);
	let reviewingId = $state('');
	let locked = $state(false);
	let error = $state('');
	let message = $state('');
	let selectedCase = $derived(cases.find((study) => study.case_id === selectedCaseId) ?? null);

	async function loadWorkspace() {
		loading = true;
		error = '';
		try {
			const [caseResponse, annotationResponse] = await Promise.all([
				Api.imageCases(),
				Api.adminImageAnnotations()
			]);
			cases = caseResponse.cases;
			annotations = annotationResponse.annotations.filter(
				(annotation) => annotation.review_status === 'pending'
			);
			if (!selectedCaseId || !cases.some((study) => study.case_id === selectedCaseId)) {
				selectedCaseId = cases[0]?.case_id ?? '';
				imageNumber = '1';
			}
		} catch (value) {
			error = apiErrorMessage(value, 'The annotation workspace could not be loaded.');
		} finally {
			loading = false;
		}
	}

	async function submitAnnotation(event: SubmitEvent) {
		event.preventDefault();
		if (!selectedCase) return;
		error = '';
		message = '';
		saving = true;
		try {
			await Api.createImageAnnotation(selectedCase.case_id, {
				image_index: Number(imageNumber) - 1,
				x_percent: Number(xPercent),
				y_percent: Number(yPercent),
				body: annotationBody.trim()
			});
			annotationBody = '';
			message = 'Annotation submitted for independent review.';
			await loadWorkspace();
		} catch (value) {
			error = apiErrorMessage(value, 'The annotation could not be submitted.');
		} finally {
			saving = false;
		}
	}

	async function decide(annotationId: string, decision: 'approved' | 'rejected') {
		const note = reviewNotes[annotationId]?.trim() ?? '';
		if (!note) return;
		error = '';
		message = '';
		reviewingId = annotationId;
		try {
			await Api.reviewImageAnnotation(
				annotationId,
				decision,
				note
			);
			message = decision === 'approved' ? 'Annotation approved.' : 'Annotation rejected.';
			await loadWorkspace();
		} catch (value) {
			error = apiErrorMessage(value, 'The annotation review could not be saved.');
		} finally {
			reviewingId = '';
		}
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		if (!adminToken()) {
			locked = true;
			loading = false;
			return;
		}
		void loadWorkspace();
	});
</script>

<h1>Image annotation review</h1>
<p class="muted intro">
	Place plain-text teaching notes on licensed images. A different authorized reviewer must make the
	final decision. Editorial review does not establish clinical validity or authorize patient care.
</p>

{#if locked}
	<section class="card" role="status">
		<p>Unlock the editorial console to use this workspace.</p>
		<a class="btn" href={`${base}/admin`}>Open editorial console</a>
	</section>
{:else if loading}
	<p class="card muted" role="status">Loading image cases and pending annotations…</p>
{:else}
	{#if error}<p class="error-text" role="alert" data-testid="image-annotation-error">{error}</p>{/if}
	{#if message}<p class="card success-text" role="status">{message}</p>{/if}

	<section class="card" aria-labelledby="annotation-create-heading">
		<h2 id="annotation-create-heading">Add a teaching annotation</h2>
		{#if cases.length === 0}
			<p class="muted">No image cases with active display rights are available for annotation.</p>
		{:else}
			<form class="annotation-form" onsubmit={submitAnnotation}>
				<label class="field" for="annotation-case">
					<span>Case</span>
					<select
						id="annotation-case"
						bind:value={selectedCaseId}
						onchange={() => (imageNumber = '1')}
					>
						{#each cases as study (study.case_id)}
							<option value={study.case_id}>{study.title}</option>
						{/each}
					</select>
				</label>
				<label class="field" for="annotation-image-number">
					<span>Image number</span>
					<input
						id="annotation-image-number"
						type="number"
						min="1"
						max={selectedCase?.images.length ?? 1}
						step="1"
						required
						bind:value={imageNumber}
					/>
				</label>
				<div class="coordinate-grid">
					<label class="field" for="annotation-x">
						<span>Horizontal position (%)</span>
						<input id="annotation-x" type="number" min="0" max="100" step="0.1" required bind:value={xPercent} />
					</label>
					<label class="field" for="annotation-y">
						<span>Vertical position (%)</span>
						<input id="annotation-y" type="number" min="0" max="100" step="0.1" required bind:value={yPercent} />
					</label>
				</div>
				<label class="field" for="annotation-body">
					<span>Annotation</span>
					<textarea id="annotation-body" maxlength="1000" rows="4" required bind:value={annotationBody}></textarea>
				</label>
				<p class="muted">Positions are percentages measured from the image’s top-left corner.</p>
				<button class="btn primary" type="submit" disabled={saving || !selectedCase}>
					{saving ? 'Submitting…' : 'Submit annotation for review'}
				</button>
			</form>
		{/if}
	</section>

	<section class="card review-queue" aria-labelledby="annotation-queue-heading" data-testid="image-annotation-queue">
		<h2 id="annotation-queue-heading">Pending annotations</h2>
		{#if annotations.length === 0}
			<p class="muted" data-testid="image-annotation-queue-empty">No annotations are waiting for review.</p>
		{:else}
			{#each annotations as annotation (annotation.annotation_id)}
				{@const study = cases.find((candidate) => candidate.case_id === annotation.case_id)}
				{@const image = study?.images[annotation.image_index]}
				<article class="review-item" data-testid={`image-annotation-${annotation.annotation_id}`}>
					<div class="review-heading">
						<div>
							<h3>{study?.title ?? 'Image case'}</h3>
							<p class="muted">Image {annotation.image_index + 1} · {annotation.x_percent}% across · {annotation.y_percent}% down</p>
						</div>
						<span class="chip">Pending</span>
					</div>
					{#if image}
						<div class="review-image">
							<div class="review-image-frame">
								<img
									src={image.url}
									alt={`Image ${annotation.image_index + 1} from ${study?.title ?? 'the case'}`}
									loading="lazy"
									decoding="async"
								/>
								<span
									class="review-marker"
									aria-hidden="true"
									style={`left:${annotation.x_percent}%;top:${annotation.y_percent}%`}
								>●</span>
							</div>
						</div>
					{:else}
						<p class="muted">The referenced image is not currently available under its display license.</p>
					{/if}
					<p class="annotation-content">{annotation.body}</p>
					<label class="field" for={`review-note-${annotation.annotation_id}`}>
						<span>Review note</span>
						<textarea
							id={`review-note-${annotation.annotation_id}`}
							required
							maxlength="500"
							rows="2"
							bind:value={reviewNotes[annotation.annotation_id]}
						></textarea>
					</label>
					<div class="review-actions">
						<button
							class="btn primary"
							type="button"
							disabled={reviewingId !== '' || !reviewNotes[annotation.annotation_id]?.trim()}
							data-testid={`approve-image-annotation-${annotation.annotation_id}`}
							onclick={() => decide(annotation.annotation_id, 'approved')}
						>Approve annotation</button>
						<button
							class="btn"
							type="button"
							disabled={reviewingId !== '' || !reviewNotes[annotation.annotation_id]?.trim()}
							data-testid={`reject-image-annotation-${annotation.annotation_id}`}
							onclick={() => decide(annotation.annotation_id, 'rejected')}
						>Reject annotation</button>
					</div>
				</article>
			{/each}
		{/if}
	</section>
{/if}

<style>
	.intro {
		max-width: 75ch;
	}
	.annotation-form {
		display: grid;
		gap: var(--space-md);
		max-width: 52rem;
	}
	.coordinate-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-md);
	}
	.review-queue {
		margin-top: var(--space-xl);
	}
	.review-item {
		display: grid;
		gap: var(--space-md);
		min-width: 0;
		padding-block: var(--space-lg);
		border-top: 1px solid var(--color-surface-elevated);
	}
	.review-heading,
	.review-actions {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: var(--space-sm);
	}
	.review-heading h3 {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.review-image {
		position: relative;
		display: grid;
		place-items: center;
		min-height: 10rem;
		max-height: 28rem;
		overflow: auto;
		background: var(--color-canvas);
	}
	.review-image img {
		display: block;
		max-width: 100%;
		max-height: 28rem;
		object-fit: contain;
	}
	.review-image-frame {
		position: relative;
		width: fit-content;
		max-width: 100%;
	}
	.annotation-form textarea,
	.review-item textarea {
		width: 100%;
		box-sizing: border-box;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: inherit;
		resize: vertical;
	}
	.annotation-form textarea:focus-visible,
	.review-item textarea:focus-visible {
		border-color: var(--color-focus);
		outline: 2px solid var(--color-focus);
		outline-offset: 0;
	}
	.review-marker {
		position: absolute;
		transform: translate(-50%, -50%);
		color: var(--color-accent);
		font-size: var(--text-xl);
		text-shadow: 0 0 3px var(--color-canvas), 0 0 3px var(--color-canvas);
	}
	.annotation-content {
		margin: 0;
		padding: var(--space-md);
		border-left: 3px solid var(--color-accent);
		background: var(--color-surface);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	@media (max-width: 38rem) {
		.coordinate-grid {
			grid-template-columns: minmax(0, 1fr);
		}
		.review-actions {
			flex-direction: column;
			align-items: stretch;
		}
	}
</style>
