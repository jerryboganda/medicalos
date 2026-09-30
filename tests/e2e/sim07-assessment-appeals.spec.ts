import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-learner-token', 'e2e-reviewer-token']);
});

const runId = '77777777-7777-4777-8777-777777777777';
const appealId = '88888888-8888-4888-8888-888888888888';
const assessedRubric = [
	{
		criterion_key: 'focused_history',
		label: 'Takes a focused history',
		max_score: 2,
		assessment_status: 'assessed',
		evidence: 'Asked when the fictional symptoms began.',
		score: 1,
		transcript_event_indexes: [0],
		transcript_uncertain: false,
		reviewed_at: '2026-09-24T08:00:00Z'
	}
];

test('learner submits an appeal and sees its independent-review status', async ({ page }) => {
	let appeal: Record<string, unknown> | null = null;
	let submittedReason = '';
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/team$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ run_id: runId, current_role: 'team_lead', current_member_id: 'leader-member', members: [{ member_id: 'leader-member', role: 'team_lead', joined_at: '2026-09-24T07:00:00Z' }] })
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/handovers$`), (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ handovers: [] }) })
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario: 'Fictional symptom history',
				scenario_version: 1,
				current_state: 'complete',
				transcript: [{ from: 'start', on: 'finish', to: 'complete' }],
				timeline: [{ index: 0, sequence: 1, from: 'start', on: 'finish', to: 'complete' }],
				available_actions: [],
				started_at: '2026-09-24T07:00:00Z',
				finished_at: '2026-09-24T07:05:00Z',
				finished: true
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/debrief$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				scenario: 'Fictional symptom history',
				scenario_version: 1,
				final_state: 'complete',
				transcript: [{ from: 'start', on: 'finish', to: 'complete' }],
				timeline: [{ index: 0, sequence: 1, from: 'start', on: 'finish', to: 'complete' }],
				available_actions: [],
				started_at: '2026-09-24T07:00:00Z',
				finished_at: '2026-09-24T07:05:00Z',
				transcript_corrections: [],
				rubric: assessedRubric,
				consequential_use_status: appeal?.decision === 'reassessment_required' ? 'reassessment_required' : 'not_authorized_by_assessment',
				appeal
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/appeals$`), async (route) => {
		submittedReason = (route.request().postDataJSON() as { reason: string }).reason;
		appeal = {
			appeal_id: appealId,
			reason: submittedReason,
			status: 'open',
			decision: null,
			rationale: null,
			created_at: '2026-09-24T08:30:00Z',
			reviewed_at: null
		};
		await route.fulfill({
			status: 201,
			contentType: 'application/json',
			body: JSON.stringify({ appeal_id: appealId, status: 'open' })
		});
	});

	await page.goto(`/scenarios/runs/${runId}`);
	await page.getByTestId('scenario-appeal-reason-input').fill(
		'I believe the recorded evidence does not reflect the station transcript.'
	);
	await page.getByTestId('scenario-appeal-submit').click();
	await expect(page.getByTestId('scenario-appeal-status')).toHaveText('Awaiting independent review');
	await expect(page.getByTestId('scenario-assessment-appeal')).toContainText(submittedReason);
	expect(submittedReason).toBe(
		'I believe the recorded evidence does not reflect the station transcript.'
	);

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});

test('independent administrator records a durable appeal decision', async ({ page }) => {
	let waiting = true;
	let recorded: Record<string, unknown> | undefined;
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-reviewer-token');
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
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				appeals: waiting
					? [{ appeal_id: appealId, run_id: runId, scenario: 'Fictional symptom history', scenario_version: 1, reason: 'I believe the recorded evidence does not reflect the station transcript.', created_at: '2026-09-24T08:30:00Z' }]
					: []
			})
		})
	);
	await page.route(new RegExp(`/v1/admin/scenario-assessment-appeals/${appealId}$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				appeal_id: appealId,
				run_id: runId,
				scenario: 'Fictional symptom history',
				scenario_version: 1,
				reason: 'I believe the recorded evidence does not reflect the station transcript.',
				created_at: '2026-09-24T08:30:00Z',
				timeline: [{ index: 0, sequence: 1, from: 'start', on: 'finish', to: 'complete' }],
				rubric: assessedRubric
			})
		})
	);
	await page.route(new RegExp(`/v1/admin/scenario-assessment-appeals/${appealId}/review$`), async (route) => {
		recorded = route.request().postDataJSON() as Record<string, unknown>;
		waiting = false;
		await route.fulfill({
			status: 201,
			contentType: 'application/json',
			body: JSON.stringify({ appeal_id: appealId, status: 'reviewed', decision: recorded.decision })
		});
	});

	await page.goto('/admin');
	await page.getByTestId(`scenario-appeal-open-${appealId}`).click();
	await expect(page.getByTestId('scenario-appeal-reason')).toContainText('recorded evidence');
	await page.getByLabel('Independent decision').selectOption('reassessment_required');
	await expect(page.getByText(/unsuitable for consequential use pending a new independent assessment/)).toBeVisible();
	await page.getByTestId('scenario-appeal-rationale').fill(
		'The original transcript does not support a conclusive score for this criterion.'
	);
	await page.getByTestId('scenario-appeal-submit').click();
	await expect(page.getByTestId('scenario-appeal-message')).toContainText('reassessment required');
	await expect(page.getByText('No assessment appeals are waiting for review.')).toBeVisible();
	expect(recorded).toEqual({
		decision: 'reassessment_required',
		rationale: 'The original transcript does not support a conclusive score for this criterion.'
	});

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
