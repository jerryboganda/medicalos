import { expect, test } from '@playwright/test';

// GROW-01 in the browser: the community page renders share cards from the
// learner's real record, and every card on the page matches the share-cards
// API exactly (page == API). Nothing is mocked here — the cards appear only
// once real answered attempts exist, and the honest unavailability reasons
// show before that.

const API = 'http://127.0.0.1:8080';

test('community page renders share cards from real data', async ({ page }) => {
	const email = `e2e-share-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const register = await fetch(`${API}/v1/auth/register`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password: 'longenough' })
	});
	expect(register.ok).toBeTruthy();
	const { user_id: userId } = await register.json();
	const login = await fetch(`${API}/v1/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password: 'longenough' })
	});
	const { token } = await login.json();
	const auth = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };

	await page.addInitScript(([stored]) => {
		localStorage.setItem('mlos_token', stored as string);
	}, [token]);

	// Fresh learner: the honest unavailability reasons, no cards.
	await page.goto('/community');
	await expect(page.getByTestId('share-unavailable-score')).toBeVisible();
	await expect(page.getByTestId('share-unavailable-consistency')).toBeVisible();

	// Two real answered attempts through the API (meets the QB-15 minimum).
	const today = await (await fetch(`${API}/v1/me/today`, { headers: auth })).json();
	const chapterId = today['tasks'][0]['chapter_id'];
	const session = await (
		await fetch(`${API}/v1/practice/sessions`, {
			method: 'POST',
			headers: auth,
			body: JSON.stringify({
				preset: 'tutor',
				chapter_id: chapterId,
				question_count: 2
			})
		})
	).json();
	const sid = session['session_id'];
	for (let index = 0; index < 2; index += 1) {
		const answer = await fetch(`${API}/v1/practice/sessions/${sid}/answers`, {
			method: 'POST',
			headers: auth,
			body: JSON.stringify({
				item_index: index,
				chosen_index: 0,
				idempotency_key: `share-e2e-${index}`
			})
		});
		expect(answer.ok).toBeTruthy();
	}
	// Refresh the engagement record so the streak state is current.
	await fetch(`${API}/v1/me/engagement`, { headers: auth });

	// The API is the source of truth for what the page must show.
	const share = await (await fetch(`${API}/v1/me/share-cards`, { headers: auth })).json();
	const scoreCard = share['cards'].find((card: { kind: string }) => card.kind === 'score');
	expect(scoreCard, 'two answered attempts produce a score card').toBeTruthy();

	// The page renders exactly what the API returned.
	await page.reload();
	await expect(page.getByTestId('share-score')).toBeVisible();
	await expect(page.getByTestId('share-score')).toContainText(scoreCard['headline']);

	const consistencyCard = share['cards'].find(
		(card: { kind: string }) => card.kind === 'consistency'
	);
	if (consistencyCard) {
		await expect(page.getByTestId('share-consistency')).toContainText(
			consistencyCard['headline']
		);
	} else {
		await expect(page.getByTestId('share-unavailable-consistency')).toBeVisible();
	}

	// The share text never carries question content (GROW-01).
	await expect(page.getByTestId('share-score')).not.toContainText(/vignette|lead_in|correct_index/i);
});
