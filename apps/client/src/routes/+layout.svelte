<script lang="ts">
	import '@fontsource/fraunces/600.css';
	import '@fontsource/manrope/400.css';
	import '@fontsource/manrope/600.css';
	import '@fontsource/manrope/700.css';
	import '@medical-os/design-system/index.css';
	import '../app.css';
	import { base } from '$app/paths';
	import { page } from '$app/state';
	import { afterNavigate, goto, onNavigate } from '$app/navigation';
	import { auth, loadAuth, clearToken } from '$lib/auth.svelte';
	import { theme, loadTheme, toggleTheme } from '$lib/theme.svelte';
	import { NAV, NAV_GROUPS, PALETTE_EXTRAS, isActive } from '$lib/nav';
	import Icon from '$lib/components/Icon.svelte';
	import Emblem from '$lib/components/Emblem.svelte';
	import CommandPalette, { type PaletteEntry } from '$lib/components/CommandPalette.svelte';
	import { tick } from 'svelte';

	let { children } = $props();
	let menuOpen = $state(false);
	let paletteOpen = $state(false);
	let menuToggle: HTMLButtonElement;
	let primaryNavigation: HTMLElement;
	loadAuth();
	loadTheme();

	const path = $derived(page.url.pathname.slice(base.length) || '/');
	const inSession = $derived(path.startsWith('/session/'));
	// §6: primary navigation is hidden during a study session; the brand
	// link stays as the way out (Focus Mode hides that too).
	const showNav = $derived(Boolean(auth.token) && !inSession);
	const wide = $derived(path.startsWith('/admin') || path.startsWith('/faculty'));
	const tabs = NAV.filter((item) => item.tab);
	const shortcut = /Mac|iPhone|iPad/.test(navigator.userAgent) ? '⌘K' : 'Ctrl K';

	const paletteEntries: PaletteEntry[] = $derived([
		...[...NAV, ...PALETTE_EXTRAS].map((item) => ({
			label: item.label,
			hint: item.group,
			icon: item.icon,
			keywords: item.keywords,
			run: () => goto(`${base}${item.href}`)
		})),
		{
			label: theme.light ? 'Switch to dark theme' : 'Switch to light reading theme',
			hint: 'Appearance',
			icon: theme.light ? 'moon' : 'sun',
			keywords: 'theme appearance contrast',
			run: () => toggleTheme()
		},
		{ label: 'Sign out', hint: 'Account', icon: 'logout', run: signOut }
	]);

	// Route changes cross-fade through the View Transitions API (design.md
	// § Motion). Same-path updates (query/hash) keep their own motion.
	onNavigate((navigation) => {
		if (!document.startViewTransition) return;
		if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		if (navigation.from?.url.pathname === navigation.to?.url.pathname) return;
		return new Promise((resolve) => {
			document.startViewTransition(async () => {
				resolve();
				await navigation.complete;
			});
		});
	});

	afterNavigate(() => {
		menuOpen = false;
	});

	function signOut() {
		menuOpen = false;
		clearToken();
		goto(`${base}/login`);
	}

	function flipTheme(event: MouseEvent) {
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		toggleTheme({ x: box.left + box.width / 2, y: box.top + box.height / 2 });
	}

	function onKeydown(event: KeyboardEvent) {
		if (menuOpen) {
			if (event.key === 'Escape') {
				event.preventDefault();
				closeMenu();
			} else if (event.key === 'Tab') {
				const focusable = menuFocusables();
				const first = focusable[0];
				const last = focusable[focusable.length - 1];
				if (!first || !last) return;

				const active = document.activeElement as HTMLElement;
				if (event.shiftKey && (active === first || !focusable.includes(active))) {
					event.preventDefault();
					last.focus({ preventScroll: true });
				} else if (!event.shiftKey && (active === last || !focusable.includes(active))) {
					event.preventDefault();
					first.focus({ preventScroll: true });
				}
			}
			return;
		}

		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			if (!showNav) return;
			event.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	function onResize() {
		if (!menuOpen || !window.matchMedia('(min-width: 64rem)').matches) return;

		const focusWasInNavigation = primaryNavigation?.contains(document.activeElement) ?? false;
		menuOpen = false;
		if (!focusWasInNavigation) {
			const currentPage =
				primaryNavigation?.querySelector<HTMLElement>('a[aria-current="page"]') ??
				primaryNavigation?.querySelector<HTMLElement>('a[href]');
			currentPage?.focus({ preventScroll: true });
		}
	}

	async function toggleMenu() {
		if (menuOpen) {
			closeMenu();
			return;
		}

		menuOpen = true;
		await tick();
		primaryNavigation?.querySelector<HTMLElement>('a[href], button:not(:disabled)')?.focus({
			preventScroll: true
		});
	}

	function closeMenu() {
		menuOpen = false;
		menuToggle?.focus({ preventScroll: true });
	}

	function menuFocusables() {
		return [
			...(menuToggle ? [menuToggle] : []),
			...(primaryNavigation
				? Array.from(primaryNavigation.querySelectorAll<HTMLElement>('a[href], button:not(:disabled)'))
				: [])
		].filter((element) => element.isConnected && element.getClientRects().length > 0);
	}
</script>

<svelte:window onkeydown={onKeydown} onresize={onResize} />

<a class="skip-link" href="#main" inert={menuOpen}>Skip to content</a>

<div class="shell" class:with-nav={showNav} class:in-session={inSession} class:menu-open={menuOpen}>
	<div class="sidebar">
		<header class="top">
			<a class="brand" href={`${base}/today`} inert={menuOpen}>
				<Emblem />
				<span class="brand-name">Medical Learning OS</span>
			</a>
			{#if showNav}
				<button
					class="jump"
					type="button"
					aria-label="Quick jump"
					aria-keyshortcuts="Control+K Meta+K"
					data-testid="quick-jump"
					inert={menuOpen}
					onclick={() => (paletteOpen = true)}
				>
					<Icon name="jump" size={18} />
					<span class="jump-label">Quick jump</span>
					<kbd class="kbd">{shortcut}</kbd>
				</button>
				<button
					class="btn menu-toggle"
					type="button"
					bind:this={menuToggle}
					aria-expanded={menuOpen}
					aria-controls="primary-navigation"
					onclick={toggleMenu}
				>
					<Icon name={menuOpen ? 'close' : 'menu'} size={18} />
					{menuOpen ? 'Close menu' : 'Menu'}
				</button>
			{/if}
		</header>

		{#if showNav}
			<nav
				id="primary-navigation"
				class="rail"
				class:open={menuOpen}
				aria-label="Main navigation"
				bind:this={primaryNavigation}
			>
				{#each NAV_GROUPS as group (group)}
					<div class="rail-group">
						<p class="rail-label" id="rail-{group}">{group}</p>
						<ul class="bare-list" aria-labelledby="rail-{group}">
							{#each NAV.filter((item) => item.group === group) as item (item.href)}
								{@const active = isActive(path, item.href)}
								<li>
									<a
										class="rail-link"
										href={`${base}${item.href}`}
										aria-current={active ? 'page' : undefined}
										data-testid={item.testid}
										onclick={() => (menuOpen = false)}
									>
										{#if active}<span class="rail-indicator" aria-hidden="true"></span>{/if}
										<Icon name={item.icon} />
										<span class="rail-text">{item.label}</span>
									</a>
								</li>
							{/each}
						</ul>
					</div>
				{/each}
				<div class="rail-foot">
					<button
						class="rail-link"
						type="button"
						aria-pressed={theme.light}
						data-testid="theme-toggle"
						onclick={flipTheme}
					>
						<Icon name={theme.light ? 'sun' : 'moon'} />
						<span class="rail-text">Light reading</span>
						<span class="switch" aria-hidden="true"></span>
					</button>
					<button class="rail-link" type="button" onclick={signOut}>
						<Icon name="logout" />
						<span class="rail-text">Sign out</span>
					</button>
				</div>
			</nav>
			<!-- Pointer convenience only; Escape and the toggle close the sheet. -->
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
			<div class="scrim" aria-hidden="true" onclick={closeMenu}></div>
		{/if}
	</div>

	<main id="main" class:enter-stagger={!inSession} class:wide inert={menuOpen}>
		{@render children()}
	</main>

	{#if showNav}
		<nav class="tabbar" aria-label="Quick navigation" inert={menuOpen}>
			{#each tabs as item (item.href)}
				{@const active = isActive(path, item.href)}
				<a
					class="tab"
					href={`${base}${item.href}`}
					aria-current={active ? 'page' : undefined}
					data-testid={`tab-${item.href.slice(1)}`}
				>
					<span class="tab-icon">
						{#if active}<span class="tab-indicator" aria-hidden="true"></span>{/if}
						<Icon name={item.icon} size={22} />
					</span>
					<span class="tab-label">{item.label}</span>
				</a>
			{/each}
		</nav>
	{/if}
</div>

{#if showNav}
	<CommandPalette bind:open={paletteOpen} entries={paletteEntries} />
{/if}
