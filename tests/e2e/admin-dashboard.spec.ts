import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-admin-user-token']);
});

const dashboard = {
	institutions: 6,
	users: 1234,
	published_questions: 785,
	articles: 54,
	rights_records: 32,
	open_incidents: 0,
	coach_turns_last_30_days: 2024
};

const settings = {
	mastery_bands: [1400, 1600],
	community_min_sample: 20,
	free_daily_questions: 10,
	free_daily_coach_turns: 1,
	retest_intervals_days: [1, 3, 7, 14],
	offline_lease_days: 14,
	max_reviews_per_day: 30,
	max_new_cards_per_day: 10,
	competition_difficulty_points: [5, 10, 15]
};

test('administrator reaches the owner dashboard and sees responsive aggregate counts', async ({ page }) => {
	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-admin-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/**', async (route) => {
		const path = new URL(route.request().url()).pathname;
		let body: unknown = {};
		if (path.endsWith('/dashboard')) body = dashboard;
		else if (path.endsWith('/settings')) body = { settings };
		else if (path.endsWith('/audit')) body = { events: [] };
		else if (path.endsWith('/reports')) body = { reports: [] };
		else if (path.endsWith('/concepts')) body = { concepts: [] };
		else if (path.endsWith('/content-rights')) body = { rights: [] };
		else if (path.endsWith('/extraction-reports')) body = { reports: [] };
		else if (path.endsWith('/pending-assessment')) body = { runs: [] };
		else if (path.endsWith('/scenario-assessment-appeals')) body = { appeals: [] };
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
	});
	// Any other API call must not hit the real API: a 401 clears the learner
	// token and the owner dashboard bounces to /login before metrics render.
	await page.route('**/v1/**', async (route) => {
		const path = new URL(route.request().url()).pathname;
		if (!path.includes('/v1/admin/')) {
			await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({}) });
			return;
		}
		await route.fallback();
	});

	await page.goto('/admin');
	await page.getByRole('link', { name: 'Owner dashboard' }).click();
	await expect(page.getByRole('heading', { name: 'Owner dashboard' })).toBeVisible();
	await expect(page.getByTestId('dashboard-metric-institutions')).toContainText('6');
	await expect(page.getByTestId('dashboard-metric-users')).toContainText('1,234');
	await expect(page.getByTestId('dashboard-metric-published_questions')).toContainText('785');
	await expect(page.getByTestId('dashboard-metric-articles')).toContainText('54');
	await expect(page.getByTestId('dashboard-metric-rights_records')).toContainText('32');
	await expect(page.getByTestId('dashboard-metric-open_incidents')).toContainText('0');
	await expect(page.getByTestId('dashboard-metric-coach_turns_last_30_days')).toContainText('2,024');
	await expect(page.getByRole('link', { name: 'Editorial console' })).toHaveAttribute('href', '/admin');

	for (const width of [320, 375, 414, 768, 1280]) {
		await page.setViewportSize({ width, height: 900 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
	}
});

test('dashboard requires a learner session before asking for the admin token', async ({ page }) => {
	await page.goto('/admin/dashboard');
	await expect(page).toHaveURL(/\/login/);
});

test('dashboard requests the admin token and recovers from access failure', async ({ page }) => {
	let requestCount = 0;
	const sentAdminTokens: string[] = [];
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-admin-user-token'));
	await page.route('**/v1/admin/dashboard', async (route) => {
		requestCount += 1;
		sentAdminTokens.push(route.request().headers()['x-admin-token'] ?? '');
		if (requestCount === 1) {
			await route.fulfill({
				status: 403,
				contentType: 'application/json',
				body: JSON.stringify({ error: { code: 'admin_required', message: 'Admin token was rejected.' } })
			});
			return;
		}
		if (requestCount === 2) {
			await route.fulfill({
				status: 503,
				contentType: 'application/json',
				body: JSON.stringify({ error: { code: 'unavailable', message: 'Dashboard request failed temporarily.' } })
			});
			return;
		}
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(dashboard) });
	});

	await page.goto('/admin/dashboard');
	await expect(page.getByLabel('Admin token')).toBeVisible();
	expect(requestCount).toBe(0);
	await page.getByLabel('Admin token').fill('invalid-admin-token');
	await page.getByRole('button', { name: 'Unlock dashboard' }).click();
	await expect(page.getByRole('alert')).toContainText('Admin token was rejected.');
	expect(sentAdminTokens).toEqual(['invalid-admin-token']);
	await expect(page.getByTestId('dashboard-metrics')).toBeHidden();
	await expect(page.getByRole('button', { name: 'Change admin token' })).toBeVisible();

	await page.getByRole('button', { name: 'Change admin token' }).click();
	await page.getByLabel('Admin token').fill('valid-admin-token');
	await page.getByRole('button', { name: 'Unlock dashboard' }).click();
	await expect(page.getByRole('alert')).toContainText('Dashboard request failed temporarily.');
	expect(sentAdminTokens).toEqual(['invalid-admin-token', 'valid-admin-token']);

	await page.getByRole('button', { name: 'Retry' }).click();
	await expect(page.getByTestId('dashboard-metric-open_incidents')).toContainText('0');
});
