import { expect, test } from '@playwright/test';

test('authenticated learner can traverse the five primary destinations', async ({ page }) => {
	const email = `e2e-nav-${Date.now()}@example.test`;

	await page.goto('/login');
	await page.getByTestId('toggle-mode').click();
	await page.getByTestId('email').fill(email);
	await page.getByTestId('password').fill('correct horse battery');
	await page.getByTestId('submit').click();
	await expect(page.getByRole('heading', { name: 'Verify your email' })).toBeVisible();
	await page.getByTestId('submit').click();
	await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
	await page.getByTestId('submit').click();

	const primary = page.getByTestId('primary-navigation');
	await expect(primary.getByRole('link')).toHaveCount(5);

	for (const destination of ['Today', 'Practice', 'Learn', 'Coach', 'Progress']) {
		await primary.getByRole('link', { name: destination, exact: true }).click();
		await expect(page.getByRole('heading', { name: destination, exact: true })).toBeVisible();
	}
});
