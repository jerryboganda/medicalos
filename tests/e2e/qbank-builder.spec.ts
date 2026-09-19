import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

test('QB-12 builder is one connected screen with presets and truthful availability', async ({ page }) => {
	const email = `e2e-qb12-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `qb12-${Date.now()}`);

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/practice');

	await expect(page.getByTestId('qbank-builder')).toBeVisible();
	await expect(page.getByTestId('builder-tree')).toBeVisible();
	await expect(page.getByTestId('builder-availability')).toContainText('questions available');
	await expect(page.getByTestId('builder-pool')).toContainText('All questions');

	await page.getByTestId('builder-search').fill('Glorbin');
	await expect(page.getByTestId('builder-node')).toContainText('Glorbin');
	await page.getByTestId('builder-search').fill('');

	await page.getByTestId('builder-preset-name').fill('Easy focus');
	await page.getByTestId('builder-save-preset').click();
	await expect(page.getByTestId('builder-saved-presets')).toContainText('Easy focus');

	await page.getByTestId('builder-count-5').click();
	await expect(page.getByTestId('builder-availability')).toContainText('Only 4 questions are available');
	await page.getByTestId('builder-start').click();
	await expect(page.getByText('Question 1 of 4')).toBeVisible();
});

test('Today Quick 10 starts a real qbank session', async ({ page }) => {
	const email = `e2e-quick10-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `quick10-${Date.now()}`);

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/today');
	await page.getByTestId('quick-10').click();
	await expect(page.getByText('Question 1 of 4')).toBeVisible();
});
