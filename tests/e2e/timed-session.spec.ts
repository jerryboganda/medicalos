import { expect, test } from '@playwright/test';

// EX-08 in the browser: the timer derives from the server-issued deadline
// (server_now skew correction), and the session auto-submits at zero.
// The e2e CI job runs the API with MIN_TIME_LIMIT_SECONDS=5; this test
// creates a 5-second timed session through the API, then drives the real
// UI against it.

const API = 'http://127.0.0.1:8080';

test('timed session shows the server countdown and auto-submits', async ({
	page
}) => {
	const email = `e2e-timed-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;

	// Arrange the account and the short timed session through the API.
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
	const authHeaders = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };

	const todayRes = await fetch(`${API}/v1/me/today`, { headers: authHeaders });
	const today = await todayRes.json();
	const chapterId = today.tasks[0].chapter_id;

	const sessionRes = await fetch(`${API}/v1/practice/sessions`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			preset: 'timed',
			chapter_id: chapterId,
			question_count: 2,
			time_limit_seconds: 5
		})
	});
	expect(sessionRes.ok()).toBeTruthy();
	const { session_id: sessionId } = await sessionRes.json();

	// Act through the real UI, authenticated via the same token.
	await page.addInitScript(
		(t) => localStorage.setItem('mlos_token', t),
		token
	);
	await page.goto(`/session/${sessionId}`);

	await expect(page.getByText(/Question 1 of/)).toBeVisible();
	await expect(page.getByTestId('timer')).toContainText(/left/);

	// Answer the first item so the auto-submit has real content.
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toBeVisible();

	// Auto-submit fires when the countdown reaches zero.
	await expect(page.getByTestId('results')).toBeVisible({ timeout: 20_000 });
	await expect(page.getByTestId('results')).toContainText('not a prediction');
});
