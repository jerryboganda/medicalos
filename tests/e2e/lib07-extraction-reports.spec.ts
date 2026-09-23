import { expect, test } from '@playwright/test';

test('admin records extraction gaps and a separate reviewer verifies critical regions', async ({ page }) => {
	const reportId = 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';
	const rightsId = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
	const checksum = 'a'.repeat(64);
	let reports: Array<Record<string, unknown>> = [];
	let submitted: Record<string, unknown> | undefined;
	let reviewBody: Record<string, unknown> | undefined;

	await page.setViewportSize({ width: 375, height: 812 });
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-author-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/audit*', (route) =>
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
	await page.route('**/v1/admin/content-rights', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				rights: [{
					rights_id: rightsId,
					ref_code: 'EXTRACTION-GRANT',
					licensor: 'Study-resource owner',
					territory: 'worldwide',
					permitted_uses: ['document_extraction'],
					valid_from: '2020-01-01',
					valid_to: null,
					notes: null,
					revoked_at: null,
					revoked_by: null,
					revocation_note: null,
					contract_ref: 'contract-1',
					contract_version: 'v1',
					asset_refs: [],
					audiences: [],
					seat_limit: null,
					offline_terms: null,
					quotation_limit_words: null,
					ai_terms: null,
					derivative_terms: null,
					attribution: null,
					royalty_terms: null,
					status: 'active'
				}]
			})
		})
	);
	await page.route('**/v1/admin/library/extraction-reports/*/review', async (route) => {
		reviewBody = route.request().postDataJSON() as Record<string, unknown>;
		reports = reports.map((report) => ({
			...report,
			status: 'complete',
			review: {
				decision: reviewBody.decision,
				reviewer_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
				verified_regions: reviewBody.verified_regions,
				note: reviewBody.note,
				reviewed_at: '2026-09-24T12:00:00Z'
			}
		}));
		await route.fulfill({ status: 201, contentType: 'application/json', body: JSON.stringify(reports[0]) });
	});
	await page.route('**/v1/admin/library/extraction-reports', async (route) => {
		if (route.request().method() === 'POST') {
			submitted = route.request().postDataJSON() as Record<string, unknown>;
			reports = [{
				report_id: reportId,
				source_label: submitted.source_label,
				source_sha256: submitted.source_sha256,
				media_type: submitted.media_type,
				parser_version: submitted.parser_version,
				rights_ref: submitted.rights_ref,
				rights_available: true,
				malware_scan_status: submitted.malware_scan_status,
				expected_regions: submitted.expected_regions,
				extracted_regions: submitted.extracted_regions,
				missing_regions: [],
				uncertain_regions: submitted.uncertain_regions,
				critical_regions: submitted.critical_regions,
				status: 'review_required',
				created_by: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc',
				created_at: '2026-09-24T12:00:00Z',
				review: null
			}];
			await route.fulfill({ status: 201, contentType: 'application/json', body: JSON.stringify(reports[0]) });
			return;
		}
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ reports }) });
	});

	await page.goto('/admin');
	await page.getByLabel('Source label').fill('electrolyte-guideline.pdf');
	await page.getByLabel('Source SHA-256').fill(checksum);
	await page.getByLabel('Parser and version').fill('layout-parser/1.2.0');
	await page.getByLabel('Document-extraction rights').selectOption('EXTRACTION-GRANT');
	await page.getByLabel('Reported malware scan state').selectOption('clean');
	await page.getByLabel('Expected region references (one per line)').fill('page:1\ntable:1:units');
	await page.getByLabel('Extracted region references (one per line)').fill('page:1\ntable:1:units');
	await page.getByLabel('Critical tables or medical quantities (one reference per line)').fill('table:1:units');
	await page.getByTestId('extraction-report-create').click();
	await expect(page.getByTestId('extraction-report-message')).toContainText('review_required');
	await expect(page.getByTestId(`extraction-report-${reportId}`)).toContainText('table:1:units');
	expect(submitted).toMatchObject({
		source_label: 'electrolyte-guideline.pdf',
		source_sha256: checksum,
		rights_ref: 'EXTRACTION-GRANT',
		malware_scan_status: 'clean',
		critical_regions: ['table:1:units']
	});
	expect(submitted).not.toHaveProperty('content');

	await page.getByLabel('Verified: table:1:units').check();
	await page.getByLabel('Review note').fill('Verified the table values against the source document.');
	await page.getByTestId(`extraction-report-review-${reportId}`).click();
	await expect(page.getByTestId(`extraction-report-${reportId}`)).toContainText('complete');
	expect(reviewBody).toMatchObject({
		decision: 'approved',
		verified_regions: ['table:1:units'],
		note: 'Verified the table values against the source document.'
	});

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
