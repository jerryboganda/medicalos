<script>
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { setToken } from '$lib/auth.svelte';

	let mode = $state('signin');
	let email = $state('');
	let password = $state('');
	let busy = $state(false);
	let error = $state('');

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
