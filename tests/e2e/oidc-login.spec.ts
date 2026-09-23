import { expect, test } from '@playwright/test';

test('institution OIDC sign-in completes the browser handoff and stays usable on mobile', async ({
	page
}) => {
	const institutionId = '10203040-5060-7080-90ab-cdef10203040';
	const authorizationUrl =
		'https://idp.test/authorize?client_id=medical-os-test&response_type=code&scope=openid&state=test-state&nonce=test-nonce&code_challenge=test-challenge&code_challenge_method=S256';

	await page.route('**/v1/institutions/*/sso/oidc/start', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ authorization_url: authorizationUrl })
		})
	);
	await page.route('https://idp.test/**', (route) =>
		route.fulfill({ status: 200, contentType: 'text/html', body: 'Identity provider' })
	);

	await page.goto('/login');
	await page.getByTestId('sso-institution-id').fill(institutionId);
	await page.getByTestId('sso-submit').click();
	await expect(page).toHaveURL(/https:\/\/idp\.test\/authorize/);

	let ticketWasRemovedBeforeExchange = false;
	await page.route('**/v1/auth/oidc/complete', async (route) => {
		ticketWasRemovedBeforeExchange = new URL(page.url()).hash === '';
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ token: 'e2e-session-token' })
		});
	});
	await page.route('**/v1/me/today', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				plan_id: 'plan', version: 1, tasks: [], revisions: [], learner: [],
				revision_budget: { automatic_used: 0, automatic_limit: 3, total_used: 0, total_limit: 8 }
			})
		})
	);
	await page.route('**/v1/mocks', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ mocks: [] })
		})
	);
	await page.route('**/v1/me/engagement', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ enabled: false })
		})
	);

	await page.goto('/login/sso/callback#ticket=one-use-browser-ticket');
	await expect(page).toHaveURL(/\/today$/);
	await expect.poll(() => page.evaluate(() => localStorage.getItem('mlos_token'))).toBe(
		'e2e-session-token'
	);
	await expect.poll(() => ticketWasRemovedBeforeExchange).toBe(true);

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 900 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `horizontal overflow at ${width}px`).toBe(true);
		const menu = page.getByRole('button', { name: 'Menu' });
		await expect(menu).toBeVisible();
		await menu.click();
		await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
		const expandedNoOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(expandedNoOverflow, `expanded nav overflow at ${width}px`).toBe(true);
		await page.getByRole('button', { name: 'Close menu' }).click();
	}
});

test('admin configures an institution provider without reading its secret back', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('mlos_admin', 'e2e-admin-token'));
	await page.route('**/v1/auth/login', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ token: 'e2e-user-token' })
		})
	);
	await page.route('**/v1/admin/audit', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ events: [] })
		})
	);
	await page.route('**/v1/me/today', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				plan_id: 'plan', version: 1, tasks: [], revisions: [], learner: [],
				revision_budget: { automatic_used: 0, automatic_limit: 3, total_used: 0, total_limit: 8 }
			})
		})
	);
	await page.route('**/v1/mocks', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ mocks: [] })
		})
	);
	await page.route('**/v1/me/engagement', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ enabled: false })
		})
	);

	const institutionId = '10203040-5060-7080-90ab-cdef10203040';
	let saved: Record<string, unknown> | undefined;
	await page.route('**/v1/admin/institutions/*/sso/oidc', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 404,
				contentType: 'application/json',
				body: JSON.stringify({
					error: { code: 'oidc_provider_not_found', message: 'provider not found' }
				})
			});
			return;
		}
		saved = route.request().postDataJSON();
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				issuer: 'https://idp.example.test/tenant',
				client_id: 'medical-os-client',
				enabled: true,
				client_secret_configured: true
			})
		});
	});

	await page.goto('/login');
	await page.getByTestId('email').fill('operator@example.test');
	await page.getByTestId('password').fill('correct horse battery');
	await page.getByTestId('submit').click();
	await expect(page).toHaveURL(/\/today$/);
	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.goto('/admin');
	await page.getByTestId('oidc-institution-id').fill(institutionId);
	await page.getByTestId('oidc-load').click();
	await expect(page.getByText('No provider is configured yet.')).toBeVisible();
	await page.getByTestId('oidc-issuer').fill('https://idp.example.test/tenant');
	await page.getByTestId('oidc-client-id').fill('medical-os-client');
	await page.getByTestId('oidc-client-secret').fill('test-secret-that-is-never-returned');
	await page.getByTestId('oidc-enabled').selectOption('true');
	await page.getByTestId('oidc-save').click();
	await expect(page.getByText('Provider settings saved.')).toBeVisible();
	await expect(page.getByRole('link', { name: 'Open learner sign-in' })).toHaveAttribute(
		'href',
		new RegExp(`/login\\?institution=${institutionId}`)
	);
	await expect(page.getByTestId('oidc-client-secret')).toHaveValue('');
	await expect(page.locator('body')).not.toContainText('test-secret-that-is-never-returned');
	expect(saved?.client_secret).toBe('test-secret-that-is-never-returned');
	expect(saved?.enabled).toBe(true);
});
