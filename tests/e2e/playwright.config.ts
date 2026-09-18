import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	timeout: 30_000,
	retries: 1,
	use: {
		baseURL: 'http://127.0.0.1:4173'
	},
	webServer: {
		command: 'npm run preview -w @medical-os/client',
		cwd: '../..',
		port: 4173,
		reuseExistingServer: true,
		timeout: 60_000
	}
});
