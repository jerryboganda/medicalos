<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { Api, ApiError } from '$lib/api';
	import { auth, clearToken, loadAuth } from '$lib/auth.svelte';
	import { browserDeviceId } from '$lib/device-identity';

	type Device = {
		device_id: string;
		device_key: string;
		label: string;
		created_at: string;
		last_seen_at: string | null;
		revoked_at: string | null;
	};

	type AccountExport = Record<string, unknown> & {
		account: Record<string, unknown>;
		attempts: unknown[];
		notes: unknown[];
		card_reviews: unknown[];
		portfolio: unknown[];
	};

	function isRecord(value: unknown): value is Record<string, unknown> {
		return typeof value === 'object' && value !== null && !Array.isArray(value);
	}

	function parseDevices(value: unknown): Device[] | null {
		if (!isRecord(value) || !Array.isArray(value.devices)) return null;
		const devices: Device[] = [];
		for (const item of value.devices) {
			if (
				!isRecord(item) ||
				typeof item.device_id !== 'string' ||
				!item.device_id ||
				typeof item.device_key !== 'string' ||
				!item.device_key ||
				typeof item.label !== 'string' ||
				typeof item.created_at !== 'string' ||
				!(item.last_seen_at === null || typeof item.last_seen_at === 'string') ||
				!(item.revoked_at === null || typeof item.revoked_at === 'string')
			) {
				return null;
			}
			devices.push({
				device_id: item.device_id,
				device_key: item.device_key,
				label: item.label.trim() || 'Device',
				created_at: item.created_at,
				last_seen_at: item.last_seen_at,
				revoked_at: item.revoked_at
			});
		}
		return devices;
	}

	function parseSessionPolicy(value: unknown): boolean | null {
		return isRecord(value) && typeof value.single_active_session === 'boolean'
			? value.single_active_session
			: null;
	}

	function isAccountExport(value: unknown): value is AccountExport {
		return (
			isRecord(value) &&
			isRecord(value.account) &&
			Array.isArray(value.attempts) &&
			Array.isArray(value.notes) &&
			Array.isArray(value.card_reviews) &&
			Array.isArray(value.portfolio)
		);
	}

	function hasConfirmation(value: unknown, field: 'revoked' | 'deleted'): boolean {
		return isRecord(value) && value[field] === true;
	}

	function messageFor(error: unknown, fallback: string): string {
		return error instanceof ApiError && error.message ? error.message : fallback;
	}

	function formatDate(value: string | null): string {
		if (!value) return 'Time unavailable';
		const timestamp = Date.parse(value);
		return Number.isFinite(timestamp)
			? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(timestamp)
			: 'Time unavailable';
	}

	let devices = $state<Device[]>([]);
	let currentDeviceKey = '';
	let loadingDevices = $state(true);
	let sessionPolicy = $state<boolean | null>(null);
	let loadingSessionPolicy = $state(true);
	let savingSessionPolicy = $state(false);
	let sessionPolicyError = $state('');
	let sessionPolicyNotice = $state('');
	let sessionPolicyReviewOpen = $state(false);
	let devicesError = $state('');
	let deviceContextError = $state('');
	let deviceNotice = $state('');
	let revokingId = $state('');
	let exporting = $state(false);
	let exportError = $state('');
	let exportNotice = $state('');
	let deleteConfirmationOpen = $state(false);
	let deleteAcknowledged = $state(false);
	let deleting = $state(false);
	let deleteError = $state('');
	let deleteReviewButton: HTMLButtonElement | undefined;
	let signedOutHeading = $state('');
	let signedOutMessage = $state('');

	function endLocalSession(heading: string, message: string) {
		clearToken();
		devices = [];
		signedOutHeading = heading;
		signedOutMessage = message;
	}

	async function loadDevices() {
		loadingDevices = true;
		devicesError = '';
		deviceContextError = '';
		deviceNotice = '';
		try {
			const response = await Api.listDevices();
			const result = parseDevices(response);
			if (!result) {
				devicesError = 'The server returned an incomplete device list. Retry to try again.';
				return;
			}
			devices = result;
			if (result.length > 0 && !result.some((device) => device.device_key === currentDeviceKey)) {
				deviceContextError = 'This browser is not registered. Revoke a device to free a slot, then reload this page to register this browser.';
			}
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in has ended. Sign in again to manage this account.');
			} else {
				devicesError = messageFor(error, 'We could not load your registered devices. Check your connection and retry.');
			}
		} finally {
			loadingDevices = false;
		}
	}

	async function loadSessionPolicy() {
		loadingSessionPolicy = true;
		sessionPolicyError = '';
		sessionPolicyNotice = '';
		try {
			const result = parseSessionPolicy(await Api.sessionPolicy());
			if (result === null) {
				sessionPolicyError = 'The server returned an incomplete session setting. Retry to try again.';
				return;
			}
			sessionPolicy = result;
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in has ended. Sign in again to manage this account.');
			} else {
				sessionPolicyError = messageFor(error, 'We could not load your session setting. Check your connection and retry.');
			}
		} finally {
			loadingSessionPolicy = false;
		}
	}

	async function saveSessionPolicy(singleActiveSession: boolean) {
		if (savingSessionPolicy) return;
		savingSessionPolicy = true;
		sessionPolicyError = '';
		sessionPolicyNotice = '';
		try {
			const saved = parseSessionPolicy(await Api.setSessionPolicy(singleActiveSession));
			if (saved !== singleActiveSession) {
				sessionPolicyError = 'The server did not confirm this setting. Retry to check its saved state.';
				return;
			}
			sessionPolicy = saved;
			sessionPolicyReviewOpen = false;
			if (saved) {
				endLocalSession('Signed out', 'Single-session protection is on. Sign in again to continue with your account.');
			} else {
				sessionPolicyNotice = 'Multiple active sessions are allowed.';
			}
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in has ended. Sign in again to manage this account.');
			} else {
				sessionPolicyError = messageFor(error, 'We could not save your session setting. Check your connection and retry.');
			}
		} finally {
			savingSessionPolicy = false;
		}
	}

	function requestSessionPolicyChange() {
		if (sessionPolicy === true) void saveSessionPolicy(false);
		else if (sessionPolicy === false) {
			sessionPolicyError = '';
			sessionPolicyNotice = '';
			sessionPolicyReviewOpen = true;
		}
	}

	async function revokeDevice(device: Device) {
		if (revokingId || device.revoked_at) return;
		deviceNotice = '';
		devicesError = '';
		revokingId = device.device_id;
		try {
			const response = await Api.revokeDevice(device.device_id);
			if (!hasConfirmation(response, 'revoked')) {
				devicesError = 'The server did not confirm that this device was revoked. Check the list and retry.';
				return;
			}
			if (device.device_key === currentDeviceKey) {
				endLocalSession('Signed out', 'This browser has been signed out by revoking its device.');
				return;
			}
			devices = devices.map((item) =>
				item.device_id === device.device_id ? { ...item, revoked_at: new Date().toISOString() } : item
			);
			deviceNotice = `${device.label} was revoked.`;
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in has ended. Sign in again to manage this account.');
			} else {
				devicesError = messageFor(error, 'We could not revoke this device. Check your connection and retry.');
			}
		} finally {
			revokingId = '';
		}
	}

	async function downloadExport() {
		if (exporting) return;
		exporting = true;
		exportError = '';
		exportNotice = '';
		try {
			const response = await Api.exportAccount();
			if (!isAccountExport(response)) {
				exportError = 'The export could not be verified, so no file was created. Please retry later.';
				return;
			}
			const blob = new Blob([JSON.stringify(response, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = `medical-os-account-export-${new Date().toISOString().slice(0, 10)}.json`;
			link.hidden = true;
			document.body.append(link);
			link.click();
			link.remove();
			window.setTimeout(() => URL.revokeObjectURL(url), 0);
			exportNotice = 'Account export downloaded. This export is partial.';
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in has ended. Sign in again to manage this account.');
			} else {
				exportError = messageFor(error, 'We could not prepare the account export. Check your connection and retry.');
			}
		} finally {
			exporting = false;
		}
	}

	function openDeleteReview() {
		deleteError = '';
		deleteAcknowledged = false;
		deleteConfirmationOpen = true;
	}

	function cancelDeleteReview() {
		deleteConfirmationOpen = false;
		deleteAcknowledged = false;
		deleteError = '';
		deleteReviewButton?.focus();
	}

	async function deleteAccount() {
		if (deleting || !deleteAcknowledged) return;
		deleting = true;
		deleteError = '';
		try {
			const response = await Api.deleteAccount();
			if (!hasConfirmation(response, 'deleted')) {
				deleteError = 'The server did not confirm account deletion. Your account may still be active; check the connection and retry.';
				return;
			}
			deleteConfirmationOpen = false;
			deleteAcknowledged = false;
			endLocalSession('Access disabled', 'Account access is disabled. Existing records remain stored.');
		} catch (error) {
			if (!auth.token) {
				endLocalSession('Signed out', 'Your sign-in ended before deletion was confirmed. Sign in and retry if you still want to disable account access.');
			} else {
				deleteError = messageFor(error, 'We could not disable account access. Check your connection and retry.');
			}
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
		currentDeviceKey = browserDeviceId();
		await loadDevices();
		if (auth.token) await loadSessionPolicy();
	});
</script>

<svelte:head>
	<title>Account · Medical Learning OS</title>
	<meta name="description" content="Manage your registered devices, account export, and account access." />
</svelte:head>

<!-- Hallmark · pre-emit critique: P5 H5 E5 S5 R5 V4 -->
<header class="account-header">
	<h1>Account</h1>
	<p class="muted">Manage registered devices, download your supported data export, or disable account access.</p>
</header>

{#if signedOutHeading}
	<section class="card" role="status" data-testid="account-signed-out">
		<h2>{signedOutHeading}</h2>
		<p>{signedOutMessage}</p>
		<a class="btn" href={`${base}/login`}>Go to sign in</a>
	</section>
{:else}
	<section class="card" aria-labelledby="devices-heading">
		<h2 id="devices-heading">Registered devices</h2>
		<p class="muted">Review devices with access to this account. Revoking this browser signs it out immediately.</p>

		{#if loadingDevices}
			<p class="muted is-loading" aria-live="polite" data-testid="devices-loading">Loading registered devices…</p>
		{:else if devicesError}
			<p class="danger-text" role="alert" data-testid="devices-error">{devicesError}</p>
			<button class="btn" type="button" onclick={loadDevices}>Retry</button>
		{:else if devices.length === 0}
			<p class="muted" data-testid="devices-empty">No registered devices are available for this account yet.</p>
		{:else}
			{#if deviceContextError}
				<p class="danger-text" role="alert">{deviceContextError}</p>
			{/if}
			<ul class="bare-list device-list" data-testid="device-list">
				{#each devices as device (device.device_id)}
					{@const current = device.device_key === currentDeviceKey}
					{@const revoked = device.revoked_at !== null}
					<li class="device-row" data-testid={`device-${device.device_id}`}>
						<div class="device-detail">
							<div class="device-heading">
								<h3>{device.label}</h3>
								{#if current && !revoked}<span class="chip info">Current device</span>{/if}
								<span class:done={!revoked} class:error={revoked} class="chip">{revoked ? 'Revoked' : 'Active'}</span>
							</div>
							<p class="muted small">
								Added {formatDate(device.created_at)} · Last active {formatDate(device.last_seen_at)}
							</p>
						</div>
						{#if !revoked}
							<button
								class="btn danger-text"
								type="button"
								disabled={revokingId !== ''}
								data-loading={revokingId === device.device_id}
								onclick={() => revokeDevice(device)}
							>
								{revokingId === device.device_id ? 'Revoking…' : current ? 'Sign out this device' : 'Revoke'}
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		{#if deviceNotice}<p class="feedback" role="status">{deviceNotice}</p>{/if}
	</section>

	<section class="card" aria-labelledby="session-policy-heading">
		<h2 id="session-policy-heading">Session security</h2>
		{#if loadingSessionPolicy}
			<p class="muted is-loading" aria-live="polite" data-testid="session-policy-loading">Loading session setting…</p>
		{:else if sessionPolicyError && sessionPolicy === null}
			<p class="danger-text" role="alert" data-testid="session-policy-error">{sessionPolicyError}</p>
			<button class="btn" type="button" disabled={savingSessionPolicy} onclick={loadSessionPolicy}>Retry session setting</button>
		{:else if sessionPolicy !== null}
			<p class="muted">{sessionPolicy ? 'Only the latest sign-in can stay active.' : 'More than one device can stay signed in.'}</p>
			<p class="tight-top">Single active session <span class="chip" class:done={sessionPolicy} class:info={!sessionPolicy}>{sessionPolicy ? 'On' : 'Off'}</span></p>
			{#if sessionPolicyError}<p class="danger-text" role="alert" data-testid="session-policy-error">{sessionPolicyError}</p>{/if}
			{#if sessionPolicyNotice}<p class="feedback" role="status">{sessionPolicyNotice}</p>{/if}
			<button class="btn" type="button" disabled={savingSessionPolicy} data-loading={savingSessionPolicy} onclick={requestSessionPolicyChange}>
				{savingSessionPolicy ? 'Saving…' : sessionPolicy ? 'Allow multiple sessions' : 'Enable single-session protection'}
			</button>
			{#if sessionPolicyReviewOpen}
				<div class="delete-confirmation" data-testid="session-policy-review">
					<p>Enabling this setting immediately signs out every active session, including this browser. You will need to sign in again; the next sign-in will stay active.</p>
					<div class="cluster">
						<button class="btn primary" type="button" disabled={savingSessionPolicy} data-loading={savingSessionPolicy} onclick={() => saveSessionPolicy(true)}>Enable and sign out</button>
						<button class="btn" type="button" disabled={savingSessionPolicy} onclick={() => (sessionPolicyReviewOpen = false)}>Cancel</button>
					</div>
				</div>
			{/if}
		{/if}
	</section>

	<section class="card" aria-labelledby="export-heading">
		<h2 id="export-heading">Export account data</h2>
		<p class="muted">
			The current JSON export includes account details, attempts, notes, reviews, and portfolio entries.
			This is a partial export; some account data is not included.
		</p>
		{#if exportError}<p class="danger-text" role="alert">{exportError}</p>{/if}
		{#if exportNotice}<p class="feedback" role="status">{exportNotice}</p>{/if}
		<button class="btn primary" type="button" disabled={exporting} data-loading={exporting} onclick={downloadExport}>
			{exporting ? 'Preparing export…' : exportError ? 'Retry export' : 'Download account export'}
		</button>
	</section>

	<section class="card" aria-labelledby="delete-heading">
		<h2 id="delete-heading">Delete account</h2>
		<p class="muted">
			Closing your account disables access. Existing records remain stored.
		</p>
		<button
			class="btn danger-text"
			type="button"
			aria-expanded={deleteConfirmationOpen}
			aria-controls="delete-confirmation"
			bind:this={deleteReviewButton}
			disabled={deleting}
			onclick={openDeleteReview}
		>
			Review account deletion
		</button>
		{#if deleteConfirmationOpen}
			<div id="delete-confirmation" class="delete-confirmation" data-testid="delete-confirmation">
				<p>
					Confirming will disable access to this account. Existing records remain stored.
				</p>
				<label class="delete-acknowledgement">
					<input type="checkbox" bind:checked={deleteAcknowledged} disabled={deleting} />
					<span>I understand that this disables account access and retains existing records.</span>
				</label>
				{#if deleteError}<p class="danger-text" role="alert">{deleteError}</p>{/if}
				<div class="cluster">
					<button
						class="btn danger-text"
						type="button"
						disabled={!deleteAcknowledged || deleting}
						data-loading={deleting}
						onclick={deleteAccount}
					>
						{deleting ? 'Disabling access…' : 'Delete account'}
					</button>
					<button class="btn" type="button" disabled={deleting} onclick={cancelDeleteReview}>Cancel</button>
				</div>
			</div>
		{/if}
	</section>
{/if}

<style>
	/* Hallmark · macrostructure: Utility cards · tone: utilitarian · anchor hue: existing violet */
	.account-header {
		margin-block-end: var(--space-lg);
	}

	.account-header p {
		margin-block: var(--space-sm) 0;
	}

	.device-list {
		margin-block-start: var(--space-md);
	}

	.device-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		padding-block: var(--space-md);
		border-top: 1px solid var(--color-border);
	}

	.device-row:first-child {
		border-top: 0;
	}

	.device-detail {
		flex: 1 1 14rem;
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.device-heading {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
	}

	.device-heading h3 {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.device-detail p {
		margin-block: var(--space-xs) 0;
	}

	.delete-confirmation {
		margin-block-start: var(--space-md);
		padding: var(--space-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-control);
		background: var(--color-surface-elevated);
	}

	.delete-acknowledgement {
		display: flex;
		min-height: 44px;
		align-items: flex-start;
		gap: var(--space-sm);
		margin-block: var(--space-md);
		color: var(--color-text-primary);
		cursor: pointer;
	}

	.delete-acknowledgement input {
		margin: var(--space-xs) 0 0;
	}

	.delete-acknowledgement span {
		min-width: 0;
		overflow-wrap: anywhere;
	}
</style>
