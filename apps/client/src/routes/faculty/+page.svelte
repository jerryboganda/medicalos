<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	// INST-02: the faculty workspace. Staff create cohorts and assignments
	// and read privacy-preserving cohort analytics (INST-07) plus the
	// institution's own audit trail (CORE-04).

	type Membership = Awaited<ReturnType<typeof Api.myInstitutions>>['memberships'][number];
	type Cohort = Awaited<ReturnType<typeof Api.institutionCohorts>>['cohorts'][number];
	type Program = Awaited<ReturnType<typeof Api.institutionPrograms>>['programs'][number];
	type CurriculumChapter = Awaited<ReturnType<typeof Api.myCurriculum>>['chapters'][number];
	type ProgramCoverage = Awaited<ReturnType<typeof Api.programCurriculumCoverage>>;
	type AnalyticsReport = Awaited<ReturnType<typeof Api.institutionAnalytics>>;

	let memberships = $state<Membership[]>([]);
	let activeId = $state('');
	let busy = $state(false);
	let message = $state('');
	let error = $state('');

	let newInstitutionName = $state('');
	let memberUserId = $state('');
	let memberRole = $state('learner');
	let cohortName = $state('');
	let cohortMemberIds = $state('');
	let cohortProgramId = $state('');
	let cohorts = $state<Cohort[]>([]);
	let programs = $state<Program[]>([]);
	let curriculum = $state<CurriculumChapter[]>([]);
	let activeProgramId = $state('');
	let newProgramName = $state('');
	let selectedChapterIds = $state<string[]>([]);
	let programCoverage = $state<ProgramCoverage | null>(null);
	let assignmentTitle = $state('');
	let assignmentCohort = $state('');
	let analyticsCohort = $state('');
	let report = $state<AnalyticsReport | null>(null);
	let audit = $state<unknown[]>([]);

	async function loadMemberships() {
		try {
			memberships = (await Api.myInstitutions()).memberships;
			if (memberships.length > 0 && !activeId) {
				activeId = memberships[0].institution_id;
				await loadWorkspace();
			}
		} catch {
			memberships = [];
		}
	}

	async function loadCohorts() {
		if (!activeId) return;
		cohorts = (await Api.institutionCohorts(activeId)).cohorts;
	}

	async function loadWorkspace() {
		if (!activeId) return;
		const institutionId = activeId;
		error = '';
		try {
			const [cohortResult, programResult] = await Promise.all([
				Api.institutionCohorts(institutionId),
				Api.institutionPrograms(institutionId)
			]);
			if (activeId !== institutionId) return;
			cohorts = cohortResult.cohorts;
			programs = programResult.programs;
			if (!programs.some((program) => program.program_id === activeProgramId)) {
				activeProgramId = programs[0]?.program_id ?? '';
			}
			selectedChapterIds =
				programs.find((program) => program.program_id === activeProgramId)?.chapter_ids ?? [];
			if (!cohortProgramId) cohortProgramId = activeProgramId;
		} catch (err) {
			if (activeId === institutionId) {
				error = err instanceof ApiError ? err.message : 'Could not load institution programs and cohorts.';
			}
		}
	}

	function changeInstitution() {
		message = '';
		report = null;
		programCoverage = null;
		audit = [];
		cohorts = [];
		programs = [];
		activeProgramId = '';
		selectedChapterIds = [];
		assignmentCohort = '';
		cohortProgramId = '';
		void loadWorkspace();
	}

	function chooseProgram() {
		selectedChapterIds =
			programs.find((program) => program.program_id === activeProgramId)?.chapter_ids ?? [];
		programCoverage = null;
		if (!cohortProgramId) cohortProgramId = activeProgramId;
	}

	function programName(programId: string | null) {
		return programs.find((program) => program.program_id === programId)?.name ?? 'No program';
	}

	async function createProgram(e: Event) {
		e.preventDefault();
		if (!activeId || busy) return;
		busy = true;
		error = '';
		try {
			const result = await Api.createInstitutionProgram(activeId, newProgramName);
			newProgramName = '';
			await loadWorkspace();
			activeProgramId = result.program_id;
			cohortProgramId = result.program_id;
			selectedChapterIds = [];
			message = 'Program created.';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not create the program.';
		} finally {
			busy = false;
		}
	}

	async function saveProgramCurriculum(e: Event) {
		e.preventDefault();
		if (!activeId || !activeProgramId || busy) return;
		busy = true;
		error = '';
		try {
			const result = await Api.setProgramCurriculum(
				activeId,
				activeProgramId,
				selectedChapterIds
			);
			programs = programs.map((program) =>
				program.program_id === activeProgramId
					? { ...program, chapter_ids: result.chapter_ids }
					: program
			);
			selectedChapterIds = result.chapter_ids;
			programCoverage = null;
			message = 'Curriculum mapping saved.';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the curriculum mapping.';
		} finally {
			busy = false;
		}
	}

	async function loadProgramCoverage() {
		if (!activeId || !activeProgramId || busy) return;
		busy = true;
		error = '';
		try {
			programCoverage = await Api.programCurriculumCoverage(activeId, activeProgramId);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load curriculum coverage.';
			programCoverage = null;
		} finally {
			busy = false;
		}
	}

	async function createInstitution(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			const res = await Api.createInstitution(newInstitutionName);
			newInstitutionName = '';
			message = 'Institution created.';
			await loadMemberships();
			activeId = res.institution_id;
			await loadWorkspace();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Creation failed.';
		} finally {
			busy = false;
		}
	}

	async function addMember(e: Event) {
		e.preventDefault();
		if (!activeId || busy) return;
		busy = true;
		error = '';
		try {
			await Api.addInstitutionMember(activeId, memberUserId.trim(), memberRole);
			memberUserId = '';
			message = 'Member added.';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not add the member.';
		} finally {
			busy = false;
		}
	}

	async function createCohort(e: Event) {
		e.preventDefault();
		if (!activeId || busy) return;
		busy = true;
		error = '';
		try {
			const ids = cohortMemberIds
				.split(',')
				.map((v) => v.trim())
				.filter(Boolean);
			await Api.createCohort(activeId, cohortName, ids, cohortProgramId || undefined);
			await loadCohorts();
			cohortName = '';
			cohortMemberIds = '';
			analyticsCohort = '';
			message = 'Cohort created.';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not create the cohort.';
		} finally {
			busy = false;
		}
	}

	async function createAssignment(e: Event) {
		e.preventDefault();
		if (!assignmentCohort || busy) return;
		busy = true;
		error = '';
		try {
			await Api.createAssignment(assignmentCohort, assignmentTitle);
			assignmentTitle = '';
			message = 'Assignment created.';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not create the assignment.';
		} finally {
			busy = false;
		}
	}

	async function loadAnalytics() {
		if (!activeId || !analyticsCohort) return;
		busy = true;
		error = '';
		try {
			report = await Api.institutionAnalytics(activeId, analyticsCohort);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not load analytics.';
			report = null;
		} finally {
			busy = false;
		}
	}

	async function loadAudit() {
		if (!activeId) return;
		busy = true;
		try {
			audit = (await Api.institutionAudit(activeId)).events;
		} catch {
			audit = [];
		} finally {
			busy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		try {
			curriculum = (await Api.myCurriculum()).chapters;
		} catch {
			curriculum = [];
		}
		await loadMemberships();
	});
</script>

<h1>Faculty workspace</h1>

<div class="card">
	<h2>Your institutions</h2>
	{#if memberships.length === 0}
		<p class="muted">
			You are not part of an institution yet. Create one to start — you
			become its administrator.
		</p>
	{:else}
		<label class="field" for="active-inst">
			<span>Active institution</span>
			<select
				id="active-inst"
				class="faculty-select"
				bind:value={activeId}
				onchange={changeInstitution}
				disabled={busy}
			>
				{#each memberships as m (m.institution_id)}
					<option value={m.institution_id}>{m.name} ({m.role})</option>
				{/each}
			</select>
		</label>
	{/if}
	<form onsubmit={createInstitution} class="faculty-form-row">
		<label class="field" for="inst-name">
			<span>New institution</span>
			<input id="inst-name" bind:value={newInstitutionName} />
		</label>
		<button class="btn" type="submit" disabled={busy || !newInstitutionName} data-testid="institution-create">
			Create
		</button>
	</form>
	{#if error}<p class="danger-text">{error}</p>{/if}
	{#if message}<p class="muted">{message}</p>{/if}
	</div>

{#if activeId}
	<div class="card">
		<h2>Members</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Add a member by account ID with a §18.1 role: admin, instructor,
			author, reviewer, or learner.
		</p>
		<form onsubmit={addMember} style="display:flex; gap:12px; flex-wrap:wrap; align-items:end;">
			<label class="field" for="member-id">
				<span>User ID</span>
				<input id="member-id" bind:value={memberUserId} />
			</label>
			<label class="field" for="member-role">
				<span>Role</span>
				<select id="member-role" bind:value={memberRole}>
					<option>learner</option>
					<option>instructor</option>
					<option>author</option>
					<option>reviewer</option>
					<option>admin</option>
				</select>
			</label>
			<button class="btn" type="submit" disabled={busy || !memberUserId}>
				Add member
			</button>
		</form>
	</div>

	<div class="card" data-testid="program-curriculum">
		<h2>Programs and curriculum</h2>
		<p class="muted">
			Map institution programs to curriculum chapters, then review aggregate learner coverage.
		</p>
		<form onsubmit={createProgram} class="faculty-form-row">
			<label class="field" for="program-name">
				<span>New program</span>
				<input id="program-name" bind:value={newProgramName} maxlength="200" />
			</label>
			<button class="btn" type="submit" disabled={busy || !newProgramName.trim()} data-testid="program-create">
				Create program
			</button>
		</form>

		{#if programs.length === 0}
			<p class="muted">No programs yet. Create a program to map its curriculum.</p>
		{:else}
			<label class="field" for="active-program">
				<span>Program</span>
				<select
					id="active-program"
					class="faculty-select"
					bind:value={activeProgramId}
					onchange={chooseProgram}
				>
					{#each programs as program (program.program_id)}
						<option value={program.program_id}>{program.name}</option>
					{/each}
				</select>
			</label>
			<form onsubmit={saveProgramCurriculum}>
				<p class="muted">Select the chapters included in this program.</p>
				{#if curriculum.length === 0}
					<p class="muted">No curriculum chapters are available.</p>
				{:else}
					<div class="chapter-list">
						{#each curriculum as chapter (chapter.chapter_id)}
							<label class="chapter-option" data-testid="chapter-option">
								<input
									type="checkbox"
									bind:group={selectedChapterIds}
									value={chapter.chapter_id}
								/>
								<span>
									<strong>{chapter.chapter_name}</strong>
									<small>{chapter.exam} · {chapter.subject} · {chapter.system}</small>
									<small>{chapter.published_questions} published questions</small>
								</span>
							</label>
						{/each}
					</div>
				{/if}
				<button
					class="btn primary"
					type="submit"
					disabled={busy || !activeProgramId}
					data-testid="curriculum-save"
				>
					Save curriculum mapping
				</button>
			</form>
			<button
				class="btn"
				type="button"
				disabled={busy || !activeProgramId}
				onclick={loadProgramCoverage}
				data-testid="coverage-load"
			>
				Load coverage
			</button>
			{#if programCoverage}
				<section class="coverage-report" aria-live="polite" data-testid="program-coverage">
					<h3>Curriculum coverage</h3>
					<p class="muted">Active learners in program cohorts: {programCoverage.cohort_size}</p>
					{#if programCoverage.suppressed}
						<p class="muted" data-testid="coverage-suppressed">
							Counts are hidden until at least {programCoverage.minimum_group_size} active learners
							belong to cohorts in this program.
						</p>
					{:else if programCoverage.chapters.length === 0}
						<p class="muted">No chapters are mapped to this program.</p>
					{:else}
						<div class="table-wrap">
							<table>
								<thead>
									<tr>
										<th>Chapter</th>
										<th>Learners with evidence</th>
										<th>Attempts</th>
										<th>Coverage</th>
									</tr>
								</thead>
								<tbody>
									{#each programCoverage.chapters as chapter (chapter.chapter_id)}
										<tr>
											<td>{chapter.chapter}</td>
											<td>{chapter.learners_with_evidence ?? '—'}</td>
											<td>{chapter.attempts ?? '—'}</td>
											<td>{chapter.coverage_percent === null ? '—' : `${Math.round(chapter.coverage_percent)}%`}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				</section>
			{/if}
		{/if}
	</div>

	<div class="card">
		<h2>Cohorts</h2>
		<form onsubmit={createCohort}>
			<label class="field" for="cohort-name">
				<span>Name</span>
				<input id="cohort-name" bind:value={cohortName} />
			</label>
			<label class="field" for="cohort-members">
				<span>Member IDs (comma-separated)</span>
				<input id="cohort-members" bind:value={cohortMemberIds} />
			</label>
			<label class="field" for="cohort-program">
				<span>Program</span>
				<select id="cohort-program" class="faculty-select" bind:value={cohortProgramId}>
					<option value="">No program</option>
					{#each programs as program (program.program_id)}
						<option value={program.program_id}>{program.name}</option>
					{/each}
				</select>
			</label>
			<button class="btn" type="submit" disabled={busy || !cohortName} data-testid="cohort-create">
				Create cohort
			</button>
		</form>
		{#if cohorts.length > 0}
			<ul>
				{#each cohorts as c (c.cohort_id)}
					<li class="cohort-row">
						<span><strong>{c.name}</strong><small>{c.members} learners · {programName(c.program_id)}</small></span>
						<button
							class="btn"
							type="button"
							disabled={busy}
							onclick={() => {
								analyticsCohort = c.cohort_id;
								loadAnalytics();
							}}
						>
							Analytics
						</button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="muted">No cohorts yet. Create one to group learners and assign a program.</p>
		{/if}
		<h3>Assignment</h3>
		<form onsubmit={createAssignment}>
			<label class="field" for="assignment-title">
				<span>Title</span>
				<input id="assignment-title" bind:value={assignmentTitle} />
			</label>
			<label class="field" for="assignment-cohort">
				<span>Cohort</span>
				<select id="assignment-cohort" class="faculty-select" bind:value={assignmentCohort}>
					<option value="">Choose a cohort</option>
					{#each cohorts as c (c.cohort_id)}
						<option value={c.cohort_id}>{c.name}</option>
					{/each}
				</select>
			</label>
			<button
				class="btn"
				type="submit"
				disabled={busy || !assignmentTitle || !assignmentCohort}
			>
				Create assignment
			</button>
		</form>
	</div>

	{#if report}
		<div class="card" data-testid="cohort-analytics">
			<h2>Cohort analytics</h2>
			{#if report.suppressed}
				<p class="muted">
					Suppressed: {report.reason} (minimum group size {report.minimum},
					this cohort has {report.cohort_size}).
				</p>
			{:else}
				<div class="table-wrap">
					<table>
					<thead>
						<tr>
							<th>Chapter</th>
							<th>Attempts</th>
							<th>Accuracy</th>
							<th>Learners</th>
						</tr>
					</thead>
					<tbody>
						{#each report.chapters ?? [] as row (row.chapter)}
							<tr>
								<td>{row.chapter ?? '—'}</td>
								<td>{row.attempts}</td>
								<td>{Math.round(row.accuracy * 100)}%</td>
								<td>{row.learners}</td>
							</tr>
						{/each}
					</tbody>
					</table>
				</div>
			{/if}
		</div>
	{/if}

	<div class="card">
		<h2>Institution audit trail</h2>
		<button class="btn" type="button" disabled={busy} onclick={loadAudit}>
			Load audit trail
		</button>
		{#if audit.length === 0}
			<p class="muted">No institution-scoped events recorded yet.</p>
		{:else}
			<ul>
				{#each audit as e (e.at)}
					<li>{e.at}: {e.action} ({e.entity})</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}

<style>
	/* Hallmark · macrostructure: App Shell · tone: calm utilitarian · anchor hue: violet · variation: chapter checklist + coverage table */
	/* Hallmark · pre-emit critique: P5 H4 E4 S5 R5 V4 · contrast: token pairs checked · mobile: E2E widths 320/375/414/768 authored; CI pending */
	.faculty-form-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: end;
		gap: var(--space-md);
	}

	.faculty-form-row .field {
		min-width: 0;
		margin-bottom: 0;
	}

	.faculty-select {
		box-sizing: border-box;
		width: 100%;
		min-height: 44px;
		padding: 0 var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: var(--text-body) var(--font-body);
		outline: 2px solid transparent;
		outline-offset: 1px;
	}

	.faculty-select:hover:not(:disabled) {
		border-color: var(--color-accent);
	}

	.faculty-select:focus-visible {
		outline-color: var(--color-focus);
	}

	.faculty-select:active:not(:disabled) {
		border-color: var(--color-action-primary);
	}

	.faculty-select:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}

	.chapter-list {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 260px), 1fr));
		gap: var(--space-sm);
		margin: var(--space-lg) 0;
	}

	.chapter-option {
		display: flex;
		align-items: flex-start;
		gap: var(--space-md);
		min-width: 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-surface);
		cursor: pointer;
	}

	.chapter-option input {
		flex: none;
		width: auto;
		min-height: 0;
		margin-top: var(--space-xs);
		accent-color: var(--color-action-primary);
	}

	.chapter-option:active {
		background: var(--color-action-wash);
	}

	.chapter-option span,
	.cohort-row span {
		display: grid;
		gap: var(--space-xs);
		min-width: 0;
	}

	.chapter-option small,
	.cohort-row small {
		color: var(--color-text-secondary);
		font-size: var(--text-sm);
		overflow-wrap: anywhere;
	}

	.coverage-report {
		margin-top: var(--space-xl);
	}

	.table-wrap {
		max-width: 100%;
		overflow-x: auto;
	}

	.cohort-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		padding: var(--space-sm) 0;
	}

	@media (max-width: 480px) {
		.faculty-form-row {
			grid-template-columns: minmax(0, 1fr);
		}

		.cohort-row {
			align-items: flex-start;
			flex-direction: column;
		}
	}
</style>
