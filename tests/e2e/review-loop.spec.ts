import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

const API = 'http://127.0.0.1:8080';

// SR-01/02 in the browser: deck and cards arrive through the API, the
// learner reviews through the real UI, and the rating persists (the card
// leaves the queue and the schedule moves it forward).

test('flashcard review loop rates a card through the UI', async ({ page }) => {
	const email = `e2e-review-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(
		email,
		'correct horse battery',
		`review-${Date.now()}`
	);
	const authHeaders = {
		authorization: `Bearer ${token}`,
		'content-type': 'application/json'
	};

	const deckRes = await fetch(`${API}/v1/decks`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({ name: 'E2E fixture deck' })
	});
	expect(deckRes.ok).toBeTruthy();
	const { deck_id: deckId } = await deckRes.json();
	await fetch(`${API}/v1/decks/${deckId}/cards`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			front: 'Fictional prompt: what does FSRS optimize?',
			back: 'Requested retention per unit of review time — nothing mystical.'
		})
	});

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto('/review');

	// New card surfaces; reveal, rate Good, reach the caught-up state.
	await expect(page.getByTestId('review-card')).toContainText('Fictional prompt');
	await page.getByTestId('reveal').click();
	await expect(page.getByTestId('back')).toContainText('Requested retention');
	const eventsPost = page.waitForResponse(
		(r) => r.url().includes('/v1/reviews/events') && r.request().method() === 'POST'
	);
	await page.getByTestId('rate-good').click();
	const resp = await eventsPost;
	expect(resp.status(), 'review event POST status').toBe(200);
	await expect(page.getByTestId('all-caught-up')).toBeVisible({ timeout: 10_000 });
});
