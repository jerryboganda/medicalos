import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	timeout: 30_000,
	retries: 1,
	use: {
		baseURL: 'http://127.0.0.1:4173'
	},
	webServer: {
		// Bind explicitly to IPv4 — vite's default `localhost` can resolve to
		// ::1 on CI while the browser dials 127.0.0.1.
		command:
			'npm run preview -w @medical-os/client -- --host 127.0.0.1 --port 4173',
		cwd: '../..',
		port: 4173,
		reuseExistingServer: true,
		timeout: 60_000
	}
});
