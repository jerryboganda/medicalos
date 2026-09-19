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
	import { page } from '$app/stores';

	let { children } = $props();
	loadAuth();

	const destinations = [
		{ label: 'Today', path: '/today' },
		{ label: 'Practice', path: '/practice' },
		{ label: 'Learn', path: '/learn' },
		{ label: 'Coach', path: '/coach' },
		{ label: 'Progress', path: '/progress' }
	];

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
	<div class="top-bar">
		<a class="brand" href={`${base}/today`}>Medical Learning OS</a>
		{#if auth.token}
			<div class="secondary-actions">
				<a class="btn" href={`${base}/account`} data-testid="nav-account">Account</a>
				<button class="btn" type="button" onclick={signOut}>Sign out</button>
			</div>
		{/if}
	</div>
	{#if auth.token}
		<nav class="primary-navigation" aria-label="Primary" data-testid="primary-navigation">
			{#each destinations as destination (destination.path)}
				<a
					class="primary-link"
					href={`${base}${destination.path}`}
					aria-current={$page.url.pathname === `${base}${destination.path}` ? 'page' : undefined}
				>
					{destination.label}
				</a>
			{/each}
		</nav>
	{/if}
</header>

<main>
	{@render children()}
</main>
