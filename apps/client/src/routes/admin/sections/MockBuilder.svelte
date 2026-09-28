<script lang="ts">
	// Mock test builder (EX-07): form type, timing, integrity policy and chapter blueprint, plus the
	// configured list. Falls back to the page-owned exam ID when no target exam is chosen.
	import { onMount } from 'svelte';
	import { Api, ApiError, type AdminCurriculumNode, type MockTest, type MockType } from '$lib/api';

	let { examId, refreshAudit }: { examId: string; refreshAudit: () => Promise<void> } = $props();

	let mocksList = $state<MockTest[]>([]);
	let mocksBusy = $state(false);
	let mocksError = $state('');
	let mocksMessage = $state('');
	let availableExams = $state<{ exam_id: string; code: string; name: string }[]>([]);
	let mockExamId = $state('');
	let mockExamChapters = $state<AdminCurriculumNode[]>([]);
	let mockChaptersBusy = $state(false);
	let mockTitle = $state('');
	let mockType = $state<MockType>('full');
	let mockTimeLimitMinutes = $state(120);
	let mockPassMarkPercent = $state(70);
	let mockAttemptsAllowed = $state(1);
	let mockLateSyncGraceMinutes = $state(5);
	let mockIntegrityPolicy = $state<'log_only' | 'warn' | 'auto_submit'>('log_only');
	let mockAwayTimeoutSeconds = $state(30);
	let mockBlueprintEntries = $state<Array<{ chapter_id: string; count: number }>>([]);
	let blueprintChapterId = $state('');
	let blueprintCount = $state(10);

	async function loadMocks() {
		mocksBusy = true;
		mocksError = '';
		try {
			const res = await Api.listMocks();
			mocksList = res.mocks;
		} catch (err) {
			mocksError = err instanceof ApiError ? err.message : 'Failed to load mock tests.';
		} finally {
			mocksBusy = false;
		}
	}

	async function loadAvailableExams() {
		try {
			const res = await Api.listExams();
			availableExams = res.exams;
			if (!mockExamId && availableExams.length > 0) {
				mockExamId = availableExams[0].exam_id;
				await loadMockExamChapters(mockExamId);
			} else if (mockExamId) {
				await loadMockExamChapters(mockExamId);
			}
		} catch {
			/* fallback to manual exam entry if catalog fails */
		}
	}

	async function loadMockExamChapters(eId: string) {
		const targetId = eId.trim();
		if (!targetId) {
			mockExamChapters = [];
			blueprintChapterId = '';
			return;
		}
		mockChaptersBusy = true;
		try {
			const res = await Api.adminHierarchy(targetId);
			mockExamChapters = res.nodes.filter((n) => n.kind === 'chapter');
			if (mockExamChapters.length > 0 && !mockExamChapters.some((c) => c.id === blueprintChapterId)) {
				blueprintChapterId = mockExamChapters[0].id;
			}
		} catch {
			mockExamChapters = [];
		} finally {
			mockChaptersBusy = false;
		}
	}

	function addBlueprintEntry() {
		const cid = blueprintChapterId.trim();
		if (!cid || blueprintCount < 1) return;
		mockBlueprintEntries = [...mockBlueprintEntries, { chapter_id: cid, count: blueprintCount }];
		blueprintCount = 10;
	}

	function removeBlueprintEntry(index: number) {
		mockBlueprintEntries = mockBlueprintEntries.filter((_, i) => i !== index);
	}

	async function createMockTest(event: Event) {
		event.preventDefault();
		const eId = mockExamId.trim() || examId.trim();
		if (!eId) {
			mocksError = 'Please select or enter an Exam ID.';
			return;
		}
		if (!mockTitle.trim()) {
			mocksError = 'Mock title is required.';
			return;
		}
		if (mockBlueprintEntries.length === 0) {
			mocksError = 'At least one blueprint chapter entry is required.';
			return;
		}
		if (mockTimeLimitMinutes < 1 || mockTimeLimitMinutes > 480) {
			mocksError = 'Time limit must be between 1 and 480 minutes (60 to 28,800 seconds).';
			return;
		}
		if (mockPassMarkPercent < 1 || mockPassMarkPercent > 100) {
			mocksError = 'Pass mark must be between 1% and 100%.';
			return;
		}
		if (mockAttemptsAllowed < 1 || mockAttemptsAllowed > 10) {
			mocksError = 'Attempts allowed must be between 1 and 10.';
			return;
		}
		if (mockLateSyncGraceMinutes < 0 || mockLateSyncGraceMinutes > 10) {
			mocksError = 'Late sync grace must be between 0 and 10 minutes (0 to 600 seconds).';
			return;
		}
		if (mockIntegrityPolicy !== 'log_only' && (mockAwayTimeoutSeconds < 15 || mockAwayTimeoutSeconds > 3600)) {
			mocksError = 'Away timeout must be between 15 and 3600 seconds.';
			return;
		}
		if (mocksBusy) return;
		mocksBusy = true;
		mocksError = '';
		mocksMessage = '';
		try {
			await Api.createMock({
				exam_id: eId,
				title: mockTitle.trim(),
				mock_type: mockType,
				time_limit_seconds: mockTimeLimitMinutes * 60,
				pass_mark_percent: mockPassMarkPercent,
				attempts_allowed: mockAttemptsAllowed,
				late_sync_grace_seconds: mockLateSyncGraceMinutes * 60,
				integrity_policy: mockIntegrityPolicy,
				...(mockIntegrityPolicy === 'log_only' ? {} : { away_timeout_seconds: mockAwayTimeoutSeconds }),
				blueprint: mockBlueprintEntries
			});
			mocksMessage = `Mock "${mockTitle.trim()}" created successfully.`;
			mockTitle = '';
			mockBlueprintEntries = [];
			await loadMocks();
			await refreshAudit();
		} catch (err) {
			mocksError = err instanceof ApiError ? err.message : 'Failed to create mock test.';
		} finally {
			mocksBusy = false;
		}
	}

	onMount(() => {
		loadMocks();
		loadAvailableExams();
	});
</script>

<section class="card" aria-labelledby="mock-builder-heading" data-testid="mock-builder">
	<h2 id="mock-builder-heading">Mock test builder (EX-07)</h2>
	<p class="muted">
		Configure examination forms with frozen blueprints, pass marks, attempt limits, and integrity constraints.
		Blueprint draws randomize questions per chapter at start time and freeze the composition for attempts.
	</p>
	{#if mocksError}<p class="error-text" role="alert" data-testid="mock-error">{mocksError}</p>{/if}
	{#if mocksMessage}<p class="muted" role="status" data-testid="mock-message">{mocksMessage}</p>{/if}

	<form onsubmit={createMockTest}>
		<div class="runtime-settings-fields">
			<label class="field" for="mock-title">
				<span>Mock title</span>
				<input id="mock-title" bind:value={mockTitle} placeholder="e.g. Cardiorespiratory Mock A" required data-testid="mock-title" />
			</label>
			<label class="field" for="mock-exam-id">
				<span>Target exam</span>
				{#if availableExams.length > 0}
					<select
						id="mock-exam-id"
						bind:value={mockExamId}
						onchange={() => loadMockExamChapters(mockExamId)}
						data-testid="mock-exam-id"
					>
						{#each availableExams as exam}
							<option value={exam.exam_id}>{exam.name} ({exam.code})</option>
						{/each}
					</select>
				{:else}
					<input
						id="mock-exam-id"
						bind:value={mockExamId}
						onblur={() => loadMockExamChapters(mockExamId)}
						placeholder={examId || 'Target exam UUID'}
						data-testid="mock-exam-id"
					/>
				{/if}
			</label>
			<label class="field" for="mock-type">
				<span>Form type</span>
				<select id="mock-type" bind:value={mockType} data-testid="mock-type">
					<option value="full">Full examination form</option>
					<option value="mini">Mini mock</option>
					<option value="subject">Subject focus</option>
					<option value="system">System focus</option>
					<option value="chapter">Chapter assessment</option>
					<option value="grand_test">Grand test</option>
					<option value="final_assessment">Final assessment</option>
				</select>
			</label>
			<label class="field" for="mock-time-limit">
				<span>Time limit (1–480 min)</span>
				<input id="mock-time-limit" type="number" min="1" max="480" step="1" bind:value={mockTimeLimitMinutes} required data-testid="mock-time-limit" />
			</label>
			<label class="field" for="mock-pass-mark">
				<span>Pass mark (1–100%)</span>
				<input id="mock-pass-mark" type="number" min="1" max="100" step="1" bind:value={mockPassMarkPercent} required data-testid="mock-pass-mark" />
			</label>
			<label class="field" for="mock-attempts">
				<span>Attempts allowed (1–10)</span>
				<input id="mock-attempts" type="number" min="1" max="10" step="1" bind:value={mockAttemptsAllowed} required data-testid="mock-attempts" />
			</label>
			<label class="field" for="mock-grace-period">
				<span>Late sync grace (0–10 min)</span>
				<input id="mock-grace-period" type="number" min="0" max="10" step="1" bind:value={mockLateSyncGraceMinutes} required data-testid="mock-grace-period" />
			</label>
			<label class="field" for="mock-integrity">
				<span>Integrity policy</span>
				<select id="mock-integrity" bind:value={mockIntegrityPolicy} data-testid="mock-integrity">
					<option value="log_only">Log only</option>
					<option value="warn">Warn on tab away</option>
					<option value="auto_submit">Auto-submit on timeout</option>
				</select>
			</label>
			{#if mockIntegrityPolicy !== 'log_only'}
				<label class="field" for="mock-away-timeout">
					<span>Away timeout (seconds)</span>
					<input id="mock-away-timeout" type="number" min="15" max="3600" step="1" bind:value={mockAwayTimeoutSeconds} required data-testid="mock-away-timeout" />
				</label>
			{/if}
		</div>

		<fieldset class="rights-record">
			<legend>Blueprint chapters</legend>
			<p class="muted">Add chapters and the question count (1–200) to sample from each chapter.</p>
			<div class="form-row">
				<label class="field" for="blueprint-chapter">
					<span>Curriculum chapter</span>
					{#if mockExamChapters.length > 0}
						<select
							id="blueprint-chapter"
							bind:value={blueprintChapterId}
							data-testid="blueprint-chapter-select"
						>
							{#each mockExamChapters as chapter}
								<option value={chapter.id}>{chapter.name}</option>
							{/each}
						</select>
					{:else}
						<input
							id="blueprint-chapter"
							bind:value={blueprintChapterId}
							placeholder={mockChaptersBusy ? 'Loading chapters…' : 'Enter chapter UUID'}
							data-testid="blueprint-chapter-id"
						/>
					{/if}
				</label>
				<label class="field blueprint-count" for="blueprint-count">
					<span>Count (1–200)</span>
					<input id="blueprint-count" type="number" min="1" max="200" step="1" bind:value={blueprintCount} data-testid="blueprint-count" />
				</label>
				<button class="btn" type="button" onclick={addBlueprintEntry} disabled={!blueprintChapterId.trim()} data-testid="add-blueprint-entry">
					Add chapter
				</button>
			</div>

			{#if mockBlueprintEntries.length > 0}
				<ul class="bare-list" data-testid="blueprint-entries">
					{#each mockBlueprintEntries as entry, index}
						{@const ch = mockExamChapters.find((c) => c.id === entry.chapter_id)}
						<li class="row blueprint-entry" data-testid="blueprint-entry-item">
							<span><strong>{ch ? ch.name : `Chapter ${entry.chapter_id}`}</strong>: {entry.count} questions</span>
							<button class="btn" type="button" onclick={() => removeBlueprintEntry(index)}>Remove</button>
						</li>
					{/each}
				</ul>
			{/if}
		</fieldset>

		<div>
			<button class="btn primary" type="submit" disabled={mocksBusy || !mockTitle.trim() || mockBlueprintEntries.length === 0} data-testid="create-mock-btn">
				{mocksBusy ? 'Creating mock…' : 'Create mock test'}
			</button>
		</div>
	</form>

	<h3>Configured mock tests</h3>
	{#if mocksBusy && mocksList.length === 0}
		<p class="muted is-loading" role="status">Loading mock tests…</p>
	{:else if mocksList.length === 0}
		<p class="muted">No mock tests configured yet.</p>
	{:else}
		<ul class="bare-list" data-testid="mock-list">
			{#each mocksList as mock (mock.mock_id)}
				<li class="rights-record" data-testid="mock-item">
					<div class="row">
						<strong>{mock.title}</strong>
						<span class="chip" data-testid="mock-item-type">{(mock.mock_type ?? 'full').replace('_', ' ')}</span>
					</div>
					<p class="muted tight small">
						Time limit: {mock.time_limit_seconds ? Math.round(mock.time_limit_seconds / 60) + 'm' : 'Unlimited'} · Pass mark: {mock.pass_mark_percent}% · Attempts: {mock.attempts_used} / {mock.attempts_allowed} · Integrity: {mock.integrity_policy}
					</p>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.runtime-settings-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr));
		gap: var(--space-md);
	}

	.rights-record {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	/* Beats the .form-row > .field default so the count stays narrow. */
	.form-row > .blueprint-count {
		flex: 0 1 8rem;
	}

	.blueprint-entry {
		padding: var(--space-xs) 0;
		border-bottom: 1px solid var(--color-surface-elevated);
	}
</style>
