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

	onMount(() => {
		institutionId = new URLSearchParams(window.location.search).get('institution') ?? '';
	});

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

<section class="card">
	<h1>{mode === 'signin' ? 'Welcome back' : 'Create your account'}</h1>
	<p class="muted">One connected learning record for your whole medical education.</p>

	{#if error}
		<p class="error-text" role="alert">{error}</p>
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
			class="btn primary"
			type="submit"
			disabled={busy}
			data-loading={busy}
			data-testid="submit"
		>
			{busy ? 'Working…' : mode === 'signin' ? 'Sign in' : 'Create account'}
		</button>
	</form>

	{#if mode === 'signin'}
		<section aria-label="Institution sign-in">
			<p class="muted">Signing in through your institution?</p>
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
