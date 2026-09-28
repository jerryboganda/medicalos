import { expect, test } from '@playwright/test';

const API = process.env.E2E_API_BASE ?? 'http://127.0.0.1:8080';

test('session tools autosave notes and marks and replay an offline answer', async ({
	page
}) => {
	const email = `e2e-tools-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
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
	const auth = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const today = await fetch(`${API}/v1/me/today`, { headers: auth }).then((r) => r.json());
	const sessionResponse = await fetch(`${API}/v1/practice/sessions`, {
		method: 'POST',
		headers: auth,
		body: JSON.stringify({
			preset: 'tutor',
			chapter_id: today.tasks[0].chapter_id,
			question_count: 2
		})
	});
	expect(sessionResponse.ok).toBeTruthy();
	const { session_id: sessionId } = await sessionResponse.json();

	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript((value) => localStorage.setItem('mlos_token', value), token);
	await page.goto(`/session/${sessionId}`);
	await expect(page.getByTestId('session-workspace')).toBeVisible();

	await page.getByTestId('text-size-3').click();
	await page.getByTestId('tools-toggle').click();
	await page.getByRole('combobox', { name: 'Medical calculator' }).selectOption('bmi');
	await page.getByTestId('calc-input-weight_kg').fill('70');
	await page.getByTestId('calc-input-height_m').fill('1.75');
	await page.getByTestId('calculate').click();
	await expect(page.getByTestId('calculator-result')).toContainText('22.86');
	await expect(page.getByTestId('calculator-disclaimer')).toContainText('not for clinical use');
	await page.getByLabel('Conversion type').selectOption('glucose');
	await page.getByTestId('conversion-value').fill('100');
	await page.getByTestId('convert').click();
	await expect(page.getByTestId('conversion-result')).toContainText('5.550 mmol/L');

	await page.getByTestId('question-mark').click();
	await expect(page.getByTestId('question-mark')).toContainText('Remove mark');
	await page.getByTestId('question-note').fill('Review this mechanism later.');
	await expect(page.getByTestId('note-save-state')).toHaveText('Saved', { timeout: 10_000 });

	await page.context().setOffline(true);
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('offline-answer')).toBeVisible();
	await page.getByTestId('next').click();
	await page.getByTestId('option-1').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('offline-answer')).toBeVisible();
	const draft = await page.evaluate((sid) => localStorage.getItem(`mlos_session_${sid}`), sessionId);
	expect(Object.keys(JSON.parse(draft).pendingAnswers)).toHaveLength(2);
	await page.context().setOffline(false);
	await expect(page.getByTestId('feedback')).toBeVisible({ timeout: 15_000 });

	const noteRows = await fetch(`${API}/v1/notes`, { headers: auth }).then((r) => r.json());
	expect(noteRows.notes).toEqual(
		expect.arrayContaining([
			expect.objectContaining({ body: 'Review this mechanism later.' })
		])
	);
	const marks = await fetch(`${API}/v1/me/marks`, { headers: auth }).then((r) => r.json());
	expect(marks.marks).toHaveLength(1);
	const overflow = await page.evaluate(
		() => document.documentElement.scrollWidth > document.documentElement.clientWidth
	);
	expect(overflow).toBe(false);
});
