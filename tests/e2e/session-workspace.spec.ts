import { expect, test } from '@playwright/test';
import { createVerifiedSession } from './auth';

const API = 'http://127.0.0.1:8080';

async function createTutorSession(questionCount = 2) {
	const email = `e2e-workspace-${Date.now()}-${Math.floor(Math.random() * 1e6)}@example.test`;
	const { token } = await createVerifiedSession(
		email,
		'correct horse battery',
		`workspace-${Date.now()}`
	);
	const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' };
	const today = await (await fetch(`${API}/v1/me/today`, { headers })).json();
	const sessionRes = await fetch(`${API}/v1/practice/sessions`, {
		method: 'POST',
		headers,
		body: JSON.stringify({
			preset: 'tutor',
			chapter_id: today.tasks[0].chapter_id,
			question_count: questionCount
		})
	});
	expect(sessionRes.ok).toBeTruthy();
	const session = await sessionRes.json();
	return { token, sessionId: session.session_id };
}

test('UX-01 session workspace supports free navigation, status filtering, shortcuts, and local restore', async ({
	page
}) => {
	const { token, sessionId } = await createTutorSession();
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.goto(`/session/${sessionId}`);

	await expect(page.getByText(/Question 1 of 2/)).toBeVisible();
	await expect(page.getByTestId('submission-status')).toContainText('0 answered');
	await expect(page.getByTestId('submission-status')).toContainText('2 unanswered');

	await page.getByTestId('navigator-toggle').click();
	await expect(page.getByTestId('navigator-question-0')).toHaveAttribute('data-state', 'current');
	await expect(page.getByTestId('navigator-question-1')).toHaveAttribute('data-state', 'not-visited');
	await page.getByTestId('navigator-question-1').click();
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();

	await page.keyboard.press('f');
	await expect(page.getByTestId('question-mark')).toHaveAttribute('aria-pressed', 'true');
	await page.keyboard.press('e');
	await expect(page.getByTestId('elimination-mode')).toHaveAttribute('aria-pressed', 'true');
	await page.keyboard.press('a');
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'true');
	await expect(page.getByTestId('answer')).toBeDisabled();
	await page.keyboard.press('e');
	await page.keyboard.press('b');
	await expect(page.getByTestId('option-1')).toHaveAttribute('aria-pressed', 'true');

	await page.reload();
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'true');
	await expect(page.getByTestId('option-1')).toHaveAttribute('aria-pressed', 'true');
	await expect(page.getByTestId('question-mark')).toHaveAttribute('aria-pressed', 'true');

	await page.getByTestId('navigator-toggle').click();
	await page.getByTestId('navigator-filter-marked').click();
	await expect(page.getByTestId('navigator-question-1')).toBeVisible();
	await expect(page.getByTestId('navigator-question-0')).toHaveCount(0);
	await page.getByTestId('navigator-filter-unanswered').click();
	await expect(page.getByTestId('navigator-question-0')).toBeVisible();
	await expect(page.getByTestId('navigator-question-1')).toBeVisible();
	await page.getByTestId('navigator-filter-all').click();
	await page.getByTestId('navigator-question-0').click();

	await page.keyboard.press('ArrowRight');
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();
	await page.keyboard.press('ArrowLeft');
	await expect(page.getByText(/Question 1 of 2/)).toBeVisible();
	await page.keyboard.press('n');
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();
	await page.keyboard.press('p');
	await expect(page.getByText(/Question 1 of 2/)).toBeVisible();

	await page.getByTestId('option-0').click();
	await page.getByTestId('answer').click();
	await expect(page.getByTestId('feedback')).toBeVisible();
	await page.getByTestId('navigator-toggle').click();
	await expect(page.getByTestId('navigator-question-0')).toHaveAttribute('data-state', 'current');
	await page.getByTestId('navigator-toggle').click();
	await page.getByTestId('next').click();
	await page.getByTestId('navigator-toggle').click();
	await expect(page.getByTestId('navigator-question-0')).toHaveAttribute('data-state', 'answered');
	await expect(page.getByTestId('submission-status')).toContainText('1 answered');
	await page.getByTestId('first-unanswered').click();
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();

	await page.getByTestId('session-tools-open').click();
	await page.getByTestId('converter-value').focus();
	await page.keyboard.press('n');
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();
	await page.getByTestId('session-tools-close').click();
	await page.keyboard.press('h');
	await expect(page.getByTestId('hint')).toBeVisible();
});

test('UX-02 Focus Mode and touch gestures progressively enhance the same workspace', async ({ browser }) => {
	const context = await browser.newContext({ hasTouch: true });
	const page = await context.newPage();
	const { token, sessionId } = await createTutorSession();
	await page.addInitScript((t) => localStorage.setItem('mlos_token', t), token);
	await page.addInitScript(() => {
		let fullscreenElement: Element | null = null;
		Object.defineProperty(document, 'fullscreenElement', {
			configurable: true,
			get: () => fullscreenElement
		});
		Element.prototype.requestFullscreen = async function () {
			fullscreenElement = this;
			document.dispatchEvent(new Event('fullscreenchange'));
		};
		document.exitFullscreen = async () => {
			fullscreenElement = null;
			document.dispatchEvent(new Event('fullscreenchange'));
		};
		Object.defineProperty(navigator, 'wakeLock', {
			configurable: true,
			value: {
				request: async () => {
					const root = document.documentElement;
					root.dataset.wakeLockRequests = String(Number(root.dataset.wakeLockRequests ?? '0') + 1);
					if (root.dataset.wakeLockDeny === 'true') throw new Error('Wake Lock denied');
					return { release: async () => undefined, addEventListener: () => undefined };
				}
			}
		});
	});
	await page.goto(`/session/${sessionId}`);

	await page.getByTestId('focus-mode').click();
	await expect(page.getByTestId('focus-mode')).toHaveAttribute('aria-pressed', 'true');
	await expect(page.getByTestId('focus-mode-note')).toContainText('Do Not Disturb');
	await expect(page.locator('html')).toHaveAttribute('data-wake-lock-requests', '1');
	await page.getByTestId('focus-mode').click();
	await expect(page.getByTestId('focus-mode')).toHaveAttribute('aria-pressed', 'false');
	await page.evaluate(() => {
		document.documentElement.dataset.wakeLockDeny = 'true';
	});
	await page.getByTestId('focus-mode').click();
	await expect(page.getByTestId('focus-mode')).toHaveAttribute('aria-pressed', 'true');
	await expect(page.getByText(/wake lock could not be enabled/i)).toBeVisible();
	await expect(page.locator('html')).toHaveAttribute('data-wake-lock-requests', '2');
	await page.getByTestId('focus-mode').click();
	await expect(page.getByTestId('focus-mode')).toHaveAttribute('aria-pressed', 'false');

	await page.getByTestId('question-swipe-surface').evaluate((node) => {
		const start = new Event('touchstart', { bubbles: true });
		Object.defineProperty(start, 'changedTouches', { value: [{ clientX: 220, clientY: 120 }] });
		node.dispatchEvent(start);
		const end = new Event('touchend', { bubbles: true });
		Object.defineProperty(end, 'changedTouches', { value: [{ clientX: 80, clientY: 126 }] });
		node.dispatchEvent(end);
	});
	await expect(page.getByText(/Question 2 of 2/)).toBeVisible();

	await page.getByTestId('option-0').evaluate((node) => {
		const start = new Event('touchstart', { bubbles: true });
		Object.defineProperty(start, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(start);
		const end = new Event('touchend', { bubbles: true });
		Object.defineProperty(end, 'changedTouches', { value: [{ clientX: 180, clientY: 84 }] });
		node.dispatchEvent(end);
	});
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'true');
	await page.getByTestId('option-0').evaluate((node) => {
		const start = new Event('touchstart', { bubbles: true });
		Object.defineProperty(start, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(start);
		const end = new Event('touchend', { bubbles: true });
		Object.defineProperty(end, 'changedTouches', { value: [{ clientX: 180, clientY: 84 }] });
		node.dispatchEvent(end);
	});
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'false');

	await page.getByTestId('option-0').evaluate(async (node) => {
		const start = new Event('touchstart', { bubbles: true });
		Object.defineProperty(start, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(start);
		await new Promise((resolve) => setTimeout(resolve, 600));
		const end = new Event('touchend', { bubbles: true });
		Object.defineProperty(end, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(end);
	});
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'true');
	await page.getByTestId('option-0').evaluate(async (node) => {
		const start = new Event('touchstart', { bubbles: true });
		Object.defineProperty(start, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(start);
		await new Promise((resolve) => setTimeout(resolve, 600));
		const end = new Event('touchend', { bubbles: true });
		Object.defineProperty(end, 'changedTouches', { value: [{ clientX: 80, clientY: 80 }] });
		node.dispatchEvent(end);
	});
	await expect(page.getByTestId('option-0')).toHaveAttribute('data-eliminated', 'false');
	await expect(page.getByTestId('answer')).toBeDisabled();
	await context.close();
});
