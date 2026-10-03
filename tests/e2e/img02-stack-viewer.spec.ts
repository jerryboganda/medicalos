import { mockDeviceRegistration } from './mock-device-registration';
import { installE2EBrowserSession, registerE2EDevice } from './device-binding';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-learner-token', 'e2e-editor-token']);
});

test('learner navigates an ordered stack, reveals findings, and reads approved annotations', async ({ page }) => {
	const imageCases = [
		{
			case_id: 'stack-case',
			title: 'Fictional chest stack',
			kind: 'stack',
			modality: 'CT',
			images: [
				{ url: 'https://cdn.example.test/stack/one.png', rights_ref: 'IMG-STACK' },
				{ url: 'https://cdn.example.test/stack/two.png', rights_ref: 'IMG-STACK' },
				{ url: 'https://cdn.example.test/stack/three.png', rights_ref: 'IMG-STACK' }
			],
			annotations: [
				{
					annotation_id: 'approved-note',
					image_index: 1,
					x_percent: 42,
					y_percent: 57,
					body: 'Reviewer-approved teaching note.'
				}
			],
			concepts: []
		},
		{
			case_id: 'still-case',
			title: 'Fictional single image',
			kind: 'still',
			modality: 'XR',
			images: [{ url: 'https://cdn.example.test/still.png', rights_ref: 'IMG-STILL' }],
			annotations: [],
			concepts: []
		}
	];
	const findingsByCase: Record<string, { section: string; text: string }[]> = {
		'stack-case': [
			{ section: 'Impression', text: 'A fictional finding is present in this teaching case.' },
			{ section: 'Context', text: 'Synthetic teaching material.' }
		],
		'still-case': [{ section: 'Impression', text: 'Single image teaching finding.' }]
	};

	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route('**/v1/me/image-cases', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ cases: imageCases }) })
	);
	await page.route(/\/v1\/me\/image-cases\/[^/?]+$/, (route) => {
		const caseId = new URL(route.request().url()).pathname.split('/').at(-1);
		const study = imageCases.find((candidate) => candidate.case_id === caseId) ?? imageCases[0];
		return route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ ...study, findings: findingsByCase[study.case_id] })
		});
	});
	await page.goto('/imaging');

	await expect(page.getByRole('heading', { name: 'Image studies' })).toBeVisible();
	await expect(page.getByTestId('image-case-stack-case')).toBeVisible();
	await expect(page.getByTestId('image-position')).toHaveText('Image 1 of 3');
	await expect(page.getByLabel('Reveal findings')).not.toBeChecked();
	await expect(page.getByText('A fictional finding is present in this teaching case.')).toBeHidden();

	await page.getByRole('button', { name: 'Next image' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByTestId('image-position')).toHaveText('Image 2 of 3');
	await page.getByRole('button', { name: 'Previous image' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByTestId('image-position')).toHaveText('Image 1 of 3');
	await page.getByRole('button', { name: 'Next image' }).click();
	await expect(page.getByTestId('image-position')).toHaveText('Image 2 of 3');
	const sequence = page.getByRole('slider', { name: 'Image in sequence' });
	await sequence.focus();
	await page.keyboard.press('ArrowRight');
	await expect(page.getByTestId('image-position')).toHaveText('Image 3 of 3');
	await page.keyboard.press('ArrowLeft');
	await expect(page.getByTestId('image-position')).toHaveText('Image 2 of 3');
	await page.getByTestId('image-stage').hover();
	await page.mouse.wheel(0, 120);
	await expect(page.getByTestId('image-position')).toHaveText('Image 3 of 3');
	await page.mouse.wheel(0, -120);
	await expect(page.getByTestId('image-position')).toHaveText('Image 2 of 3');
	await expect(page.getByTestId('image-annotation-approved-note')).toBeVisible();
	await page.getByTestId('image-annotation-approved-note').click();
	await expect(page.getByText('Reviewer-approved teaching note.')).toBeVisible();
	await page.getByRole('button', { name: 'Zoom in' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByTestId('image-frame')).toHaveAttribute('data-zoom', '1.25');
	await page.getByLabel('Reveal findings').check();
	await expect(page.getByText('A fictional finding is present in this teaching case.')).toBeVisible();
	await expect(page.getByRole('heading', { name: 'Impression' })).toBeVisible();
	await expect(page.getByText('Synthetic teaching material.')).toBeVisible();

	const renderedImage = page.getByTestId('study-image');
	await expect(renderedImage).toHaveAttribute('src', imageCases[0].images[1].url);
	const filter = await renderedImage.evaluate((element) => getComputedStyle(element).filter);
	expect(filter).toBe('none');

	await page.getByTestId('image-case-still-case').click();
	await expect(page.getByTestId('image-position')).toHaveText('Image 1 of 1');
	await expect(page.getByRole('button', { name: 'Previous image' })).toBeDisabled();
	await expect(page.getByRole('button', { name: 'Next image' })).toBeDisabled();

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});

test('findings stay with their case when detail requests resolve out of order', async ({ page }) => {
	const imageCases = ['first-case', 'second-case'].map((case_id) => ({
		case_id,
		title: case_id,
		kind: 'still',
		modality: 'XR',
		images: [{ url: `https://cdn.example.test/${case_id}.png`, rights_ref: 'IMG-RACE' }],
		annotations: [],
		concepts: []
	}));
	const findings: Record<string, { section: string; text: string }[]> = {
		'first-case': [{ section: 'Impression', text: 'Findings for the first case.' }],
		'second-case': [{ section: 'Impression', text: 'Findings for the second case.' }]
	};
	let releaseFirst!: () => void;
	let signalFirstStarted!: () => void;
	const firstStarted = new Promise<void>((resolve) => {
		signalFirstStarted = resolve;
	});
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route('**/v1/me/image-cases', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ cases: imageCases }) })
	);
	await page.route(/\/v1\/me\/image-cases\/[^/?]+$/, async (route) => {
		const caseId = new URL(route.request().url()).pathname.split('/').at(-1) ?? '';
		const study = imageCases.find((candidate) => candidate.case_id === caseId);
		if (!study) {
			await route.fulfill({ status: 404, contentType: 'application/json', body: '{}' });
			return;
		}
		if (caseId === 'first-case') {
			await new Promise<void>((resolve) => {
				releaseFirst = resolve;
				signalFirstStarted();
			});
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				...study,
				findings: findings[caseId]
			})
		});
	});

	await page.goto('/imaging');
	await page.getByLabel('Reveal findings').check();
	await firstStarted;
	await page.getByTestId('image-case-second-case').click();
	await page.getByLabel('Reveal findings').check();
	await expect(page.getByText(findings['second-case'][0].text)).toBeVisible();
	const firstResponse = page.waitForResponse((response) => response.url().includes('/first-case'));
	releaseFirst();
	await firstResponse;
	await page.evaluate(
		() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
	);
	await expect(page.getByText(findings['second-case'][0].text)).toBeVisible();
	await expect(page.getByText(findings['first-case'][0].text)).toHaveCount(0);
});

test('findings failures remain visible after the detail request is rejected', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route('**/v1/me/image-cases', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				cases: [
					{
						case_id: 'revoked-case',
						title: 'Revoked teaching case',
						kind: 'still',
						images: [{ url: 'https://cdn.example.test/revoked.png', rights_ref: 'IMG-REVOKED' }],
						annotations: [],
						concepts: []
					}
				]
			})
		})
	);
	await page.route('**/v1/me/image-cases/revoked-case', (route) =>
		route.fulfill({
			status: 403,
			contentType: 'application/json',
			body: JSON.stringify({ error: { message: 'Display rights are no longer active.' } })
		})
	);

	await page.goto('/imaging');
	const reveal = page.getByLabel('Reveal findings');
	await reveal.check();
	await expect(reveal).toBeChecked();
	await expect(page.getByRole('alert')).toContainText('Display rights are no longer active.');
});

test('editor submits an annotation and a reviewer makes one final decision', async ({ page }) => {
	const caseId = 'annotation-case';
	const annotationId = 'pending-annotation';
	let annotations: Record<string, unknown>[] = [
		{
			annotation_id: annotationId,
			case_id: caseId,
			case_title: 'Fictional review case',
			image_index: 1,
			x_percent: 44,
			y_percent: 55,
			body: 'Existing pending note.',
			review_status: 'pending',
			created_at: '2026-09-25T00:00:00Z'
		}
	];
	let createdAnnotation: Record<string, unknown> | undefined;
	let submittedDecision: Record<string, unknown> | undefined;
	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-editor-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/me/image-cases', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				cases: [
					{
						case_id: caseId,
						title: 'Fictional review case',
						kind: 'stack',
						images: [
							{ url: 'https://cdn.example.test/review/one.png', rights_ref: 'IMG-REVIEW' },
							{ url: 'https://cdn.example.test/review/two.png', rights_ref: 'IMG-REVIEW' }
						],
						annotations: [],
						concepts: []
					}
				]
			})
		})
	);
	await page.route('**/v1/admin/image-annotations', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ annotations })
		})
	);
	await page.route('**/v1/admin/concepts', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ concepts: [] })
		})
	);
	await page.route(`**/v1/admin/image-cases/${caseId}/concepts`, (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ case_id: caseId, concepts: [] })
		})
	);
	await page.route(`**/v1/admin/image-cases/${caseId}/annotations`, async (route) => {
		createdAnnotation = route.request().postDataJSON() as Record<string, unknown>;
		annotations = [
			...annotations,
			{
				annotation_id: 'created-annotation',
				case_id: caseId,
				case_title: 'Fictional review case',
				created_at: '2026-09-25T00:00:00Z',
				...createdAnnotation,
				review_status: 'pending'
			}
		];
		await route.fulfill({
			status: 201,
			contentType: 'application/json',
			body: JSON.stringify({ annotation_id: 'created-annotation', review_status: 'pending' })
		});
	});
	await page.route(`**/v1/admin/image-annotations/${annotationId}/review`, async (route) => {
		submittedDecision = route.request().postDataJSON() as Record<string, unknown>;
		annotations = annotations.filter((annotation) => annotation.annotation_id !== annotationId);
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ decision: 'approved' }) });
	});
	await page.goto('/admin/image-annotations');
	await expect(page.getByRole('heading', { name: 'Image annotation review' })).toBeVisible();
	await page.locator('#annotation-case').selectOption(caseId);
	await page.getByLabel('Image number').fill('2');
	await page.getByLabel('Horizontal position (%)').fill('30');
	await page.getByLabel('Vertical position (%)').fill('64');
	await page.getByRole('textbox', { name: 'Annotation' }).fill('A bounded editorial note.');
	await page.getByRole('button', { name: 'Submit annotation for review' }).click();
	await expect(page.getByTestId('image-annotation-created-annotation')).toContainText('A bounded editorial note.');
	expect(createdAnnotation).toMatchObject({
		image_index: 1,
		x_percent: 30,
		y_percent: 64,
		body: 'A bounded editorial note.'
	});

	const reviewNote = page.getByLabel('Review note').first();
	const approve = page.getByRole('button', { name: 'Approve annotation' }).first();
	await expect(approve).toBeDisabled();
	await reviewNote.fill('Checked against the licensed teaching image.');
	await expect(approve).toBeEnabled();
	await approve.click();
	expect(submittedDecision).toMatchObject({ decision: 'approved', note: 'Checked against the licensed teaching image.' });
	await expect(page.getByTestId('image-annotation-pending-annotation')).toHaveCount(0);
});

test('editor maps an image case to a pinned concept version the learner can see', async ({ page }) => {
	test.setTimeout(60_000);
	const api = process.env.VITE_API_BASE ?? 'http://127.0.0.1:8080';
	const admin = process.env.ADMIN_TOKEN;
	if (!admin) throw new Error('E2E ADMIN_TOKEN is required for the real image-concept flow');
	const suffix = `${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
	const email = `e2e-img04-${suffix}@example.test`;
	const password = 'correct horse battery';
	const register = await fetch(`${api}/v1/auth/register`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password })
	});
	expect(register.ok).toBeTruthy();
	const login = await fetch(`${api}/v1/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password })
	});
	expect(login.ok).toBeTruthy();
	const { token } = await login.json();
	const deviceKey = await registerE2EDevice(api, token);
	const authHeaders = {
		authorization: `Bearer ${token}`,
		'content-type': 'application/json',
		'x-admin-token': admin
	};

	const conceptResponse = await fetch(`${api}/v1/admin/concepts`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			canonical_key: `img04-e2e-${suffix}`,
			display_name: 'Fictional thoracic landmark',
			definition: 'Version one teaching description.'
		})
	});
	expect(conceptResponse.ok).toBeTruthy();
	const { concept_id: conceptId } = await conceptResponse.json();

	const rightsRef = `IMG04-E2E-${suffix}`;
	const rightsResponse = await fetch(`${api}/v1/admin/content-rights`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			ref_code: rightsRef,
			licensor: 'Synthetic E2E fixture',
			permitted_uses: ['display'],
			valid_from: '2020-01-01'
		})
	});
	expect(rightsResponse.ok).toBeTruthy();
	const imageResponse = await fetch(`${api}/v1/admin/image-cases`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			title: `IMG-04 fixture ${suffix}`,
			kind: 'still',
			modality: 'XR',
			images: [{ url: 'https://cdn.example.test/img04.png', rights_ref: rightsRef }],
			findings: 'Synthetic educational finding.'
		})
	});
	expect(imageResponse.ok).toBeTruthy();
	const { case_id: caseId } = await imageResponse.json();

	await installE2EBrowserSession(page, token, deviceKey);
	await page.addInitScript((adminToken) => localStorage.setItem('mlos_admin', adminToken), admin);
	await page.goto('/admin/image-annotations');
	await expect(page.getByTestId('image-concept-mapping')).toBeVisible();
	await page.getByLabel('Image case').selectOption(caseId);
	const conceptChoice = page.getByTestId(`image-concept-${conceptId}`);
	await conceptChoice.check();
	await page.getByRole('button', { name: 'Save concept links' }).click();
	await expect(page.getByTestId('image-concept-mapping-message')).toContainText(/saved/i);

	const newVersion = await fetch(`${api}/v1/admin/concepts/${conceptId}/versions`, {
		method: 'POST',
		headers: authHeaders,
		body: JSON.stringify({
			display_name: 'Revised fictional thoracic landmark',
			definition: 'Version two teaching description.'
		})
	});
	expect(newVersion.ok).toBeTruthy();

	await page.goto('/imaging');
	await page.getByTestId(`image-case-${caseId}`).click();
	const linkedConcept = page.getByTestId('image-case-concepts');
	await expect(linkedConcept).toContainText('Fictional thoracic landmark');
	await expect(linkedConcept).toContainText('Version one teaching description.');
	await expect(linkedConcept).toContainText('v1');
	await expect(linkedConcept).not.toContainText('Revised fictional thoracic landmark');

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
