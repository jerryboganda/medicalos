<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import type { ReviewDebtResponse } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	interface LearnerChapter {
		chapter_id: string;
		chapter_name: string;
		mastery_index: number | null;
		evidence_level: string;
		independent_count: number;
	}

	interface SelectionPolicy {
		estimator: {
			model: string;
			base: number;
			k_rule: string;
			counted_evidence: string;
		};
		selection_rules: string[];
		your_chapters: {
			chapter: string;
			ability: number | null;
			current_k: number;
			evidence_count: number;
		}[];
	}

	interface HeatChapter {
		chapter_id: string;
		chapter_name: string;
		ability: number | null;
		evidence_count: number | null;
		band: string;
		filtered_accuracy?: number | null;
	}

	interface HeatSystem {
		system_id: string;
		system_name: string;
		chapters: HeatChapter[];
	}

	interface Revision {
		id: string;
		to_version: number;
		reason_code: string;
		explanation: string;
		automatic: boolean;
		undone: boolean;
	}

	let revisions = $state<Revision[]>([]);
	let learner = $state<LearnerChapter[]>([]);
	let heatSystems = $state<HeatSystem[]>([]);
	let drillSystem = $state('');
	let filterDifficulty = $state('');
	let trendDays = $state('');
	let heatLoading = $state(false);
	let debt = $state<ReviewDebtResponse | null>(null);
	let debtUnavailable = $state(false);
	let policy = $state<SelectionPolicy | null>(null);
	let policyUnavailable = $state(false);
	let loading = $state(true);

	async function loadHeatmap() {
		heatLoading = true;
		try {
			const res = await Api.masteryHeatmap({
				system_id: drillSystem || undefined,
				difficulty: filterDifficulty || undefined,
				trend_days: trendDays ? Number(trendDays) : undefined
			});
			heatSystems = res.systems;
		} catch {
			heatSystems = [];
		} finally {
			heatLoading = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		try {
			const today = await Api.today();
			learner = today.learner;
			revisions = (today.revisions ?? []) as Revision[];
		} finally {
			loading = false;
		}
		await loadHeatmap();
		try {
			debt = await Api.reviewDebt();
		} catch {
			debtUnavailable = true;
		}
		try {
			policy = await Api.selectionPolicy();
		} catch {
			policyUnavailable = true;
		}
	});
</script>

<h1>Progress</h1>

{#if loading}
	<p class="muted is-loading">Loading your record…</p>
{:else if learner.length === 0}
	<div class="card">
		<p class="muted">
			No evidence yet. Answer questions in Practice and your chapters appear
			here with observed accuracy — never invented numbers.
		</p>
		<a class="btn primary" href={`${base}/today`}>Go to Today</a>
	</div>
{:else}
	{#each learner as chapter (chapter.chapter_id)}
		<div class="card">
			<strong>{chapter.chapter_name}</strong>
			<div class="stat-row">
				<span>Independent answers<strong>{chapter.independent_count}</strong></span>
				<span>
					Observed accuracy<strong>
						{chapter.mastery_index === null ? '—' : `${chapter.mastery_index}%`}
					</strong>
				</span>
				<span>Evidence<strong class="evidence-level">{chapter.evidence_level.replace('_', ' ')}</strong></span>
			</div>
		</div>
	{/each}

	<div class="card">
		<h2>Mastery map</h2>
		<p class="muted small">
			Bands come from your real ability estimates (config thresholds:
			weak / developing / strong). Drill into one system, restrict to a
			difficulty, or add a trend window — every number here traces to
			attempts.
		</p>
		<div class="form-row">
			<label class="field" for="heat-system">
				<span>System</span>
				<select id="heat-system" bind:value={drillSystem} onchange={loadHeatmap}>
					<option value="">All systems</option>
					{#each heatSystems as s (s.system_id)}
						<option value={s.system_id}>{s.system_name}</option>
					{/each}
				</select>
			</label>
			<label class="field" for="heat-difficulty">
				<span>Difficulty</span>
				<select
					id="heat-difficulty"
					bind:value={filterDifficulty}
					onchange={loadHeatmap}
				>
					<option value="">Any</option>
					<option value="easy">Easy</option>
					<option value="medium">Medium</option>
					<option value="hard">Hard</option>
				</select>
			</label>
			<label class="field" for="heat-trend">
				<span>Trend window (days)</span>
				<input
					id="heat-trend"
					type="number"
					min="1"
					max="365"
					bind:value={trendDays}
					onchange={loadHeatmap}
				/>
			</label>
			<button
				class="btn"
				type="button"
				disabled={heatLoading}
				onclick={loadHeatmap}
			>
				{heatLoading ? 'Loading…' : 'Apply'}
			</button>
		</div>
		{#if heatSystems.length === 0}
			<p class="muted">
				{heatLoading
					? 'Loading…'
					: 'No curriculum systems to map yet — they appear with the exam content.'}
			</p>
		{:else}
			{#each heatSystems as sys (sys.system_id)}
				<h3>{sys.system_name}</h3>
				<ul class="bare-list band-list small">
					{#each sys.chapters as ch (ch.chapter_id)}
						<li>
							{ch.chapter_name} — <strong class="band band-{ch.band}">{ch.band}</strong>
							{#if ch.evidence_count !== null}, {ch.evidence_count} answer(s){/if}
							{#if ch.filtered_accuracy !== undefined && ch.filtered_accuracy !== null}
								· accuracy at this filter: {ch.filtered_accuracy}%
							{/if}
						</li>
					{/each}
				</ul>
			{/each}
		{/if}
	</div>

	<div class="card">
		<h2>Plan timeline</h2>
		<p class="muted small">
			Every revision of today's plan, oldest rule first: why it changed,
			whether the agent or you made it, and whether it was undone.
		</p>
		{#if revisions.length === 0}
			<p class="muted">No revisions today — the plan is unchanged.</p>
		{:else}
			<ol class="timeline small">
				{#each revisions as r (r.id)}
					<li data-testid="plan-revision">
						<strong>v{r.to_version}</strong>
						· {r.reason_code.replace('_', ' ')}
						· {r.automatic ? 'agent' : 'you'}
						{#if r.undone}
							· <span class="muted">undone</span>
						{/if}
						<br />
						<span class="muted">{r.explanation}</span>
					</li>
				{/each}
			</ol>
		{/if}
	</div>

	<div class="card">
		<h2>Review debt</h2>
		{#if debtUnavailable}
			<p class="muted">Debt numbers are unavailable right now.</p>
		{:else if debt}
			<div class="stat-row">
				<span>Due now<strong>{debt.due_now}</strong></span>
				<span>
					Cleared last 7 days<strong>{debt.completed_last_7_days}</strong>
				</span>
				<span>
					Projected backlog<strong>
						{debt.projected_backlog_days === null
							? '—'
							: `${debt.projected_backlog_days} day(s)`}
					</strong></span
				>
			</div>
			<p class="muted small">{debt.note}</p>
		{/if}
	</div>

	<div class="card">
		<h2>How your tasks are chosen</h2>
		{#if policyUnavailable}
			<p class="muted">The policy read-out is unavailable right now.</p>
		{:else if policy}
			<p class="muted small">
				{policy.estimator.counted_evidence}. K rule: {policy.estimator.k_rule}.
			</p>
			<ul class="small">
				{#each policy.selection_rules as rule (rule)}
					<li>{rule}</li>
				{/each}
			</ul>
			{#if policy.your_chapters.length === 0}
				<p class="muted">
					No ability estimates yet — answer questions in a chapter and the
					estimator's state appears here.
				</p>
			{:else}
				<ul class="small">
					{#each policy.your_chapters as ch (ch.chapter)}
						<li>
							{ch.chapter}: ability {ch.ability === null
								? '—'
								: ch.ability.toFixed(0)}, K {ch.current_k.toFixed(1)} over
							{ch.evidence_count} answer(s)
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</div>
{/if}

<svelte:head>
	<title>Progress | Medical Learning OS</title>
</svelte:head>

<style>
	.evidence-level {
		font-size: var(--text-body-lg);
		text-transform: capitalize;
	}

	.band-list {
		display: grid;
		gap: var(--space-sm);
		margin-bottom: var(--space-lg);
	}

	.band {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 2px var(--space-sm);
		border-radius: var(--radius-pill);
		border: 1px solid var(--color-border-strong);
		font-weight: 700;
		text-transform: capitalize;
	}

	.band::before {
		content: '';
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.band-weak {
		color: var(--color-error);
		border-color: color-mix(in oklab, var(--color-error) 50%, transparent);
	}

	.band-developing {
		color: var(--color-warning);
		border-color: color-mix(in oklab, var(--color-warning) 50%, transparent);
	}

	.band-strong {
		color: var(--color-success);
		border-color: color-mix(in oklab, var(--color-success) 50%, transparent);
	}

	.timeline {
		position: relative;
		margin: 0;
		padding: 0 0 0 var(--space-xl);
		list-style: none;
	}

	.timeline::before {
		content: '';
		position: absolute;
		left: 7px;
		top: 6px;
		bottom: 6px;
		width: 2px;
		border-radius: var(--radius-pill);
		background: var(--color-border-strong);
	}

	.timeline li {
		position: relative;
		padding-bottom: var(--space-md);
	}

	.timeline li::before {
		content: '';
		position: absolute;
		left: calc(-1 * var(--space-xl) + 3px);
		top: 6px;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--color-surface);
		box-shadow: inset 0 0 0 2px var(--color-accent);
	}
</style>
