import { expect, test } from '@playwright/test';

const API = process.env.E2E_API_BASE ?? process.env.VITE_API_BASE ?? 'http://127.0.0.1:8080';

test('a learner at the device limit can revoke a device and register this browser', async ({ page, request }) => {
	const email = `e2e-device-limit-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const password = 'correct horse battery';
	expect((await request.post(`${API}/v1/auth/register`, { data: { email, password } })).ok()).toBeTruthy();
	for (let index = 0; index < 5; index += 1) {
		const login = await request.post(`${API}/v1/auth/login`, { data: { email, password } });
		expect(login.ok()).toBeTruthy();
		const { token } = await login.json();
		const registration = await request.post(`${API}/v1/me/devices`, {
			headers: { authorization: `Bearer ${token}` },
			data: { device_key: `limit-fixture-${index}`, label: `Study device ${index + 1}` }
		});
		expect(registration.ok()).toBeTruthy();
	}
	const login = await request.post(`${API}/v1/auth/login`, { data: { email, password } });
	expect(login.ok()).toBeTruthy();
	const { token } = await login.json();
	await page.addInitScript((bearer) => localStorage.setItem('mlos_token', bearer), token);
	await page.goto('/account');
	await expect(page.getByText('This browser is not registered.', { exact: false })).toBeVisible();
	await expect(page.getByTestId('device-list').getByRole('button', { name: 'Revoke', exact: true })).toHaveCount(5);
	await page.getByTestId('device-list').getByRole('button', { name: 'Revoke', exact: true }).first().click();
	await expect(page.getByTestId('device-list').getByText('Revoked', { exact: true })).toHaveCount(1);
	await page.reload();
	await expect(page.getByText('Current device', { exact: true })).toBeVisible();
	await expect(page.getByText('This browser is not registered.', { exact: false })).toHaveCount(0);
	const headers = { authorization: `Bearer ${token}` };
	const response = await request.get(`${API}/v1/me/devices`, { headers });
	expect(response.ok()).toBeTruthy();
	const { devices } = await response.json();
	expect(devices.filter((device: { revoked_at: string | null }) => device.revoked_at === null)).toHaveLength(5);
	const key = await page.evaluate(() => localStorage.getItem('mlos_pack_device'));
	expect(devices.some((device: { device_key: string }) => device.device_key === key)).toBe(true);
});

test('password sign-in binds the browser device and revocation rejects its bearer', async ({
	page,
	request
}) => {
	const email = `e2e-device-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const password = 'correct horse battery';
	const apiRequests: Array<{ method: string; path: string }> = [];
	const apiOrigin = new URL(API).origin;

	page.on('request', (req) => {
		const url = new URL(req.url());
		if (url.origin === apiOrigin && url.pathname.startsWith('/v1/')) {
			apiRequests.push({ method: req.method(), path: url.pathname });
		}
	});

	await page.goto('/login');
	await page.getByTestId('toggle-mode').click();
	await page.getByTestId('email').fill(email);
	await page.getByTestId('password').fill(password);
	await page.getByTestId('submit').click();
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	await expect
		.poll(() => apiRequests.some(({ path }) => path === '/v1/me/today'))
		.toBe(true);

	const { browserLoginToken, deviceKey } = await page.evaluate(() => ({
		browserLoginToken: localStorage.getItem('mlos_token'),
		deviceKey: localStorage.getItem('mlos_pack_device')
	}));
	const token = browserLoginToken;
	expect(token).toBeTruthy();
	expect(deviceKey).toBeTruthy();
	if (!token || !deviceKey) {
		throw new Error('Sign-in did not persist its token and browser device key.');
	}

	const registrationIndex = apiRequests.findIndex(
		({ method, path }) => method === 'POST' && path === '/v1/me/devices'
	);
	const firstProtectedIndex = apiRequests.findIndex(
		({ path }) =>
			path.startsWith('/v1/me/') && path !== '/v1/me/devices' && path !== '/v1/me/devices/'
	);
	expect(registrationIndex).toBeGreaterThanOrEqual(0);
	expect(registrationIndex).toBeLessThan(firstProtectedIndex);

	// A separately issued bearer injected into storage follows the same
	// registration path when the browser reloads and hydrates its auth state.
	const loadedLogin = await request.post(`${API}/v1/auth/login`, {
		headers: { 'content-type': 'application/json' },
		data: { email, password }
	});
	expect(loadedLogin.ok()).toBeTruthy();
	const loadedLoginBody = (await loadedLogin.json()) as { token: string };
	const loadedToken = loadedLoginBody.token;
	const beforeReload = apiRequests.length;
	await page.evaluate((value) => localStorage.setItem('mlos_token', value), loadedToken);
	await page.reload();
	await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();
	await expect
		.poll(() => apiRequests.slice(beforeReload).some(({ path }) => path === '/v1/me/today'))
		.toBe(true);

	const loadedRequests = apiRequests.slice(beforeReload);
	const loadedRegistrationIndex = loadedRequests.findIndex(
		({ method, path }) => method === 'POST' && path === '/v1/me/devices'
	);
	const loadedProtectedIndex = loadedRequests.findIndex(
		({ path }) => path.startsWith('/v1/me/') && path !== '/v1/me/devices'
	);
	expect(loadedRegistrationIndex).toBeGreaterThanOrEqual(0);
	expect(loadedRegistrationIndex).toBeLessThan(loadedProtectedIndex);
	expect(await page.evaluate(() => localStorage.getItem('mlos_token'))).toBe(loadedToken);

	const headers = { authorization: `Bearer ${token}` };
	const loadedHeaders = { authorization: `Bearer ${loadedToken}` };
	const devicesResponse = await request.get(`${API}/v1/me/devices`, { headers: loadedHeaders });
	expect(devicesResponse.ok()).toBeTruthy();
	const { devices } = (await devicesResponse.json()) as {
		devices: Array<{ device_id: string; device_key: string; revoked_at: string | null }>;
	};
	const browserDevice = devices.find((device) => device.device_key === deviceKey);
	expect(browserDevice).toBeDefined();
	expect(browserDevice?.revoked_at).toBeNull();

	const revokeResponse = await request.delete(
		`${API}/v1/me/devices/${browserDevice?.device_id}`,
		{ headers: loadedHeaders }
	);
	expect(revokeResponse.ok()).toBeTruthy();

	const [originalDenied, loadedDenied] = await Promise.all([
		request.get(`${API}/v1/me/today`, { headers }),
		request.get(`${API}/v1/me/today`, { headers: loadedHeaders })
	]);
	expect(originalDenied.status()).toBe(401);
	expect(loadedDenied.status()).toBe(401);
	const packResourcesDenied = await request.post(
		`${API}/v2/packs/00000000-0000-4000-8000-000000000000/resources`,
		{
			headers: loadedHeaders,
			data: {
				device_id: deviceKey,
				chapters: ['00000000-0000-4000-8000-000000000001'],
				question_version_ids: ['00000000-0000-4000-8000-000000000002']
			}
		}
	);
	expect(packResourcesDenied.status()).toBe(401);

	// A reload has to revalidate the stored bearer; the revoked session is
	// refused during device registration and the client clears it on 401.
	await page.reload();
	await expect
		.poll(() => page.evaluate(() => localStorage.getItem('mlos_token')), { timeout: 10_000 })
		.toBeNull();
});
