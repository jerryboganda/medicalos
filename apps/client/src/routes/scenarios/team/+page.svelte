<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	let inviteCode = $state('');
	let busy = $state(false);
	let error = $state('');

	async function joinTeam(event: Event) {
		event.preventDefault();
		if (busy || !inviteCode.trim()) return;
		busy = true;
		error = '';
		try {
			const team = await Api.joinScenarioTeam(inviteCode.trim());
			await goto(`${base}/scenarios/runs/${encodeURIComponent(team.run_id)}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'The team invitation could not be accepted.';
		} finally {
			busy = false;
		}
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) {
			void goto(`${base}/login`);
			return;
		}
		inviteCode = new URLSearchParams(window.location.search).get('code') ?? '';
	});
</script>

<svelte:head>
	<title>Join a simulation team · Medical Learning OS</title>
</svelte:head>

<p><a href={`${base}/scenarios`}>← Published stations</a></p>
<main class="join-team">
	<p class="eyebrow">Fictional simulation · formative learning</p>
	<h1>Join a case team</h1>
	<p class="muted">Enter the single-use code shared by the team lead. The code expires after 24 hours.</p>
	<form onsubmit={joinTeam}>
		<label class="field" for="scenario-team-code">
			<span>Invitation code</span>
			<input
				id="scenario-team-code"
				bind:value={inviteCode}
				maxlength="36"
				minlength="32"
				pattern="[0-9a-fA-F-]{32,36}"
				autocomplete="off"
				spellcheck="false"
				required
				disabled={busy}
				data-testid="scenario-team-code"
			/>
		</label>
		<p class="muted">Do not use simulation handovers for real patient details or urgent clinical decisions.</p>
		{#if error}<p class="error-text" role="alert">{error}</p>{/if}
		<button class="btn primary" type="submit" disabled={busy || inviteCode.trim().length < 32} data-testid="scenario-team-join">
			{busy ? 'Joining…' : 'Join case team'}
		</button>
	</form>
</main>

<style>
	.join-team {
		max-width: 38rem;
		margin-inline: auto;
		padding-block: var(--space-xl);
	}

	.join-team h1 {
		overflow-wrap: anywhere;
	}

	.eyebrow {
		color: var(--color-accent);
		font-size: var(--text-sm);
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}
</style>
