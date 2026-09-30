import { expect, test, type Page } from '@playwright/test';

// Visual evidence for design review (design.md; compute policy: screenshots
// are produced in CI, never locally). Captures the key screens at a desktop
// and a phone size, dark and light, into tests/e2e/screenshots/ — uploaded as
// the `ui-gallery` artifact. Reduced motion makes every frame deterministic.

const API = 'http://127.0.0.1:8080';
const SHOTS = 'screenshots';
const viewports = [
	{ name: 'desktop', width: 1280, height: 800 },
	{ name: 'phone', width: 390, height: 844 }
];
const pages = ['/today', '/practice', '/progress', '/library', '/coach', '/notifications', '/account'];

async function shoot(page: Page, name: string, fullPage = true) {
	await page.waitForLoadState('networkidle');
	await page.screenshot({ path: `${SHOTS}/${name}.png`, fullPage });
}

test('app gallery', async ({ page }) => {
	test.setTimeout(180_000);
	await page.emulateMedia({ reducedMotion: 'reduce' });
	const email = `e2e-gallery-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const headers = { 'content-type': 'application/json' };
	const body = JSON.stringify({ email, password: 'correct horse battery' });
	expect((await fetch(`${API}/v1/auth/register`, { method: 'POST', headers, body })).ok).toBeTruthy();
	const { token } = await (await fetch(`${API}/v1/auth/login`, { method: 'POST', headers, body })).json();
	const auth = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const today = await (await fetch(`${API}/v1/me/today`, { headers: auth })).json();
	const session = await (
		await fetch(`${API}/v1/practice/sessions`, {
			method: 'POST',
			headers: auth,
			body: JSON.stringify({ preset: 'tutor', chapter_id: today.tasks[0].chapter_id, question_count: 2 })
		})
	).json();

	for (const vp of viewports) {
		await page.setViewportSize({ width: vp.width, height: vp.height });
		await page.goto('/login');
		await shoot(page, `app-${vp.name}-login`);
	}

	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	for (const theme of ['dark', 'light']) {
		if (theme === 'light') {
			await page.addInitScript(() => localStorage.setItem('mlos_theme', 'light'));
		}
		for (const vp of viewports) {
			await page.setViewportSize({ width: vp.width, height: vp.height });
			for (const path of pages) {
				await page.goto(path);
				await shoot(page, `app-${vp.name}-${theme}${path.replace('/', '-')}`);
			}
			await page.goto(`/session/${session.session_id}`);
			await expect(page.getByText(/Question \d of/)).toBeVisible();
			if (await page.getByTestId('answer').isVisible()) {
				await page.getByTestId('option-0').click();
				await page.getByTestId('answer').click();
			}
			await page.getByTestId('feedback').waitFor({ timeout: 5_000 }).catch(() => {});
			await shoot(page, `app-${vp.name}-${theme}-session`);
		}

		await page.setViewportSize({ width: 390, height: 844 });
		await page.goto('/today');
		await page.getByRole('button', { name: 'Menu' }).click();
		await shoot(page, `app-phone-${theme}-menu-sheet`, false);
		await page.setViewportSize({ width: 1280, height: 800 });
		await page.goto('/today');
		// The palette toggles from a window keydown listener registered at
		// hydration; under CI load the first press can land before the
		// listener exists or while the desktop nav is still hidden. Retry
		// the keystroke until it sticks instead of racing one press.
		const palette = page.getByTestId('command-palette');
		await expect
			.poll(
				async () => {
					await page.keyboard.press('Control+k');
					try {
						await palette.waitFor({ state: 'visible', timeout: 1_500 });
						return true;
					} catch {
						return palette.isVisible();
					}
				},
				{ timeout: 20_000, intervals: [500] },
			)
			.toBe(true);
		await expect(palette).toBeVisible();
		await shoot(page, `app-desktop-${theme}-quick-jump`, false);
	}
});

test('site gallery', async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	for (const vp of viewports) {
		await page.setViewportSize({ width: vp.width, height: vp.height });
		for (const path of ['/', '/exams/', '/pricing/', '/help/']) {
			await page.goto(`http://127.0.0.1:4174${path}`);
			const slug = path === '/' ? 'home' : path.replaceAll('/', '');
			await shoot(page, `site-${vp.name}-${slug}`);
		}
	}
});
