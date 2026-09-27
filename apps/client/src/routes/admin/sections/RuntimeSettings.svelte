<script lang="ts">
	// Runtime settings: mastery bands, free-tier caps, re-test intervals, offline lease, review caps
	// and competition points. No requirement ID cited inline; e2e: admin06-settings.spec.ts.
	import { onMount } from 'svelte';
	import { Api, ApiError, type AdminSettings } from '$lib/api';

	let { refreshAudit }: { refreshAudit: () => Promise<void> } = $props();

	let settingsDraft = $state({
		masteryLow: 1400,
		masteryHigh: 1600,
		communityMinSample: 20,
		freeDailyQuestions: 10,
		freeDailyCoachTurns: 1,
		offlineLeaseDays: 14,
		maxReviewsPerDay: 30,
		maxNewCardsPerDay: 10,
		competitionEasyPoints: 5,
		competitionMediumPoints: 10,
		competitionHardPoints: 15
	});
	let settingsIntervals = $state('1, 3, 7, 14');
	let settingsLoaded = $state(false);
	let settingsBusy = $state(false);
	let settingsMessage = $state('');
	let settingsError = $state('');

	async function loadRuntimeSettings() {
		settingsBusy = true;
		settingsError = '';
		try {
			const { settings } = await Api.adminSettings();
			settingsDraft = {
				masteryLow: settings.mastery_bands[0],
				masteryHigh: settings.mastery_bands[1],
				communityMinSample: settings.community_min_sample,
				freeDailyQuestions: settings.free_daily_questions,
				freeDailyCoachTurns: settings.free_daily_coach_turns,
				offlineLeaseDays: settings.offline_lease_days,
				maxReviewsPerDay: settings.max_reviews_per_day,
				maxNewCardsPerDay: settings.max_new_cards_per_day,
				competitionEasyPoints: settings.competition_difficulty_points[0],
				competitionMediumPoints: settings.competition_difficulty_points[1],
				competitionHardPoints: settings.competition_difficulty_points[2]
			};
			settingsIntervals = settings.retest_intervals_days.join(', ');
			settingsLoaded = true;
		} catch (err) {
			settingsError = err instanceof ApiError ? err.message : 'Runtime settings could not be loaded.';
		} finally {
			settingsBusy = false;
		}
	}

	async function saveRuntimeSettings(event: Event) {
		event.preventDefault();
		if (settingsBusy || !settingsLoaded) return;
		const intervals = settingsIntervals
			.split(',')
			.map((value) => value.trim())
			.filter(Boolean)
			.map(Number);
		if (intervals.some((value) => !Number.isSafeInteger(value))) {
			settingsError = 'Re-test intervals must be comma-separated whole numbers.';
			return;
		}
		settingsBusy = true;
		settingsError = '';
		settingsMessage = '';
		try {
			const settings: AdminSettings = {
				mastery_bands: [settingsDraft.masteryLow, settingsDraft.masteryHigh],
				community_min_sample: settingsDraft.communityMinSample,
				free_daily_questions: settingsDraft.freeDailyQuestions,
				free_daily_coach_turns: settingsDraft.freeDailyCoachTurns,
				retest_intervals_days: intervals,
				offline_lease_days: settingsDraft.offlineLeaseDays,
				max_reviews_per_day: settingsDraft.maxReviewsPerDay,
				max_new_cards_per_day: settingsDraft.maxNewCardsPerDay,
				competition_difficulty_points: [
					settingsDraft.competitionEasyPoints,
					settingsDraft.competitionMediumPoints,
					settingsDraft.competitionHardPoints
				]
			};
			await Api.updateAdminSettings(settings);
			settingsMessage = 'Runtime settings saved.';
			await refreshAudit();
		} catch (err) {
			settingsError = err instanceof ApiError ? err.message : 'Runtime settings could not be saved.';
		} finally {
			settingsBusy = false;
		}
	}

	onMount(loadRuntimeSettings);
</script>

<section class="card" aria-labelledby="runtime-settings-heading" data-testid="runtime-settings">
	<h2 id="runtime-settings-heading">Runtime settings</h2>
	<p class="muted">
		Changes apply to new requests immediately. Active offline leases keep their expiry; new and renewed
		leases use this window. Review caps affect the next queue request; due overflow remains scheduled.
		Community disclosure keeps a positive minimum sample; changes are audited. Competition points apply
		to newly created competitions; ties use accuracy, total time, then submission time.
	</p>
	{#if settingsError}<p class="error-text" role="alert">{settingsError}</p>{/if}
	{#if !settingsLoaded}
		<p class="muted" class:is-loading={settingsBusy} role="status">{settingsBusy ? 'Loading runtime settings…' : 'Runtime settings are unavailable.'}</p>
	{:else}
		<form onsubmit={saveRuntimeSettings}>
			<div class="runtime-settings-fields">
				<label class="field" for="settings-mastery-low">
					<span>Lower mastery boundary</span>
					<input id="settings-mastery-low" type="number" min="0" max="2999" step="1" bind:value={settingsDraft.masteryLow} required />
				</label>
				<label class="field" for="settings-mastery-high">
					<span>Higher mastery boundary</span>
					<input id="settings-mastery-high" type="number" min="1" max="3000" step="1" bind:value={settingsDraft.masteryHigh} required />
				</label>
				<label class="field" for="settings-community-sample">
					<span>Minimum community sample</span>
					<input id="settings-community-sample" aria-label="Minimum community sample" type="number" min="1" max="1000000" step="1" bind:value={settingsDraft.communityMinSample} required />
				</label>
				<label class="field" for="settings-free-questions">
					<span>Daily free questions</span>
					<input id="settings-free-questions" aria-label="Daily free questions" type="number" min="0" max="5000" step="1" bind:value={settingsDraft.freeDailyQuestions} required />
				</label>
				<label class="field" for="settings-free-coach-turns">
					<span>Daily free Coach turns</span>
					<input id="settings-free-coach-turns" aria-label="Daily free Coach turns" type="number" min="0" max="1000" step="1" bind:value={settingsDraft.freeDailyCoachTurns} required />
				</label>
				<label class="field" for="settings-retest-intervals">
					<span>Re-test intervals in days</span>
					<input id="settings-retest-intervals" aria-label="Re-test intervals in days" bind:value={settingsIntervals} maxlength="100" required />
				</label>
				<label class="field" for="settings-offline-lease-days">
					<span>Offline lease length in days</span>
					<input id="settings-offline-lease-days" type="number" min="1" max="30" step="1" bind:value={settingsDraft.offlineLeaseDays} required />
				</label>
				<label class="field" for="settings-due-review-cap">
					<span>Daily due-review cap</span>
					<input id="settings-due-review-cap" type="number" min="0" max="5000" step="1" bind:value={settingsDraft.maxReviewsPerDay} required />
				</label>
				<label class="field" for="settings-new-card-cap">
					<span>Daily new-card cap</span>
					<input id="settings-new-card-cap" type="number" min="0" max="1000" step="1" bind:value={settingsDraft.maxNewCardsPerDay} required />
				</label>
				<label class="field" for="settings-competition-easy-points">
					<span>Easy competition points</span>
					<input id="settings-competition-easy-points" type="number" min="1" max="1000" step="1" bind:value={settingsDraft.competitionEasyPoints} required />
				</label>
				<label class="field" for="settings-competition-medium-points">
					<span>Medium competition points</span>
					<input id="settings-competition-medium-points" type="number" min="2" max="1000" step="1" bind:value={settingsDraft.competitionMediumPoints} required />
				</label>
				<label class="field" for="settings-competition-hard-points">
					<span>Hard competition points</span>
					<input id="settings-competition-hard-points" type="number" min="3" max="1000" step="1" bind:value={settingsDraft.competitionHardPoints} required />
				</label>
			</div>
			<button class="btn primary" type="submit" disabled={settingsBusy} data-testid="settings-save">
				{settingsBusy ? 'Saving…' : 'Save runtime settings'}
			</button>
		</form>
	{/if}
	{#if settingsMessage}<p class="muted" role="status" data-testid="settings-message">{settingsMessage}</p>{/if}
</section>

<style>
	.runtime-settings-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr));
		gap: var(--space-md);
	}
</style>
