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
	let notificationLoading = $state(true);
	let notificationSaving = $state(false);
	let notificationError = $state('');
	let notifications = $state(null);
	let settings = $state(null);

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

	async function loadNotifications() {
		notificationLoading = true;
		notificationError = '';
		notifications = null;
		settings = null;
		try {
			const [preferences, inbox] = await Promise.all([
				Api.notificationPreferences(),
				Api.notifications()
			]);
			settings = {
				...preferences,
				quiet_start: preferences.quiet_start ?? '',
				quiet_end: preferences.quiet_end ?? ''
			};
			notifications = inbox.notifications;
		} catch (err) {
			notificationError =
				err instanceof ApiError ? err.message : 'Could not load notification data.';
		} finally {
			notificationLoading = false;
		}
	}

	async function saveNotificationSettings(event) {
		event.preventDefault();
		if (notificationSaving || !settings) return;
		notificationSaving = true;
		notificationError = '';
		status = '';
		try {
			const saved = await Api.updateNotificationPreferences({
				timezone: settings.timezone,
				quiet_start: settings.quiet_start || null,
				quiet_end: settings.quiet_end || null,
				categories: settings.categories
			});
			settings = {
				...saved,
				quiet_start: saved.quiet_start ?? '',
				quiet_end: saved.quiet_end ?? ''
			};
			status = 'Notification settings saved.';
		} catch (err) {
			notificationError =
				err instanceof ApiError ? err.message : 'Could not save notification settings.';
		} finally {
			notificationSaving = false;
		}
	}

	async function markNotificationRead(notification) {
		if (!notifications || notification.read_at) return;
		try {
			const result = await Api.markNotificationRead(notification.id);
			notifications = notifications.map((item) =>
				item.id === notification.id ? { ...item, read_at: result.read_at } : item
			);
		} catch (err) {
			notificationError = err instanceof ApiError ? err.message : 'Could not mark notification read.';
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await Promise.all([loadSessions(), loadNotifications()]);
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

<section class="card" aria-labelledby="notifications-settings-heading" data-testid="notification-settings">
	<h2 id="notifications-settings-heading">Notification settings</h2>
	<p class="muted">
		Choose which study notifications you want. Mobile push delivery will use these settings when the native push adapter is available.
	</p>

	{#if notificationError}
		<p class="error-text" role="alert">{notificationError}</p>
	{/if}

	{#if notificationLoading}
		<p class="muted">Loading notification settings…</p>
	{:else if settings === null}
		<p class="muted" data-testid="notification-settings-unavailable">Notification settings could not be loaded.</p>
	{:else}
		<form onsubmit={saveNotificationSettings}>
			<label class="field" for="notification-timezone">
				<span>Time zone</span>
				<input id="notification-timezone" bind:value={settings.timezone} required data-testid="notification-timezone" />
			</label>

			<div class="quiet-grid">
				<label class="field" for="quiet-start">
					<span>Quiet hours start</span>
					<input id="quiet-start" type="time" bind:value={settings.quiet_start} data-testid="quiet-start" />
				</label>
				<label class="field" for="quiet-end">
					<span>Quiet hours end</span>
					<input id="quiet-end" type="time" bind:value={settings.quiet_end} data-testid="quiet-end" />
				</label>
			</div>

			<fieldset class="notification-toggles">
				<legend>Categories</legend>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.plan_review_reminders} data-testid="notify-plan-review" />
					<span><strong>Plan and review reminders</strong><small>Study-plan and spaced-review reminders.</small></span>
				</label>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.mock_assignment} />
					<span><strong>New mock or assignment</strong><small>New assessment or assigned work.</small></span>
				</label>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.competition} />
					<span><strong>Competition start and end</strong><small>Competition lifecycle alerts when that feature is available.</small></span>
				</label>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.duel_invitation} />
					<span><strong>Duel invitations</strong><small>Direct challenge invitations when social competition is available.</small></span>
				</label>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.report_resolved} />
					<span><strong>Report resolved</strong><small>Outcome of an issue you reported after editorial review.</small></span>
				</label>
				<label class="toggle-row">
					<input type="checkbox" bind:checked={settings.categories.subscription_events} />
					<span><strong>Subscription events</strong><small>Service changes related to your subscription.</small></span>
				</label>
			</fieldset>

			<button class="btn primary" type="submit" disabled={notificationSaving} data-loading={notificationSaving} data-testid="save-notification-settings">
				{notificationSaving ? 'Saving…' : 'Save settings'}
			</button>
		</form>
	{/if}
</section>

<section class="card" aria-labelledby="inbox-heading" data-testid="notification-inbox">
	<h2 id="inbox-heading">Notifications</h2>
	{#if notificationLoading}
		<p class="muted">Loading notifications…</p>
	{:else if notifications === null}
		<p class="muted" data-testid="notification-unavailable">Notifications could not be loaded.</p>
	{:else if notifications.length === 0}
		<p class="muted" data-testid="notification-empty">No notifications yet.</p>
	{:else}
		<ul class="notification-list">
			{#each notifications as notification (notification.id)}
				<li>
					<div>
						<strong>{notification.title}</strong>
						<p>{notification.body}</p>
					</div>
					<div class="notification-actions">
						<a class="btn" href={`${base}${notification.deep_link}`}>Open</a>
						{#if !notification.read_at}
							<button class="btn" type="button" onclick={() => markNotificationRead(notification)}>Mark read</button>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>

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

	.quiet-grid {
		display: grid;
		gap: var(--space-md);
	}

	.notification-toggles {
		border: 0;
		padding: 0;
		margin: 0 0 var(--space-lg);
	}

	.notification-toggles legend {
		margin-bottom: var(--space-sm);
		font-weight: 600;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		gap: var(--space-md);
		min-height: calc(var(--space-2xl) + var(--space-md));
		padding: var(--space-sm) 0;
		cursor: pointer;
	}

	.toggle-row input {
		width: var(--space-xl);
		min-height: var(--space-xl);
		padding: 0;
		flex: none;
	}

	.toggle-row span,
	.toggle-row small {
		display: block;
	}

	.toggle-row small {
		color: var(--color-text-secondary);
	}

	.notification-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		gap: var(--space-lg);
	}

	.notification-list li {
		border-top: 1px solid var(--color-surface-elevated);
		padding-top: var(--space-md);
	}

	.notification-list p {
		margin-bottom: var(--space-sm);
	}

	.notification-actions {
		display: flex;
		gap: var(--space-sm);
		flex-wrap: wrap;
	}

	@media (min-width: 768px) {
		.quiet-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
