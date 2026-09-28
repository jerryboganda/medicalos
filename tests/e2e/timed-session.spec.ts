import { expect, test, type Page } from '@playwright/test';

// EX-08 in the browser: the timer derives from the server-issued deadline
// (server_now skew correction), and the session auto-submits at zero.
// The e2e CI job runs the API with MIN_TIME_LIMIT_SECONDS=5; the 15-second
// session leaves time to verify integrity signals before auto-submit.

const API = process.env.E2E_API_BASE ?? 'http://127.0.0.1:8080';
const ADMIN_TOKEN = process.env.ADMIN_TOKEN;

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

async function registerLearner(prefix: string) {
	const email = `${prefix}-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
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
	expect(login.ok).toBeTruthy();
	const { token } = await login.json();
	return token as string;
}

async function startPolicyMockSession(
	token: string,
	policy: 'warn' | 'auto_submit'
) {
	if (!ADMIN_TOKEN) throw new Error('E2E ADMIN_TOKEN is required to create policy mocks');
	const authHeaders = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const [todayRes, examsRes] = await Promise.all([
		fetch(`${API}/v1/me/today`, { headers: authHeaders }),
		fetch(`${API}/v1/exams`, { headers: authHeaders })
	]);
	expect(todayRes.ok).toBeTruthy();
	expect(examsRes.ok).toBeTruthy();
	const today = await todayRes.json();
	const exams = await examsRes.json();
	const chapterId = today.tasks.find((task) => task.chapter_id)?.chapter_id;
	const examId = exams.exams[0]?.exam_id;
	if (!chapterId || !examId) throw new Error('E2E seed must provide a chapter and exam');

	const created = await fetch(`${API}/v1/mocks`, {
		method: 'POST',
		headers: { ...authHeaders, 'x-admin-token': ADMIN_TOKEN },
		body: JSON.stringify({
			title: `Browser ${policy} fixture`,
			exam_id: examId,
			blueprint: [{ chapter_id: chapterId, count: 2 }],
			time_limit_seconds: 180,
			pass_mark_percent: 50,
			attempts_allowed: 1,
			late_sync_grace_seconds: 0,
			integrity_policy: policy,
			away_timeout_seconds: 15
		})
	});
	expect(created.ok).toBeTruthy();
	const { mock_id: mockId } = await created.json();
	const started = await fetch(`${API}/v1/mocks/${mockId}/start`, {
		method: 'POST',
		headers: authHeaders
	});
	expect(started.ok).toBeTruthy();
	const { session_id: sessionId } = await started.json();
	return { sessionId: sessionId as string, authHeaders };
}

test('timed session shows the server countdown and auto-submits', async ({
	page
}) => {
	// Arrange the account and the short timed session through the API.
	const token = await registerLearner('e2e-timed');
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

test('mock warning is server-enforced and integrity listeners stop after leaving', async ({
	page
}) => {
	test.setTimeout(45_000);
	const token = await registerLearner('e2e-integrity-warn');
	const { sessionId, authHeaders } = await startPolicyMockSession(token, 'warn');
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto(`/session/${sessionId}`);
	await expect(page.getByText(/Question 1 of/)).toBeVisible();

	const background = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'background'
	);
	await page.evaluate(() => {
		Object.defineProperty(document, 'visibilityState', {
			configurable: true,
			value: 'hidden'
		});
		document.dispatchEvent(new Event('visibilitychange'));
	});
	expect((await background).ok()).toBeTruthy();
	await page.waitForTimeout(16_000);

	const foreground = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'foreground'
	);
	await page.evaluate(() => {
		Reflect.deleteProperty(document, 'visibilityState');
		document.dispatchEvent(new Event('visibilitychange'));
	});
	const warningResponse = await foreground;
	expect(warningResponse.ok()).toBeTruthy();
	const warning = await warningResponse.json();
	expect(warning).toMatchObject({ recorded: true, action: 'warn' });
	expect(warning.away_seconds).toBeGreaterThanOrEqual(15);
	await expect(page.getByTestId('integrity-warning')).toContainText(/away for/i);

	const session = await fetch(`${API}/v1/practice/sessions/${sessionId}`, {
		headers: authHeaders
	});
	expect((await session.json()).status).toBe('open');

	await page.getByRole('link', { name: 'Medical Learning OS' }).click();
	await expect(page).toHaveURL(/\/today$/);
	const staleListenerRequest = page
		.waitForRequest(
			(request) =>
				request.url().endsWith('/v1/integrity-events') &&
				request.postDataJSON()?.signal_type === 'window_blur',
			{ timeout: 750 }
		)
		.then(
			() => true,
			() => false
		);
	await page.evaluate(() => window.dispatchEvent(new Event('blur')));
	expect(await staleListenerRequest).toBe(false);
});

test('mock auto-submit worker returns its real persisted receipt to the session', async ({
	page
}) => {
	test.setTimeout(60_000);
	const token = await registerLearner('e2e-integrity-auto');
	const { sessionId, authHeaders } = await startPolicyMockSession(token, 'auto_submit');
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto(`/session/${sessionId}`);
	await expect(page.getByText(/Question 1 of/)).toBeVisible();

	const background = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'background'
	);
	await page.evaluate(() => {
		Object.defineProperty(document, 'visibilityState', {
			configurable: true,
			value: 'hidden'
		});
		document.dispatchEvent(new Event('visibilitychange'));
	});
	expect((await background).ok()).toBeTruthy();

	await expect
		.poll(
			async () => {
				const response = await fetch(`${API}/v1/practice/sessions/${sessionId}`, {
					headers: authHeaders
				});
				if (!response.ok) return `http_${response.status}`;
				return (await response.json()).status;
			},
			{ timeout: 35_000, intervals: [500, 1000, 2000] }
		)
		.toBe('submitted');

	const foreground = page.waitForResponse(
		(response) =>
			response.url().endsWith('/v1/integrity-events') &&
			response.request().postDataJSON()?.signal_type === 'foreground'
	);
	await page.evaluate(() => {
		Reflect.deleteProperty(document, 'visibilityState');
		document.dispatchEvent(new Event('visibilitychange'));
	});
	const response = await foreground;
	expect(response.ok()).toBeTruthy();
	const result = await response.json();
	expect(result.action).toBe('auto_submitted');
	expect(result.receipt).toBeTruthy();
	await expect(page.getByTestId('results')).toBeVisible();
	await expect(page.getByTestId('integrity-auto-submitted')).toContainText(/time-away limit/i);
});
