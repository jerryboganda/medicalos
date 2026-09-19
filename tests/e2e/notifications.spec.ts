import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

test('Account persists notification preferences and shows an honest empty inbox', async ({ page }) => {
	const email = `e2e-notifications-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(
		email,
		'correct horse battery',
		`notifications-${Date.now()}`
	);

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/account');

	await expect(page.getByTestId('notification-settings')).toBeVisible();
	await expect(page.getByTestId('notification-timezone')).toHaveValue('UTC');
	await expect(page.getByTestId('notification-empty')).toHaveText('No notifications yet.');

	await page.getByTestId('notification-timezone').fill('Asia/Karachi');
	await page.getByTestId('quiet-start').fill('22:30');
	await page.getByTestId('quiet-end').fill('07:00');
	await page.getByTestId('notify-plan-review').uncheck();
	await page.getByTestId('save-notification-settings').click();
	await expect(page.getByRole('status')).toContainText('Notification settings saved.');

	await page.reload();
	await expect(page.getByTestId('notification-timezone')).toHaveValue('Asia/Karachi');
	await expect(page.getByTestId('quiet-start')).toHaveValue('22:30');
	await expect(page.getByTestId('quiet-end')).toHaveValue('07:00');
	await expect(page.getByTestId('notify-plan-review')).not.toBeChecked();
	await expect(page.getByTestId('notification-empty')).toHaveText('No notifications yet.');
});

test('Account does not present a failed inbox request as an empty inbox', async ({ page }) => {
	const email = `e2e-notifications-error-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(
		email,
		'correct horse battery',
		`notifications-error-${Date.now()}`
	);

	await page.route('**/v1/notifications', async (route) => {
		await route.fulfill({
			status: 500,
			contentType: 'application/json',
			body: JSON.stringify({ error: { code: 'test_failure', message: 'test failure' } })
		});
	});
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/account');

	await expect(page.getByTestId('notification-unavailable')).toHaveText(
		'Notifications could not be loaded.'
	);
	await expect(page.getByTestId('notification-settings-unavailable')).toHaveText(
		'Notification settings could not be loaded.'
	);
	await expect(page.getByTestId('notification-timezone')).toHaveCount(0);
	await expect(page.getByTestId('notification-empty')).toHaveCount(0);
});
