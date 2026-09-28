import { expect, test } from '@playwright/test';

// QB-08 in the browser: the learner reports a problem on an answered item
// through the real UI, and the thanks state renders (the report persists
// through the real API).

test('question report flow records a report from the session UI', async ({
	page
}) => {
	const email = `e2e-report-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;

	await page.goto('/login');
	await page.getByTestId('toggle-mode').click();
	await page.getByTestId('email').fill(email);
	await page.getByTestId('password').fill('correct horse battery');
	await page.getByTestId('submit').click();

	// Today: cold-start plan with one pending task.
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	await page.getByTestId('task-start').click();
	await expect(page.getByText('Question 1 of 2')).toBeVisible({
		timeout: 15_000
	});

	// Answer item 1 to reveal the feedback + report control.
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toBeVisible();

	// Report flow: open, pick typo, send, see the thanks state.
	await page.getByTestId('report-open').click();
	await expect(page.getByTestId('report-form')).toBeVisible();
	await page.getByTestId('report-cat-typo').click();
	await page.getByTestId('report-note').fill('E2E fixture: spelling looks off');
	const reportPost = page.waitForResponse(
		(r) =>
			r.url().includes('/reports') && r.request().method() === 'POST'
	);
	await page.getByTestId('report-submit').click();
	const resp = await reportPost;
	expect(resp.status(), 'report POST status').toBe(200);
	// The POST's 200 asserts the report is recorded; the panel surfaces
	// either the first acknowledgement or the deduplicated replay notice.
	await expect(page.getByTestId('report-done')).toBeVisible();

	// Cleanup: this report is real and would keep the fixture question out
	// of plan sizing (open reports shrink a chapter's task), breaking specs
	// that run later on the same seeded database. Resolve it so the suite is
	// order-independent and re-runnable.
	const adminToken = process.env.ADMIN_TOKEN;
	if (!adminToken) throw new Error('ADMIN_TOKEN required for report cleanup');
	const api = process.env.E2E_API_BASE ?? 'http://127.0.0.1:8080';
	const queue = await (
		await fetch(`${api}/v1/admin/reports`, { headers: { 'x-admin-token': adminToken } })
	).json();
	for (const entry of queue.reports ?? []) {
		const resolved = await fetch(`${api}/v1/reports/${entry.report_id}/resolve`, {
			method: 'POST',
			headers: { 'content-type': 'application/json', 'x-admin-token': adminToken },
			body: JSON.stringify({
				status: 'resolved_rejected',
				resolution_note: 'E2E cleanup: fixture restored',
				correction_note: null
			})
		});
		expect(resolved.ok, 'cleanup resolve status').toBe(true);
	}
});

test('admin resolves a grouped report with private feedback and a public correction note', async ({ page }) => {
	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/audit', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ events: [] })
		})
	);

	let releaseFirstQueue!: () => void;
	const firstQueueGate = new Promise<void>((resolve) => (releaseFirstQueue = resolve));
	let queueCalls = 0;
	const reportId = '11111111-1111-4111-8111-111111111111';
	let resolution: Record<string, unknown> | undefined;
	await page.route('**/v1/admin/reports*', async (route) => {
		queueCalls += 1;
		if (queueCalls === 1) await firstQueueGate;
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				reports:
					queueCalls === 1
						? [
								{
									report_id: reportId,
									question_version_id: '22222222-2222-4222-8222-222222222222',
									question_id: '33333333-3333-4333-8333-333333333333',
									version: 1,
									vignette: 'A learner case with an unclear key.',
									lead_in: 'Which answer is best?',
									category: 'wrong_answer',
									reporter_feedback: [
										{
											category: 'wrong_answer',
											note: 'The supplied key conflicts with the rationale.'
										},
										{
											category: 'bad_explanation',
											note: 'The explanation cites the wrong option.'
										}
									],
									feedback_truncated: false,
									report_count: 2,
									first_reported_at: '2026-09-23T08:00:00Z',
									acknowledgement_due_at: '2026-09-24T08:00:00Z',
									resolution_due_at: '2026-09-26T08:00:00Z',
									acknowledgements_on_time: true,
									resolution_overdue: false,
									quarantined: false
								}
							]
						: []
			})
		});
	});
	await page.route('**/v1/reports/*/resolve', async (route) => {
		resolution = route.request().postDataJSON();
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				status: 'resolved_fixed',
				question_version_id: '22222222-2222-4222-8222-222222222222',
				corrected_version_id: '44444444-4444-4444-8444-444444444444',
				resolved_reports: 2,
				notified_reporters: 2
			})
		});
	});

	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.goto('/admin');
	await expect(page.getByTestId('report-queue-refresh')).toContainText('Refreshing…');
	releaseFirstQueue();
	await expect(page.getByTestId('report-item')).toContainText('2 reports');
	await expect(page.getByTestId('report-item')).toContainText('Version 1');
	await expect(page.getByTestId('report-item')).toContainText(
		'The explanation cites the wrong option.'
	);
	const noMobileOverflow = await page.evaluate(
		() => document.documentElement.scrollWidth <= window.innerWidth
	);
	expect(noMobileOverflow, 'report cards fit a 375px viewport').toBe(true);

	await page.getByTestId('report-resolution-note').fill('Reviewed: the explanation has been corrected.');
	await expect(page.getByTestId('report-resolve-fixed')).toBeDisabled();
	await page
		.getByTestId('report-correction-note')
		.fill('The explanation now follows the cited source.');
	await page.getByTestId('report-resolve-fixed').click();
	await expect(page.getByTestId('report-queue-success')).toContainText('2 reports resolved');
	expect(resolution).toEqual({
		status: 'resolved_fixed',
		resolution_note: 'Reviewed: the explanation has been corrected.',
		correction_note: 'The explanation now follows the cited source.'
	});
	await expect(page.getByTestId('report-queue-empty')).toBeVisible();
});

test('admin report queue recovers from an API error and shows its empty state', async ({ page }) => {
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/audit', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ events: [] })
		})
	);
	let calls = 0;
	await page.route('**/v1/admin/reports*', (route) => {
		calls += 1;
		return calls === 1
			? route.fulfill({
						status: 503,
						contentType: 'application/json',
						body: JSON.stringify({
							error: { code: 'reports_unavailable', message: 'Report service is temporarily unavailable.' }
						})
					})
			: route.fulfill({
						status: 200,
						contentType: 'application/json',
						body: JSON.stringify({ reports: [] })
					});
	});

	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.goto('/admin');
	await expect(page.getByTestId('report-queue-error')).toContainText(
		'Report service is temporarily unavailable.'
	);
	await page.getByTestId('report-queue-refresh').click();
	await expect(page.getByTestId('report-queue-empty')).toBeVisible();
});
