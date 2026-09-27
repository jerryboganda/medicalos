<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError, adminToken, type AdminDashboard } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';

	const metrics = [
		{ key: 'institutions', label: 'Institutions', description: 'All registered institutions.' },
		{ key: 'users', label: 'Accounts', description: 'Accounts not marked as deleted.' },
		{ key: 'published_questions', label: 'Published questions', description: 'Question versions currently published.' },
		{ key: 'articles', label: 'Articles', description: 'Records in the article catalogue.' },
		{ key: 'rights_records', label: 'Rights records', description: 'Content permission records.' },
		{ key: 'open_incidents', label: 'Open incidents', description: 'Incidents not marked resolved.' },
		{
			key: 'coach_turns_last_30_days',
			label: 'Coach turns · 30 days',
			description: 'Coach requests recorded in the last 30 days.'
		}
	] as const satisfies readonly {
		key: keyof AdminDashboard;
		label: string;
		description: string;
	}[];
	const countFormat = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });

	let token = $state('');
	let tokenRequired = $state(true);
	let dashboard = $state<AdminDashboard | null>(null);
	let loading = $state(false);
	let error = $state('');

	function formatCount(value: number): string {
		return countFormat.format(value);
	}

	async function loadDashboard() {
		if (loading) return;
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		if (!adminToken()) {
			tokenRequired = true;
			dashboard = null;
			return;
		}

		tokenRequired = false;
		loading = true;
		error = '';
		dashboard = null;
		try {
			dashboard = await Api.adminDashboard();
		} catch (err) {
			if (err instanceof ApiError && err.status === 401) {
				goto(`${base}/login`);
				return;
			}
			error = err instanceof ApiError ? err.message : 'Dashboard metrics could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function submitToken(event: SubmitEvent) {
		event.preventDefault();
		const candidate = token.trim();
		if (!candidate || loading) return;

		try {
			localStorage.setItem('mlos_admin', candidate);
		} catch {
			error = 'Browser storage is unavailable. Allow local storage to use the admin dashboard.';
			return;
		}

		token = candidate;
		await loadDashboard();
	}

	function changeAdminToken() {
		try {
			localStorage.removeItem('mlos_admin');
		} catch {
			error = 'Browser storage is unavailable. Clear the saved admin token in browser settings.';
			return;
		}
		token = '';
		dashboard = null;
		error = '';
		tokenRequired = true;
	}

	onMount(() => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}

		token = adminToken();
		tokenRequired = !token;
		if (token) void loadDashboard();
	});
</script>

<svelte:head>
	<title>Owner dashboard | Medical Learning OS</title>
</svelte:head>

<header class="dashboard-heading">
	<div>
		<h1>Owner dashboard</h1>
		<p class="muted">
			Current platform totals from aggregate records. This page does not expose institution- or learner-level rows.
		</p>
	</div>
	<nav class="dashboard-links" aria-label="Admin navigation">
		<a class="btn primary" href={`${base}/admin`}>Editorial console</a>
		<a class="btn" href={`${base}/admin/image-annotations`}>Image annotation review</a>
	</nav>
</header>

{#if tokenRequired}
	<form class="card token-form" onsubmit={submitToken} data-testid="dashboard-token-gate">
		<h2>Administrator access</h2>
		<p class="muted">Use the administrator token for the editorial console.</p>
		<label class="field" for="dashboard-admin-token">
			<span>Admin token</span>
			<input
				id="dashboard-admin-token"
				type="password"
				autocomplete="current-password"
				bind:value={token}
				data-testid="dashboard-admin-token"
			/>
		</label>
		{#if error}<p class="error-text" role="alert">{error}</p>{/if}
		<button class="btn primary" type="submit" disabled={loading || !token.trim()}>
			{loading ? 'Checking access…' : 'Unlock dashboard'}
		</button>
	</form>
{:else if error}
	<section class="card dashboard-error" aria-labelledby="dashboard-error-heading" data-testid="dashboard-error">
		<h2 id="dashboard-error-heading">Dashboard unavailable</h2>
		<p class="error-text" role="alert">{error}</p>
		<div class="dashboard-links">
			<button class="btn primary" type="button" onclick={() => loadDashboard()}>Retry</button>
			<button class="btn" type="button" onclick={changeAdminToken}>Change admin token</button>
		</div>
	</section>
{/if}

{#if loading || dashboard}
	<section class="dashboard-metrics" aria-labelledby="dashboard-metrics-heading" aria-busy={loading} data-testid="dashboard-metrics">
		<h2 id="dashboard-metrics-heading">Current platform totals</h2>
		<p class="muted">Coach activity covers the most recent 30 days; the other totals reflect current records.</p>
		{#if loading}
			<p class="is-loading" role="status">Loading platform totals…</p>
			<div class="metric-grid" aria-hidden="true">
				{#each metrics as metric (metric.key)}
					<article class="metric-card metric-skeleton">
						<span class="skeleton-line skeleton-label"></span>
						<span class="skeleton-line skeleton-value"></span>
						<span class="skeleton-line skeleton-description"></span>
					</article>
				{/each}
			</div>
		{:else if dashboard}
			<div class="metric-grid">
				{#each metrics as metric (metric.key)}
					<article class="metric-card" data-testid={`dashboard-metric-${metric.key}`}>
						<h3>{metric.label}</h3>
						<p class="metric-value">{formatCount(dashboard[metric.key])}</p>
						<p class="metric-description">{metric.description}</p>
					</article>
				{/each}
			</div>
		{/if}
	</section>
{/if}

<style>
	/* Hallmark · macrostructure: App Shell (Owner Dashboard) · tone: calm utilitarian · anchor hue: violet
	 * theme: Midnight-equivalent (owner-locked) · enrichment: none · motion: cut
	 * pre-emit critique: P4 H5 E4 S5 R5 V4
	 * contrast: pass (40–41) · mobile: pending final E2E (34, 49, 50–57)
	 */
	.dashboard-heading {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-md);
		margin-bottom: var(--space-xl);
	}

	.dashboard-heading > div {
		min-width: 0;
		flex: 1 1 18rem;
	}

	.dashboard-heading p {
		max-width: 52ch;
		margin: 0;
	}

	.dashboard-links {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
	}

	.dashboard-metrics h2 {
		margin-bottom: var(--space-xs);
	}

	.dashboard-metrics > p {
		margin-top: 0;
	}

	.metric-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 15rem), 1fr));
		gap: var(--space-md);
		margin-top: var(--space-lg);
	}

	.metric-card {
		min-width: 0;
		margin: 0;
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card);
		background: var(--color-surface);
	}

	.metric-card h3 {
		margin: 0;
		color: var(--color-text-secondary);
		font: 600 var(--text-sm) / 1.4 var(--font-body);
		overflow-wrap: anywhere;
	}

	.metric-value {
		margin: var(--space-sm) 0;
		color: var(--color-accent);
		font: 600 var(--text-xl) / 1.2 var(--font-display);
		font-variant-numeric: tabular-nums;
		overflow-wrap: anywhere;
	}

	.metric-description {
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--text-sm);
	}

	.token-form {
		max-width: 32rem;
	}

	.token-form h2,
	.dashboard-error h2 {
		margin-top: 0;
	}

	.metric-skeleton {
		display: grid;
		align-content: start;
		gap: var(--space-sm);
	}

	.skeleton-line {
		display: block;
		border-radius: var(--radius-control);
		background: var(--color-surface-elevated);
	}

	.skeleton-label {
		width: 55%;
		height: var(--space-md);
	}

	.skeleton-value {
		width: 38%;
		height: var(--space-xl);
	}

	.skeleton-description {
		width: 82%;
		height: var(--space-sm);
	}
</style>
