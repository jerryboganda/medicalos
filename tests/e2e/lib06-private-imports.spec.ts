import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

test('learner imports and reads a Markdown document under active rights', async ({ page }) => {
	const documentId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
	const rights = [
		{
			rights_id: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
			ref_code: 'PRIVATE-IMPORT-OK',
			licensor: 'Study-resource owner',
			valid_to: null,
			search_allowed: true
		}
	];
	let documents: Array<Record<string, unknown>> = [];
	let savedImport: Record<string, unknown> | undefined;

	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/library/import-rights', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ rights })
		})
	);
	await page.route('**/v1/me/library/imports', async (route) => {
		if (route.request().method() === 'POST') {
			savedImport = route.request().postDataJSON() as Record<string, unknown>;
			documents = [
				{
					document_id: documentId,
					title: savedImport.title,
					media_type: savedImport.media_type,
					rights_ref: savedImport.rights_ref,
					sha256: 'fixture-content-hash',
					available: true
				}
			];
			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({
					document_id: documentId,
					...savedImport,
					sha256: 'fixture-content-hash',
					available: true
				})
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ documents })
		});
	});
	await page.route('**/v1/library/search*', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				results: [],
				private_documents: [
					{
						document_id: documentId,
						content_type: 'private_document',
						title: 'Cardiac notes',
						media_type: 'text/markdown',
						rights_ref: 'PRIVATE-IMPORT-OK',
						sha256: 'fixture-content-hash',
						created_at: '2026-09-24T12:00:00Z',
						available: true,
						excerpt: 'The cardiac cycle has four phases.'
					}
				]
			})
		})
	);
	await page.route(/\/v1\/me\/library\/imports\/[^/]+$/, async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					document_id: documentId,
					title: 'Cardiac notes',
					media_type: 'text/markdown',
					content: '# Cardiac notes\n\nThe cardiac cycle has four phases.',
					rights_ref: 'PRIVATE-IMPORT-OK',
					sha256: 'fixture-content-hash',
					available: true
				})
			});
			return;
		}
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ deleted: true }) });
	});
	await page.goto('/library');
	await page.getByLabel('Document title').fill('Cardiac notes');
	await page.getByLabel('Rights reference').selectOption('PRIVATE-IMPORT-OK');
	await page.getByLabel('Document file').setInputFiles({
		name: 'cardiac-notes.md',
		mimeType: 'text/markdown',
		buffer: Buffer.from('# Cardiac notes\n\nThe cardiac cycle has four phases.')
	});
	await page.getByTestId('private-import-submit').click();
	await expect(page.getByTestId('private-import-message')).toContainText('Imported');
	expect(savedImport).toMatchObject({
		title: 'Cardiac notes',
		media_type: 'text/markdown',
		rights_ref: 'PRIVATE-IMPORT-OK',
		content: '# Cardiac notes\n\nThe cardiac cycle has four phases.'
	});
	await expect(page.getByTestId(`private-import-${documentId}`)).toContainText('PRIVATE-IMPORT-OK');
	await expect(page.getByTestId(`private-import-${documentId}`)).toContainText('fixture-content-hash');
	await page.getByLabel('Search reviewed articles').fill('cardiac');
	await page.getByRole('button', { name: 'Search' }).click();
	await expect(page.getByTestId(`private-search-${documentId}`)).toContainText('Private document');
	await page.getByTestId(`private-search-open-${documentId}`).click();
	await expect(page.getByTestId('private-import-content')).toContainText(
		'The cardiac cycle has four phases.'
	);
	await page.getByTestId(`private-import-read-${documentId}`).click();
	await expect(page.getByTestId('private-import-content')).toContainText(
		'The cardiac cycle has four phases.'
	);

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});

test('Library explains when no active private-import rights are available', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/library/import-rights', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ rights: [] })
		})
	);
	await page.route('**/v1/me/library/imports', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ documents: [] })
		})
	);

	await page.goto('/library');
	await expect(page.getByTestId('private-import-no-rights')).toContainText(
		'No current rights record permits private import and display.'
	);
	await expect(page.getByText('You have no private imports yet.')).toBeVisible();
});

test('Library can retry when the private-rights service is unavailable', async ({ page }) => {
	let rightsRequests = 0;
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-user-token'));
	await page.route('**/v1/me/library/import-rights', (route) => {
		rightsRequests += 1;
		return rightsRequests === 1
			? route.fulfill({
					status: 503,
					contentType: 'application/json',
					body: JSON.stringify({ error: { code: 'temporarily_unavailable', message: 'Rights service is offline.' } })
				})
			: route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: JSON.stringify({ rights: [] })
				});
	});
	await page.route('**/v1/me/library/imports', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ documents: [] })
		})
	);

	await page.goto('/library');
	await expect(page.getByRole('alert')).toContainText('Rights service is offline.');
	await page.getByRole('button', { name: 'Retry' }).click();
	await expect(page.getByTestId('private-import-no-rights')).toBeVisible();
});

test('admin records and revokes rights used by private imports', async ({ page }) => {
	const rightsId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
	let rights: Array<Record<string, unknown>> = [];
	let created: Record<string, unknown> | undefined;
	let revocation: Record<string, unknown> | undefined;

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
	await page.route('**/v1/admin/concepts', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ concepts: [] }) })
	);
	await page.route('**/v1/admin/scenarios/runs/pending-assessment', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ runs: [] }) })
	);
	await page.route('**/v1/admin/scenario-assessment-appeals', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ appeals: [] }) })
	);
	await page.route('**/v1/admin/content-rights', async (route) => {
		if (route.request().method() === 'POST') {
			created = route.request().postDataJSON() as Record<string, unknown>;
			rights = [{
				rights_id: rightsId,
				...created,
				ref_code: 'PRIVATE-IMPORT-UI',
				revoked_at: null,
				revoked_by: null,
				revocation_note: null,
				contract_ref: created.contract_ref ?? null,
				contract_version: created.contract_version ?? null,
				asset_refs: created.asset_refs ?? [],
				audiences: created.audiences ?? [],
				seat_limit: created.seat_limit ?? null,
				offline_terms: created.offline_terms ?? null,
				quotation_limit_words: created.quotation_limit_words ?? null,
				ai_terms: created.ai_terms ?? null,
				derivative_terms: created.derivative_terms ?? null,
				attribution: created.attribution ?? null,
				royalty_terms: created.royalty_terms ?? null,
				status: 'active'
			}];
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ rights_id: rightsId, ref_code: 'PRIVATE-IMPORT-UI' })
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ rights })
		});
	});
	await page.route(/\/v1\/admin\/content-rights\/[^/]+\/revoke$/, async (route) => {
		revocation = route.request().postDataJSON() as Record<string, unknown>;
		rights = rights.map((record) => ({
			...record,
			revoked_at: '2026-09-24T12:00:00Z',
			revoked_by: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
			revocation_note: revocation.reason,
			status: 'revoked'
		}));
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ revoked: true, already_revoked: false })
		});
	});

	await page.goto('/admin');
	await page.getByLabel('Reference code').fill('PRIVATE-IMPORT-UI');
	await page.getByLabel('Licensor').fill('Study-resource owner');
	await page.getByLabel('Private learner imports').check();
	await page.getByLabel('Valid from').fill('2020-01-01');
	await page.getByText('Contract and scope terms').click();
	await page.getByLabel('Contract reference').fill('library-contract-ui');
	await page.getByLabel('Contract version').fill('2026-r1');
	await page.getByLabel('Covered asset references (one per line)').fill('cardiology:chapter-3');
	await page.getByLabel('Permitted audiences (one per line)').fill('learners');
	await page.getByLabel('Seat limit (optional)').fill('250');
	await page.getByLabel('Quotation limit in words').fill('300');
	await page.getByTestId('content-rights-create').click();
	await expect(page.getByTestId('content-rights-message')).toContainText('created');
	expect(created?.permitted_uses).toEqual(['display', 'private_import']);
	expect(created).toMatchObject({
		contract_ref: 'library-contract-ui',
		contract_version: '2026-r1',
		asset_refs: ['cardiology:chapter-3'],
		audiences: ['learners'],
		seat_limit: 250,
		quotation_limit_words: 300
	});

	const record = page.getByTestId('content-right-' + rightsId);
	await expect(record).toContainText('PRIVATE-IMPORT-UI');
	await expect(record).toContainText('library-contract-ui');
	await page.getByLabel('Reason to revoke PRIVATE-IMPORT-UI').fill('Grant ended');
	await record.getByRole('button', { name: 'Revoke rights' }).click();
	await expect(record).toContainText('Revoked');
	await expect(record).toContainText('Grant ended');
	expect(revocation).toEqual({ reason: 'Grant ended' });

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
