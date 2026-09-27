<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E4 S4 R4 V4 */
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, adminToken, type AdminContentRight } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';
	import RuntimeSettings from './sections/RuntimeSettings.svelte';
	import ExamHierarchy from './sections/ExamHierarchy.svelte';
	import ScenarioAssessments from './sections/ScenarioAssessments.svelte';
	import AssessmentAppeals from './sections/AssessmentAppeals.svelte';
	import ContentRights from './sections/ContentRights.svelte';
	import ExtractionReports from './sections/ExtractionReports.svelte';
	import ConceptIdentities from './sections/ConceptIdentities.svelte';
	import InstitutionSso from './sections/InstitutionSso.svelte';
	import BulkImport from './sections/BulkImport.svelte';
	import EditorialWorkflow from './sections/EditorialWorkflow.svelte';
	import QuestionReports from './sections/QuestionReports.svelte';
	import MockBuilder from './sections/MockBuilder.svelte';

	// ADMIN-06 baseline console: hierarchy management, question creation, and
	// CSV/XLSX plus JSON bulk import with dry-run/rollback. Gated by the admin token —
	// role-aware accounts (§18.1) replace this gate later.
	// Each section in ./sections owns its state and loads its own data when it mounts, which is
	// on unlock. The page keeps the unlock gate, the audit trail and what several sections share.
	let token = $state('');
	let unlocked = $state(false);
	// One editorial call at a time across the hierarchy, import and workflow sections.
	let busy = $state(false);
	let audit = $state<unknown[]>([]);
	// Read by hierarchy, concepts, import and mocks.
	let examId = $state('');
	// Loaded by the rights ledger; extraction QA picks its grant from it.
	let contentRights = $state<AdminContentRight[]>([]);

	async function unlock() {
		unlocked = adminToken().length > 0;
		if (unlocked) await refreshAudit();
	}

	async function refreshAudit() {
		try {
			audit = (await Api.listAdminAudit()).events;
		} catch {
			audit = [];
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await unlock();
	});
</script>

<svelte:head>
	<title>Editorial console | Medical Learning OS</title>
</svelte:head>

<h1>Editorial console</h1>

{#if !unlocked}
	<div class="card">
		<label class="field" for="admin-token">
			<span>Admin token</span>
			<input
				id="admin-token"
				type="password"
				bind:value={token}
				data-testid="admin-token"
			/>
		</label>
		<button
			class="btn primary"
			type="button"
			onclick={() => {
				localStorage.setItem('mlos_admin', token);
				unlock();
			}}
		>
			Unlock console
		</button>
	</div>
{:else}
	<nav class="admin-shortcuts" aria-label="Admin tools">
		<a class="btn" href={base + '/admin/articles'}>Article workspace</a>
		<a class="btn primary" href={`${base}/admin/dashboard`}>Owner dashboard</a>
		<a class="btn" href={`${base}/admin/image-annotations`}>Image annotation review</a>
	</nav>
	<RuntimeSettings {refreshAudit} />
	<ExamHierarchy bind:examId bind:busy {refreshAudit} />
	<ScenarioAssessments {refreshAudit} />
	<AssessmentAppeals {refreshAudit} />
	<ContentRights bind:contentRights {refreshAudit} />
	<ExtractionReports {contentRights} {refreshAudit} />
	<ConceptIdentities {examId} {refreshAudit} />
	<InstitutionSso {refreshAudit} />
	<BulkImport {examId} bind:busy {refreshAudit} />
	<EditorialWorkflow bind:busy {refreshAudit} />
	<QuestionReports {refreshAudit} />
	<MockBuilder {examId} {refreshAudit} />

	<div class="card">
		<h2>Audit trail (last 50)</h2>
		{#if audit.length === 0}
			<p class="muted">No editorial actions recorded yet.</p>
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
	/* Hallmark · macrostructure: App Shell (existing Editorial Console) · tone: utilitarian · anchor hue: violet */
	.admin-shortcuts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
		margin-bottom: var(--space-lg);
	}
</style>
