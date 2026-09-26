import { expect, test } from '@playwright/test';

test('learner follows an authored station and explores a read-only alternate timeline', async ({ page }) => {
	const runId = '22222222-2222-4222-8222-222222222222';
	let events: string[] = [];
	let replayBody: Record<string, unknown> | undefined;
	const transcriptCorrection = {
		event_index: 0,
		original_event: 'ask_symptom_onset',
		corrected_text: 'Asked about symptom onset more clearly.',
		corrected_by: '00000000-0000-4000-8000-000000000001',
		created_at: '2026-09-23T10:01:00Z'
	};
	let includeTranscriptCorrection = true;

	await page.addInitScript(() => localStorage.setItem('mlos_token', 'e2e-learner-token'));
	await page.route('**/v1/scenarios', async (route) => {
		if (route.request().method() === 'POST') {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					run_id: runId,
					scenario: 'Fictional symptom history',
					scenario_slug: 'fictional-symptom-history',
					scenario_version: 3,
					current_state: 'start',
					available_actions: ['ask_symptom_onset'],
					finished: false
				})
			});
			return;
		}
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				scenarios: [{ slug: 'fictional-symptom-history', title: 'Fictional symptom history', version: 3 }]
			})
		});
	});
	await page.route('**/v1/scenarios/runs', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario: 'Fictional symptom history',
				scenario_slug: 'fictional-symptom-history',
				scenario_version: 3,
				current_state: 'start',
				available_actions: ['ask_symptom_onset'],
				finished: false
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario: 'Fictional symptom history',
				scenario_version: 3,
				current_state: events.length === 2 ? 'complete' : 'start',
				transcript: events.map((on) => ({ from: 'start', on, to: on === 'finish' ? 'complete' : 'start' })),
				timeline: events.map((on, index) => ({ index, sequence: index + 1, from: 'start', on, to: on === 'finish' ? 'complete' : 'start' })),
				available_actions: events.length === 0 ? ['ask_symptom_onset'] : events.length === 1 ? ['finish'] : [],
				started_at: '2026-09-23T09:50:00Z',
				finished_at: events.length === 2 ? '2026-09-23T10:00:00Z' : null,
				finished: events.length === 2
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/team$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				current_role: 'team_lead',
				current_member_id: 'leader-member',
				members: [{ member_id: 'leader-member', role: 'team_lead', joined_at: '2026-09-23T09:50:00Z' }]
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/handovers$`), (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ handovers: [] }) })
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/events$`), async (route) => {
		const body = route.request().postDataJSON() as { event: string };
		events = [...events, body.event];
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ run_id: runId, current_state: body.event === 'finish' ? 'complete' : 'start', finished: body.event === 'finish', available_actions: body.event === 'finish' ? [] : ['finish'], timeline: [] })
		});
	});
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/debrief$`), (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				scenario: 'Fictional symptom history',
				scenario_version: 3,
				final_state: 'complete',
				transcript: events.map((on) => ({ from: 'start', on, to: on === 'finish' ? 'complete' : 'start' })),
				timeline: events.map((on, index) => ({ index, sequence: index + 1, from: 'start', on, to: on === 'finish' ? 'complete' : 'start' })),
				available_actions: ['ask_symptom_onset', 'finish'],
				started_at: '2026-09-23T09:50:00Z',
				finished_at: '2026-09-23T10:00:00Z',
				rubric: [],
				transcript_corrections: includeTranscriptCorrection ? [transcriptCorrection] : [],
				consequential_use_status: 'not_authorized_by_assessment',
				appeal: null
			})
		})
	);
	await page.route(new RegExp(`/v1/scenarios/runs/${runId}/counterfactual$`), async (route) => {
		replayBody = route.request().postDataJSON() as Record<string, unknown>;
		const path = (replayBody.events as string[]).map((on, index) => ({
			index,
			sequence: index + 1,
			from: 'start',
			on,
			to: on === 'finish' ? 'complete' : 'start',
			terminal: on === 'finish'
		}));
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				run_id: runId,
				scenario_version: 3,
				original_timeline: events.map((on, index) => ({ index, sequence: index + 1, from: 'start', on, to: on === 'finish' ? 'complete' : 'start' })),
				counterfactual_timeline: path,
				final_state: path[path.length - 1]?.to ?? 'start',
				terminal: path[path.length - 1]?.terminal ?? false
			})
		});
	});

	await page.goto('/scenarios');
	await page.getByTestId('scenario-start-fictional-symptom-history').click();
	await expect(page).toHaveURL(new RegExp(`/scenarios/runs/${runId}$`));
	await page.getByTestId('scenario-action-ask_symptom_onset').click();
	await page.getByTestId('scenario-action-finish').click();
	await expect(page.getByTestId('scenario-timeline')).toContainText('Event 1: ask symptom onset');
	await expect(page.getByTestId('scenario-transcript-corrections')).toContainText('Event 1');
	await expect(page.getByTestId('scenario-transcript-corrections')).toContainText('ask symptom onset');
	await expect(page.getByTestId('scenario-transcript-corrections')).toContainText('Asked about symptom onset more clearly.');
	await page.getByLabel('Alternate actions, one per line').fill('finish');
	await page.getByTestId('counterfactual-submit').click();
	await expect(page.getByTestId('counterfactual-result')).toContainText('Event 1: finish');
	await expect(page.getByTestId('scenario-timeline')).toContainText('Event 1: ask symptom onset');
	expect(replayBody?.events).toEqual(['finish']);

	includeTranscriptCorrection = false;
	await page.reload();
	await expect(page.getByTestId('scenario-transcript-corrections')).toHaveCount(0);
	await expect(page.getByTestId('scenario-timeline')).toContainText('Event 1: ask symptom onset');

	for (const width of [320, 375, 414, 768]) {
		await page.setViewportSize({ width, height: 812 });
		const noHorizontalOverflow = await page.evaluate(
			() => document.documentElement.scrollWidth <= window.innerWidth
		);
		expect(noHorizontalOverflow, `${width}px viewport`).toBe(true);
	}
});
