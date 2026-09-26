import { expect, test, type Page } from '@playwright/test';

// EX-08 in the browser: the timer derives from the server-issued deadline
// (server_now skew correction), and the session auto-submits at zero.
// The e2e CI job runs the API with MIN_TIME_LIMIT_SECONDS=5; the 15-second
// session leaves time to verify integrity signals before auto-submit.

const API = 'http://127.0.0.1:8080';

async function expectIntegritySignal(
	page: Page,
	sessionId: string,
	signalType: string,
	trigger: () => Promise<unknown>
) {
	const responsePromise = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === signalType
	);
	await trigger();
	const response = await responsePromise;
	expect(response.ok()).toBeTruthy();
	expect(response.request().postDataJSON()).toMatchObject({
		session_id: sessionId,
		signal_type: signalType
	});
}

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
			time_limit_seconds: 15
		})
	});
	expect(sessionRes.ok).toBeTruthy();
	const { session_id: sessionId } = await sessionRes.json();

	// Act through the real UI, authenticated via the same token.
	await page.addInitScript(
		(t) => localStorage.setItem('mlos_token', t),
		token
	);
	await page.goto(`/session/${sessionId}`);

	await expect(page.getByText(/Question 1 of/)).toBeVisible();
	await expect(page.getByTestId('timer')).toContainText(/left/);

	await expectIntegritySignal(page, sessionId, 'window_blur', () =>
		page.evaluate(() => window.dispatchEvent(new Event('blur')))
	);
	await expectIntegritySignal(page, sessionId, 'clock_change', () =>
		page.evaluate(() => {
			const realNow = Date.now.bind(Date);
			Date.now = () => realNow() + 60_000;
		})
	);
	await expectIntegritySignal(page, sessionId, 'background', () =>
		page.evaluate(() => {
			Object.defineProperty(document, 'visibilityState', {
				configurable: true,
				value: 'hidden'
			});
			document.dispatchEvent(new Event('visibilitychange'));
		})
	);
	await expectIntegritySignal(page, sessionId, 'foreground', () =>
		page.evaluate(() => {
			Reflect.deleteProperty(document, 'visibilityState');
			document.dispatchEvent(new Event('visibilitychange'));
		})
	);
	await page.route('**/v1/integrity-events', async (route) => {
		if (route.request().postDataJSON()?.signal_type !== 'foreground') {
			await route.continue();
			return;
		}
		const upstream = await route.fetch();
		const body = await upstream.json();
		await route.fulfill({
			response: upstream,
			json: { ...body, action: 'warn', away_seconds: 30 }
		});
	});
	await expectIntegritySignal(page, sessionId, 'foreground', () =>
		page.evaluate(() => window.dispatchEvent(new Event('focus')))
	);
	await expect(page.getByTestId('integrity-warning')).toContainText(/away for/i);
	await page.unroute('**/v1/integrity-events');
	await page.evaluate(() => {
		Object.defineProperty(document, 'fullscreenElement', {
			configurable: true,
			value: document.documentElement
		});
		document.dispatchEvent(new Event('fullscreenchange'));
	});
	await expectIntegritySignal(page, sessionId, 'fullscreen_exit', () =>
		page.evaluate(() => {
			Object.defineProperty(document, 'fullscreenElement', {
				configurable: true,
				value: null
			});
			document.dispatchEvent(new Event('fullscreenchange'));
		})
	);
	await page.evaluate(() => Reflect.deleteProperty(document, 'fullscreenElement'));
	await page.route('**/v1/integrity-events', (route) =>
		route.fulfill({ status: 503, body: 'temporarily unavailable' })
	);
	const failedSignal = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'window_blur'
	);
	await page.evaluate(() => window.dispatchEvent(new Event('blur')));
	expect((await failedSignal).status()).toBe(503);
	await page.unroute('**/v1/integrity-events');

	// Timed exam-style sessions defer answer feedback until submission.
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('assessment-recorded')).toBeVisible();
	await expect(page.getByTestId('feedback')).toHaveCount(0);

	// Auto-submit fires when the countdown reaches zero.
	await expect(page.getByTestId('results')).toBeVisible({ timeout: 30_000 });
	await expect(page.getByTestId('results')).toContainText('not a prediction');
});

test('session UI displays the server auto-submit receipt after a return signal', async ({
	page
}) => {
	const email = `e2e-integrity-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
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
	const sessionRes = await fetch(`${API}/v1/practice/sessions`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			preset: 'timed',
			chapter_id: today.tasks[0].chapter_id,
			question_count: 2,
			time_limit_seconds: 120
		})
	});
	expect(sessionRes.ok).toBeTruthy();
	const { session_id: sessionId } = await sessionRes.json();

	let receipt: Record<string, unknown> | null = null;
	let deliverReceipt = false;
	await page.route('**/v1/integrity-events', async (route) => {
		if (
			route.request().postDataJSON()?.signal_type !== 'foreground' ||
			!deliverReceipt ||
			!receipt
		) {
			await route.continue();
			return;
		}
		deliverReceipt = false;
		const upstream = await route.fetch();
		const body = await upstream.json();
		await route.fulfill({
			response: upstream,
			json: { ...body, action: 'auto_submitted', receipt }
		});
	});
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto(`/session/${sessionId}`);
	await expect(page.getByText(/Question 1 of/)).toBeVisible();

	// Use the persisted API receipt while the page still holds its open session
	// state, then exercise the same return-signal response path as a mock policy.
	const submitted = await fetch(`${API}/v1/practice/sessions/${sessionId}/submit`, {
		method: 'POST',
		headers: authHeaders
	});
	expect(submitted.ok).toBeTruthy();
	receipt = await submitted.json();

	const foreground = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'foreground'
	);
	deliverReceipt = true;
	await page.evaluate(() => window.dispatchEvent(new Event('focus')));
	expect((await foreground).ok()).toBeTruthy();
	await expect(page.getByTestId('results')).toBeVisible();
	await expect(page.getByTestId('integrity-auto-submitted')).toContainText(/time-away limit/i);
});
