<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError, adminToken } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	// ADMIN-06 baseline console: hierarchy management, question creation, and
	// JSON bulk import with dry-run/rollback. Gated by the admin token —
	// role-aware accounts (§18.1) replace this gate later.
	let token = $state('');
	let unlocked = $state(false);
	let busy = $state(false);
	let message = $state('');
	let error = $state('');
	let audit = $state<unknown[]>([]);

	// hierarchy
	let examId = $state('');
	let nodeKind = $state('chapter');
	let nodeName = $state('');
	let nodes = $state<unknown[]>([]);

	// import
	let importJson = $state('');
	let importReport = $state('');
	let lastBatch = $state('');

	// editorial workflow (INST-05)
	let workflowIds = $state('');
	let workflowReport = $state<unknown[]>([]);

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

	async function createNode(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			await Api.createNode({
				exam_id: examId,
				kind: nodeKind,
				name: nodeName
			});
			message = `Node "${nodeName}" created.`;
			nodeName = '';
			await refreshAudit();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Create failed.';
		} finally {
			busy = false;
		}
	}

	async function runImport(dryRun: boolean) {
		busy = true;
		error = '';
		importReport = '';
		try {
			const rows = JSON.parse(importJson);
			const res = await Api.importQuestions({
				exam_id: examId,
				dry_run: dryRun,
				rows
			});
			lastBatch = res.batch_id;
			importReport = JSON.stringify(res, null, 2);
			await refreshAudit();
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Import failed — is the JSON valid?';
		} finally {
			busy = false;
		}
	}

	async function runWorkflow(action: 'submit' | 'approve' | 'reject' | 'publish') {
		busy = true;
		error = '';
		workflowReport = [];
		try {
			const versionIds = workflowIds
				.split(',')
				.map((v) => v.trim())
				.filter(Boolean);
			const res = await Api.assessmentWorkflow(action, versionIds);
			workflowReport = res.results;
			await refreshAudit();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Workflow call failed.';
		} finally {
			busy = false;
		}
	}

	async function rollback() {
		if (!lastBatch || busy) return;
		busy = true;
		try {
			const res = await Api.rollbackImport(lastBatch);
			importReport = JSON.stringify(res, null, 2);
			await refreshAudit();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Rollback failed.';
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
		await unlock();
	});
</script>

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
	<div class="card">
		<h2>Exam</h2>
		<label class="field" for="exam-id">
			<span>Exam ID (from the seeded pilot exam)</span>
			<input id="exam-id" bind:value={examId} data-testid="exam-id" />
		</label>
		<h3>Hierarchy</h3>
		<form onsubmit={createNode}>
			<label class="field" for="node-kind">
				<span>Kind</span>
				<select id="node-kind" bind:value={nodeKind}>
					<option>chapter</option>
					<option>system</option>
					<option>subject</option>
				</select>
			</label>
			<label class="field" for="node-name">
				<span>Name</span>
				<input id="node-name" bind:value={nodeName} data-testid="node-name" />
			</label>
			<button class="btn" type="submit" disabled={busy || !examId}>
				Create node
			</button>
		</form>
	</div>

	<div class="card">
		<h2>Bulk import (JSON rows)</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Rows is a JSON array of objects, each with: chapter_id, difficulty
			(easy/medium/hard), vignette, lead_in, options (text + rationale,
			2-10 of them), correct_index, key_learning_point (40 words max),
			and source_ref. Dry run validates and creates nothing; rollback
			refuses once learners have answered.
		</p>
		<label class="field" for="import-json">
			<span>Rows JSON</span>
			<textarea
				id="import-json"
				bind:value={importJson}
				rows="6"
				data-testid="import-json"
			></textarea>
		</label>
		<div style="display:flex; gap:12px; flex-wrap:wrap;">
			<button
				class="btn"
				type="button"
				disabled={busy || !importJson}
				data-testid="import-dry"
				onclick={() => runImport(true)}
			>
				Dry run
			</button>
			<button
				class="btn primary"
				type="button"
				disabled={busy || !importJson}
				data-testid="import-apply"
				onclick={() => runImport(false)}
			>
				Apply import
			</button>
			{#if lastBatch}
				<button
					class="btn danger-text"
					type="button"
					disabled={busy}
					data-testid="import-rollback"
					onclick={rollback}
				>
					Roll back last batch
				</button>
			{/if}
		</div>
		{#if importReport}
			<pre
				style="white-space:pre-wrap; font-size: var(--text-sm);"
				data-testid="import-report">{importReport}</pre>
		{/if}
	</div>

	<div class="card">
		<h2>Editorial workflow</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Authored and imported items are born drafts. A draft must be
			submitted, approved, and published before learners ever see it —
			and the author of an item cannot be its approver. Paste one or more
			version IDs (comma-separated).
		</p>
		<label class="field" for="workflow-ids">
			<span>Question version IDs</span>
			<textarea
				id="workflow-ids"
				bind:value={workflowIds}
				rows="2"
				data-testid="workflow-ids"
			></textarea>
		</label>
		<div style="display:flex; gap:12px; flex-wrap:wrap;">
			<button
				class="btn"
				type="button"
				disabled={busy || !workflowIds}
				data-testid="workflow-submit"
				onclick={() => runWorkflow('submit')}
			>
				Submit for review
			</button>
			<button
				class="btn"
				type="button"
				disabled={busy || !workflowIds}
				data-testid="workflow-approve"
				onclick={() => runWorkflow('approve')}
			>
				Approve
			</button>
			<button
				class="btn"
				type="button"
				disabled={busy || !workflowIds}
				onclick={() => runWorkflow('reject')}
			>
				Reject to draft
			</button>
			<button
				class="btn primary"
				type="button"
				disabled={busy || !workflowIds}
				data-testid="workflow-publish"
				onclick={() => runWorkflow('publish')}
			>
				Publish
			</button>
		</div>
		{#if workflowReport.length > 0}
			<ul style="font-size: var(--text-sm);">
				{#each workflowReport as r (r.version_id)}
					<li>
						{r.version_id}:
						{#if r.status}
							<strong>{r.status}</strong>
						{:else}
							<span class="danger-text"
								>{r.error.code} — {r.error.message}</span
							>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>

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
