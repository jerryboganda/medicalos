import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

const API = 'http://127.0.0.1:8080';

test('QB-13 tutor session exposes connected tools and records a hint-assisted answer', async ({ page }) => {
	const email = `e2e-qb13-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `qb13-${Date.now()}`);
	const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const today = await (await fetch(`${API}/v1/me/today`, { headers })).json();
	const session = await (
		await fetch(`${API}/v1/practice/sessions`, {
			method: 'POST',
			headers,
			body: JSON.stringify({ preset: 'tutor', chapter_id: today.tasks[0].chapter_id, question_count: 1 })
		})
	).json();

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto(`/session/${session.session_id}`);
	await page.getByTestId('session-tools-open').click();
	await expect(page.getByTestId('session-tools')).toBeVisible();
	await expect(page.getByTestId('session-tools-close')).toBeFocused();
	await expect(page.getByTestId('text-size-options').getByRole('button')).toHaveCount(4);
	await page.getByTestId('text-size-options').getByRole('button', { name: 'Large' }).click();

	await page.getByTestId('converter-value').fill('10');
	await page.getByTestId('converter-kind').selectOption('cm-in');
	await expect(page.getByTestId('converter-result')).toContainText('3.94');

	await page.getByTestId('calculator-kind').selectOption('bmi');
	await page.getByTestId('calc-weight_kg').fill('70');
	await page.getByTestId('calc-height_m').fill('1.75');
	await page.getByTestId('calculator-run').click();
	await expect(page.getByTestId('calculator-result')).toContainText('22.86');

	await page.getByTestId('session-tools-close').click();
	await expect(page.getByTestId('session-tools-open')).toBeFocused();
	await page.reload();
	await page.getByTestId('session-tools-open').click();
	await expect(
		page.getByTestId('text-size-options').getByRole('button', { name: 'Large' })
	).toHaveAttribute('aria-pressed', 'true');
	await page.getByTestId('session-tools-close').click();
	await page.getByTestId('hint-open').click();
	await expect(page.getByTestId('hint')).toBeVisible();
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toBeVisible();
});
