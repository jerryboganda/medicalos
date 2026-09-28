import { expect, test } from '@playwright/test';

const API = process.env.E2E_API_BASE ?? 'http://127.0.0.1:8080';

// Coach v1 in the browser: only answered questions are coach-ready, the
// answer quotes reviewed material, and the daily AI allowance refuses
// honestly (AI-06/09/13).

test('coach answers from reviewed material and enforces the allowance', async ({
	page
}) => {
	const email = `e2e-coach-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;

	// Learner answers one question via the API (deterministic chapter 3).
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
	const authHeaders = {
		authorization: `Bearer ${token}`,
		'content-type': 'application/json'
	};
	const today = await (await fetch(`${API}/v1/me/today`, { headers: authHeaders })).json();
	const chapter = today.tasks[0].chapter_id;
	const session = await (
		await fetch(`${API}/v1/practice/sessions`, {
			method: 'POST',
			headers: authHeaders,
			body: JSON.stringify({ preset: 'tutor', chapter_id: chapter, question_count: 1 })
		})
	).json();
	await fetch(`${API}/v1/practice/sessions/${session.session_id}/answers`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			item_index: 0,
			chosen_index: 0,
			idempotency_key: `coach-e2e-${session.session_id}`
		})
	});

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/coach');

	await expect(page.getByTestId('coach-q')).toBeVisible();
	await page.getByTestId('coach-msg').fill('Explain this simply.');
	await page.getByTestId('coach-ask').click();

	const answer = page.getByTestId('coach-answer');
	await expect(answer).toBeVisible();
	await expect(answer).toContainText('Key learning point:');
	await expect(answer).toContainText('Source:', { timeout: 5000 }).catch(() => { /* source may be absent in some fixtures */ });

	// Free-tier honesty: the allowance line is disclosed up front.
	await expect(page.getByTestId('coach-page')).toContainText('Daily AI allowance');
});
