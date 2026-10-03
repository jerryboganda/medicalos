import { expect, test, type Page } from '@playwright/test';
import { mockDeviceRegistration } from './mock-device-registration';

const token = 'lti-session-token-for-browser-test';
test.beforeEach(async ({ page }) => mockDeviceRegistration(page, [token]));

async function expectResponsive(page: Page) {
	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 900 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
	}
}

test('resource launch hands the learner into the linked library article', async ({ page }) => {
	await page.addInitScript(({ sessionToken }) => {
		localStorage.setItem('mlos_token', sessionToken);
		sessionStorage.setItem(
			'mlos_lti_launch',
			JSON.stringify({
				message_type: 'LtiResourceLinkRequest',
				resource_link: {
					title: 'ECG fundamentals',
					custom: {
						medicalos_content_type: 'article',
						medicalos_article_slug: 'ecg-fundamentals',
						medicalos_jurisdiction: 'PK'
					}
				}
			})
		);
	}, { sessionToken: token });

	await page.goto('/lti/handoff');
	await expect(page.getByRole('heading', { name: 'ECG fundamentals' })).toBeVisible();
	await expect(page.getByRole('link', { name: 'Continue to article' })).toBeVisible();
	expect(await page.evaluate(() => localStorage.getItem('mlos_token'))).toBe(token);
	await expectResponsive(page);

	await page.getByRole('link', { name: 'Continue to article' }).click();
	await expect(page).toHaveURL(/\/library\/articles\/ecg-fundamentals\?jurisdiction=PK$/);
});

test('picker returns selected published articles in an LTI response form', async ({ page }) => {
	let submittedItems: unknown[] = [];
	let returnedForm: URLSearchParams | undefined;
	await page.addInitScript(({ sessionToken }) => {
		localStorage.setItem('mlos_token', sessionToken);
		sessionStorage.setItem(
			'mlos_lti_launch',
			JSON.stringify({
				message_type: 'LtiDeepLinkingRequest',
				deep_linking_settings: {
					accept_types: ['ltiResourceLink'],
					accept_presentation_document_targets: ['window'],
					accept_multiple: true,
					deep_link_return_url: 'https://lms.example.test/deep-links/return'
				}
			})
		);
	}, { sessionToken: token });

	await page.route('**/v1/library/search**', async (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				results: [
					{
						content_type: 'editorial_article',
						article_id: 'article-1',
						slug: 'ecg-fundamentals',
						title: 'ECG fundamentals',
						version: 2,
						jurisdiction: 'PK',
						effective_from: null,
						effective_to: null,
						as_of: '2026-10-02',
						score: 10,
						source_ref: 'Reviewed article',
						excerpt: 'A practical guide to ECG interpretation.'
					},
					{
						content_type: 'editorial_article',
						article_id: 'article-2',
						slug: 'acute-chest-pain',
						title: 'Acute chest pain',
						version: 1,
						jurisdiction: null,
						effective_from: null,
						effective_to: null,
						as_of: '2026-10-02',
						score: 8,
						source_ref: 'Reviewed article',
						excerpt: 'An approach to acute chest pain.'
					},
					{
						content_type: 'private_document',
						article_id: 'private-1',
						slug: 'private-notes',
						title: 'Private notes',
						version: 1,
						jurisdiction: null,
						effective_from: null,
						effective_to: null,
						as_of: '2026-10-02',
						score: 1,
						source_ref: '',
						excerpt: 'Must not be offered to an LMS.'
					}
				],
				private_documents: [{ title: 'Private notes' }]
			})
		})
	);
	await page.route('**/v1/lti/deep-links', async (route) => {
		submittedItems = route.request().postDataJSON().items;
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				post_url: 'https://lms.example.test/deep-links/return',
				jwt: 'signed-deep-link-response',
				platform_issuer: 'https://lms.example.test',
				deployed_at: '2026-10-02T00:00:00Z'
			})
		});
	});
	await page.route('https://lms.example.test/deep-links/return', async (route) => {
		returnedForm = new URLSearchParams(route.request().postData() ?? '');
		await route.fulfill({ status: 200, contentType: 'text/html', body: '<p>Returned to LMS</p>' });
	});

	await page.goto('/lti/deep-links');
	await expect(page.getByRole('heading', { name: 'Choose Medical OS content' })).toBeVisible();
	await page.getByRole('searchbox', { name: 'Search published articles' }).fill('ECG');
	await page.getByRole('button', { name: 'Search' }).click();
	await expect(page.getByText('ECG fundamentals', { exact: true })).toBeVisible();
	await expect(page.getByText('Acute chest pain', { exact: true })).toBeVisible();
	await expect(page.getByText('Private notes', { exact: true })).toHaveCount(0);
	await page.getByRole('checkbox', { name: 'ECG fundamentals' }).check();
	await page.getByRole('checkbox', { name: 'Acute chest pain' }).check();
	await expectResponsive(page);
	await page.getByRole('button', { name: 'Add selected articles' }).click();
	expect(submittedItems).toEqual([
		{
			type: 'ltiResourceLink',
			title: 'ECG fundamentals',
			url: 'http://127.0.0.1:4173/api/v1/lti/launch',
			custom: {
				medicalos_content_type: 'article',
				medicalos_article_slug: 'ecg-fundamentals',
				medicalos_jurisdiction: 'PK'
			}
		},
		{
			type: 'ltiResourceLink',
			title: 'Acute chest pain',
			url: 'http://127.0.0.1:4173/api/v1/lti/launch',
			custom: {
				medicalos_content_type: 'article',
				medicalos_article_slug: 'acute-chest-pain'
			}
		}
	]);
	await expect(page.getByText('Returned to LMS')).toBeVisible();
	expect(returnedForm?.get('JWT')).toBe('signed-deep-link-response');
});

test('picker can return an empty selection to close the LMS flow', async ({ page }) => {
	await page.addInitScript(({ sessionToken }) => {
		localStorage.setItem('mlos_token', sessionToken);
		sessionStorage.setItem(
			'mlos_lti_launch',
			JSON.stringify({
				message_type: 'LtiDeepLinkingRequest',
				deep_linking_settings: {
					accept_types: ['ltiResourceLink'],
					accept_presentation_document_targets: ['window'],
					accept_multiple: false,
					deep_link_return_url: 'https://lms.example.test/deep-links/return'
				}
			})
		);
	}, { sessionToken: token });
	let submittedItems: unknown[] | undefined;
	let returnedJwt: string | null = null;
	await page.route('**/v1/lti/deep-links', async (route) => {
		submittedItems = route.request().postDataJSON().items;
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				post_url: 'https://lms.example.test/deep-links/return',
				jwt: 'signed-empty-response',
				platform_issuer: 'https://lms.example.test',
				deployed_at: '2026-10-02T00:00:00Z'
			})
		});
	});
	await page.route('https://lms.example.test/deep-links/return', async (route) => {
		returnedJwt = new URLSearchParams(route.request().postData() ?? '').get('JWT');
		await route.fulfill({ status: 200, contentType: 'text/html', body: '<p>LMS closed without adding a link</p>' });
	});
	await page.goto('/lti/deep-links');
	await page.getByRole('button', { name: 'Return to LMS without adding content' }).click();
	await expect(page.getByText('LMS closed without adding a link')).toBeVisible();
	expect(submittedItems).toEqual([]);
	expect(returnedJwt).toBe('signed-empty-response');
});

test('picker enforces a platform request for a single selected item', async ({ page }) => {
	await page.addInitScript(({ sessionToken }) => {
		localStorage.setItem('mlos_token', sessionToken);
		sessionStorage.setItem(
			'mlos_lti_launch',
			JSON.stringify({
				message_type: 'LtiDeepLinkingRequest',
				deep_linking_settings: {
					accept_types: ['ltiResourceLink'],
					accept_presentation_document_targets: ['window'],
					accept_multiple: false,
					deep_link_return_url: 'https://lms.example.test/deep-links/return'
				}
			})
		);
	}, { sessionToken: token });
	await page.route('**/v1/library/search**', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				results: [
					{
						content_type: 'editorial_article',
						article_id: 'article-1',
						slug: 'ecg-fundamentals',
						title: 'ECG fundamentals',
						version: 2,
						jurisdiction: null,
						effective_from: null,
						effective_to: null,
						as_of: '2026-10-02',
						score: 10,
						source_ref: 'Reviewed article',
						excerpt: 'A practical guide to ECG interpretation.'
					},
					{
						content_type: 'editorial_article',
						article_id: 'article-2',
						slug: 'acute-chest-pain',
						title: 'Acute chest pain',
						version: 1,
						jurisdiction: null,
						effective_from: null,
						effective_to: null,
						as_of: '2026-10-02',
						score: 8,
						source_ref: 'Reviewed article',
						excerpt: 'An approach to acute chest pain.'
					}
				],
				private_documents: []
			})
		})
	);
	await page.goto('/lti/deep-links');
	await page.getByRole('searchbox', { name: 'Search published articles' }).fill('clinical');
	await page.getByRole('button', { name: 'Search' }).click();
	await page.getByRole('checkbox', { name: 'ECG fundamentals' }).check();
	await expect(page.getByRole('checkbox', { name: 'Acute chest pain' })).toBeDisabled();
});
