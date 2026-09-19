<script>
	import '@fontsource/fraunces/600.css';
	import '@fontsource/manrope/400.css';
	import '@fontsource/manrope/600.css';
	import '@fontsource/manrope/700.css';
	import '@medical-os/design-system/tokens.css';
	import '../app.css';
	import { base } from '$app/paths';
	import { Api } from '$lib/api';
	import { auth, loadAuth, clearAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let { children } = $props();
	loadAuth();

	async function signOut() {
		try {
			await Api.logout();
		} catch {
			/* local sign-out still proceeds if the server is unreachable */
		}
		clearAuth();
		goto(`${base}/login`);
	}
</script>

<header class="top">
	<a class="brand" href={`${base}/today`}>Medical Learning OS</a>
	{#if auth.token}
		<nav style="display:flex; gap:12px; align-items:center;">
			<a class="btn" href={`${base}/review`} data-testid="nav-review">Review</a>
			<a class="btn" href={`${base}/account`} data-testid="nav-account">Account</a>
			<button class="btn" type="button" onclick={signOut}>Sign out</button>
		</nav>
	{/if}
</header>

<main>
	{@render children()}
</main>
