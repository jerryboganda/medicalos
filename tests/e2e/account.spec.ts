import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';

type DeviceFixture = {
	device_id: string;
	device_key: string;
	label: string;
	created_at: string;
	last_seen_at: string | null;
	revoked_at: string | null;
};

type ApiRequest = { method: string; path: string; authorization: string | undefined };

type SetupOptions = {
	devices?: DeviceFixture[];
	failFirstDeviceList?: boolean;
	holdFirstDeviceList?: boolean;
	failFirstSessionPolicy?: boolean;
	deviceRegistrationExhausted?: boolean;
	singleActiveSession?: boolean;
};

const token = 'synthetic-account-token';
const currentDeviceKey = 'current-device-key';
const currentDeviceId = '11111111-1111-4111-8111-111111111111';
const otherDeviceId = '22222222-2222-4222-8222-222222222222';
const devicesFixture: DeviceFixture[] = [
	{
		device_id: currentDeviceId,
		device_key: currentDeviceKey,
		label: 'This browser',
		created_at: '2026-10-01T08:00:00.000Z',
		last_seen_at: '2026-10-01T08:30:00.000Z',
		revoked_at: null
	},
	{
		device_id: otherDeviceId,
		device_key: 'study-tablet-key',
		label: 'Study tablet',
		created_at: '2026-09-20T08:00:00.000Z',
		last_seen_at: '2026-09-30T08:30:00.000Z',
		revoked_at: null
	}
];

const exportFixture = {
	exported_at: '2026-10-01T08:30:00.000Z',
	account: { email: 'learner@example.test', tier: 'free', created_at: '2026-09-01T08:00:00.000Z' },
	attempts: [{ question_version_id: 'question-1', chosen_index: 1, correct: true }],
	notes: [{ title: 'Study note', body: 'Fixture note' }],
	card_reviews: [{ card_id: 'card-1', rating: 4 }],
	portfolio: [{ kind: 'achievement', title: 'Fixture item' }]
};

async function setupAccount(page: Page, options: SetupOptions = {}) {
	const devices = (options.devices ?? devicesFixture).map((device) => ({ ...device }));
	const requests: ApiRequest[] = [];
	let listReads = 0;
	let sessionPolicy = options.singleActiveSession ?? false;
	let sessionPolicyReads = 0;
	let accountDeleted = false;
	let releaseFirstDeviceList!: () => void;
	let signalFirstDeviceList!: () => void;
	const firstDeviceListGate = new Promise<void>((resolve) => (releaseFirstDeviceList = resolve));
	const firstDeviceListStarted = new Promise<void>((resolve) => (signalFirstDeviceList = resolve));
	if (!options.holdFirstDeviceList) releaseFirstDeviceList();

	await page.addInitScript(({ authToken, deviceKey }) => {
		localStorage.setItem('mlos_token', authToken);
		localStorage.setItem('mlos_pack_device', deviceKey);
	}, { authToken: token, deviceKey: currentDeviceKey });

	await page.route('**/v1/**', async (route) => {
		const request = route.request();
		const url = new URL(request.url());
		requests.push({
			method: request.method(),
			path: url.pathname,
			authorization: request.headers().authorization
		});

		if (url.pathname === '/v1/me/devices' && request.method() === 'POST') {
			expect(request.postDataJSON().device_key).toBe(currentDeviceKey);
			if (options.deviceRegistrationExhausted) {
				await route.fulfill({
					status: 403,
					json: { error: { code: 'devices_exhausted', message: 'device limit reached' } }
				});
				return;
			}
			await route.fulfill({ status: 200, json: devicesFixture[0] });
			return;
		}

		if (url.pathname === '/v1/me/devices' && request.method() === 'GET') {
			listReads += 1;
			if (listReads === 1) {
				signalFirstDeviceList();
				await firstDeviceListGate;
				if (options.failFirstDeviceList) {
					await route.fulfill({
						status: 503,
						contentType: 'application/json',
						body: JSON.stringify({ error: { code: 'unavailable', message: 'Device service unavailable' } })
					});
					return;
				}
			}
			await route.fulfill({ status: 200, json: { devices } });
			return;
		}

		if (url.pathname === '/v1/me/session-policy' && request.method() === 'GET') {
			sessionPolicyReads += 1;
			if (options.failFirstSessionPolicy && sessionPolicyReads === 1) {
				await route.fulfill({
					status: 503,
					contentType: 'application/json',
					body: JSON.stringify({ error: { code: 'unavailable', message: 'Session settings unavailable' } })
				});
				return;
			}
			await route.fulfill({ status: 200, json: { single_active_session: sessionPolicy } });
			return;
		}

		if (url.pathname === '/v1/me/session-policy' && request.method() === 'PATCH') {
			sessionPolicy = request.postDataJSON()?.single_active_session;
			await route.fulfill({ status: 200, json: { single_active_session: sessionPolicy } });
			return;
		}

		if (url.pathname.startsWith('/v1/me/devices/') && request.method() === 'DELETE') {
			const deviceId = decodeURIComponent(url.pathname.split('/').at(-1) ?? '');
			const device = devices.find((item) => item.device_id === deviceId && item.revoked_at === null);
			if (!device) {
				await route.fulfill({ status: 404, json: { error: { code: 'device_not_found', message: 'Device not found' } } });
				return;
			}
			device.revoked_at = '2026-10-01T09:00:00.000Z';
			await route.fulfill({ status: 200, json: { revoked: true } });
			return;
		}

		if (url.pathname === '/v1/me/export' && request.method() === 'GET') {
			await route.fulfill({ status: 200, json: exportFixture });
			return;
		}

		if (url.pathname === '/v1/me/account' && request.method() === 'DELETE') {
			accountDeleted = true;
			await route.fulfill({ status: 200, json: { deleted: true } });
			return;
		}

		if (accountDeleted && url.pathname.startsWith('/v1/me/')) {
			await route.fulfill({ status: 401, json: { error: { code: 'unauthorized', message: 'Account access is disabled' } } });
			return;
		}

		await route.fulfill({ status: 404, json: { error: { code: 'not_found', message: 'Unexpected test request' } } });
	});

	return { requests, firstDeviceListStarted, releaseFirstDeviceList, get listReads() { return listReads; } };
}

test('learner sees the saved session policy and reviews its sign-out effect before enabling it', async ({ page }) => {
	const api = await setupAccount(page);
	await page.goto('/account');
	const security = page.getByRole('region', { name: 'Session security' });
	await expect(security).toContainText('Single active session Off');
	const enable = security.getByRole('button', { name: 'Enable single-session protection' });
	await expect(enable).toHaveAttribute('aria-expanded', 'false');
	await expect(enable).toHaveAttribute('aria-controls', 'session-policy-review');
	await enable.click();
	await expect(enable).toHaveAttribute('aria-expanded', 'true');
	await expect(page.getByTestId('session-policy-review')).toContainText(/signs out every active session, including this browser/i);
	await page.getByTestId('session-policy-review').getByRole('button', { name: 'Cancel' }).click();
	await expect(page.getByTestId('session-policy-review')).toHaveCount(0);
	await expect(enable).toBeFocused();
	await security.getByRole('button', { name: 'Enable single-session protection' }).click();
	await page.getByTestId('session-policy-review').getByRole('button', { name: 'Enable and sign out' }).click();
	await expect(page.getByRole('heading', { name: 'Signed out' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Go to sign in' })).toBeVisible();
	expect(await page.evaluate(() => localStorage.getItem('mlos_token'))).toBeNull();
	expect(api.requests).toContainEqual({ method: 'GET', path: '/v1/me/session-policy', authorization: `Bearer ${token}` });
	expect(api.requests).toContainEqual({ method: 'PATCH', path: '/v1/me/session-policy', authorization: `Bearer ${token}` });
});

test('learner can retry loading a session policy without seeing a default value', async ({ page }) => {
	await setupAccount(page, { failFirstSessionPolicy: true });
	await page.goto('/account');
	const security = page.getByRole('region', { name: 'Session security' });
	await expect(security.getByTestId('session-policy-error')).toBeVisible();
	await expect(security.getByText(/Single active session/)).toHaveCount(0);
	await security.getByRole('button', { name: 'Retry session setting' }).click();
	await expect(security).toContainText('Single active session Off');
});

test('learner can change session policy when a new browser has exhausted the device limit', async ({ page }) => {
	const devices = Array.from({ length: 5 }, (_, index) => ({
		...devicesFixture[0],
		device_id: `other-device-${index}`,
		device_key: `other-device-key-${index}`,
		label: `Study device ${index + 1}`
	}));
	const api = await setupAccount(page, {
		devices,
		deviceRegistrationExhausted: true,
		singleActiveSession: true
	});
	await page.goto('/account');
	const security = page.getByRole('region', { name: 'Session security' });
	await expect(security).toContainText('Single active session On');
	await security.getByRole('button', { name: 'Allow multiple sessions' }).click();
	await expect(security).toContainText('Multiple active sessions are allowed.');
	expect(api.requests).toContainEqual({
		method: 'PATCH',
		path: '/v1/me/session-policy',
		authorization: `Bearer ${token}`
	});
});

test('learner reviews devices, revokes another device, and signs out by revoking this browser', async ({ page }) => {
	const api = await setupAccount(page);
	await page.setViewportSize({ width: 1280, height: 800 });
	await page.goto('/account');

	await expect(page.getByRole('heading', { name: 'Account', exact: true })).toBeVisible();
	await expect(page.getByTestId('nav-account')).toHaveAttribute('aria-current', 'page');
	await expect(page.locator('nav[aria-label="Quick navigation"] a')).toHaveCount(5);
	await expect(page.getByTestId(`device-${currentDeviceId}`)).toContainText('Current device');
	await expect(page.getByTestId(`device-${otherDeviceId}`)).toContainText('Study tablet');

	await page.keyboard.press('Control+k');
	const palette = page.getByTestId('command-palette');
	await page.getByTestId('command-input').fill('privacy');
	await expect(palette.getByRole('option', { name: /Account/ })).toBeVisible();
	await page.keyboard.press('Escape');

	await page.getByTestId(`device-${otherDeviceId}`).getByRole('button', { name: 'Revoke' }).click();
	await expect(page.getByTestId(`device-${otherDeviceId}`)).toContainText('Revoked');
	await expect(page.getByTestId(`device-${otherDeviceId}`).getByRole('button')).toHaveCount(0);

	await page.getByTestId(`device-${currentDeviceId}`).getByRole('button', { name: 'Sign out this device' }).click();
	await expect(page.getByRole('heading', { name: 'Signed out' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Go to sign in' })).toBeVisible();
	expect(await page.evaluate(() => localStorage.getItem('mlos_token'))).toBeNull();

	expect(api.requests).toContainEqual({
		method: 'DELETE',
		path: `/v1/me/devices/${otherDeviceId}`,
		authorization: `Bearer ${token}`
	});
	expect(api.requests).toContainEqual({
		method: 'DELETE',
		path: `/v1/me/devices/${currentDeviceId}`,
		authorization: `Bearer ${token}`
	});
});

test('learner downloads the supported partial account export as JSON', async ({ page }) => {
	await setupAccount(page);
	await page.goto('/account');
	await expect(page.getByText(/partial export/i)).toBeVisible();
	await expect(page.getByText(/some account data is not included/i)).toBeVisible();
	await expect(page.getByText(/account.*attempts.*notes.*reviews.*portfolio/i)).toBeVisible();

	const [download] = await Promise.all([
		page.waitForEvent('download'),
		page.getByRole('button', { name: 'Download account export' }).click()
	]);
	expect(download.suggestedFilename()).toMatch(/^medical-os-account-export-\d{4}-\d{2}-\d{2}\.json$/);
	const contents = JSON.parse(await readFile(await download.path(), 'utf8'));
	expect(contents).toMatchObject({
		account: { email: 'learner@example.test' },
		attempts: [{ question_version_id: 'question-1' }],
		notes: [{ title: 'Study note' }],
		card_reviews: [{ card_id: 'card-1' }],
		portfolio: [{ title: 'Fixture item' }]
	});
	await expect(page.getByRole('status')).toContainText('export downloaded');
});

test('account deletion is reversible until confirmed and clears local authentication', async ({ page }) => {
	const api = await setupAccount(page);
	await page.goto('/account');
	await page.getByRole('button', { name: 'Review account deletion' }).click();

	const confirmation = page.getByTestId('delete-confirmation');
	const acknowledgement = confirmation.getByRole('checkbox');
	const deleteButton = confirmation.getByRole('button', { name: 'Delete account' });
	await expect(deleteButton).toBeDisabled();
	await acknowledgement.check();
	await confirmation.getByRole('button', { name: 'Cancel' }).click();
	await expect(confirmation).toBeHidden();
	await expect(page.getByRole('button', { name: 'Review account deletion' })).toBeFocused();
	expect(api.requests.some((request) => request.method === 'DELETE' && request.path === '/v1/me/account')).toBe(false);

	await page.getByRole('button', { name: 'Review account deletion' }).click();
	const reopened = page.getByTestId('delete-confirmation');
	await expect(reopened.getByRole('checkbox')).not.toBeChecked();
	await reopened.getByRole('checkbox').check();
	await reopened.getByRole('button', { name: 'Delete account' }).click();

	await expect(page.getByRole('heading', { name: 'Access disabled' })).toBeVisible();
	await expect(page.getByText(/existing records remain stored/i)).toBeVisible();
	expect(await page.evaluate(() => localStorage.getItem('mlos_token'))).toBeNull();
	expect(api.requests).toContainEqual({
		method: 'DELETE',
		path: '/v1/me/account',
		authorization: `Bearer ${token}`
	});
});

test('device loading exposes an error, retry, and the empty state', async ({ page }) => {
	const api = await setupAccount(page, { devices: [], failFirstDeviceList: true, holdFirstDeviceList: true });
	const navigation = page.goto('/account');
	await api.firstDeviceListStarted;
	await expect(page.getByTestId('devices-loading')).toBeVisible();
	api.releaseFirstDeviceList();
	await navigation;

	await expect(page.getByTestId('devices-error')).toBeVisible();
	await page.getByRole('button', { name: 'Retry' }).click();
	await expect(page.getByTestId('devices-empty')).toBeVisible();
	expect(api.listReads).toBe(2);
});
