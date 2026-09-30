import { mockDeviceRegistration } from './mock-device-registration';

import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-owner-token', 'e2e-history-taker-token']);
});

test('team lead invites a role, teammate acts, and recipient acknowledges a handover', async ({ page }) => {
	const runId = '99999999-9999-4999-8999-999999999999';
	const handoverId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
	const inviteCode = 'b'.repeat(32);
	let actingRole: 'team_lead' | 'history_taker' = 'team_lead';
	let joined = false;
	let acknowledged = false;
	let handoverBody: Record<string, unknown> | undefined;
	const members = [
		{ member_id: 'leader-member', role: 'team_lead', joined_at: '2026-09-24T07:00:00Z' },
		{ member_id: 'history-member', role: 'history_taker', joined_at: '2026-09-24T07:10:00Z' }
	];

	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-owner-token'));
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario: 'Fictional team case',
				scenario_version: 1,
				current_state: 'start',
				transcript: [],
				timeline: [],
				available_actions: ['take_history', 'finish'],
				started_at: '2026-09-24T07:00:00Z',
				finished_at: null,
				finished: false
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/team$`), (route) => {
		const currentMember = actingRole === 'team_lead' ? 'leader-member' : 'history-member';
		return route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				current_role: actingRole,
				current_member_id: currentMember,
				members: joined ? members : members.slice(0, 1)
			})
		});
	});
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/team/invites$`), async (route) => {
		await route.fulfill({
			status: 201,
			contentType: 'application/json',
			body: JSON.stringify({ invite_code: inviteCode, role: 'history_taker', expires_at: '2026-09-25T07:00:00Z' })
		});
	});
	await page.route('**/v1/scenario-team-invites/join', async (route) => {
		joined = true;
		actingRole = 'history_taker';
		await route.fulfill({
			status: 201,
			contentType: 'application/json',
			body: JSON.stringify({ run_id: runId, member_id: 'history-member', role: 'history_taker' })
		});
	});
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/events$`), async (route) => {
		const { event } = route.request().postDataJSON() as { event: string };
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				current_state: 'start',
				finished: false,
				available_actions: ['take_history', 'finish'],
				timeline: [{ index: 0, sequence: 1, from: 'start', on: event, to: 'start', actor_role: actingRole }]
			})
		});
	});
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/handovers$`), async (route) => {
		if (route.request().method() === 'POST') {
			handoverBody = route.request().postDataJSON() as Record<string, unknown>;
			await route.fulfill({
				status: 201,
				contentType: 'application/json',
				body: JSON.stringify({ handover_id: handoverId })
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				handovers: handoverBody
					? [{
							handover_id: handoverId,
							from_role: 'history_taker',
							to_role: 'team_lead',
							situation: handoverBody.situation,
							background: handoverBody.background,
							assessment: handoverBody.assessment,
							recommendation: handoverBody.recommendation,
							created_at: '2026-09-24T07:15:00Z',
							acknowledged,
							acknowledged_at: acknowledged ? '2026-09-24T07:16:00Z' : null,
							can_ack: actingRole === 'team_lead'
						}]
					: []
			})
		});
	});
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/handovers/${handoverId}/ack$`), async (route) => {
		acknowledged = true;
		await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ handover_id: handoverId, acknowledged: true }) });
	});

	await page.goto(`/scenarios/runs/${runId}`);
	await page.getByLabel('Invite role').selectOption('history_taker');
	await page.getByTestId('scenario-team-invite').click();
	await expect(page.getByTestId('scenario-team-invite-code')).toContainText(inviteCode);

	await page.evaluate(() => localStorage.setItem('mlos_token', 'e2e-history-taker-token'));
	await page.goto(`/scenarios/team?code=${inviteCode}`);
	await page.getByTestId('scenario-team-join').click();
	await expect(page).toHaveURL(new RegExp(`/scenarios/runs/${runId}$`));
	await expect(page.getByTestId('scenario-team')).toContainText('history taker');
	await page.getByTestId('scenario-action-take_history').click();
	await expect(page.getByTestId('scenario-handovers')).toBeVisible();
	await page.getByLabel('Send to role').selectOption('leader-member');
	await page.getByLabel('Situation').fill('A fictional patient reports a new symptom pattern.');
	await page.getByLabel('Background').fill('The role-play history is now complete.');
	await page.getByLabel('Assessment').fill('The authored state remains stable.');
	await page.getByLabel('Recommendation').fill('Continue with the next authored action.');
	await page.getByTestId('scenario-handover-submit').click();
	await expect(page.getByTestId(`scenario-handover-${handoverId}`)).toContainText('A fictional patient reports a new symptom pattern.');
	expect(handoverBody).toMatchObject({ recipient_member_id: 'leader-member' });
	expect(handoverBody?.situation).toBe('A fictional patient reports a new symptom pattern.');

	actingRole = 'team_lead';
	await page.evaluate(() => localStorage.setItem('mlos_token', 'e2e-owner-token'));
	await page.goto(`/scenarios/runs/${runId}`);
	await page.getByTestId(`scenario-handover-ack-${handoverId}`).click();
	await expect(page.getByTestId(`scenario-handover-${handoverId}`)).toContainText('Handover acknowledged');

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
