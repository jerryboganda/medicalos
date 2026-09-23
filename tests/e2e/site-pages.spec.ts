import { expect, test } from '@playwright/test';

// GROW-02 in the browser: every public marketing page renders its real
// content, CTAs route only to the live app or other site pages, and the
// honest-commerce rule holds — no prices, no payment form, no fabricated
// outcome claims anywhere (TRUST-01).

test.use({ baseURL: 'http://127.0.0.1:4174' });

const APP_ORIGIN = 'https://medicalos.polytronx.com';

const pages = [
	{ path: '/', heading: 'Study what the plan chooses' },
	{ path: '/exams/', heading: 'Exam registry' },
	{ path: '/exams/usmle/', heading: 'USMLE' },
	{ path: '/pricing/', heading: 'Pricing' },
	{ path: '/help/', heading: 'Help' },
	{ path: '/download/', heading: 'Get the app' },
	{ path: '/legal/terms/', heading: 'Terms of use' },
	{ path: '/legal/privacy/', heading: 'Privacy' }
];

for (const { path, heading } of pages) {
	test(`renders ${path}`, async ({ page }) => {
		const response = await page.goto(path);
		expect(response?.status()).toBe(200);
		await expect(page.getByRole('heading', { level: 1 })).toContainText(heading);
	});
}

test('every page stays inside its viewport on a narrow phone', async ({
	page
}) => {
	await page.setViewportSize({ width: 320, height: 720 });
	for (const { path } of pages) {
		await page.goto(path);
		const overflow = await page.evaluate(
			() => document.documentElement.scrollWidth - window.innerWidth
		);
		expect(overflow, `${path} overflows at 320px`).toBeLessThanOrEqual(0);
	}
});

test('site CTAs link to the live app, never to a payment form', async ({
	page
}) => {
	await page.goto('/');
	const external = await page.evaluate(() =>
		[...document.querySelectorAll('a[href^="https://"]')].map((a) =>
			(a as HTMLAnchorElement).href
		)
	);
	expect(external.length).toBeGreaterThan(0);
	for (const href of external) {
		expect(href.startsWith(`${APP_ORIGIN}/`), `${href} is not the app`).toBe(
			true
		);
	}
});

test('pricing shows no invented prices or payment forms', async ({ page }) => {
	await page.goto('/pricing/');
	const body = await page.locator('body').innerText();
	expect(body).toContain('To be announced');
	expect(body).not.toContain('$');
	await expect(page.locator('form')).toHaveCount(0);
});

test('no fabricated outcome claims anywhere', async ({ page }) => {
	for (const { path } of pages) {
		await page.goto(path);
		const body = await page.locator('body').innerText();
		expect(body, `${path} claims a success rate`).not.toMatch(
			/\d+(\.\d+)?\s?%\s?(pass|success)/i
		);
		expect(body, `${path} claims testimonials`).not.toMatch(
			/testimonial|trusted by/i
		);
	}
});

test('deep-link association files are served and claim nothing yet', async ({
	request
}) => {
	const assetlinks = await request.get('/.well-known/assetlinks.json');
	expect(assetlinks.status()).toBe(200);
	// No signed Tauri Android beta exists yet — the statement list stays empty.
	expect(await assetlinks.json()).toEqual([]);

	const apple = await request.get('/.well-known/apple-app-site-association');
	expect(apple.status()).toBe(200);
	expect(await apple.json()).toEqual({ applinks: { apps: [], details: [] } });
});

test('unknown addresses get the honest 404', async ({ page }) => {
	const response = await page.goto('/no-such-page/');
	expect(response?.status()).toBe(404);
	await expect(page.getByRole('heading', { level: 1 })).toContainText(
		'Page not found'
	);
});
