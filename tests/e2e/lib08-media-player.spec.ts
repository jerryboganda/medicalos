import { expect, test } from '@playwright/test';

const videoId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const audioId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';

test('article media renders native players, timed captions, rights, and chapter seeking', async ({ page }) => {
	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route('**/v1/library/articles/media-guideline*', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				slug: 'media-guideline',
				title: 'Fictional media guideline',
				version: 1,
				body: 'Fictional article text.',
				source_ref: 'Synthetic source',
				selected_jurisdiction: null,
				as_of: '2026-09-26',
				jurisdiction: null,
				effective_from: null,
				effective_to: null,
				citations: [],
				media: [
					{
						media_id: videoId,
						url: 'https://cdn.example.test/lesson.mp4',
						kind: 'video',
						duration_seconds: 600,
						captions: [
							{ start_ms: 0, end_ms: 2000, text: 'Welcome to the lesson.' },
							{ start_ms: 2000, end_ms: 4200, text: 'Review the fictional finding.' }
						],
						chapters: [
							{ at_ms: 0, title: 'Introduction' },
							{ at_ms: 60_000, title: 'Fictional finding' }
						],
						rights_ref: 'LIC-2026-014'
					},
					{
						media_id: audioId,
						url: 'https://cdn.example.test/recap.mp3',
						kind: 'audio',
						duration_seconds: 30,
						captions: [],
						chapters: [],
						rights_ref: 'LIC-2026-015'
					}
				]
			})
		})
	);
	await page.route('https://cdn.example.test/**', (route) => route.abort());

	await page.goto('/library/articles/media-guideline?as_of=2026-09-26');
	await expect(page.getByTestId('library-article')).toBeVisible();

	const video = page.getByTestId(`article-media-player-${videoId}`);
	await expect(video).toHaveAttribute('controls', '');
	await expect(video).toHaveAttribute('preload', 'none');
	await expect(video).toHaveAttribute('src', 'https://cdn.example.test/lesson.mp4');
	const captions = page.getByTestId(`article-media-captions-${videoId}`);
	await expect(captions).toHaveAttribute('kind', 'captions');
	await expect(captions).toHaveAttribute('src', /^blob:/);
	const captionSource = await captions.getAttribute('src');
	const captionFile = await page.evaluate(async (source) => {
		const response = await fetch(source!);
		return response.text();
	}, captionSource);
	expect(captionFile).toContain('00:00:00.000 --> 00:00:02.000');
	expect(captionFile).toContain('Welcome to the lesson.');
	await expect(page.getByTestId(`article-media-transcript-${videoId}`)).toContainText(
		'Welcome to the lesson.'
	);
	await expect(page.getByTestId(`article-media-rights-${videoId}`)).toContainText('LIC-2026-014');

	await video.evaluate((element) => {
		Object.defineProperty(element, 'currentTime', {
			configurable: true,
			writable: true,
			value: 0
		});
		Object.defineProperty(element, 'readyState', {
			configurable: true,
			value: HTMLMediaElement.HAVE_METADATA
		});
	});
	await page.getByTestId(`article-media-chapter-${videoId}-60000`).click();
	await expect.poll(() => video.evaluate((element) => (element as HTMLVideoElement).currentTime)).toBe(60);

	const audio = page.getByTestId(`article-media-player-${audioId}`);
	await expect(audio).toHaveJSProperty('tagName', 'AUDIO');
	await expect(page.getByTestId(`article-media-rights-${audioId}`)).toContainText('LIC-2026-015');
	await expect(page.getByText('Media files are not saved in offline packs.')).toBeVisible();

	for (const width of [320, 375, 414, 768, 1280]) {
		await page.setViewportSize({ width, height: 900 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
	}
});
