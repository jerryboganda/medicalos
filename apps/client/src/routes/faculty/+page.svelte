<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	// INST-02: the faculty workspace. Staff create cohorts and assignments
	// and read privacy-preserving cohort analytics (INST-07) plus the
	// institution's own audit trail (CORE-04).

	interface Membership {
		institution_id: string;
		name: string;
		role: string;
	}

	interface Cohort {
		cohort_id: string;
		name: string;
		members: number;
	}

	interface AnalyticsReport {
		cohort_size: number;
		suppressed: boolean;
		reason?: string;
		minimum?: number;
		chapters?: {
			chapter: string | null;
			attempts: number;
			correct: number;
			accuracy: number;
			learners: number;
		}[];
	}

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
	let cohorts = $state<Cohort[]>([]);
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
				await loadCohorts();
			}
		} catch {
			memberships = [];
		}
	}

	async function loadCohorts() {
		// Cohorts are created through the institution endpoints; the
		// workspace keeps the ids it sees created this session and reads
		// analytics/audit by institution.
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
			const res = await Api.createCohort(activeId, cohortName, ids);
			cohorts = [...cohorts, { cohort_id: res.cohort_id, name: cohortName, members: ids.length }];
			cohortName = '';
			cohortMemberIds = '';
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
				bind:value={activeId}
				onchange={() => {
					cohorts = [];
					report = null;
				}}
			>
				{#each memberships as m (m.institution_id)}
					<option value={m.institution_id}>{m.name} ({m.role})</option>
				{/each}
			</select>
		</label>
	{/if}
	<form onsubmit={createInstitution} style="display:flex; gap:12px; align-items:end;">
		<label class="field" for="inst-name">
			<span>New institution</span>
			<input id="inst-name" bind:value={newInstitutionName} />
		</label>
		<button class="btn" type="submit" disabled={busy || !newInstitutionName}>
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
			<button class="btn" type="submit" disabled={busy || !cohortName}>
				Create cohort
			</button>
		</form>
		{#if cohorts.length > 0}
			<ul>
				{#each cohorts as c (c.cohort_id)}
					<li>
						<strong>{c.name}</strong>
						<button
							class="btn"
							type="button"
							disabled={busy || !assignmentTitle}
							onclick={() => (assignmentCohort = c.cohort_id)}
						>
							Target for assignment
						</button>
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
		{/if}
		<h3>Assignment</h3>
		<form onsubmit={createAssignment}>
			<label class="field" for="assignment-title">
				<span>Title</span>
				<input id="assignment-title" bind:value={assignmentTitle} />
			</label>
			<label class="field" for="assignment-cohort">
				<span>Cohort ID</span>
				<input id="assignment-cohort" bind:value={assignmentCohort} />
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
