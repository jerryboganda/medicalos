<script>
	import '@fontsource/fraunces/600.css';
	import '@fontsource/manrope/400.css';
	import '@fontsource/manrope/600.css';
	import '@fontsource/manrope/700.css';
	import '@medical-os/design-system/tokens.css';
	import '../app.css';
	import { auth, loadAuth, clearToken } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let { children } = $props();
	loadAuth();

	function signOut() {
		clearToken();
		goto('/login');
	}
</script>

<header class="top">
	<a class="brand" href="/today">Medical Learning OS</a>
	{#if auth.token}
		<button class="btn" type="button" onclick={signOut}>Sign out</button>
	{/if}
</header>

<main>
	{@render children()}
</main>
