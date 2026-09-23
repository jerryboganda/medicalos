<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { setToken } from '$lib/auth.svelte';

	let error = $state('');

	onMount(async () => {
		const url = new URL(window.location.href);
		const ticket = new URLSearchParams(url.hash.slice(1)).get('ticket');
		const failed = url.searchParams.has('error');
		window.history.replaceState(null, '', url.pathname);

		if (!ticket || failed) {
			error = 'Institution sign-in could not be completed. Return to sign in and try again.';
			return;
		}

		try {
			const { token } = await Api.completeInstitutionSso(ticket);
			setToken(token);
			await goto(`${base}/today`);
		} catch {
			error = 'This sign-in link expired or was already used. Return to sign in and try again.';
		}
	});
</script>

<section class="card" aria-live="polite" aria-busy={!error}>
	<h1>{error ? 'Sign-in incomplete' : 'Finishing sign-in'}</h1>
	{#if error}
		<p class="error-text" role="alert">{error}</p>
		<a class="btn" href={`${base}/login`}>Return to sign in</a>
	{:else}
		<p class="muted">Connecting your institution account to your learning record…</p>
	{/if}
</section>
