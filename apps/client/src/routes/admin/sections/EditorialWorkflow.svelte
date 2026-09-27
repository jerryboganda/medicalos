<script lang="ts">
	// Editorial workflow (INST-05): submit, approve, reject or publish question versions by ID;
	// holds the page-owned busy lock while a call runs.
	import { Api, ApiError } from '$lib/api';

	let {
		busy = $bindable(),
		refreshAudit
	}: { busy: boolean; refreshAudit: () => Promise<void> } = $props();

	// Written but never rendered, as before the split.
	let error = $state('');
	let workflowIds = $state('');
	let workflowReport = $state<unknown[]>([]);

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
</script>

<div class="card">
	<h2>Editorial workflow</h2>
	<p class="muted small">
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
	<div class="cluster">
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
		<ul class="small">
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
	{#if error}
		<p class="error-text" role="alert">{error}</p>
	{/if}
</div>
