import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

test('one-tap tutoring cards stay available from the saved session when API is offline', async ({
	page
}) => {
	const sessionId = 'ai18-offline-session';
	const versionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
	const cards = [
		{
			prompt_type: 'explain',
			content: 'Simple version: The receptor step is blocked.',
			source_ref: 'Reviewed reference 12'
		},
		{
			prompt_type: 'why_wrong',
			content: 'Compare your selected option with the keyed answer using the feedback rationales.',
			source_ref: 'Reviewed reference 12'
		},
		{
			prompt_type: 'compare',
			content: 'Compare the receptor and synthesis steps.',
			source_ref: 'Reviewed reference 12'
		},
		{
			prompt_type: 'mnemonic',
			content: 'Block the receptor; the level can stay normal.',
			source_ref: 'Reviewed reference 12'
		},
		{
			prompt_type: 'test_me',
			content:
				'Recall: State the key learning point for this question.\nAnswer: Receptor action can change independently of level.',
			source_ref: 'Reviewed reference 12'
		}
	];
	let apiOffline = false;

	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route(`**/v1/practice/sessions/${sessionId}`, async (route) => {
		if (apiOffline) {
			await route.abort();
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				session_id: sessionId,
				preset: 'tutor',
				status: 'open',
				time_limit_seconds: null,
				deadline: null,
				server_now: null,
				items: [
					{
						item_index: 0,
						question_version_id: versionId,
						vignette: 'A fictional model has a receptor blocker.',
						lead_in: 'Which step explains the reduced effect?',
						difficulty: 'medium',
						options: [
							{ text: 'Receptor action falls', rationale: 'The receptor is blocked.' },
							{ text: 'Synthesis rises', rationale: 'No evidence supports this.' }
						],
						answered: true,
						chosen_index: 1,
						correct: false,
						correct_index: 0,
						key_learning_point: 'Receptor action can change independently of level.',
						exam_tip: null,
						hint_available: false,
						report_status: null,
						tutoring_cards: cards
					}
				]
			})
		});
	});
	await page.route('**/v1/me/marks', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ marks: [] }) })
	);
	await page.route('**/v1/notes', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ notes: [] }) })
	);

	await page.goto(`/session/${sessionId}`);
	await page.getByTestId('pregen-card-explain').click();
	await expect(page.getByTestId('tutor-card-content')).toContainText('receptor step is blocked');
	await page.getByTestId('pregen-card-why_wrong').click();
	await expect(page.getByTestId('tutor-card-content')).toContainText('No evidence supports this.');
	await expect(page.getByTestId('tutor-card-content')).toContainText('The receptor is blocked.');
	await expect(page.getByTestId('tutor-card-content')).toHaveAttribute('aria-live', 'polite');
	await page.getByTestId('pregen-card-test_me').click();
	await expect(page.getByTestId('tutor-card-content')).toContainText(
		'State the key learning point for this question.'
	);
	await expect(page.getByTestId('tutor-card-content')).not.toContainText(
		'Receptor action can change independently of level.'
	);
	await page.getByTestId('reveal-tutor-answer').click();
	await expect(page.getByTestId('tutor-card-content')).toContainText(
		'Receptor action can change independently of level.'
	);
	const saved = await page.evaluate((sid) => {
		const draft = JSON.parse(localStorage.getItem(`mlos_session_${sid}`) ?? 'null');
		return draft?.session?.items?.[0]?.tutoring_cards?.length ?? 0;
	}, sessionId);
	expect(saved).toBe(5);

	apiOffline = true;
	await page.addInitScript(() =>
		Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false })
	);
	await page.reload();
	await page.getByTestId('pregen-card-mnemonic').click();
	await expect(page.getByTestId('tutor-card-content')).toContainText('Block the receptor');
});
