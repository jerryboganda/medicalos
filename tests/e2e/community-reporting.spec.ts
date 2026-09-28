import { expect, test, type Page } from '@playwright/test';

type Post = { post_id: string; body: string; status: string; handle: string; at: string };
type QueueReport = {
	report_id: string;
	post_id: string;
	reason: string;
	note: string | null;
	created_at: string;
	post_body: string;
	author_handle: string;
};

const groupId = 'group-1';
const stamp = '2026-09-25T12:00:00.000Z';

async function communityPage(page: Page) {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'community-e2e-token'));
	await page.route('**/v1/community/me', (route) =>
		route.fulfill({ status: 200, json: { opted_in: true, handle: 'learner-1' } })
	);
	await page.route('**/v1/me/duels', (route) => route.fulfill({ json: { duels: [] } }));
	await page.route('**/v1/community/groups', (route) =>
		route.fulfill({ json: { groups: [{ group_id: groupId, name: 'Anatomy', members: 3 }] } })
	);
	await page.route('**/v1/me/curriculum', (route) => route.fulfill({ json: { chapters: [] } }));
	await page.route('**/v1/me/share-cards', (route) =>
		route.fulfill({ json: { cards: [], unavailable: [] } })
	);
}

test('nonmembers must join before the feed exposes post and report controls', async ({ page }) => {
	await communityPage(page);
	let joined = false;
	await page.route(`**/v1/community/groups/${groupId}/join`, async (route) => {
		joined = true;
		await route.fulfill({ json: { joined: true } });
	});
	await page.route(`**/v1/community/groups/${groupId}/posts`, async (route) => {
		if (!joined) {
			await route.fulfill({
				status: 403,
				json: { error: { code: 'membership_required', message: 'join the group first' } }
			});
			return;
		}
		await route.fulfill({
			json: {
				posts: [
					{
						post_id: 'post-1',
						body: 'Member discussion',
						status: 'visible',
						handle: 'author-1',
						at: stamp
					}
				],
				is_moderator: false
			}
		});
	});
	await page.goto('/community');
	await page.getByRole('button', { name: 'Anatomy' }).click();
	await expect(page.getByRole('button', { name: 'Join this group' })).toBeVisible();
	await expect(page.getByLabel('Post to the group')).toHaveCount(0);
	await expect(page.getByRole('button', { name: 'Report', exact: true })).toHaveCount(0);

	await page.getByRole('button', { name: 'Join this group' }).click();
	await expect(page.getByLabel('Post to the group')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Report', exact: true })).toBeVisible();
});

test('members can report a post and see only their own report status', async ({ page }) => {
	await communityPage(page);
	const post: Post = {
		post_id: 'post-1',
		body: 'A group discussion',
		status: 'visible',
		handle: 'author-1',
		at: stamp
	};
	let ownReports: Record<string, unknown>[] = [];
	let sentReport: Record<string, unknown> | undefined;
	await page.route('**/v1/community/me/reports', (route) =>
		route.fulfill({ json: { reports: ownReports } })
	);
	await page.route(`**/v1/community/groups/${groupId}/posts`, async (route) => {
		if (route.request().method() === 'POST') {
			await route.fulfill({ status: 201, json: { post_id: 'new-post' } });
			return;
		}
		await route.fulfill({ json: { posts: [post], is_moderator: false } });
	});
	await page.route(`**/v1/community/groups/${groupId}/posts/post-1/reports`, async (route) => {
		sentReport = route.request().postDataJSON() as Record<string, unknown>;
		ownReports = [
			{
				report_id: 'report-1',
				group_id: groupId,
				group_name: 'Anatomy',
				post_id: post.post_id,
				reason: sentReport['reason'],
				status: 'open',
				created_at: stamp
			}
		];
		await route.fulfill({
			status: 201,
			json: { report_id: 'report-1', status: 'open', created_at: stamp }
		});
	});
	await page.goto('/community');
	await page.getByRole('button', { name: 'Anatomy' }).click();
	await expect(page.getByTestId('community-post')).toContainText(post.body);
	await expect(page.getByTestId('moderator-remove-post')).toHaveCount(0);
	await expect(page.getByTestId('moderator-report-queue')).toHaveCount(0);

	await page.getByRole('button', { name: 'Report', exact: true }).click();
	await page.getByLabel('Reason').selectOption('harassment');
	await page.getByLabel('Note (optional, up to 500 characters)').fill('Private context for moderators');
	const reportResponsePromise = page.waitForResponse(
		(response) =>
			response.url().includes(`/v1/community/groups/${groupId}/posts/post-1/reports`) &&
			response.request().method() === 'POST'
	);
	await page.getByRole('button', { name: 'Send report' }).click();
	await reportResponsePromise;

	expect(sentReport).toMatchObject({
		reason: 'harassment',
		note: 'Private context for moderators'
	});
	await expect(page.getByTestId('my-community-reports')).toContainText('open');
	await expect(page.getByTestId('community-post')).not.toContainText('Private context for moderators');
	await expect(page.getByTestId('my-community-reports')).not.toContainText('Private context for moderators');
});

test('moderators can dismiss a report or remove a post from their group queue', async ({ page }) => {
	await communityPage(page);
	let posts: Post[] = [
		{ post_id: 'post-1', body: 'Keep this post', status: 'visible', handle: 'author-1', at: stamp },
		{ post_id: 'post-2', body: 'Remove this post', status: 'visible', handle: 'author-2', at: stamp }
	];
	let queue: QueueReport[] = [
		{
			report_id: 'report-1',
			post_id: 'post-1',
			reason: 'spam',
			note: 'Review this repeated link',
			created_at: stamp,
			post_body: 'Keep this post',
			author_handle: 'author-1'
		},
		{
			report_id: 'report-2',
			post_id: 'post-2',
			reason: 'harassment',
			note: null,
			created_at: stamp,
			post_body: 'Remove this post',
			author_handle: 'author-2'
		}
	];
	await page.route('**/v1/community/me/reports', (route) =>
		route.fulfill({ json: { reports: [] } })
	);
	await page.route(`**/v1/community/groups/${groupId}/posts`, (route) =>
		route.fulfill({ json: { posts, is_moderator: true } })
	);
	await page.route(`**/v1/community/groups/${groupId}/reports`, (route) =>
		route.fulfill({ json: { reports: queue } })
	);
	await page.route(`**/v1/community/groups/${groupId}/reports/*/resolve`, async (route) => {
		const reportId = route.request().url().split('/').at(-2);
		const action = (route.request().postDataJSON() as { action: 'dismiss' | 'remove' }).action;
		if (action === 'dismiss') {
			queue = queue.filter((report) => report.report_id !== reportId);
		} else {
			const report = queue.find((item) => item.report_id === reportId);
			if (report) {
				posts = posts.map((post) =>
					post.post_id === report.post_id ? { ...post, status: 'removed' } : post
				);
				queue = queue.filter((item) => item.post_id !== report.post_id);
			}
		}
		await route.fulfill({ json: { resolved_reports: 1 } });
	});

	await page.goto('/community');
	await page.getByRole('button', { name: 'Anatomy' }).click();
	await expect(page.getByTestId('moderator-remove-post')).toHaveCount(2);
	await expect(page.getByTestId('moderator-report-queue')).toContainText('Review this repeated link');
	await expect(page.getByTestId('moderation-report')).toHaveCount(2);

	await page.getByTestId('moderation-report').nth(0).getByRole('button', { name: 'Dismiss report' }).click();
	await expect(page.getByTestId('moderation-report')).toHaveCount(1);
	await expect(page.getByTestId('community-post').nth(0)).toContainText('Keep this post');

	await page.getByTestId('moderation-report').getByRole('button', { name: 'Remove post' }).click();
	await expect(page.getByTestId('community-post').nth(1)).toContainText('(removed by a moderator)');
	await expect(page.getByTestId('moderation-report')).toHaveCount(0);
});
