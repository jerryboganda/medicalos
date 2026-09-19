<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, clearAuth, loadAuth } from '$lib/auth.svelte';

	let sessions = $state([]);
	let loading = $state(true);
	let error = $state('');
	let status = $state('');
	let signingOutOthers = $state(false);
	let deleting = $state(false);
	let password = $state('');

	function formatDate(value) {
		return new Intl.DateTimeFormat(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		}).format(new Date(value));
	}

	async function loadSessions() {
		loading = true;
		error = '';
		try {
			sessions = (await Api.sessions()).sessions;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load active devices.';
		} finally {
			loading = false;
		}
	}

	async function signOutOthers() {
		if (signingOutOthers) return;
		signingOutOthers = true;
		error = '';
		status = '';
		try {
			const result = await Api.signOutOthers();
			status = result.revoked === 1 ? 'Signed out 1 other device.' : `Signed out ${result.revoked} other devices.`;
			await loadSessions();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not sign out other devices.';
		} finally {
			signingOutOthers = false;
		}
	}

	async function requestDeletion(event) {
		event.preventDefault();
		if (deleting || !password) return;
		deleting = true;
		error = '';
		try {
			await Api.requestAccountDeletion(password);
			clearAuth();
			goto(`${base}/login`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not start account deletion.';
		} finally {
			deleting = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await loadSessions();
	});
</script>

<h1>Account</h1>
<p class="muted">Manage the devices signed in to this personal account.</p>

{#if error}
	<p class="error-text" role="alert">{error}</p>
{/if}
{#if status}
	<p class="muted" role="status" aria-live="polite">{status}</p>
{/if}

<section class="card" aria-labelledby="devices-heading">
	<div class="section-heading">
		<div>
			<h2 id="devices-heading">Active devices</h2>
			<p class="muted">A personal account can have up to two active device sessions.</p>
		</div>
		<button
			class="btn"
			type="button"
			disabled={loading || signingOutOthers || sessions.filter((session) => !session.current).length === 0}
			onclick={signOutOthers}
			data-testid="sign-out-others"
		>
			{signingOutOthers ? 'Signing out…' : 'Sign out other devices'}
		</button>
	</div>

	{#if loading}
		<p class="muted">Loading active devices…</p>
	{:else if sessions.length === 0}
		<p class="muted">No active device sessions were found.</p>
	{:else}
		<ul class="session-list" data-testid="session-list">
			{#each sessions as session (session.session_id)}
				<li>
					<div>
						<strong>{session.device_name}</strong>
						<p class="muted">Last active {formatDate(session.last_seen_at)}</p>
					</div>
					{#if session.current}<span class="chip done">Current device</span>{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>

<section class="card" aria-labelledby="delete-heading">
	<h2 id="delete-heading">Delete account</h2>
	<p class="muted">
		Starting deletion signs out every device immediately and blocks future sign-in while the request is pending.
	</p>
	<form onsubmit={requestDeletion}>
		<label class="field" for="delete-password">
			<span>Confirm your password</span>
			<input
				id="delete-password"
				type="password"
				bind:value={password}
				autocomplete="current-password"
				required
				data-testid="delete-password"
			/>
		</label>
		<button
			class="btn danger-text"
			type="submit"
			disabled={deleting || !password}
			data-loading={deleting}
			data-testid="delete-account"
		>
			{deleting ? 'Starting deletion…' : 'Start account deletion'}
		</button>
	</form>
</section>

<style>
	.section-heading,
	.session-list li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		flex-wrap: wrap;
	}

	.section-heading h2,
	.section-heading p,
	.session-list,
	.session-list p {
		margin: 0;
	}

	.session-list {
		list-style: none;
		padding: 0;
		display: grid;
		gap: var(--space-md);
		margin-top: var(--space-lg);
	}

	.session-list li {
		padding-top: var(--space-md);
		border-top: 1px solid var(--color-surface-elevated);
	}
</style>
