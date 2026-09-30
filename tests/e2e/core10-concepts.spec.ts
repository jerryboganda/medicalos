import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

test('editor creates a versioned concept and maps it to a curriculum node on mobile', async ({
	page
}) => {
	const examId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
	const chapterId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
	const conceptId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
	let concepts: Array<Record<string, unknown>> = [];
	let mappedConceptIds: string[] = [];
	let savedMapping: string[] = [];

	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/audit', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ events: [] }) })
	);
	await page.route('**/v1/admin/reports*', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ reports: [] }) })
	);
	await page.route('**/v1/admin/hierarchy?exam_id=*', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				nodes: [{ id: chapterId, kind: 'chapter', name: 'Cardiac physiology' }]
			})
		})
	);
	await page.route('**/v1/admin/concepts**', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ concepts })
			});
			return;
		}
		const body = route.request().postDataJSON() as Record<string, string>;
		if (route.request().url().endsWith('/versions')) {
			concepts = concepts.map((concept) =>
				concept.concept_id === conceptId
					? { ...concept, current_version: 2, display_name: body.display_name, definition: body.definition }
					: concept
			);
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ concept_id: conceptId, current_version: 2 })
			});
			return;
		}
		concepts = [
			{
				concept_id: conceptId,
				canonical_key: body.canonical_key,
				display_name: body.display_name,
				definition: body.definition,
				current_version: 1
			}
		];
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ concept_id: conceptId, current_version: 1 })
		});
	});
	await page.route(/\/v1\/admin\/hierarchy\/[^/]+\/concepts$/, async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					concepts: concepts.filter((concept) => mappedConceptIds.includes(String(concept.concept_id)))
				})
			});
			return;
		}
		savedMapping = (route.request().postDataJSON() as { concept_ids: string[] }).concept_ids;
		mappedConceptIds = savedMapping;
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ mapped: savedMapping.length })
		});
	});

	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.goto('/admin');
	await page.getByTestId('exam-id').fill(examId);
	await page.getByTestId('concept-load-hierarchy').click();
	await page.getByTestId('concept-key').fill('cardiac-output');
	await page.getByTestId('concept-name').fill('Cardiac output');
	await page.getByTestId('concept-definition').fill('Blood pumped per unit time.');
	await page.getByTestId('concept-create').click();
	await expect(page.getByTestId('concept-message')).toContainText('Created');

	await page.getByTestId('concept-map-node').selectOption(chapterId);
	await page.getByTestId(`concept-map-${conceptId}`).check();
	await page.getByTestId('concept-map-save').click();
	await expect(page.getByTestId('concept-message')).toContainText('Mapping saved');
	expect(savedMapping).toEqual([conceptId]);

	await page.getByTestId('concept-version-target').selectOption(conceptId);
	await page.getByTestId('concept-version-name').fill('Cardiac output');
	await page.getByTestId('concept-version-definition').fill('Blood pumped by a ventricle per unit time.');
	await page.getByTestId('concept-version-save').click();
	await expect(page.getByTestId('concept-message')).toContainText('Version 2');
	await expect(
				page.getByText('Cardiac output · cardiac-output · v2').first()
			).toBeVisible();

	const noHorizontalOverflow = await page.evaluate(
		() => document.documentElement.scrollWidth <= window.innerWidth
	);
	expect(noHorizontalOverflow).toBe(true);
});
