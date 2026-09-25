import { readFile } from 'node:fs/promises';
import { expect, test } from '@playwright/test';

test('administrator loads and saves live runtime settings', async ({ page }) => {
	let savedSettings: Record<string, unknown> | undefined;
	let rejectFirstSave = true;
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-admin-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/**', async (route) => {
		const url = new URL(route.request().url());
		if (url.pathname.endsWith('/settings')) {
			if (route.request().method() === 'PATCH') {
				savedSettings = route.request().postDataJSON() as Record<string, unknown>;
				if (rejectFirstSave) {
					rejectFirstSave = false;
					await route.fulfill({
						status: 422,
						contentType: 'application/json',
						body: JSON.stringify({ error: { message: 'Mastery boundaries must be increasing.' } })
					});
					return;
				}
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({ updated: Object.keys(savedSettings) })
				});
				return;
			}
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
	await expect(page.getByRole('heading', { name: 'Runtime settings' })).toBeVisible();
	await expect(page.getByLabel('Minimum community sample')).toHaveValue('20');
	await expect(page.getByLabel('Offline lease length in days')).toHaveValue('14');
	await expect(page.getByLabel('Daily due-review cap')).toHaveValue('30');
	await expect(page.getByLabel('Daily new-card cap')).toHaveValue('10');
	await expect(page.getByLabel('Easy competition points')).toHaveValue('5');
	await expect(page.getByLabel('Medium competition points')).toHaveValue('10');
	await expect(page.getByLabel('Hard competition points')).toHaveValue('15');
	await page.getByLabel('Lower mastery boundary').fill('1350');
	await page.getByLabel('Higher mastery boundary').fill('1650');
	await page.getByLabel('Minimum community sample').fill('25');
	await page.getByLabel('Daily free questions').fill('12');
	await page.getByLabel('Daily free Coach turns').fill('3');
	await page.getByLabel('Re-test intervals in days').fill('1, 4, 9, 16');
	await page.getByLabel('Offline lease length in days').fill('21');
	await page.getByLabel('Daily due-review cap').fill('24');
	await page.getByLabel('Daily new-card cap').fill('6');
	await page.getByLabel('Easy competition points').fill('6');
	await page.getByLabel('Medium competition points').fill('12');
	await page.getByLabel('Hard competition points').fill('18');
	await page.getByRole('button', { name: 'Save runtime settings' }).click();
	await expect(page.getByRole('alert')).toContainText('Mastery boundaries must be increasing.');
	await page.getByRole('button', { name: 'Save runtime settings' }).click();

	expect(savedSettings).toEqual({
		mastery_bands: [1350, 1650],
		community_min_sample: 25,
		free_daily_questions: 12,
		free_daily_coach_turns: 3,
		retest_intervals_days: [1, 4, 9, 16],
		offline_lease_days: 21,
		max_reviews_per_day: 24,
		max_new_cards_per_day: 6,
		competition_difficulty_points: [6, 12, 18]
	});
	await expect(page.getByTestId('settings-message')).toContainText('Runtime settings saved.');
});

test('administrator downloads the question template and previews a CSV import', async ({ page }) => {
	let upload: { url: string; contentType: string | undefined; body: string | undefined } | undefined;
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-admin-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
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
		if (url.pathname.endsWith('/import-file')) {
			upload = {
				url: url.toString(),
				contentType: route.request().headers()['content-type'],
				body: route.request().postData()
			};
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ batch_id: 'csv-preview', status: 'dry_run', rows: 1, valid: 1, issues: [] })
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
	await expect(page.getByRole('heading', { name: 'Bulk question import' })).toBeVisible();
	const [download] = await Promise.all([
		page.waitForEvent('download'),
		page.getByRole('button', { name: 'Download CSV template' }).click()
	]);
	expect(download.suggestedFilename()).toBe('questions-template.csv');
	expect(await readFile(await download.path(), 'utf8')).toContain('source_ref,rights_ref');
	await page.getByLabel('Exam ID (from the seeded pilot exam)').fill('fixture-exam');
	await page.getByLabel('Question bank file').setInputFiles({
		name: 'questions.csv',
		mimeType: 'text/csv',
		buffer: Buffer.from(
			'chapter_id,difficulty,vignette,lead_in,option_1,rationale_1,option_2,rationale_2,correct_option,key_learning_point,source_ref,rights_ref\nfixture,medium,Example,Question?,A,Why A,B,Why B,1,Key point,source,RIGHTS-REF\n'
		)
	});
	await page.getByRole('button', { name: 'Preview file' }).click();
	await expect(page.getByTestId('import-report')).toContainText('dry_run');
	expect(upload?.contentType).toBe('text/csv');
	expect(upload?.url).toContain('exam_id=fixture-exam');
	expect(upload?.url).toContain('dry_run=true');
	expect(upload?.body).toContain('source_ref,rights_ref');
	expect(upload?.body).toContain('RIGHTS-REF');
});
