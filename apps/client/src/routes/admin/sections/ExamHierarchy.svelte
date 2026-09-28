<script lang="ts">
	// Exam ID and curriculum node creation (ADMIN-06 hierarchy management). examId and busy are
	// page-owned: concepts, import and mocks read the exam; import and workflow share the lock.
	import { Api, ApiError, type CreateNodeRequest } from '$lib/api';

	let {
		examId = $bindable(),
		busy = $bindable(),
		refreshAudit
	}: { examId: string; busy: boolean; refreshAudit: () => Promise<void> } = $props();

	// Written but never rendered, as before the split.
	let message = $state('');
	let error = $state('');
	let nodeKind = $state<CreateNodeRequest['kind']>('chapter');
	let nodeName = $state('');

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
</script>

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
	{#if error}
		<p class="error-text" role="alert">{error}</p>
	{/if}
	{#if message}
		<p class="success-text" role="status">{message}</p>
	{/if}
</div>
