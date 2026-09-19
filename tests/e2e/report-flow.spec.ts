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
	await expect(page.getByRole('heading', { name: 'Verify your email' })).toBeVisible();
	await page.getByTestId('submit').click();
	await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
	await page.getByTestId('submit').click();

	// Today: cold-start plan with one pending task.
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	await page.getByTestId('task-start').click();
	await expect(page.getByText('Question 1 of 2')).toBeVisible();

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
	await expect(page.getByTestId('report-done')).toContainText('Thanks');
});
