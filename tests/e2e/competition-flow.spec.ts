import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test, type Page } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

const competitionId = 'competition-1';
const firstQuestionId = 'question-1';
const secondQuestionId = 'question-2';

function event(competition_id: string, title: string, starts_at: string, ends_at: string) {
	return {
		competition_id,
		title,
		exam_id: 'exam-1',
		exam: 'OET Medicine',
		cadence: 'one_off',
		series_id: null,
		starts_at,
		ends_at,
		status: 'open',
		entered: false,
		attempt_status: null
	};
}

async function mockPracticeBasics(page: Page, optedIn: boolean) {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/mocks', (route) =>
		route.fulfill({ status: 200, json: { mocks: [] } })
	);
	await page.route('**/v1/me/curriculum', (route) =>
		route.fulfill({ status: 200, json: { chapters: [] } })
	);
	await page.route('**/v1/community/me', (route) =>
		route.fulfill({
			status: 200,
			json: optedIn ? { opted_in: true, handle: 'learner-1' } : { opted_in: false }
		})
	);
	await page.route('**/v1/leagues/*', (route) =>
		route.fulfill({ status: 200, json: { joined: false } })
	);
}

test('Practice routes nonparticipants through the optional community opt-in', async ({ page }) => {
	await mockPracticeBasics(page, false);
	await page.route('**/v1/competitions', (route) =>
		route.fulfill({
			status: 200,
			json: {
				competitions: [
					event(
						competitionId,
						'Open event',
						new Date(Date.now() - 60_000).toISOString(),
						new Date(Date.now() + 86_400_000).toISOString()
					),
					event(
						'competition-upcoming',
						'Upcoming event',
						new Date(Date.now() + 86_400_000).toISOString(),
						new Date(Date.now() + 172_800_000).toISOString()
					)
				]
			}
		})
	);

	await page.goto('/practice');
	await expect(page.getByTestId('competition-status').nth(0)).toHaveText('Open');
	await expect(page.getByTestId('competition-status').nth(1)).toHaveText('Upcoming');
	const optInLink = page.getByRole('link', { name: 'Set up in Community' });
	await expect(optInLink).toBeVisible();
	await optInLink.click();
	await expect(page).toHaveURL(/\/community$/);
});

test('competition answers advance once, retry safely, and show the recorded result', async ({ page }) => {
	await mockPracticeBasics(page, true);
	await page.route('**/v1/competitions', (route) =>
		route.fulfill({
			status: 200,
			json: {
				competitions: [
					event(
						competitionId,
						'Opt-in knowledge check',
						new Date(Date.now() - 60_000).toISOString(),
						new Date(Date.now() + 86_400_000).toISOString()
					)
				]
			}
		})
	);
	await page.route(`**/v1/competitions/${competitionId}/entry`, (route) =>
		route.fulfill({
			status: 200,
			json: {
				attempt_id: 'attempt-1',
				submitted: false,
				question: {
					question_version_id: firstQuestionId,
					question_number: 1,
					total_questions: 2,
					vignette: 'First clinical vignette',
					lead_in: 'Choose the best answer.',
					options: [{ text: 'First choice' }, { text: 'Second choice' }]
				}
			}
		})
	);

	const submittedBodies: Record<string, unknown>[] = [];
	let rejectFirstAnswer = true;
	await page.route(`**/v1/competitions/${competitionId}/entry/answer`, async (route) => {
		const body = route.request().postDataJSON() as Record<string, unknown>;
		submittedBodies.push(body);
		if (rejectFirstAnswer) {
			rejectFirstAnswer = false;
			await route.fulfill({
				status: 503,
				json: { error: { code: 'temporary_failure', message: 'Try submitting again.' } }
			});
			return;
		}
		const isSecondQuestion = body.question_version_id === secondQuestionId;
		await route.fulfill({
			status: 200,
			json: isSecondQuestion
				? {
						attempt_id: 'attempt-1',
						submitted: true,
						entry_id: 'entry-1',
						score: 25,
						questions: 2,
						total_time_ms: 4200
					}
				: {
						attempt_id: 'attempt-1',
						submitted: false,
						question: {
							question_version_id: secondQuestionId,
							question_number: 2,
							total_questions: 2,
							vignette: 'Second clinical vignette',
							lead_in: 'Choose one response.',
							options: [{ text: 'Third choice' }, { text: 'Fourth choice' }]
						}
					}
		});
	});
	await page.route(`**/v1/competitions/${competitionId}/leaderboard`, (route) =>
		route.fulfill({
			status: 200,
			json: {
				prize_reviewed: false,
				status: 'open',
				entries: [
					{
						rank: 4,
						handle: 'learner-1',
						score: 25,
						accuracy: 0.5,
						questions_attempted: 2,
						average_response_time_ms: 2100,
						total_time_ms: 4200,
						is_me: true,
						prize_eligible: false
					}
				]
			}
		})
	);

	await page.goto('/practice');
	await page.getByTestId('competition-enter').click();
	await expect(page.getByRole('heading', { name: 'Question 1 of 2' })).toBeFocused();
	await expect(page.getByText('First clinical vignette')).toBeVisible();
	await expect(page.getByText('Correct rationale')).toHaveCount(0);
	await page.getByLabel('First choice').check();
	await page.getByTestId('competition-answer').click();
	await expect(page.getByTestId('competition-error')).toContainText('Try submitting again.');
	await page.getByTestId('competition-answer').click();
	await expect(page.getByRole('heading', { name: 'Question 2 of 2' })).toBeFocused();
	await expect(page.getByText('First clinical vignette')).toHaveCount(0);
	await page.getByLabel('Fourth choice').check();
	await page.getByTestId('competition-answer').click();
	await expect(page.getByTestId('competition-result')).toContainText('25');
	await expect(page.getByTestId('competition-leaderboard')).toContainText('#4 learner-1 · you');
	expect(submittedBodies).toHaveLength(3);
	expect(submittedBodies[0].idempotency_key).toBe(submittedBodies[1].idempotency_key);
	expect(submittedBodies[0].elapsed_ms).toBeUndefined();
	expect(submittedBodies[1].elapsed_ms).toBeUndefined();
});
