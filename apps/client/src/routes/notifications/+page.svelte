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
	let prefs = $state({ plan_reminders: true, mock_results: true, reports: true });
	let showPrefs = $state(false);
	let busy = $state(false);

	async function load() {
		items = (await Api.inbox()).notifications;
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
		<p class="muted">No notifications. When the plan changes or a report resolves, it shows here.</p>
	{:else}
		{#each items as n (n.id)}
			<div class="card" style="margin-bottom: var(--space-md);">
				<div style="display:flex; justify-content:space-between; gap:12px; align-items:center;">
					<strong>{n.title}</strong>
					<span class="chip {n.read ? 'done' : ''}">{n.read ? 'Read' : 'New'}</span>
				</div>
				<p style="margin: var(--space-xs) 0 0;">{n.body}</p>
			</div>
		{/each}
	{/if}
</div>

<div class="card">
	<h2>Preferences</h2>
	{#if showPrefs}
		<form onsubmit={savePrefs}>
			<label><input type="checkbox" bind:checked={prefs.plan_reminders} /> Plan reminders</label><br />
			<label><input type="checkbox" bind:checked={prefs.mock_results} /> Mock results</label><br />
			<label><input type="checkbox" bind:checked={prefs.reports} /> Report updates</label><br /><br />
			<button class="btn primary" type="submit" disabled={busy}>Save preferences</button>
		</form>
	{:else}
		<button class="btn" type="button" onclick={() => (showPrefs = true)}>
			Edit notification preferences
		</button>
	{/if}
</div>
