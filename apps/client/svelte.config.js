import adapter from '@sveltejs/adapter-static';

// Static SPA build: one client for web/PWA and both Tauri shells
// (ADR 0003). Server-only SvelteKit features are out of scope.
// BASE_PATH supports GitHub Pages project-site hosting (/medicalos).
const base = process.env.BASE_PATH ?? '';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({ fallback: 'index.html' }),
		paths: { base }
	}
};

export default config;
