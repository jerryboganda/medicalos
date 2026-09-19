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

	let learner = $state<LearnerChapter[]>([]);
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
{/if}
