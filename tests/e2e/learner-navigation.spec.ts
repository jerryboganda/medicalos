import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

const API = 'http://127.0.0.1:8080';

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
		await expect(primary.getByRole('link', { name: destination, exact: true })).toHaveAttribute(
			'aria-current',
			'page'
		);
	}
});

test('Practice starts real tutor and timed sessions with explicit takeover', async ({ page }) => {
	const email = `e2e-practice-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `practice-${Date.now()}`);

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/practice');
	await expect(page.getByTestId('practice-task')).toBeVisible();
	await page.getByTestId('practice-tutor').first().click();
	await expect(page.getByText('Question 1 of 2')).toBeVisible();
	await expect(page.getByTestId('primary-navigation')).toHaveCount(0);

	await page.goto('/practice');
	await page.getByTestId('practice-timed').first().click();
	await expect(page.getByRole('alert')).toContainText('Another study session is already active');
	await page.getByTestId('confirm-takeover').click();
	await expect(page.getByTestId('timer')).toContainText(/left/);
});

test('Learn exposes the real spaced-review queue and existing review flow', async ({ page }) => {
	const email = `e2e-learn-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `learn-${Date.now()}`);
	const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };

	const deckRes = await fetch(`${API}/v1/decks`, {
		method: 'POST',
		headers,
		body: JSON.stringify({ name: 'CORE-05 learning queue' })
	});
	expect(deckRes.ok).toBeTruthy();
	const { deck_id: deckId } = await deckRes.json();
	const cardRes = await fetch(`${API}/v1/decks/${deckId}/cards`, {
		method: 'POST',
		headers,
		body: JSON.stringify({ front: 'CORE-05 review prompt', back: 'CORE-05 review answer' })
	});
	expect(cardRes.ok).toBeTruthy();

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/learn');
	await expect(page.getByTestId('learn-queue')).toContainText('1');
	await page.getByTestId('learn-review').click();
	await expect(page.getByTestId('review-card')).toContainText('CORE-05 review prompt');
});

test('Coach and Progress expose persisted revision and learner evidence', async ({ page }) => {
	const email = `e2e-evidence-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(email, 'correct horse battery', `evidence-${Date.now()}`);
	const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };

	const todayRes = await fetch(`${API}/v1/me/today`, { headers });
	expect(todayRes.ok).toBeTruthy();
	const today = await todayRes.json();
	const practiceTask = today.tasks.find((task) => task.kind === 'practice' && task.chapter_id);
	expect(practiceTask).toBeTruthy();

	const sessionRes = await fetch(`${API}/v1/practice/sessions`, {
		method: 'POST',
		headers,
		body: JSON.stringify({
			preset: 'tutor',
			chapter_id: practiceTask.chapter_id,
			question_count: 2
		})
	});
	expect(sessionRes.ok).toBeTruthy();
	const { session_id: sessionId } = await sessionRes.json();

	for (const itemIndex of [0, 1]) {
		const answerRes = await fetch(`${API}/v1/practice/sessions/${sessionId}/answers`, {
			method: 'POST',
			headers,
			body: JSON.stringify({
				item_index: itemIndex,
				chosen_index: 0,
				idempotency_key: `core05-${sessionId}-${itemIndex}`
			})
		});
		expect(answerRes.ok).toBeTruthy();
	}

	const submitRes = await fetch(`${API}/v1/practice/sessions/${sessionId}/submit`, {
		method: 'POST',
		headers
	});
	expect(submitRes.ok).toBeTruthy();

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/coach');
	await expect(page.getByTestId('coach-revision')).toContainText('missed question');

	await page.goto('/progress');
	await expect(page.getByTestId('progress-chapter')).toBeVisible();
	await expect(page.getByTestId('progress-chapter')).toContainText('Not enough evidence yet');
});
