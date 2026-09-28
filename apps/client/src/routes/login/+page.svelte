<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { setToken } from '$lib/auth.svelte';

	let mode = $state('signin');
	let email = $state('');
	let password = $state('');
	let institutionId = $state('');
	let busy = $state(false);
	let error = $state('');
	// Platform sign-in (Zitadel) buttons appear only when it is configured.
	let options = $state({ platform: false, google: false, apple: false });

	onMount(async () => {
		institutionId = new URLSearchParams(window.location.search).get('institution') ?? '';
		try {
			options = await Api.signInOptions();
		} catch {
			/* offline or older API: password sign-in still works */
		}
	});

	async function platformSignIn(idp) {
		if (busy) return;
		busy = true;
		error = '';
		try {
			const { authorization_url } = await Api.startPlatformSignIn(idp);
			window.location.assign(authorization_url);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Sign-in is unavailable right now. Check your connection and try again.';
			busy = false;
		}
	}

	async function submit(event) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		try {
			if (mode === 'register') {
				await Api.register(email.trim(), password);
			}
			const { token } = await Api.login(email.trim(), password);
			setToken(token);
			goto(`${base}/today`);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Something went wrong. Check your connection and try again.';
		} finally {
			busy = false;
		}
	}

	async function beginInstitutionSignIn() {
		if (busy) return;
		const id = institutionId.trim();
		if (!id) {
			error = 'Enter your institution ID to continue.';
			return;
		}
		busy = true;
		error = '';
		try {
			const { authorization_url } = await Api.startInstitutionSso(id);
			window.location.assign(authorization_url);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Institution sign-in is unavailable. Check the ID and try again.';
			busy = false;
		}
	}
</script>

<section class="card login-card">
	<h1>{mode === 'signin' ? 'Welcome back' : 'Create your account'}</h1>
	<p class="muted lede">One connected learning record for your whole medical education.</p>

	{#if error}
		<p class="error-text" role="alert">{error}</p>
	{/if}

	{#if options.platform && mode === 'signin'}
		<div class="platform">
			<button
				class="btn primary wide-btn"
				type="button"
				disabled={busy}
				data-testid="platform-sign-in"
				onclick={() => platformSignIn()}
			>
				Continue with email
			</button>
			{#if options.google}
				<button class="btn wide-btn" type="button" disabled={busy} onclick={() => platformSignIn('google')}>
					Continue with Google
				</button>
			{/if}
			{#if options.apple}
				<button class="btn wide-btn" type="button" disabled={busy} onclick={() => platformSignIn('apple')}>
					Continue with Apple
				</button>
			{/if}
		</div>
		<p class="muted divider"><span>Or use your password</span></p>
	{/if}

	<form onsubmit={submit}>
		<label class="field" for="email">
			<span>Email</span>
			<input
				id="email"
				type="email"
				bind:value={email}
				autocomplete="email"
				required
				data-testid="email"
			/>
		</label>
		<label class="field" for="password">
			<span>Password</span>
			<input
				id="password"
				type="password"
				bind:value={password}
				autocomplete={mode === 'signin' ? 'current-password' : 'new-password'}
				minlength={8}
				required
				data-testid="password"
			/>
		</label>
		<button
			class="btn {options.platform ? '' : 'primary'} wide-btn"
			type="submit"
			disabled={busy}
			data-loading={busy}
			data-testid="submit"
		>
			{busy ? 'Working…' : mode === 'signin' ? 'Sign in' : 'Create account'}
		</button>
	</form>

	{#if mode === 'signin'}
		<section class="sso" aria-label="Institution sign-in">
			<p class="muted divider"><span>Signing in through your institution?</span></p>
			<label class="field" for="sso-institution-id">
				<span>Institution ID</span>
				<input
					id="sso-institution-id"
					bind:value={institutionId}
					autocomplete="off"
					data-testid="sso-institution-id"
				/>
			</label>
			<button
				class="btn"
				type="button"
				disabled={busy || !institutionId.trim()}
				data-loading={busy}
				data-testid="sso-submit"
				onclick={beginInstitutionSignIn}
			>
				{busy ? 'Connecting…' : 'Institution sign-in'}
			</button>
		</section>
	{/if}

	<button
		class="linklike"
		type="button"
		onclick={() => {
			mode = mode === 'signin' ? 'register' : 'signin';
			error = '';
		}}
		data-testid="toggle-mode"
	>
		{mode === 'signin'
			? 'New here? Create an account'
			: 'Already have an account? Sign in'}
	</button>
</section>

<svelte:head>
	<title>Sign in | Medical Learning OS</title>
</svelte:head>

<style>
	.login-card {
		justify-self: center;
		width: min(100%, 440px);
		box-sizing: border-box;
		padding: var(--space-2xl) var(--space-xl);
		box-shadow: var(--shadow-3);
	}

	.login-card h1 {
		margin-bottom: var(--space-sm);
	}

	.login-card .lede {
		margin: 0 0 var(--space-xl);
		font-size: var(--text-body);
	}

	.wide-btn {
		width: 100%;
	}

	.sso {
		margin-top: var(--space-xl);
	}

	.platform {
		display: grid;
		gap: var(--space-sm);
		margin-bottom: var(--space-lg);
	}

	/* "Signing in through your institution?" sits on a hairline divider. */
	.divider {
		display: flex;
		align-items: center;
		gap: var(--space-md);
		margin: 0 0 var(--space-lg);
		font-size: var(--text-sm);
		text-align: center;
	}

	.divider::before,
	.divider::after {
		content: '';
		flex: 1;
		height: 1px;
		background: var(--color-border);
	}
</style>
