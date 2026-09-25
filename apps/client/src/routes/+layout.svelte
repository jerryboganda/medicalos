<script>
	import '@fontsource/fraunces/600.css';
	import '@fontsource/manrope/400.css';
	import '@fontsource/manrope/600.css';
	import '@fontsource/manrope/700.css';
	import '@medical-os/design-system/tokens.css';
	import '../app.css';
	import { base } from '$app/paths';
	import { auth, loadAuth, clearToken } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	let { children } = $props();
	let menuOpen = $state(false);
	loadAuth();

	function signOut() {
		menuOpen = false;
		clearToken();
		goto(`${base}/login`);
	}
</script>

<header class="top">
	<a class="brand" href={`${base}/today`}>Medical Learning OS</a>
	{#if auth.token}
		<button
			class="btn menu-toggle"
			type="button"
			aria-expanded={menuOpen}
			aria-controls="primary-navigation"
			onclick={() => (menuOpen = !menuOpen)}
		>
			{menuOpen ? 'Close menu' : 'Menu'}
		</button>
		<nav
			id="primary-navigation"
			class="top-nav"
			class:open={menuOpen}
			aria-label="Main navigation"
		>
			<a class="btn" href={`${base}/progress`} data-testid="nav-progress" onclick={() => (menuOpen = false)}
				>Progress</a
			>
			<a class="btn" href={`${base}/coach`} data-testid="nav-coach" onclick={() => (menuOpen = false)}
				>Coach</a
			>
			<a class="btn" href={`${base}/notes`} data-testid="nav-notes" onclick={() => (menuOpen = false)}
				>Notes</a
			>
			<a class="btn" href={`${base}/library`} data-testid="nav-library" onclick={() => (menuOpen = false)}
				>Library</a
			>
			<a class="btn" href={`${base}/imaging`} data-testid="nav-imaging" onclick={() => (menuOpen = false)}
				>Images</a
			>
			<a
				class="btn"
				href={`${base}/notifications`}
				data-testid="nav-notifications"
				onclick={() => (menuOpen = false)}
			>
				Notifications
			</a>
			<a class="btn" href={`${base}/practice`} data-testid="nav-practice" onclick={() => (menuOpen = false)}
				>Practice</a
			>
			<a class="btn" href={`${base}/scenarios`} data-testid="nav-scenarios" onclick={() => (menuOpen = false)}
				>Simulations</a
			>
			<a class="btn" href={`${base}/review`} data-testid="nav-review" onclick={() => (menuOpen = false)}
				>Review</a
			>
			<a
				class="btn"
				href={`${base}/community`}
				data-testid="nav-community"
				onclick={() => (menuOpen = false)}
				>Community</a
			>
			<a class="btn" href={`${base}/faculty`} data-testid="nav-faculty" onclick={() => (menuOpen = false)}
				>Faculty</a
			>
			<a class="btn" href={`${base}/admin`} data-testid="nav-admin" onclick={() => (menuOpen = false)}
				>Console</a
			>
			<button class="btn" type="button" onclick={signOut}>Sign out</button>
		</nav>
	{/if}
</header>

<main>
	{@render children()}
</main>
