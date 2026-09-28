<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError, type ScenarioSummary } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let scenarios = $state<ScenarioSummary[]>([]);
	let busy = $state(false);
	let startingSlug = $state('');
	let error = $state('');

	async function loadScenarios() {
		busy = true;
		error = '';
		try {
			scenarios = (await Api.listScenarios()).scenarios;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Published stations could not be loaded.';
		} finally {
			busy = false;
		}
	}

	async function startScenario(scenario: ScenarioSummary) {
		if (startingSlug) return;
		startingSlug = scenario.slug;
		error = '';
		try {
			const run = await Api.startScenario(scenario.slug);
			await goto(`${base}/scenarios/runs/${encodeURIComponent(run.run_id)}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'The station could not be started.';
		} finally {
			startingSlug = '';
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			await goto(`${base}/login`);
			return;
		}
		await loadScenarios();
	});
</script>

<svelte:head>
	<title>Clinical practice stations · Medical Learning OS</title>
</svelte:head>

<h1>Clinical practice stations</h1>
<p class="muted">
	These stations follow authored, deterministic text actions. They do not generate patient dialogue
	or validate clinical competence.
</p>

{#if busy}
	<p class="muted is-loading" role="status">Loading published stations…</p>
{:else if error}
	<p class="error-text" role="alert">{error}</p>
	<button class="btn" type="button" onclick={loadScenarios}>Retry</button>
{:else if scenarios.length === 0}
	<section class="station-empty" aria-labelledby="station-empty-heading">
		<h2 id="station-empty-heading">No published stations yet</h2>
		<p class="muted">A station appears here after an editor publishes an authored scenario version.</p>
	</section>
{:else}
	<ul class="station-list">
		{#each scenarios as scenario (scenario.slug)}
			<li class="station-item" data-testid={`scenario-${scenario.slug}`}>
				<div>
					<h2>{scenario.title}</h2>
					<p class="muted">Published script · version {scenario.version}</p>
				</div>
				<button
					class="btn primary"
					type="button"
					disabled={!!startingSlug}
					onclick={() => startScenario(scenario)}
					data-testid={`scenario-start-${scenario.slug}`}
				>
					{startingSlug === scenario.slug ? 'Starting…' : 'Start station'}
				</button>
			</li>
		{/each}
	</ul>
{/if}

{#if error && !busy && scenarios.length > 0}
	<p class="error-text" role="alert">{error}</p>
{/if}

<style>
	.station-list {
		list-style: none;
		margin: var(--space-lg) 0;
		padding: 0;
	}

	.station-item,
	.station-empty {
		min-width: 0;
		padding: var(--space-lg) 0;
		border-top: 1px solid var(--color-surface-elevated);
	}

	.station-item {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
	}

	.station-item h2 {
		margin: 0;
	}

	.station-item p {
		margin-bottom: 0;
	}

	.station-empty {
		margin-top: var(--space-xl);
	}
</style>
