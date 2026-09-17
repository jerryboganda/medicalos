import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		// Static SPA build: one client for web/PWA and both Tauri shells
		// (ADR 0003). Server-only SvelteKit features are out of scope.
		adapter: adapter({ fallback: 'index.html' })
	}
};

export default config;
