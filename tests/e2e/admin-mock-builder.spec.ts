import { expect, test } from '@playwright/test';

// EX-07: editorial mock builder supporting all 7 form types, blueprints, and time parameters.
test('administrator creates mock test with form type and blueprint', async ({ page }) => {
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

	await page.goto('/admin');
	await expect(page.getByRole('heading', { name: 'Mock test builder (EX-07)' })).toBeVisible();

	// Fill mock form
	await page.getByTestId('mock-title').fill('Cardiorespiratory Mini Mock');
	await page.getByTestId('mock-type').selectOption('mini');
	await page.getByTestId('mock-time-limit').fill('45');
	await page.getByTestId('mock-pass-mark').fill('65');

	// Add blueprint chapter
	await page.getByTestId('blueprint-chapter-id').fill('00000000-0000-0000-0000-000000000001');
	await page.getByTestId('blueprint-count').fill('15');
	await page.getByTestId('add-blueprint-entry').click();

	await expect(page.getByTestId('blueprint-entries')).toBeVisible();
	await expect(page.getByTestId('blueprint-entry-item')).toContainText('00000000-0000-0000-0000-000000000001');
	await expect(page.getByTestId('blueprint-entry-item')).toContainText('15 questions');

	// Submit creation
	await page.getByTestId('create-mock-btn').click();

	// Verify creation feedback and updated list
	await expect(page.getByTestId('mock-message')).toBeVisible();
	await expect(page.getByTestId('mock-message')).toContainText('Cardiorespiratory Mini Mock');

	expect(createdMockPayload).toBeDefined();
	expect(createdMockPayload?.title).toBe('Cardiorespiratory Mini Mock');
	expect(createdMockPayload?.mock_type).toBe('mini');
	expect(createdMockPayload?.time_limit_seconds).toBe(2700);
	expect(createdMockPayload?.pass_mark_percent).toBe(65);

	// Verify configured mocks list contains new mock and type chip
	const mockItems = page.getByTestId('mock-item');
	await expect(mockItems).toHaveCount(2);
	await expect(page.getByTestId('mock-item').filter({ hasText: 'Cardiorespiratory Mini Mock' })).toBeVisible();
	await expect(page.getByTestId('mock-item').filter({ hasText: 'Cardiorespiratory Mini Mock' }).getByTestId('mock-item-type')).toHaveText('mini');
});
