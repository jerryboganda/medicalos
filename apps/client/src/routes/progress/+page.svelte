<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	interface LearnerChapter {
		chapter_id: string;
		chapter_name: string;
		mastery_index: number | null;
		evidence_level: string;
		independent_count: number;
	}

	interface ReviewDebt {
		due_now: number;
		completed_last_7_days: number;
		daily_rate: number | null;
		projected_backlog_days: number | null;
		note: string;
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

	let learner = $state<LearnerChapter[]>([]);
	let debt = $state<ReviewDebt | null>(null);
	let debtUnavailable = $state(false);
	let policy = $state<SelectionPolicy | null>(null);
	let policyUnavailable = $state(false);
	let loading = $state(true);

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		try {
			const today = await Api.today();
			learner = today.learner;
		} finally {
			loading = false;
		}
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
	<p class="muted">Loading your record…</p>
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
				<span>Evidence<strong style="font-size: var(--text-body-lg);">{chapter.evidence_level.replace('_', ' ')}</strong></span>
			</div>
		</div>
	{/each}

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
			<p class="muted" style="font-size: var(--text-sm);">{debt.note}</p>
		{/if}
	</div>

	<div class="card">
		<h2>How your tasks are chosen</h2>
		{#if policyUnavailable}
			<p class="muted">The policy read-out is unavailable right now.</p>
		{:else if policy}
			<p class="muted" style="font-size: var(--text-sm);">
				{policy.estimator.counted_evidence}. K rule: {policy.estimator.k_rule}.
			</p>
			<ul style="font-size: var(--text-sm);">
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
				<ul style="font-size: var(--text-sm);">
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
