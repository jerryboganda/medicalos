import { expect, test } from '@playwright/test';

const API = 'http://127.0.0.1:8080';

// EX-07 in the browser: the seeded mock's form is deterministic (both
// chapter-1 questions; keys A and B) so answering A on both yields exactly
// 50% = pass, with the community percentile honestly hidden.

test('mock flow: start, deferred feedback, pass, percentile hidden', async ({
	page
}) => {
	const email = `e2e-mock-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;

	const register = await fetch(`${API}/v1/auth/register`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password: 'correct horse battery' })
	});
	expect(register.ok).toBeTruthy();
	const login = await fetch(`${API}/v1/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password: 'correct horse battery' })
	});
	const { token } = await login.json();

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/today');

	// Start the seeded mock from the Today page.
	await expect(page.getByTestId('mock-card')).toBeVisible();
	await page.getByTestId('mock-start').click();

	// Exam-style: nothing is revealed after recording.
	await expect(page.getByText(/Question 1 of/)).toBeVisible();
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('mock-recorded')).toBeVisible();
	await expect(page.getByTestId('feedback')).toHaveCount(0);

	await page.getByTestId('next').click();
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await page.getByTestId('submit-session').click();

	await expect(page.getByTestId('mock-result')).toBeVisible();
	await expect(page.getByTestId('score')).toHaveText('50%');
	await expect(page.getByTestId('mock-result')).toContainText('Passed');
	await expect(page.getByTestId('mock-result')).toContainText(
		'nothing is invented meanwhile'
	);
});
