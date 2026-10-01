import { expect, test, type Page } from '@playwright/test';
import { installE2EBrowserSession, registerE2EDevice } from './device-binding';

const API = 'http://127.0.0.1:8080';

// App shell (design.md § App shell; plan §6, §7.3, §7.4): side rail on
// desktop, tab bar + sheet on mobile, the ⌘K quick jump, the light reading
// theme, and no primary navigation inside a study session.

async function signIn(page: Page): Promise<string> {
	const email = `e2e-shell-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const headers = { 'content-type': 'application/json' };
	const body = JSON.stringify({ email, password: 'correct horse battery' });
	expect((await fetch(`${API}/v1/auth/register`, { method: 'POST', headers, body })).ok).toBeTruthy();
	const { token } = await (await fetch(`${API}/v1/auth/login`, { method: 'POST', headers, body })).json();
	const deviceKey = await registerE2EDevice(API, token);
	await installE2EBrowserSession(page, token, deviceKey);
	return token;
}

test('desktop rail marks the current page and quick jump navigates', async ({ page }) => {
	await signIn(page);
	await page.setViewportSize({ width: 1280, height: 800 });
	await page.goto('/today');
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();

	await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
	await expect(page.getByTestId('nav-today')).toHaveAttribute('aria-current', 'page');
	await expect(page.getByRole('button', { name: 'Menu' })).toBeHidden();
	await expect(page.getByRole('navigation', { name: 'Quick navigation' })).toBeHidden();

	await page.getByTestId('nav-progress').click();
	await expect(page).toHaveURL(/\/progress$/);
	await expect(page.getByTestId('nav-progress')).toHaveAttribute('aria-current', 'page');
	await expect(page.getByTestId('nav-today')).not.toHaveAttribute('aria-current', 'page');

	// Ctrl+K opens the palette; typing filters; Enter opens the first match.
	await page.keyboard.press('Control+k');
	const palette = page.getByTestId('command-palette');
	await expect(palette).toBeVisible();
	await page.getByTestId('command-input').fill('libr');
	await page.keyboard.press('Enter');
	await expect(page).toHaveURL(/\/library$/);
	await expect(palette).toBeHidden();

	// The visible trigger opens it too, and Escape closes it.
	await page.getByTestId('quick-jump').click();
	await expect(palette).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(palette).toBeHidden();
});

test('mobile tab bar and the Menu sheet', async ({ page }) => {
	await signIn(page);
	await page.setViewportSize({ width: 375, height: 812 });
	await page.goto('/today');
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();

	await expect(page.getByRole('navigation', { name: 'Quick navigation' })).toBeVisible();
	await expect(page.getByTestId('tab-today')).toHaveAttribute('aria-current', 'page');
	await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeHidden();

	await page.getByTestId('tab-practice').click();
	await expect(page).toHaveURL(/\/practice$/);
	await expect(page.getByTestId('tab-practice')).toHaveAttribute('aria-current', 'page');

	const sheet = page.getByRole('navigation', { name: 'Main navigation' });
	// "Menu" becomes "Close menu" while open; Escape closes it too.
	await page.getByRole('button', { name: 'Menu' }).click();
	await expect(sheet).toBeVisible();
	await page.getByRole('button', { name: 'Close menu' }).click();
	await expect(sheet).toBeHidden();
	await page.getByRole('button', { name: 'Menu' }).click();
	await expect(sheet).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(sheet).toBeHidden();

	await page.getByRole('button', { name: 'Menu' }).click();
	await sheet.getByTestId('nav-notes').click();
	await expect(page).toHaveURL(/\/notes$/);
	await expect(sheet).toBeHidden();
	await expect(page.locator('#main')).toHaveJSProperty('inert', false);
	await expect(page.locator('nav[aria-label="Quick navigation"]')).toHaveJSProperty('inert', false);

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		await expect(page.getByRole('navigation', { name: 'Quick navigation' })).toBeVisible();
		expect(
			await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
			`shell overflows at ${width}px`
		).toBe(true);
	}
});

test('mobile menu manages focus, inertness, and desktop resize', async ({ page }) => {
	await signIn(page);
	await page.setViewportSize({ width: 375, height: 812 });
	await page.goto('/today');

	const toggle = page.getByRole('button', { name: 'Menu' });
	const menu = page.getByRole('navigation', { name: 'Main navigation' });
	const closeToggle = page.getByRole('button', { name: 'Close menu' });
	const main = page.locator('#main');
	const tabs = page.locator('nav[aria-label="Quick navigation"]');
	const brand = page.locator('.brand');
	const skipLink = page.locator('.skip-link');
	const firstItem = menu.locator('a').first();
	const lastItem = menu.locator('a, button').last();

	await toggle.focus();
	await page.keyboard.press('Enter');
	await expect(menu).toBeVisible();
	await expect(closeToggle).toHaveAttribute('aria-expanded', 'true');
	await expect(firstItem).toBeFocused();
	await expect(main).toHaveJSProperty('inert', true);
	await expect(tabs).toHaveJSProperty('inert', true);
	await expect(page.getByTestId('quick-jump')).toHaveJSProperty('inert', true);
	await expect(brand).toHaveJSProperty('inert', true);
	await expect(skipLink).toHaveJSProperty('inert', true);

	// The close toggle and sheet items form one keyboard focus cycle.
	await page.keyboard.press('Shift+Tab');
	await expect(closeToggle).toBeFocused();
	await page.keyboard.press('Shift+Tab');
	await expect(lastItem).toBeFocused();
	await page.keyboard.press('Tab');
	await expect(closeToggle).toBeFocused();
	await page.keyboard.press('Tab');
	await expect(firstItem).toBeFocused();

	await page.keyboard.press('Escape');
	await expect(menu).toBeHidden();
	await expect(toggle).toHaveAttribute('aria-expanded', 'false');
	await expect(toggle).toBeFocused();
	await expect(main).toHaveJSProperty('inert', false);
	await expect(tabs).toHaveJSProperty('inert', false);
	await expect(page.getByTestId('quick-jump')).toHaveJSProperty('inert', false);
	await expect(brand).toHaveJSProperty('inert', false);
	await expect(skipLink).toHaveJSProperty('inert', false);

	// Crossing the desktop breakpoint closes the mobile state and keeps focus
	// on a visible navigation item if the disappearing toggle held focus.
	await toggle.click();
	await expect(firstItem).toBeFocused();
	await closeToggle.focus();
	await page.setViewportSize({ width: 1280, height: 800 });
	await expect(menu).toBeVisible();
	await expect(page.getByTestId('nav-today')).toBeFocused();
	await expect(main).toHaveJSProperty('inert', false);
	await expect(tabs).toHaveJSProperty('inert', false);
	await lastItem.focus();
	await page.keyboard.press('Tab');
	await expect(menu.locator(':focus')).toHaveCount(0);
});

test('the light reading theme persists across reloads', async ({ page }) => {
	await signIn(page);
	await page.setViewportSize({ width: 1280, height: 800 });
	await page.goto('/today');
	const html = page.locator('html');
	const toggle = page.getByTestId('theme-toggle');
	await expect(toggle).toHaveAttribute('aria-pressed', 'false');

	await toggle.click();
	await expect(html).toHaveAttribute('data-theme', 'light');
	await expect(toggle).toHaveAttribute('aria-pressed', 'true');

	await page.reload();
	await expect(html).toHaveAttribute('data-theme', 'light');
	await page.getByTestId('theme-toggle').click();
	await expect(html).not.toHaveAttribute('data-theme', 'light');
});

test('a study session hides primary navigation but keeps the way out', async ({ page }) => {
	const token = await signIn(page);
	const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const today = await (await fetch(`${API}/v1/me/today`, { headers })).json();
	const session = await (
		await fetch(`${API}/v1/practice/sessions`, {
			method: 'POST',
			headers,
			body: JSON.stringify({ preset: 'tutor', chapter_id: today.tasks[0].chapter_id, question_count: 1 })
		})
	).json();

	await page.setViewportSize({ width: 1280, height: 800 });
	await page.goto(`/session/${session.session_id}`);
	await expect(page.getByText('Question 1 of 1')).toBeVisible();
	await expect(page.getByRole('navigation', { name: 'Main navigation' })).toHaveCount(0);
	await expect(page.getByTestId('quick-jump')).toHaveCount(0);
	await page.keyboard.press('Control+k');
	await expect(page.getByTestId('command-palette')).toHaveCount(0);

	await page.getByRole('link', { name: 'Medical Learning OS' }).click();
	await expect(page).toHaveURL(/\/today$/);
	await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
});
