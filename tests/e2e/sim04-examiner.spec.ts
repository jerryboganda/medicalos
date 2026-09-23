import { expect, test } from '@playwright/test';

test('examiner records criterion scores against observed transcript events', async ({ page }) => {
	const runId = '11111111-1111-4111-8111-111111111111';
	let pending = true;
	let recorded: Record<string, unknown> | undefined;

	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-examiner-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/audit*', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ events: [] }) })
	);
	await page.route('**/v1/admin/reports*', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ reports: [] }) })
	);
	await page.route('**/v1/admin/concepts', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ concepts: [] }) })
	);
	await page.route('**/v1/admin/content-rights', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ rights: [] }) })
	);
	await page.route('**/v1/admin/library/extraction-reports', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ reports: [] }) })
	);
	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				runs: pending
					? [{
							run_id: runId,
							scenario: 'Fictional symptom history',
							scenario_version: 2,
							finished_at: '2026-09-23T10:00:00Z',
							criterion_count: 2
						}]
					: []
			})
		})
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.route(/\/v1\/admin\/scenarios\/runs\/[^/]+\/assessment$/, async (route) => {
		if (route.request().method() === 'POST') {
			recorded = route.request().postDataJSON() as Record<string, unknown>;
			pending = false;
			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({ recorded_criteria: 2, not_assessed: 1 })
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario: 'Fictional symptom history',
				scenario_version: 2,
				started_at: '2026-09-23T09:50:00Z',
				finished_at: '2026-09-23T10:00:00Z',
				transcript: [
					{ from: 'start', on: 'ask_symptom_onset', to: 'start' },
					{ from: 'start', on: 'finish', to: 'complete' }
				],
				rubric: [
					{
						criterion_key: 'focused_history',
						label: 'Takes a focused history',
						max_score: 2,
						assessment_status: 'not_assessed',
						evidence: null,
						score: null,
						transcript_event_indexes: [],
						transcript_uncertain: false,
						reviewed_at: null
					},
					{
						criterion_key: 'physical_exam',
						label: 'Performs a focused examination',
						max_score: 1,
						assessment_status: 'not_assessed',
						evidence: null,
						score: null,
						transcript_event_indexes: [],
						transcript_uncertain: false,
						reviewed_at: null
					}
				]
			})
		});
	});

	await page.goto('/admin');
	await expect(page.getByTestId(`pending-scenario-${runId}`)).toContainText('Fictional symptom history');
	await page.getByTestId(`scenario-assessment-open-${runId}`).click();
	await expect(page.getByText('Event 1: ask_symptom_onset')).toBeVisible();
	await page.getByLabel('Assessment for focused_history').selectOption('assessed');
	await page.getByLabel('Score for focused_history').fill('1.5');
	await page.getByLabel('Use event 1: ask_symptom_onset').check();
	await page.getByLabel('Evidence or not-assessed reason for focused_history').fill(
		'Asked when the fictional symptoms began.'
	);
	await page.getByLabel('Evidence or not-assessed reason for physical_exam').fill(
		'No examination action was observed in the transcript.'
	);
	await page.getByTestId('scenario-assessment-record').click();
	await expect(page.getByTestId('scenario-assessment-message')).toContainText('Recorded 2 criterion results');
	expect(recorded?.criteria).toEqual([
		{
			criterion_key: 'focused_history',
			assessment_status: 'assessed',
			score: 1.5,
			evidence: 'Asked when the fictional symptoms began.',
			transcript_event_indexes: [0],
			transcript_uncertain: false
		},
		{
			criterion_key: 'physical_exam',
			assessment_status: 'not_assessed',
			score: null,
			evidence: 'No examination action was observed in the transcript.',
			transcript_event_indexes: [],
			transcript_uncertain: false
		}
	]);
	await expect(page.getByText('No finished stations are waiting for assessment.')).toBeVisible();

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
