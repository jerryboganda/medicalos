import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

test('Today requests a time-bounded next action and explains when nothing fits', async ({ page }) => {
	const taskId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
	let hasRecommendation = true;
	let emptyReason = 'no_task_fits';
	let allowance: { limit: number; used: number; remaining: number; required?: number } | null = null;
	const requestedBudgets: number[] = [];
	const requestedPreferences: Array<{ activity: string; multiplier: number }> = [];

	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/today', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				plan_id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
				version: 1,
				tasks: [
					{
						id: taskId,
						kind: 'practice',
						title: 'Weak chapter practice',
						chapter_id: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc',
						source_session_id: null,
						question_count: 5,
						estimated_minutes: 6,
						status: 'pending',
						protected: false
					}
				],
				revisions: [],
				learner: [],
				revision_budget: {
					automatic_used: 0,
					automatic_limit: 3,
					total_used: 0,
					total_limit: 8
				}
			})
		})
	);
	await page.route('**/v1/mocks', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ mocks: [] }) })
	);
	await page.route('**/v1/me/engagement', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ enabled: false }) })
	);
	await page.route('**/v1/me/plan/next-action**', async (route) => {
		const requestUrl = new URL(route.request().url());
		requestedBudgets.push(Number(requestUrl.searchParams.get('available_minutes')));
		requestedPreferences.push({
			activity: requestUrl.searchParams.get('activity_preference') ?? '',
			multiplier: Number(requestUrl.searchParams.get('time_multiplier'))
		});
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				available_minutes: requestedBudgets[requestedBudgets.length - 1],
				activity_preference: requestedPreferences[requestedPreferences.length - 1].activity,
				time_multiplier: requestedPreferences[requestedPreferences.length - 1].multiplier,
				exam_date: null,
				plan_id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
				plan_version: 1,
				reason_code: hasRecommendation ? 'recommended' : emptyReason,
				allowance,
				recommended_action: hasRecommendation
					? {
							task_id: taskId,
							kind: 'practice',
							title: 'Weak chapter practice',
							chapter_id: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc',
							source_session_id: null,
							question_count: 5,
							estimated_minutes: 6,
							adjusted_estimated_minutes: 9,
							protected: false,
							reason_code: 'lower_observed_accuracy',
							independent_count: 10
						}
					: null
			})
		});
	});

	await page.goto('/today');
	await page.getByLabel('Daily available minutes').fill('12');
	await page.getByLabel('Preferred activity').selectOption('practice');
	await page.getByLabel('Study-time adjustment').selectOption('1.5');
	await page.getByTestId('recommend-next-action').click();
	await expect(page.getByTestId('next-action-result')).toContainText('Weak chapter practice');
	await expect(page.getByTestId('next-action-result')).toContainText('Est. 9 min (adjusted for study time)');
	await expect(page.getByTestId('next-action-result')).toContainText('10 independent attempts');
	expect(requestedBudgets).toEqual([12]);
	expect(requestedPreferences).toEqual([{ activity: 'practice', multiplier: 1.5 }]);

	hasRecommendation = false;
	await page.getByTestId('recommend-next-action').click();
	await expect(page.getByTestId('next-action-empty')).toContainText('No pending task fits 12 minutes');
	expect(requestedBudgets).toEqual([12, 12]);
	expect(requestedPreferences).toEqual([
		{ activity: 'practice', multiplier: 1.5 },
		{ activity: 'practice', multiplier: 1.5 }
	]);

	emptyReason = 'content_unavailable';
	await page.getByTestId('recommend-next-action').click();
	await expect(page.getByTestId('next-action-empty')).toContainText(
		'No fitting task currently has enough available questions to start.'
	);

	emptyReason = 'free_allowance_insufficient';
	allowance = { limit: 10, used: 9, remaining: 1, required: 5 };
	await page.getByTestId('recommend-next-action').click();
	await expect(page.getByTestId('next-action-empty')).toContainText(
		'Only 1 of 10 free questions remain today; this planned task needs 5.'
	);
});
