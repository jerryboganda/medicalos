import { expect, test, type Page } from '@playwright/test';

const examId = 'exam-oet-medicine';
const today = new Date();
const monday = new Date(
	Date.UTC(
		today.getUTCFullYear(),
		today.getUTCMonth(),
		today.getUTCDate() - ((today.getUTCDay() + 6) % 7)
	)
);
const weekStart = monday.toISOString().slice(0, 10);
const nextMonday = new Date(monday.getTime() + 7 * 24 * 60 * 60 * 1000)
	.toISOString()
	.slice(0, 10);

test('learners can opt into and leave a private weekly competition league', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'league-e2e-token'));
	await page.route('**/v1/mocks', (route) => route.fulfill({ json: { mocks: [] } }));
	await page.route('**/v1/me/curriculum', (route) =>
		route.fulfill({
			json: {
				chapters: [
					{
						chapter_id: 'chapter-1',
						chapter_name: 'Medicine',
						system: 'Medicine',
						subject: 'General medicine',
						exam_id: examId,
						exam: 'OET Medicine',
						published_questions: 20
					}
				]
			}
		})
	);
	await page.route('**/v1/competitions', (route) =>
		route.fulfill({
			json: {
				competitions: [
					{
						competition_id: 'event-weekly',
						title: 'Weekly medicine challenge',
						exam_id: examId,
						exam: 'OET Medicine',
						cadence: 'weekly',
						series_id: 'series-weekly',
						starts_at: new Date(Date.now() - 60_000).toISOString(),
						ends_at: new Date(Date.now() + 86_400_000).toISOString(),
						status: 'open',
						entered: false,
						attempt_status: null
					}
				]
			}
		})
	);
	await page.route('**/v1/community/me', (route) =>
		route.fulfill({ json: { opted_in: true, handle: 'learner-1' } })
	);
	let joined = false;
	const cohortState = {
		joined: true,
		exam_id: examId,
		cohort_id: 'cohort-1',
		week_start: weekStart,
		week_end: nextMonday,
		division: 2,
		cohort_number: 1,
		standings: [
			{ rank: 1, handle: 'learner-1', points: 42, accuracy: 0.9, total_time_ms: 12000, is_me: true },
			{ rank: 2, handle: 'learner-2', points: 38, accuracy: 0.8, total_time_ms: 14000, is_me: false }
		]
	};
	await page.route(`**/v1/leagues/${examId}`, (route) =>
		route.fulfill({ json: joined ? cohortState : { joined: false } })
	);
	await page.route(`**/v1/leagues/${examId}/join`, async (route) => {
		joined = true;
		await route.fulfill({ json: cohortState });
	});
	await page.route(`**/v1/leagues/${examId}/membership`, async (route) => {
		joined = false;
		await route.fulfill({ json: { left: true } });
	});

	await page.goto('/practice');
	await expect(page.getByTestId('competition-cadence')).toHaveText('Weekly');
	await expect(page.getByTestId('league-join')).toBeVisible();
	await page.getByTestId('league-join').click();
	await expect(page.getByTestId('league-standings')).toContainText('Division 2 · Cohort 1');
	await expect(page.getByTestId('league-standings')).toContainText('learner-1 · you');
	await expect(page.getByTestId('league-standings')).toContainText('learner-2');
	await page.getByTestId('league-leave').click();
	await expect(page.getByTestId('league-join')).toBeVisible();
});
