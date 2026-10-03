import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

test('notification preferences persist source correction updates', async ({ page }) => {
	let contentUpdates = false;
	let savedContentUpdates: boolean | undefined;
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/notifications', async (route) => {
		if (route.request().method() === 'PATCH') {
			savedContentUpdates = (route.request().postDataJSON() as { content_updates: boolean })
				.content_updates;
			contentUpdates = savedContentUpdates;
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ updated: true })
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				notifications: [],
				preferences: {
					plan_reminders: true,
					mock_results: true,
					reports: true,
					content_updates: contentUpdates,
					quiet_hours_start: 22,
					quiet_hours_end: 7
				}
			})
		});
	});

	await page.goto('/notifications');
	await page.getByRole('button', { name: 'Edit notification preferences' }).click();
	const sourceUpdates = page.getByLabel('Source and content corrections');
	await expect(sourceUpdates).not.toBeChecked();
	await sourceUpdates.check();
	await page.getByRole('button', { name: 'Save preferences' }).click();
	await expect.poll(() => savedContentUpdates).toBe(true);

	await page.reload();
	await page.getByRole('button', { name: 'Edit notification preferences' }).click();
	await expect(page.getByLabel('Source and content corrections')).toBeChecked();
});
