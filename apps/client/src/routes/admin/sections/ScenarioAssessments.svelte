<script lang="ts">
	// Examiner review of finished clinical scenario stations: rubric scores must cite transcript
	// events. No requirement ID cited inline; e2e: sim04-examiner.spec.ts.
	import { onMount } from 'svelte';
	import {
		Api,
		ApiError,
		type AdminScenarioAssessment,
		type PendingScenarioAssessment
	} from '$lib/api';

	let { refreshAudit }: { refreshAudit: () => Promise<void> } = $props();

	let pendingAssessments = $state<PendingScenarioAssessment[]>([]);
	let pendingAssessmentBusy = $state(false);
	let pendingAssessmentError = $state('');
	let pendingAssessmentMessage = $state('');
	let selectedAssessment = $state<AdminScenarioAssessment | null>(null);
	let assessmentDrafts = $state<Record<string, ScenarioAssessmentDraft>>({});
	let assessmentSaving = $state(false);
	type ScenarioAssessmentDraft = {
		status: 'assessed' | 'not_assessed';
		score: string;
		evidence: string;
		eventIndexes: number[];
		uncertain: boolean;
	};

	async function loadPendingAssessments() {
		pendingAssessmentBusy = true;
		pendingAssessmentError = '';
		try {
			pendingAssessments = (await Api.pendingScenarioAssessments()).runs;
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Assessment queue could not be loaded.';
		} finally {
			pendingAssessmentBusy = false;
		}
	}

	async function openScenarioAssessment(runId: string) {
		pendingAssessmentBusy = true;
		pendingAssessmentError = '';
		pendingAssessmentMessage = '';
		selectedAssessment = null;
		try {
			const assessment = await Api.getScenarioAssessment(runId);
			selectedAssessment = assessment;
			assessmentDrafts = Object.fromEntries(
				assessment.rubric.map((criterion) => [criterion.criterion_key, {
					status: 'not_assessed',
					score: '',
					evidence: '',
					eventIndexes: [],
					uncertain: false
				} satisfies ScenarioAssessmentDraft])
			);
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Scenario assessment could not be opened.';
		} finally {
			pendingAssessmentBusy = false;
		}
	}

	function updateScenarioAssessmentDraft(
		criterionKey: string,
		patch: Partial<ScenarioAssessmentDraft>
	) {
		const current = assessmentDrafts[criterionKey];
		if (!current) return;
		assessmentDrafts = {
			...assessmentDrafts,
			[criterionKey]: { ...current, ...patch }
		};
	}

	function toggleScenarioAssessmentEvent(criterionKey: string, index: number, checked: boolean) {
		const current = assessmentDrafts[criterionKey];
		if (!current) return;
		const indexes = new Set(current.eventIndexes);
		if (checked) indexes.add(index);
		else indexes.delete(index);
		updateScenarioAssessmentDraft(criterionKey, { eventIndexes: [...indexes].sort((a, b) => a - b) });
	}

	async function recordScenarioAssessment(event: Event) {
		event.preventDefault();
		if (!selectedAssessment || assessmentSaving) return;
		assessmentSaving = true;
		pendingAssessmentError = '';
		pendingAssessmentMessage = '';
		try {
			const result = await Api.recordScenarioAssessment(selectedAssessment.run_id, {
				criteria: selectedAssessment.rubric.map((criterion) => {
					const draft = assessmentDrafts[criterion.criterion_key];
					const assessed = draft.status === 'assessed';
					return {
						criterion_key: criterion.criterion_key,
						assessment_status: draft.status,
						score: assessed && draft.score.trim() ? Number(draft.score) : null,
						evidence: draft.evidence.trim(),
						transcript_event_indexes: assessed ? draft.eventIndexes : [],
						transcript_uncertain: assessed && draft.uncertain
					};
				})
			});
			pendingAssessmentMessage = `Recorded ${result.recorded_criteria} criterion results (${result.not_assessed} not assessed).`;
			selectedAssessment = null;
			assessmentDrafts = {};
			await Promise.all([loadPendingAssessments(), refreshAudit()]);
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Scenario assessment could not be recorded.';
		} finally {
			assessmentSaving = false;
		}
	}

	onMount(loadPendingAssessments);
</script>

<section class="card" aria-labelledby="scenario-assessments-heading" data-testid="scenario-assessments">
	<h2 id="scenario-assessments-heading">Clinical scenario assessments</h2>
	<p class="muted">
		Review finished stations against their versioned rubric. Scores must cite observed transcript
		events; mark a criterion not assessed when the evidence is absent. This records examiner
		judgment and does not validate clinical competence.
	</p>
	{#if pendingAssessmentBusy && pendingAssessments.length === 0}
		<p class="muted is-loading" role="status">Loading finished stations…</p>
	{:else if pendingAssessmentError}
		<p class="error-text" role="alert">{pendingAssessmentError}</p>
	{:else if pendingAssessments.length === 0}
		<p class="muted">No finished stations are waiting for assessment.</p>
	{:else}
		<ul class="scenario-assessment-list">
			{#each pendingAssessments as run (run.run_id)}
				<li class="scenario-assessment-row" data-testid={`pending-scenario-${run.run_id}`}>
					<div>
						<strong>{run.scenario}</strong>
						<p class="muted">Version {run.scenario_version} · {run.criterion_count} criteria · finished {new Date(run.finished_at).toLocaleString()}</p>
					</div>
					<button class="btn" type="button" disabled={pendingAssessmentBusy} onclick={() => openScenarioAssessment(run.run_id)} data-testid={`scenario-assessment-open-${run.run_id}`}>
						{pendingAssessmentBusy ? 'Opening…' : 'Review station'}
					</button>
				</li>
			{/each}
		</ul>
	{/if}
	{#if pendingAssessmentMessage}
		<p class="muted" role="status" data-testid="scenario-assessment-message">{pendingAssessmentMessage}</p>
	{/if}
	<button class="btn" type="button" disabled={pendingAssessmentBusy} onclick={loadPendingAssessments}>Refresh assessment queue</button>

	{#if selectedAssessment}
		<section class="scenario-review" aria-labelledby="scenario-review-heading">
			<h3 id="scenario-review-heading">{selectedAssessment.scenario} · version {selectedAssessment.scenario_version}</h3>
			<p class="muted">Run {selectedAssessment.run_id} · finished {new Date(selectedAssessment.finished_at).toLocaleString()}</p>
			<h4>Immutable station transcript</h4>
			<ol class="scenario-transcript">
				{#each selectedAssessment.transcript as event, index (index)}
					<li><strong>Event {index + 1}: {event.on}</strong><span class="muted">{event.from} → {event.to}{event.actor_role ? ` · ${event.actor_role.replaceAll('_', ' ')}` : ''}</span></li>
				{/each}
			</ol>
			<form onsubmit={recordScenarioAssessment}>
				{#each selectedAssessment.rubric as criterion (criterion.criterion_key)}
					{@const draft = assessmentDrafts[criterion.criterion_key]}
					<fieldset class="scenario-assessment-criterion" disabled={assessmentSaving}>
						<legend>{criterion.label} · max {criterion.max_score}</legend>
						<label class="field" for={`scenario-status-${criterion.criterion_key}`}>
							<span>Assessment for {criterion.criterion_key}</span>
							<select id={`scenario-status-${criterion.criterion_key}`} value={draft.status} onchange={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { status: event.currentTarget.value as ScenarioAssessmentDraft['status'] })}>
								<option value="assessed">Assessed from transcript</option>
								<option value="not_assessed">Not assessed</option>
							</select>
						</label>
						{#if draft.status === 'assessed'}
							<label class="field" for={`scenario-score-${criterion.criterion_key}`}>
								<span>Score for {criterion.criterion_key}</span>
								<input id={`scenario-score-${criterion.criterion_key}`} type="number" min="0" max={criterion.max_score} step="any" value={draft.score} oninput={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { score: event.currentTarget.value })} required />
							</label>
							<p class="muted">Select the transcript events that support this score.</p>
							{#each selectedAssessment.transcript as transcriptEvent, index (index)}
								<label class="field" for={`scenario-event-${criterion.criterion_key}-${index}`}>
									<input id={`scenario-event-${criterion.criterion_key}-${index}`} type="checkbox" checked={draft.eventIndexes.includes(index)} onchange={(event) => toggleScenarioAssessmentEvent(criterion.criterion_key, index, event.currentTarget.checked)} />
									<span>Use event {index + 1}: {transcriptEvent.on}</span>
								</label>
							{/each}
							<label class="field" for={`scenario-uncertain-${criterion.criterion_key}`}>
								<input id={`scenario-uncertain-${criterion.criterion_key}`} type="checkbox" checked={draft.uncertain} onchange={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { uncertain: event.currentTarget.checked })} />
								<span>Transcript evidence is uncertain</span>
							</label>
						{/if}
						<label class="field" for={`scenario-evidence-${criterion.criterion_key}`}>
							<span>Evidence or not-assessed reason for {criterion.criterion_key}</span>
							<textarea id={`scenario-evidence-${criterion.criterion_key}`} value={draft.evidence} oninput={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { evidence: event.currentTarget.value })} minlength="1" maxlength="2000" rows="2" required></textarea>
						</label>
					</fieldset>
				{/each}
				<button class="btn primary" type="submit" disabled={assessmentSaving || selectedAssessment.rubric.some((criterion) => !assessmentDrafts[criterion.criterion_key]?.evidence.trim())} data-testid="scenario-assessment-record">
					{assessmentSaving ? 'Recording…' : 'Record examiner assessment'}
				</button>
			</form>
		</section>
	{/if}
</section>

<style>
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

	.scenario-assessment-criterion {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}
</style>
