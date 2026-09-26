import { expect, test } from '@playwright/test';

const articleId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const firstVersionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const secondVersionId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';

type ArticleCitationFixture = { kind: string; anchor: string; target: string };
type ArticleFieldsFixture = {
	body: string;
	source_ref: string;
	jurisdiction: string | null;
	effective_from: string | null;
	effective_to: string | null;
	citations: ArticleCitationFixture[];
};
type ArticleVersionFixture = ArticleFieldsFixture & {
	article_id: string;
	version_id: string;
	version: number;
	status: 'draft' | 'published';
	slug: string;
	title: string;
};

test('editor creates, revises, publishes, and keeps published versions immutable', async ({ page }) => {
	const versions: ArticleVersionFixture[] = [];
	let createdPayload: ArticleVersionFixture | undefined;
	let savedPayload: ArticleFieldsFixture | undefined;

	await page.addInitScript(() => {
		localStorage.setItem('mlos_token', 'e2e-admin-user-token');
		localStorage.setItem('mlos_admin', 'e2e-admin-token');
	});
	await page.route('**/v1/admin/**', async (route) => {
		const url = new URL(route.request().url());
		const path = url.pathname.replace(/^\/api(?=\/v1\/)/, '');
		const method = route.request().method();
		const respond = (body: unknown, status = 200) =>
			route.fulfill({
				status,
				contentType: 'application/json',
				body: JSON.stringify(body)
			});

		if (path === '/v1/admin/articles' && method === 'GET') {
			const latest = Math.max(0, ...versions.map((version) => version.version));
			return respond({
				articles: [...versions].reverse().map((version) => ({
					article_id: version.article_id,
					slug: version.slug,
					title: version.title,
					version_id: version.version_id,
					version: version.version,
					status: version.status,
					jurisdiction: version.jurisdiction,
					effective_from: version.effective_from,
					effective_to: version.effective_to,
					is_latest: version.version === latest
				}))
			});
		}

		if (path === '/v1/admin/articles' && method === 'POST') {
			const payload = route.request().postDataJSON() as ArticleFieldsFixture & {
				slug: string;
				title: string;
			};
			const created: ArticleVersionFixture = {
				article_id: articleId,
				version_id: firstVersionId,
				version: 1,
				status: 'draft',
				slug: payload.slug,
				title: payload.title,
				body: payload.body,
				source_ref: payload.source_ref,
				jurisdiction: payload.jurisdiction,
				effective_from: payload.effective_from,
				effective_to: payload.effective_to,
				citations: payload.citations
			};
			createdPayload = created;
			versions.push({
				...created
			});
			return respond(versions[0]);
		}

		if (path.endsWith('/versions') && method === 'POST') {
			const latest = versions.at(-1);
			if (!latest) throw new Error('An article must exist before a revision can be created.');
			const revision: ArticleVersionFixture = {
				...latest,
				version_id: secondVersionId,
				version: 2,
				status: 'draft'
			};
			versions.push(revision);
			return respond(revision);
		}

		if (path.endsWith('/publish') && method === 'POST') {
			const versionId = path.split('/').at(-2);
			const version = versions.find((item) => item.version_id === versionId);
			if (version) version.status = 'published';
			return respond({
				article_id: articleId,
				version_id: versionId,
				version: version?.version ?? 1,
				status: 'published'
			});
		}

		if (path.includes('/versions/') && method === 'PATCH') {
			savedPayload = route.request().postDataJSON() as ArticleFieldsFixture;
			const versionId = path.split('/').at(-1);
			const version = versions.find((item) => item.version_id === versionId);
			if (version) Object.assign(version, savedPayload ?? {});
			return respond(version);
		}

		if (path.includes('/versions/') && method === 'GET') {
			const versionId = path.split('/').at(-1);
			return respond(versions.find((item) => item.version_id === versionId) ?? {});
		}

		return respond({});
	});

	await page.setViewportSize({ width: 375, height: 900 });
	await page.goto('/admin/articles');
	await expect(page.getByRole('heading', { name: 'Article workspace' })).toBeVisible();
	await page.getByRole('button', { name: 'New article' }).click();
	await page.getByLabel('Permanent slug').fill('source-linked-guideline');
	await page.getByLabel('Title').fill('Source-linked guideline');
	await page.getByTestId('article-body').fill('Recommendation: use the reviewed approach.');
	await page.getByLabel('Source reference').fill('Fixture guideline, 2026');
	await page.getByLabel('Country code (blank means global)').fill('pk');
	await page.getByLabel('Effective from (inclusive)').fill('2026-01-01');
	await page.getByLabel('Effective to (inclusive)').fill('2026-12-31');
	await page.getByTestId('article-add-citation').click();
	await page.getByLabel('Body anchor').fill('Recommendation');
	await page.getByLabel('Target').fill('page 12');
	await page.getByTestId('article-create').click();

	await expect(page.getByTestId('article-editor-message')).toContainText('Draft created');
	expect(createdPayload).toMatchObject({
		slug: 'source-linked-guideline',
		title: 'Source-linked guideline',
		jurisdiction: 'PK',
		effective_from: '2026-01-01',
		effective_to: '2026-12-31',
		citations: [{ kind: 'page', anchor: 'Recommendation', target: 'page 12' }]
	});

	await page.getByTestId('article-body').fill('Recommendation: the saved draft is version one.');
	await page.getByTestId('article-save').click();
	await expect(page.getByTestId('article-editor-message')).toContainText('Draft saved');
	expect(savedPayload?.body).toContain('saved draft is version one');

	await page.getByTestId('article-publish').click();
	await expect(page.getByText('Published · version 1')).toBeVisible();
	await expect(page.getByTestId('article-published-body')).toContainText('version one');
	await expect(page.getByTestId('article-publish')).toHaveCount(0);
	await expect(page.getByRole('button', { name: 'New revision' })).toBeVisible();

	await page.getByRole('button', { name: 'New revision' }).click();
	await expect(page.getByText('Revision created from the latest published version.')).toBeVisible();
	await expect(page.getByTestId('article-draft-form')).toBeVisible();
	await expect(page.getByTestId('article-body')).toHaveValue(
		'Recommendation: the saved draft is version one.'
	);
	await expect(page.getByTestId('article-citation')).toContainText('page 12');

	for (const width of [320, 375, 414, 768, 1280]) {
		await page.setViewportSize({ width, height: 900 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
	}
});

test('learner searches by country/date, reads global fallback as text, and gets an honest unavailable state', async ({
	page
}) => {
	const requestedScopes: Array<{ jurisdiction: string | null; asOf: string | null }> = [];
	await page.setViewportSize({ width: 375, height: 900 });
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
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
	await page.route('**/v1/library/search*', async (route) => {
		const url = new URL(route.request().url());
		requestedScopes.push({
			jurisdiction: url.searchParams.get('jurisdiction'),
			asOf: url.searchParams.get('as_of')
		});
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				results: [
					{
						content_type: 'editorial_article',
						article_id: articleId,
						slug: 'scope-guideline',
						title: 'Scope guideline',
						version: 3,
						jurisdiction: null,
						effective_from: '2026-01-01',
						effective_to: '2026-12-31',
						as_of: '2026-06-01',
						score: 10,
						source_ref: 'Fixture global guideline',
						excerpt: 'Global source-linked recommendation.'
					}
				],
				private_documents: []
			})
		});
	});
	await page.route('**/v1/library/articles/scope-guideline*', async (route) => {
		const url = new URL(route.request().url());
		const asOf = url.searchParams.get('as_of');
		if (asOf === '2027-01-01') {
			await route.fulfill({
				status: 404,
				contentType: 'application/json',
				body: JSON.stringify({
					error: {
						code: 'article_not_available_for_region',
						message: 'The article is unavailable for the selected region and date.'
					}
				})
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				slug: 'scope-guideline',
				title: 'Scope guideline',
				version: 3,
				body: '<script>window.studyContentRan = true</script>\nGlobal source-linked recommendation.',
				source_ref: 'Fixture global guideline',
				selected_jurisdiction: 'PK',
				as_of: asOf,
				jurisdiction: null,
				effective_from: '2026-01-01',
				effective_to: '2026-12-31',
				citations: [{ kind: 'page', anchor: 'recommendation', target: 'page 8' }],
				media: []
			})
		});
	});

	await page.goto('/library');
	await page.getByTestId('lib-q').fill('recommendation');
	await page.getByTestId('lib-jurisdiction').fill('pk');
	await page.getByTestId('lib-as-of').fill('2026-06-01');
	await page.getByRole('button', { name: 'Search' }).click();
	await expect(page.getByTestId('library-result-scope-guideline')).toBeVisible();
	expect(requestedScopes).toEqual([{ jurisdiction: 'PK', asOf: '2026-06-01' }]);
	await page.getByTestId('library-reader-scope-guideline').click();

	await expect(page.getByTestId('library-article')).toBeVisible();
	await expect(page.getByText('Global guidance')).toBeVisible();
	await expect(page.getByText(/Global guidance is shown/)).toBeVisible();
	await expect(page.getByTestId('article-body')).toContainText('<script>window.studyContentRan = true</script>');
	await expect(page.locator('.article-body script')).toHaveCount(0);
	await expect(page.getByTestId('article-citation')).toContainText('page 8');

	await page.getByTestId('reader-as-of').fill('2027-01-01');
	await page.getByTestId('reader-apply-scope').click();
	await expect(page.getByTestId('article-unavailable')).toContainText(
		'Content from another country was not substituted.'
	);

	for (const width of [320, 375, 414, 768, 1280]) {
		await page.setViewportSize({ width, height: 900 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
	}
});
