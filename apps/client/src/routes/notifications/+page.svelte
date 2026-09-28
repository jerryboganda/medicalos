<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	interface InboxItem {
		id: string;
		category: string;
		title: string;
		body: string;
		deep_link: string | null;
		read: boolean;
	}

	let items = $state<InboxItem[] | null>(null);
	let prefs = $state({
		plan_reminders: true,
		mock_results: true,
		reports: true,
		content_updates: true
	});
	let showPrefs = $state(false);
	let busy = $state(false);

	async function load() {
		const inbox = await Api.inbox();
		items = inbox.notifications;
		prefs = {
			plan_reminders: inbox.preferences.plan_reminders,
			mock_results: inbox.preferences.mock_results,
			reports: inbox.preferences.reports,
			content_updates: inbox.preferences.content_updates
		};
	}

	async function savePrefs(e: Event) {
		e.preventDefault();
		busy = true;
		try {
			await Api.updateNotificationPrefs({ ...prefs });
			showPrefs = false;
		} finally {
			busy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await load();
	});
</script>

<h1>Notifications</h1>

<div class="card">
	{#if items === null || items.length === 0}
		<p class="muted">No notifications. Plan, report, and source updates appear here.</p>
	{:else}
		{#each items as n (n.id)}
			<div class="card">
				<div class="row">
					<strong>{n.title}</strong>
					<span class="chip {n.read ? 'done' : 'info'}">{n.read ? 'Read' : 'New'}</span>
				</div>
				<p class="tight-top">{n.body}</p>
			</div>
		{/each}
	{/if}
</div>

<div class="card">
	<h2>Preferences</h2>
	{#if showPrefs}
		<form onsubmit={savePrefs}>
			<div class="prefs">
				<label><input type="checkbox" bind:checked={prefs.plan_reminders} /> Plan reminders</label>
				<label><input type="checkbox" bind:checked={prefs.mock_results} /> Mock results</label>
				<label><input type="checkbox" bind:checked={prefs.reports} /> Report updates</label>
				<label><input type="checkbox" bind:checked={prefs.content_updates} /> Source and content corrections</label>
			</div>
			<button class="btn primary" type="submit" disabled={busy}>Save preferences</button>
		</form>
	{:else}
		<button class="btn" type="button" onclick={() => (showPrefs = true)}>
			Edit notification preferences
		</button>
	{/if}
</div>

<svelte:head>
	<title>Notifications | Medical Learning OS</title>
</svelte:head>

<style>
	.prefs {
		display: grid;
		gap: 2px;
		margin-bottom: var(--space-lg);
	}

	.prefs label {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		min-height: 44px;
		padding: 0 var(--space-md);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background-color var(--dur-fast) var(--ease-out);
	}

	.prefs label:hover {
		background: var(--color-surface-hover);
	}
</style>
