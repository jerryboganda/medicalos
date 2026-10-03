import { randomUUID } from 'node:crypto';
import type { Page } from '@playwright/test';

export async function registerE2EDevice(apiBase: string, token: string): Promise<string> {
	const deviceKey = `e2e-${randomUUID()}`;
	const response = await fetch(`${apiBase}/v1/me/devices`, {
		method: 'POST',
		headers: {
			'content-type': 'application/json',
			authorization: `Bearer ${token}`
		},
		body: JSON.stringify({ device_key: deviceKey, label: 'Playwright' })
	});
	if (!response.ok) {
		throw new Error(`E2E device registration failed (${response.status})`);
	}
	return deviceKey;
}

export async function installE2EBrowserSession(
	page: Page,
	token: string,
	deviceKey: string
): Promise<void> {
	await page.addInitScript(
		({ bearer, key }) => {
			localStorage.setItem('mlos_token', bearer);
			localStorage.setItem('mlos_pack_device', key);
		},
		{ bearer: token, key: deviceKey }
	);
}
