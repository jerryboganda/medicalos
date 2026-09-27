import { expect, test } from '@playwright/test';

// EX-07: editorial mock builder supporting all 7 form types, API-backed catalog selections,
// learner mock badges on /practice, and time analysis rendering on completed sessions.
test('administrator creates mock test with form type and blueprint, learner sees badge and time analysis', async ({
	page
}) => {
	let createdMockPayload: Record<string, unknown> | undefined;
	const mockList = [
		{
			mock_id: 'mock-existing-1',
			title: 'Seeded Full Examination Form',
			mock_type: 'full',
			time_limit_seconds: 7200,
			pass_mark_percent: 70,
			attempts_allowed: 1,
			attempts_used: 0,
			late_sync_grace_seconds: 300,
			integrity_policy: 'log_only',
			away_timeout_seconds: null
		}
	];

	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-admin-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});

	// Mock exams catalog
	await page.route('**/v1/exams**', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				exams: [
					{
						exam_id: '00000000-0000-0000-0000-000000000001',
						code: 'PILT',
						name: 'Pilot Exam'
					}
				]
			})
		});
	});

	// Mock hierarchy for the exam
	await page.route('**/v1/admin/hierarchy?exam_id=**', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				nodes: [
					{
						id: '00000000-0000-0000-0000-000000000002',
						kind: 'chapter',
						name: 'Cardiorespiratory Physiology',
						parent_id: null,
						display_order: 1,
						status: 'active'
					}
				]
			})
		});
	});

	// Mock tests endpoint
	await page.route('**/v1/mocks**', async (route) => {
		const req = route.request();
		if (req.method() === 'POST' && !req.url().includes('/start')) {
			createdMockPayload = req.postDataJSON() as Record<string, unknown>;
			const newMock = {
				mock_id: 'mock-new-2',
				title: createdMockPayload.title,
				mock_type: createdMockPayload.mock_type ?? 'full',
				time_limit_seconds: createdMockPayload.time_limit_seconds ?? 3600,
				pass_mark_percent: createdMockPayload.pass_mark_percent ?? 70,
				attempts_allowed: createdMockPayload.attempts_allowed ?? 1,
				attempts_used: 0,
				late_sync_grace_seconds: createdMockPayload.late_sync_grace_seconds ?? 300,
				integrity_policy: createdMockPayload.integrity_policy ?? 'log_only',
				away_timeout_seconds: createdMockPayload.away_timeout_seconds ?? null
			};
			mockList.push(newMock);
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ mock: newMock })
			});
			return;
		}
		if (req.method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ mocks: mockList })
			});
			return;
		}
		await route.continue();
	});

	await page.route('**/v1/admin/**', async (route) => {
		const url = new URL(route.request().url());
		if (url.pathname.endsWith('/hierarchy')) {
			// the dedicated hierarchy mock (registered earlier) owns this one
			await route.fallback();
			return;
		}
		if (url.pathname.endsWith('/settings')) {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					settings: {
						mastery_bands: [1400, 1600],
						community_min_sample: 20,
						free_daily_questions: 10,
						free_daily_coach_turns: 1,
						retest_intervals_days: [1, 3, 7, 14],
						offline_lease_days: 14,
						max_reviews_per_day: 30,
						max_new_cards_per_day: 10,
						competition_difficulty_points: [5, 10, 15]
					}
				})
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ events: [], reports: [], concepts: [], rights: [], runs: [], appeals: [] })
		});
	});

	await page.route('**/v1/me/curriculum**', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ chapters: [] })
		});
	});

	await page.route('**/v1/competitions**', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ competitions: [] })
		});
	});

	await page.route('**/v1/practice/sessions/sess-e2e-time-1', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				session_id: 'sess-e2e-time-1',
				preset: 'mock',
				status: 'submitted',
				result: {
					score: 100,
					correct: 2,
					total: 2,
					mock: {
						mock_type: 'mini',
						score_percent: 100,
						passed: true,
						pass_mark_percent: 65,
						percentile: null,
						total_time_seconds: 75,
						avg_time_per_question_seconds: 37,
						breakdown: [
							{
								chapter: 'Cardiorespiratory Physiology',
								correct: 2,
								total: 2,
								time_seconds: 75
							}
						]
					}
				},
				items: []
			})
		});
	});

	// 1. Admin navigates to /admin
	await page.goto('/admin');
	await expect(page.getByRole('heading', { name: 'Mock test builder (EX-07)' })).toBeVisible();

	// Fill mock form with API-backed exam and chapter dropdown selections
	await page.getByTestId('mock-title').fill('Cardiorespiratory Mini Mock');
	await page.getByTestId('mock-type').selectOption('mini');
	await page.getByTestId('mock-time-limit').fill('45');
	await page.getByTestId('mock-pass-mark').fill('65');

	// Verify target exam is populated from API
	await expect(page.getByTestId('mock-exam-id')).toBeVisible();

	// Verify curriculum chapter is loaded from hierarchy API and select it
	await expect(page.getByTestId('blueprint-chapter-select')).toBeVisible();
	await page.getByTestId('blueprint-count').fill('15');
	await page.getByTestId('add-blueprint-entry').click();

	await expect(page.getByTestId('blueprint-entries')).toBeVisible();
	await expect(page.getByTestId('blueprint-entry-item')).toContainText('Cardiorespiratory Physiology');
	await expect(page.getByTestId('blueprint-entry-item')).toContainText('15 questions');

	// Submit creation
	await page.getByTestId('create-mock-btn').click();

	// Verify creation feedback and updated admin list
	await expect(page.getByTestId('mock-message')).toBeVisible();
	await expect(page.getByTestId('mock-message')).toContainText('Cardiorespiratory Mini Mock');

	expect(createdMockPayload).toBeDefined();
	expect(createdMockPayload?.title).toBe('Cardiorespiratory Mini Mock');
	expect(createdMockPayload?.mock_type).toBe('mini');
	expect(createdMockPayload?.exam_id).toBe('00000000-0000-0000-0000-000000000001');
	expect(createdMockPayload?.time_limit_seconds).toBe(2700);
	expect(createdMockPayload?.pass_mark_percent).toBe(65);
	expect(createdMockPayload?.blueprint).toEqual([
		{
			chapter_id: '00000000-0000-0000-0000-000000000002',
			count: 15
		}
	]);

	// Verify configured mocks list in admin contains new mock and mini chip
	await expect(page.getByTestId('mock-item').filter({ hasText: 'Cardiorespiratory Mini Mock' })).toBeVisible();
	await expect(page.getByTestId('mock-item').filter({ hasText: 'Cardiorespiratory Mini Mock' }).getByTestId('mock-item-type')).toHaveText('mini');

	// 2. Learner navigates to /practice and sees the non-default mock type badge
	await page.goto('/practice');
	await expect(page.getByText('Cardiorespiratory Mini Mock')).toBeVisible();
	const miniChip = page.getByTestId('mock-type-chip').filter({ hasText: 'mini' });
	await expect(miniChip).toBeVisible();

	// 3. Learner navigates to completed mock session result page and verifies time analysis
	await page.goto('/session/sess-e2e-time-1');
	const timeAnalysis = page.getByTestId('mock-time-analysis');
	await expect(timeAnalysis).toBeVisible();
	await expect(timeAnalysis).toContainText('1m 15s');
	await expect(timeAnalysis).toContainText('37s/question avg');

	// Assert completed result's per-chapter timing line
	const mockResult = page.getByTestId('mock-result');
	await expect(mockResult).toBeVisible();
	await expect(mockResult).toContainText('Cardiorespiratory Physiology: 2/2');
	await expect(mockResult).toContainText('(1m 15s)');
});
