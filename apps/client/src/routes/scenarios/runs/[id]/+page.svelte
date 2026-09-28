<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import {
		Api,
		ApiError,
		type ScenarioCounterfactualReplay,
		type ScenarioDebrief,
		type ScenarioHandover,
		type ScenarioRun,
		type ScenarioTeam,
		type ScenarioTeamRole
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let { data } = $props();
	const runId: string = data.id;
	let run = $state<ScenarioRun | null>(null);
	let debrief = $state<ScenarioDebrief | null>(null);
	let team = $state<ScenarioTeam | null>(null);
	let handovers = $state<ScenarioHandover[]>([]);
	let replay = $state<ScenarioCounterfactualReplay | null>(null);
	let replayEvents = $state('');
	let inviteRole = $state<Exclude<ScenarioTeamRole, 'team_lead'>>('history_taker');
	let inviteCode = $state('');
	let inviteExpiresAt = $state('');
	let inviteBusy = $state(false);
	let handoverRecipient = $state('');
	let handoverSituation = $state('');
	let handoverBackground = $state('');
	let handoverAssessment = $state('');
	let handoverRecommendation = $state('');
	let handoverBusy = $state(false);
	let handoverError = $state('');
	let acknowledgingHandover = $state('');
	let appealReason = $state('');
	let busy = $state(false);
	let appealBusy = $state(false);
	let error = $state('');
	let replayError = $state('');
	let appealError = $state('');

	async function loadRun() {
		busy = true;
		error = '';
		try {
			const [loadedRun, loadedTeam, loadedHandovers] = await Promise.all([
				Api.getScenarioRun(runId),
				Api.getScenarioTeam(runId),
				Api.listScenarioHandovers(runId)
			]);
			run = loadedRun;
			team = loadedTeam;
			handovers = loadedHandovers.handovers;
			if (run.finished) debrief = await Api.getScenarioDebrief(runId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'This station could not be loaded.';
		} finally {
			busy = false;
		}
	}

	async function createTeamInvite(event: Event) {
		event.preventDefault();
		if (!team || team.current_role !== 'team_lead' || inviteBusy) return;
		inviteBusy = true;
		error = '';
		inviteCode = '';
		try {
			const invite = await Api.createScenarioTeamInvite(runId, inviteRole);
			inviteCode = invite.invite_code;
			inviteExpiresAt = invite.expires_at;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'The team invitation could not be created.';
		} finally {
			inviteBusy = false;
		}
	}

	async function createHandover(event: Event) {
		event.preventDefault();
		if (!team || handoverBusy || !handoverRecipient) return;
		handoverBusy = true;
		handoverError = '';
		try {
			await Api.createScenarioHandover(runId, {
				recipient_member_id: handoverRecipient,
				situation: handoverSituation,
				background: handoverBackground,
				assessment: handoverAssessment,
				recommendation: handoverRecommendation
			});
			handoverSituation = '';
			handoverBackground = '';
			handoverAssessment = '';
			handoverRecommendation = '';
			handovers = (await Api.listScenarioHandovers(runId)).handovers;
		} catch (err) {
			handoverError = err instanceof ApiError ? err.message : 'The handover could not be recorded.';
		} finally {
			handoverBusy = false;
		}
	}

	async function acknowledgeHandover(handoverId: string) {
		if (acknowledgingHandover) return;
		acknowledgingHandover = handoverId;
		handoverError = '';
		try {
			await Api.acknowledgeScenarioHandover(runId, handoverId);
			handovers = (await Api.listScenarioHandovers(runId)).handovers;
		} catch (err) {
			handoverError = err instanceof ApiError ? err.message : 'The handover could not be acknowledged.';
		} finally {
			acknowledgingHandover = '';
		}
	}

	async function chooseAction(action: string) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			await Api.advanceScenario(runId, action);
			await loadRun();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'The station action could not be recorded.';
		} finally {
			busy = false;
		}
	}

	async function replayAlternative(event: Event) {
		event.preventDefault();
		if (!debrief || busy) return;
		busy = true;
		replayError = '';
		replay = null;
		try {
			replay = await Api.replayScenario(
				runId,
				replayEvents.split(/\r?\n/).map((action) => action.trim()).filter(Boolean)
			);
		} catch (err) {
			replayError = err instanceof ApiError ? err.message : 'The alternate sequence could not be replayed.';
		} finally {
			busy = false;
		}
	}

	async function submitAssessmentAppeal(event: Event) {
		event.preventDefault();
		if (!debrief || appealBusy || appealReason.trim().length < 10) return;
		appealBusy = true;
		appealError = '';
		try {
			await Api.appealScenarioAssessment(runId, { reason: appealReason.trim() });
			debrief = await Api.getScenarioDebrief(runId);
			appealReason = '';
		} catch (err) {
			appealError = err instanceof ApiError ? err.message : 'The assessment appeal could not be submitted.';
		} finally {
			appealBusy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			await goto(`${base}/login`);
			return;
		}
		await loadRun();
	});
</script>

<svelte:head>
	<title>Station debrief · Medical Learning OS</title>
</svelte:head>

<p><a href={`${base}/scenarios`}>← Published stations</a></p>
{#if busy && !run}
	<p class="muted is-loading" role="status">Loading station…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={loadRun}>Retry</button>
{:else if run}
	<h1>{run.scenario}</h1>
	<p class="muted">Version {run.scenario_version} · state: {run.current_state}</p>
	{#if !run.finished}
		<section class="team-panel" aria-labelledby="scenario-team-heading" data-testid="scenario-team">
			<div class="team-heading">
				<div>
					<h2 id="scenario-team-heading">Case team</h2>
					<p class="muted">Fictional simulation · formative learning only</p>
				</div>
				<span class="chip">Your role: {team?.current_role?.replaceAll('_', ' ')}</span>
			</div>
			<ul class="team-members" aria-label="Team members">
				{#each team?.members ?? [] as member (member.member_id)}
					<li>{member.role.replaceAll('_', ' ')}</li>
				{/each}
			</ul>
			{#if team?.current_role === 'team_lead'}
				<form class="team-invite" onsubmit={createTeamInvite}>
					<label class="field" for="scenario-invite-role">
						<span>Invite role</span>
						<select id="scenario-invite-role" bind:value={inviteRole} disabled={inviteBusy}>
							<option value="history_taker">History taker</option>
							<option value="scribe">Scribe</option>
							<option value="observer">Observer</option>
						</select>
					</label>
					<button class="btn" type="submit" disabled={inviteBusy} data-testid="scenario-team-invite">
						{inviteBusy ? 'Creating…' : 'Create invitation'}
					</button>
				</form>
				{#if inviteCode}
					<p class="invite-code" role="status" data-testid="scenario-team-invite-code">
						Share this one-use code with the intended teammate: <code>{inviteCode}</code>
						<span class="muted">Expires {new Date(inviteExpiresAt).toLocaleString()} · join at <a href={`${base}/scenarios/team`}>team invite</a></span>
					</p>
				{/if}
			{/if}
			<p class="muted">Observers can follow the transcript but cannot advance the case. Do not enter real patient information.</p>
		</section>
	{/if}
	{#if !run.finished}
		<section aria-labelledby="station-action-heading">
			<h2 id="station-action-heading">Choose an authored action</h2>
			<p class="muted">Each action follows the published state machine and is added to your transcript.</p>
			{#if run.available_actions.length === 0}
				<p class="muted">No action is available from this state. The station remains open for an editor to review.</p>
			{:else}
				<div class="actions">
					{#each run.available_actions as action (action)}
						<button class="btn" type="button" disabled={busy || team?.current_role === 'observer'} onclick={() => chooseAction(action)} data-testid={`scenario-action-${action}`}>
							{action.replaceAll('_', ' ')}
						</button>
					{/each}
				</div>
			{/if}
		</section>
	{/if}

	{#if team}
		<section class="team-panel" aria-labelledby="handover-heading" data-testid="scenario-handovers">
			<h2 id="handover-heading">Team handover</h2>
			<p class="muted">Record a simulation-only SBAR handover. Each section is preserved with the case and visible to its invited team.</p>
			{#if !run.finished && team.members.some((member) => member.member_id !== team?.current_member_id)}
				<form onsubmit={createHandover}>
					<label class="field" for="handover-recipient">
						<span>Send to role</span>
						<select id="handover-recipient" bind:value={handoverRecipient} required disabled={handoverBusy}>
							<option value="" disabled>Select teammate</option>
							{#each team.members.filter((member) => member.member_id !== team?.current_member_id) as member (member.member_id)}
								<option value={member.member_id}>{member.role.replaceAll('_', ' ')}</option>
							{/each}
						</select>
					</label>
					<label class="field" for="handover-situation"><span>Situation</span><textarea id="handover-situation" bind:value={handoverSituation} minlength="1" maxlength="2000" rows="2" required disabled={handoverBusy}></textarea></label>
					<label class="field" for="handover-background"><span>Background</span><textarea id="handover-background" bind:value={handoverBackground} minlength="1" maxlength="2000" rows="2" required disabled={handoverBusy}></textarea></label>
					<label class="field" for="handover-assessment"><span>Assessment</span><textarea id="handover-assessment" bind:value={handoverAssessment} minlength="1" maxlength="2000" rows="2" required disabled={handoverBusy}></textarea></label>
					<label class="field" for="handover-recommendation"><span>Recommendation</span><textarea id="handover-recommendation" bind:value={handoverRecommendation} minlength="1" maxlength="2000" rows="2" required disabled={handoverBusy}></textarea></label>
					{#if handoverError}<p class="error-text" role="alert">{handoverError}</p>{/if}
					<button class="btn" type="submit" disabled={handoverBusy || !handoverRecipient || !handoverSituation.trim() || !handoverBackground.trim() || !handoverAssessment.trim() || !handoverRecommendation.trim()} data-testid="scenario-handover-submit">{handoverBusy ? 'Saving…' : 'Record handover'}</button>
				</form>
			{/if}
			{#if handovers.length > 0}
				<ul class="handover-list">
					{#each handovers as handover (handover.handover_id)}
						<li class="handover-item" data-testid={`scenario-handover-${handover.handover_id}`}>
							<p class="muted">{handover.from_role.replaceAll('_', ' ')} → {handover.to_role.replaceAll('_', ' ')} · {new Date(handover.created_at).toLocaleString()}</p>
							<dl>
								<dt>Situation</dt><dd>{handover.situation}</dd>
								<dt>Background</dt><dd>{handover.background}</dd>
								<dt>Assessment</dt><dd>{handover.assessment}</dd>
								<dt>Recommendation</dt><dd>{handover.recommendation}</dd>
							</dl>
							{#if handover.acknowledged}
								<p class="muted" role="status">Handover acknowledged</p>
							{:else if handover.can_ack}
								<button class="btn" type="button" disabled={acknowledgingHandover === handover.handover_id} onclick={() => acknowledgeHandover(handover.handover_id)} data-testid={`scenario-handover-ack-${handover.handover_id}`}>
									{acknowledgingHandover === handover.handover_id ? 'Acknowledging…' : 'Acknowledge handover'}
								</button>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}

	{#if debrief}
		<section aria-labelledby="station-timeline-heading">
			<h2 id="station-timeline-heading">Debrief timeline</h2>
			{#if debrief.timeline.length === 0}
				<p class="muted">No actions were recorded.</p>
			{:else}
				<ol class="timeline" data-testid="scenario-timeline">
					{#each debrief.timeline as item (item.index)}
						<li>
							<strong>Event {item.sequence}: {item.on?.replaceAll('_', ' ')}</strong>
							<span class="muted">{item.from} → {item.to}{item.actor_role ? ` · ${item.actor_role.replaceAll('_', ' ')}` : ''}</span>
						</li>
					{/each}
				</ol>
			{/if}
		</section>

		{#if debrief.transcript_corrections.length > 0}
			<section class="transcript-corrections" aria-labelledby="transcript-corrections-heading" data-testid="scenario-transcript-corrections">
				<h2 id="transcript-corrections-heading">Transcript corrections</h2>
				<p class="muted">These corrections add context. The original transcript and timeline remain unchanged.</p>
				<ol>
					{#each debrief.transcript_corrections as correction (correction.event_index)}
						<li>
							<strong>Event {correction.event_index + 1}</strong>
							<p><span class="muted">Original:</span> {correction.original_event.replaceAll('_', ' ')}</p>
							<p><span class="muted">Corrected:</span> {correction.corrected_text}</p>
						</li>
					{/each}
				</ol>
			</section>
		{/if}

		<section aria-labelledby="station-rubric-heading">
			<h2 id="station-rubric-heading">Examiner rubric</h2>
			<p class="muted">This supervised learning record does not establish clinical competence or authorize independent practice.</p>
			{#if debrief.rubric.length === 0}
				<p class="muted">This station version has no configured rubric.</p>
			{:else}
				<ul class="rubric-list">
					{#each debrief.rubric as criterion (criterion.criterion_key)}
						<li>
							<strong>{criterion.label}</strong>
							<span class="chip">{criterion.assessment_status.replaceAll('_', ' ')}</span>
							{#if criterion.assessment_status === 'assessed'}
								<p>Score: {criterion.score} / {criterion.max_score}</p>
								<p>{criterion.evidence}</p>
								<p class="muted">Evidence events: {criterion.transcript_event_indexes.map((index) => index + 1).join(', ') || 'none'}{criterion.transcript_uncertain ? ' · transcript uncertain' : ''}</p>
							{:else}
								<p class="muted">{criterion.evidence || 'No examiner result has been recorded.'}</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		{#if debrief.appeal}
			<section aria-labelledby="assessment-appeal-heading" data-testid="scenario-assessment-appeal">
				<h2 id="assessment-appeal-heading">Assessment appeal</h2>
				<p class="chip" data-testid="scenario-appeal-status">
					{#if debrief.appeal.status === 'open'}
						Awaiting independent review
					{:else if debrief.appeal.decision === 'reassessment_required'}
						New independent assessment required
					{:else}
						Original assessment confirmed
					{/if}
				</p>
				<p>{debrief.appeal.reason}</p>
				{#if debrief.appeal.rationale}
					<h3>Reviewer’s rationale</h3>
					<p>{debrief.appeal.rationale}</p>
				{/if}
				{#if debrief.appeal.decision === 'reassessment_required'}
					<p class="muted" data-testid="consequential-use-status">The original examiner score is preserved and is unsuitable for consequential use until a new independent assessment is completed.</p>
				{/if}
			</section>
		{:else if debrief.rubric.some((criterion) => criterion.evidence !== null && criterion.evidence !== undefined)}
			<section aria-labelledby="assessment-appeal-heading" data-testid="scenario-assessment-appeal">
				<h2 id="assessment-appeal-heading">Appeal this assessment</h2>
				<p class="muted">A different administrator will review your reason and the original station evidence. The original examiner assessment remains unchanged.</p>
				<form onsubmit={submitAssessmentAppeal}>
					<label class="field" for="assessment-appeal-reason">
						<span>Why are you appealing? (10–2000 characters)</span>
						<textarea id="assessment-appeal-reason" bind:value={appealReason} minlength="10" maxlength="2000" rows="4" required disabled={appealBusy} data-testid="scenario-appeal-reason-input"></textarea>
					</label>
					{#if appealError}<p class="error-text" role="alert">{appealError}</p>{/if}
					<button class="btn" type="submit" disabled={appealBusy || appealReason.trim().length < 10} data-testid="scenario-appeal-submit">
						{appealBusy ? 'Submitting…' : 'Submit appeal'}
					</button>
				</form>
			</section>
		{/if}

		<section aria-labelledby="counterfactual-heading">
			<h2 id="counterfactual-heading">Try an alternate path</h2>
			<p class="muted">Read-only replay uses this run’s version and does not change its transcript or score.</p>
			<p class="muted">Authored actions: {debrief.available_actions.join(', ') || 'none'}</p>
			<form onsubmit={replayAlternative}>
				<label class="field" for="counterfactual-events">
					<span>Alternate actions, one per line</span>
					<textarea id="counterfactual-events" bind:value={replayEvents} rows="3" maxlength="12000" required data-testid="counterfactual-events"></textarea>
				</label>
				<button class="btn" type="submit" disabled={busy || !replayEvents.trim()} data-testid="counterfactual-submit">{busy ? 'Replaying…' : 'Replay path'}</button>
			</form>
			{#if replayError}<p class="error-text" role="alert">{replayError}</p>{/if}
			{#if replay}
				<div class="alternate-result" role="status" data-testid="counterfactual-result">
					<h3>Alternate timeline · {replay.terminal ? 'terminal' : 'station remains open'}</h3>
					<ol class="timeline">
						{#each replay.counterfactual_timeline as item (item.index)}
							<li><strong>Event {item.sequence}: {item.on?.replaceAll('_', ' ')}</strong><span class="muted">{item.from} → {item.to}</span></li>
						{/each}
					</ol>
					<p>Final state: {replay.final_state}</p>
				</div>
			{/if}
		</section>
	{/if}
{/if}

<style>
	/* Hallmark · macrostructure: single-column debrief flow · tone: calm utilitarian · anchor hue: violet · genre: atmospheric */
	/* Hallmark · pre-emit critique: P4 H4 E4 S5 R5 V3 */
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
	}

	.timeline,
	.rubric-list {
		padding-left: var(--space-lg);
	}

	.transcript-corrections {
		margin-block: var(--space-lg);
	}

	.transcript-corrections li {
		margin-block: var(--space-md);
	}

	.transcript-corrections p {
		margin-block: var(--space-xs);
		overflow-wrap: anywhere;
	}

	.timeline li,
	.rubric-list li {
		margin: var(--space-md) 0;
	}

	.timeline li {
		display: grid;
		gap: var(--space-xs);
	}

	.rubric-list li {
		padding-bottom: var(--space-sm);
		border-bottom: 1px solid var(--color-surface-elevated);
	}

	.alternate-result {
		margin-top: var(--space-lg);
		padding: var(--space-md);
		border-left: 3px solid var(--color-accent);
	}

	.team-panel {
		min-width: 0;
		margin-block: var(--space-lg);
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.team-heading,
	.team-invite {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
	}

	.team-heading h2 {
		margin-bottom: var(--space-xs);
	}

	.team-members {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
		padding: 0;
		list-style: none;
	}

	.team-members li {
		padding: var(--space-xs) var(--space-sm);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.invite-code,
	.handover-item {
		min-width: 0;
		margin-top: var(--space-md);
		padding: var(--space-md);
		border-left: 3px solid var(--color-accent);
		background: var(--color-surface-elevated);
	}

	.invite-code code {
		overflow-wrap: anywhere;
	}

	.invite-code span {
		display: block;
	}

	.handover-list {
		padding: 0;
		list-style: none;
	}

	.handover-item dl {
		display: grid;
		grid-template-columns: minmax(7rem, auto) minmax(0, 1fr);
		gap: var(--space-xs) var(--space-md);
		margin-block: var(--space-md);
	}

	.handover-item dd {
		min-width: 0;
		margin: 0;
		overflow-wrap: anywhere;
	}

	@media (max-width: 480px) {
		.handover-item dl {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
