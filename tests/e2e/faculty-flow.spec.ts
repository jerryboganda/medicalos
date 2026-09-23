import { expect, test } from '@playwright/test';

test('faculty maps a program curriculum and sees privacy-suppressed coverage', async ({ page }) => {
	const suffix = `${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
	const email = `faculty-${suffix}@example.test`;
	const programName = `Program ${suffix}`;

	await page.goto('/login');
	await page.getByTestId('toggle-mode').click();
	await page.getByTestId('email').fill(email);
	await page.getByTestId('password').fill('correct horse battery');
	await page.getByTestId('submit').click();
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();

	await page.goto('/faculty');
	await expect(page.getByRole('heading', { name: 'Faculty workspace' })).toBeVisible();
	await page.locator('#inst-name').fill(`Institute ${suffix}`);
	await page.getByTestId('institution-create').click();
	await expect(page.getByTestId('program-curriculum')).toBeVisible();

	await page.locator('#program-name').fill(programName);
	await page.getByTestId('program-create').click();
	await expect(page.locator('#active-program')).toContainText(programName);

	const firstChapter = page.getByTestId('chapter-option').first();
	await expect(firstChapter).toBeVisible();
	await firstChapter.locator('input').check();
	await page.getByTestId('curriculum-save').click();
	await expect(page.getByText('Curriculum mapping saved.')).toBeVisible();

	await page.locator('#cohort-name').fill(`Cohort ${suffix}`);
	await page.locator('#cohort-program').selectOption({ label: programName });
	await page.getByTestId('cohort-create').click();
	await expect(page.locator('.cohort-row').filter({ hasText: `Cohort ${suffix}` })).toBeVisible();

	await page.reload();
	await expect(page.locator('#active-program')).toContainText(programName);
	await expect(page.getByTestId('chapter-option').first().locator('input')).toBeChecked();
	await expect(page.locator('.cohort-row').filter({ hasText: `Cohort ${suffix}` })).toBeVisible();

	await page.getByTestId('coverage-load').click();
	await expect(page.getByTestId('coverage-suppressed')).toContainText(
		'Counts are hidden until at least 5 active learners'
	);

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 900 });
		const hasNoHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(hasNoHorizontalOverflow, 'horizontal overflow at ' + width).toBe(true);
	}
});
