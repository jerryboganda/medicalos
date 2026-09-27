// Light reading theme (§7.3). Dark stays the product default; the choice
// persists per device. app.html applies it before first paint (no flash);
// this module keeps the in-app toggle in sync.

const KEY = 'mlos_theme';

export const theme = $state({ light: false });

export function loadTheme(): void {
	theme.light = document.documentElement.dataset.theme === 'light';
}

function apply(light: boolean): void {
	theme.light = light;
	if (light) document.documentElement.dataset.theme = 'light';
	else delete document.documentElement.dataset.theme;
	// Browser/OS chrome (mobile status bar, PWA title bar) follows the canvas.
	document
		.querySelector('meta[name="theme-color"]')
		?.setAttribute('content', getComputedStyle(document.documentElement).getPropertyValue('--color-canvas').trim());
	try {
		localStorage.setItem(KEY, light ? 'light' : 'dark');
	} catch {
		/* storage unavailable (private mode) — session-only choice */
	}
}

/** Toggle, revealing the new theme as a circle from the pressed control. */
export function toggleTheme(origin?: { x: number; y: number }): void {
	const next = !theme.light;
	const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
	if (!document.startViewTransition || reduce || !origin) {
		apply(next);
		return;
	}
	const root = document.documentElement;
	root.style.setProperty('--reveal-x', `${origin.x}px`);
	root.style.setProperty('--reveal-y', `${origin.y}px`);
	root.dataset.themeSwitch = '';
	const transition = document.startViewTransition(() => apply(next));
	transition.finished.finally(() => delete root.dataset.themeSwitch);
}
