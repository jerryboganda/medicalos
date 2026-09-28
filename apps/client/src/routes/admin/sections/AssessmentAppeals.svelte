<script lang="ts">
	// Independent review of learner appeals against scenario assessments; decisions never rewrite
	// the original result. No requirement ID cited inline; e2e: sim07-assessment-appeals.spec.ts.
	import { onMount } from 'svelte';
	import {
		Api,
		ApiError,
		type ScenarioAppealDecision,
		type ScenarioAssessmentAppeal,
		type ScenarioAssessmentAppealQueueItem
	} from '$lib/api';

	let { refreshAudit }: { refreshAudit: () => Promise<void> } = $props();

	let assessmentAppeals = $state<ScenarioAssessmentAppealQueueItem[]>([]);
	let appealQueueBusy = $state(false);
	let appealQueueError = $state('');
	let appealQueueMessage = $state('');
	let selectedAssessmentAppeal = $state<ScenarioAssessmentAppeal | null>(null);
	let appealDecision = $state<ScenarioAppealDecision>('confirmed');
	let appealRationale = $state('');

	async function loadScenarioAssessmentAppeals() {
		appealQueueBusy = true;
		appealQueueError = '';
		try {
			assessmentAppeals = (await Api.listScenarioAssessmentAppeals()).appeals;
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Assessment appeals could not be loaded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	async function openScenarioAssessmentAppeal(appealId: string) {
		appealQueueBusy = true;
		appealQueueError = '';
		appealQueueMessage = '';
		try {
			selectedAssessmentAppeal = await Api.getScenarioAssessmentAppeal(appealId);
			appealDecision = 'confirmed';
			appealRationale = '';
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Appeal details could not be loaded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	async function reviewScenarioAssessmentAppeal(event: Event) {
		event.preventDefault();
		if (!selectedAssessmentAppeal || appealQueueBusy || !appealRationale.trim()) return;
		appealQueueBusy = true;
		appealQueueError = '';
		appealQueueMessage = '';
		try {
			const result = await Api.reviewScenarioAssessmentAppeal(
				selectedAssessmentAppeal.appeal_id,
				{ decision: appealDecision, rationale: appealRationale.trim() }
			);
			appealQueueMessage = `Appeal reviewed: ${result.decision.replaceAll('_', ' ')}.`;
			selectedAssessmentAppeal = null;
			await Promise.all([loadScenarioAssessmentAppeals(), refreshAudit()]);
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Appeal decision could not be recorded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	onMount(loadScenarioAssessmentAppeals);
</script>

<section class="card" aria-labelledby="scenario-appeals-heading" data-testid="scenario-assessment-appeals">
	<h2 id="scenario-appeals-heading">Independent assessment appeals</h2>
	<p class="muted">
		Review the learner’s reason and the original station evidence. The learner and every original
		examiner are barred from deciding the appeal. Decisions are permanent and do not rewrite the
		original assessment.
	</p>
	{#if appealQueueBusy && assessmentAppeals.length === 0}
		<p class="muted is-loading" role="status">Loading assessment appeals…</p>
	{:else if appealQueueError}
		<p class="error-text" role="alert">{appealQueueError}</p>
	{:else if assessmentAppeals.length === 0}
		<p class="muted">No assessment appeals are waiting for review.</p>
	{:else}
		<ul class="scenario-assessment-list">
			{#each assessmentAppeals as appeal (appeal.appeal_id)}
				<li class="scenario-assessment-row" data-testid={`scenario-appeal-${appeal.appeal_id}`}>
					<div>
						<strong>{appeal.scenario}</strong>
						<p class="muted">Version {appeal.scenario_version} · submitted {new Date(appeal.created_at).toLocaleString()}</p>
					</div>
					<button class="btn" type="button" disabled={appealQueueBusy} onclick={() => openScenarioAssessmentAppeal(appeal.appeal_id)} data-testid={`scenario-appeal-open-${appeal.appeal_id}`}>
						Review appeal
					</button>
				</li>
			{/each}
		</ul>
	{/if}
	{#if appealQueueMessage}
		<p class="muted" role="status" data-testid="scenario-appeal-message">{appealQueueMessage}</p>
	{/if}
	<button class="btn" type="button" disabled={appealQueueBusy} onclick={loadScenarioAssessmentAppeals}>Refresh appeal queue</button>

	{#if selectedAssessmentAppeal}
		<section class="scenario-review" aria-labelledby="scenario-appeal-review-heading">
			<h3 id="scenario-appeal-review-heading">{selectedAssessmentAppeal.scenario} · version {selectedAssessmentAppeal.scenario_version}</h3>
			<p class="muted">Run {selectedAssessmentAppeal.run_id} · submitted {new Date(selectedAssessmentAppeal.created_at).toLocaleString()}</p>
			<h4>Learner’s appeal reason</h4>
			<p class="appeal-reason" data-testid="scenario-appeal-reason">{selectedAssessmentAppeal.reason}</p>
			<h4>Original station transcript</h4>
			<ol class="scenario-transcript">
				{#each selectedAssessmentAppeal.timeline as event, index (index)}
					<li><strong>Event {index + 1}: {event.on}</strong><span class="muted">{event.from} → {event.to}</span></li>
				{/each}
			</ol>
			<h4>Original rubric</h4>
			<ul class="scenario-assessment-list">
				{#each selectedAssessmentAppeal.rubric as criterion (criterion.criterion_key)}
					<li class="rights-record">
						<strong>{criterion.label}</strong>
						<p class="muted">{criterion.assessment_status.replaceAll('_', ' ')}{criterion.score === null ? '' : ` · ${criterion.score} / ${criterion.max_score}`}</p>
						<p>{criterion.evidence || 'No examiner evidence recorded.'}</p>
					</li>
				{/each}
			</ul>
			<form onsubmit={reviewScenarioAssessmentAppeal}>
				<label class="field" for="scenario-appeal-decision">
					<span>Independent decision</span>
					<select id="scenario-appeal-decision" bind:value={appealDecision} disabled={appealQueueBusy}>
						<option value="confirmed">Confirm original assessment</option>
						<option value="reassessment_required">Require a new independent assessment</option>
					</select>
				</label>
				{#if appealDecision === 'reassessment_required'}
					<p class="muted">The original score remains visible and is flagged as unsuitable for consequential use pending a new independent assessment.</p>
				{/if}
				<label class="field" for="scenario-appeal-rationale">
					<span>Decision rationale</span>
					<textarea id="scenario-appeal-rationale" bind:value={appealRationale} minlength="10" maxlength="2000" rows="3" required disabled={appealQueueBusy} data-testid="scenario-appeal-rationale"></textarea>
				</label>
				<button class="btn primary" type="submit" disabled={appealQueueBusy || appealRationale.trim().length < 10} data-testid="scenario-appeal-submit">
					{appealQueueBusy ? 'Recording…' : 'Record independent decision'}
				</button>
			</form>
		</section>
	{/if}
</section>

<style>
	.rights-record {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.scenario-assessment-list {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.scenario-assessment-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		padding: var(--space-md) 0;
		border-bottom: 1px solid var(--color-surface-elevated);
	}

	.scenario-review {
		min-width: 0;
		margin-top: var(--space-lg);
		padding-top: var(--space-lg);
		border-top: 1px solid var(--color-surface-elevated);
	}

	.scenario-transcript {
		padding-left: var(--space-lg);
	}

	.scenario-transcript li {
		display: grid;
		gap: var(--space-xs);
		margin: var(--space-sm) 0;
	}
</style>
