import { expect, test } from '@playwright/test';

// Browser E2E of the connected question loop against the real API and real
// database (plan §27.1: browser end-to-end tests of the web build).
// Fixture guarantee: the pilot chapter has exactly two questions; q1's key is
// A (index 0), q2's key is B (index 1) — so answering A on both items always
// yields exactly 1 correct + 1 incorrect, and the plan revision always fires.

test('learner loop: register, plan, answer, submit, revision, undo', async ({
	page
}) => {
	const email = `e2e-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;

	await page.goto('/login');
	await page.getByTestId('toggle-mode').click();
	await page.getByTestId('email').fill(email);
	await page.getByTestId('password').fill('correct horse battery');
	await page.getByTestId('submit').click();

	// Today: cold-start plan with one pending task (AI-02).
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	const dailyGoal = page.getByTestId('daily-goal');
	const goalMode = page.getByTestId('daily-goal-mode');
	await expect(goalMode).toHaveValue('questions');
	await goalMode.selectOption('minutes');
	await expect(dailyGoal).toContainText('0/60 minutes');
	const availableMinutes = page.getByLabel('Daily available minutes');
	await availableMinutes.fill('5');
	await availableMinutes.press('Tab');
	await expect(dailyGoal).toContainText('0/5 minutes');
	await page.reload();
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	const persistedGoalMode = page.getByTestId('daily-goal-mode');
	const persistedDailyGoal = page.getByTestId('daily-goal');
	await expect(persistedGoalMode).toHaveValue('minutes');
	await expect(persistedDailyGoal).toContainText('0/5 minutes');
	await expect(page.getByLabel('Daily available minutes')).toHaveValue('5');
	await persistedGoalMode.selectOption('questions');
	await expect(persistedDailyGoal).toContainText('0/20 questions');
	await persistedGoalMode.selectOption('minutes');
	await expect(persistedDailyGoal).toContainText('0/5 minutes');
	const qotdExam = page.getByTestId('qotd-exam');
	await expect(qotdExam).toBeVisible();
	await expect(page.getByTestId('qotd-needs-exam')).toBeVisible();
	await expect(qotdExam.locator('option').nth(1)).toBeAttached();
	await qotdExam.selectOption({ index: 1 });
	await expect(page.getByTestId('qotd-vignette')).toBeVisible();
	await page.getByTestId('qotd-option-0').click();
	await expect(page.getByTestId('qotd-answered')).toBeVisible();
	await expect(qotdExam).toBeDisabled();
	await expect(page.getByTestId('qotd-exam-lock')).toContainText('unlocks');

	const start = page.getByTestId('task-start');
	await expect(start).toBeVisible();

	// Start the tutor session.
	await start.click();
	await expect(page.getByText('Question 1 of 2')).toBeVisible();
	const questionWatermark = page.getByTestId('question-watermark');
	await expect(questionWatermark).toBeVisible();
	const decorativeWatermark = questionWatermark.locator('.watermark-tiles');
	await expect(decorativeWatermark).toHaveAttribute('aria-hidden', 'true');
	await expect(decorativeWatermark).toHaveAttribute(
		'data-watermark',
		/^Account [0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
	);
	await expect(decorativeWatermark).toHaveCSS('pointer-events', 'none');

	// Item 1: choose A, see tutor feedback with the key learning point.
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toContainText('Key learning point');
	await expect(page.getByTestId('explanation-watermark')).toBeVisible();

	// Item 2: same.
	await page.getByTestId('next').click();
	await expect(page.getByText('Question 2 of 2')).toBeVisible();
	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toBeVisible();

	// Submit: deterministic 1 correct, 1 incorrect.
	await page.getByTestId('submit-session').click();
	await expect(page.getByTestId('score')).toHaveText('50%');
	await expect(page.getByTestId('results')).toContainText('not a prediction');

	// Back to Today: task done, justified automatic revision, undo works.
	await page.getByTestId('back-today').click();
	const revision = page.getByTestId('revision-card');
	await expect(revision).toBeVisible();
	await expect(revision).toContainText('missed question');
	await revision.getByTestId('undo').click();
	await expect(revision).toContainText('Undone');
});
