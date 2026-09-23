import { expect, test } from '@playwright/test';

test('learner protects plan work and refreshes after a stale capacity replan', async ({ page }) => {
	const planId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
	const taskId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
	const taskKey = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
	const optionalTaskId = 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';
	const optionalTaskKey = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
	const chapterId = '12121212-1212-4121-8121-121212121212';
	const replanBodies: Array<{ daily_minutes: number; expected_version: number }> = [];
	const undoCalls: string[] = [];
	let planVersion = 1;
	let protectedTask = false;
	let deferredRevision: {
		id: string;
		to_version: number;
		reason_code: string;
		explanation: string;
		automatic: boolean;
		undone: boolean;
		deferred_tasks: string[];
	} | null = null;

	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/today', (route) => {
		const tasks = [
			{
				id: taskId,
				task_key: taskKey,
				kind: 'practice',
				title: 'Review a chapter',
				chapter_id: chapterId,
				source_session_id: null,
				question_count: 4,
				estimated_minutes: 6,
				status: 'pending',
				protected: protectedTask
			}
		];
		if (planVersion === 1) {
			tasks.push({
				id: optionalTaskId,
				task_key: optionalTaskKey,
				kind: 'practice',
				title: 'Optional timed practice',
				chapter_id: chapterId,
				source_session_id: null,
				question_count: 8,
				estimated_minutes: 15,
				status: 'pending',
				protected: false
			});
		}
		return route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				plan_id: planId,
				version: planVersion,
				tasks,
				revisions: deferredRevision ? [deferredRevision] : [],
				learner: [],
				revision_budget: {
					automatic_used: 0,
					automatic_limit: 3,
					total_used: planVersion - 1,
					total_limit: 8
				}
			})
		});
	});
	await page.route('**/v1/mocks', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ mocks: [] }) })
	);
	await page.route('**/v1/me/engagement', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ enabled: false }) })
	);
	await page.route('**/v1/practice/sessions', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ session_id: 'abababab-abab-4aba-8aba-abababababab' })
		});
	});
	await page.route(/\/v1\/plans\/[^/]+\/tasks\/[^/]+\/protection$/, async (route) => {
		protectedTask = (route.request().postDataJSON() as { protected: boolean }).protected;
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ task_id: taskId, protected: protectedTask })
		});
	});
	await page.route('**/v1/me/plan/replan', async (route) => {
		replanBodies.push(route.request().postDataJSON() as (typeof replanBodies)[number]);
		if (replanBodies.length === 1) {
			planVersion = 2;
			deferredRevision = {
				id: 'edededed-eded-4ede-8ede-edededededed',
				to_version: 2,
				reason_code: 'capacity_change',
				explanation: 'Plan trimmed to your 10-minute budget: 1 task deferred.',
				automatic: false,
				undone: false,
				deferred_tasks: ['Optional timed practice']
			};
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ replanned: true, version: 2, deferred_tasks: 1 })
			});
			return;
		}
		planVersion = 3;
		await route.fulfill({
			status: 409,
			contentType: 'application/json',
			body: JSON.stringify({
				error: {
					code: 'stale_plan_version',
					message: 'Your plan changed. Refresh it before replanning.'
				}
			})
		});
	});
	await page.route(/\/v1\/plans\/[^/]+\/revisions\/[^/]+\/undo$/, async (route) => {
		undoCalls.push(route.request().url());
		planVersion = 4;
		await route.fulfill({
			status: 409,
			contentType: 'application/json',
			body: JSON.stringify({
				error: {
					code: 'stale_plan_version',
					message: 'Your plan changed. Refresh it before undoing a revision.'
				}
			})
		});
	});

	await page.goto('/today');
	await expect(page.getByTestId('plan-version')).toContainText('1');
	await expect(page.getByTestId(`task-estimate-${taskId}`)).toContainText('Est. 6 min');
	await page.getByTestId(`task-protection-${taskId}`).click();
	await expect(page.getByTestId(`task-protection-${taskId}`)).toContainText('Unprotect');

	await page.getByLabel('Daily available minutes').fill('10');
	await page.getByTestId('replan-submit').click();
	await expect(page.getByTestId('plan-version')).toContainText('2');
	await expect(page.getByTestId('revision-card')).toContainText('Deferred: Optional timed practice');
	await page.getByTestId('replan-submit').click();
	await expect(page.getByTestId('plan-version')).toContainText('3');
	await expect(page.getByTestId('plan-error')).toContainText('Your plan changed');
	await page.getByTestId('undo').click();
	await expect(page.getByTestId('plan-version')).toContainText('4');
	await expect(page.getByTestId('plan-error')).toContainText('Refresh it before undoing');
	expect(undoCalls).toHaveLength(1);
	expect(replanBodies).toEqual([
		{ daily_minutes: 10, expected_version: 1 },
		{ daily_minutes: 10, expected_version: 2 }
	]);

	const timedLaunch = page.waitForRequest((request) => request.url().includes('/v1/practice/sessions'));
	await page.getByTestId('task-timed').click();
	expect((await timedLaunch).postDataJSON()).toMatchObject({
		preset: 'timed',
		plan_task_key: taskKey,
		question_count: 4,
		time_limit_seconds: 300
	});
});
