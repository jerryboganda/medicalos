import type { Page } from '@playwright/test';

// Existing intercepted UI fixtures use fixed synthetic bearers. Keep their
// device seam in the test harness; genuine login sessions reach the API.
export async function mockDeviceRegistration(page: Page, tokens: string[]): Promise<void> {
	await page.route('**/v1/me/devices', async (route) => {
		const bearer = route.request().headers()['authorization']?.replace(/^Bearer /, '');
		if (route.request().method() !== 'POST' || !bearer || !tokens.includes(bearer)) {
			await route.fallback();
			return;
		}
		await route.fulfill({ json: { device_id: 'synthetic-ui-device' } });
	});
}
