<script lang="ts">
	// Concept identities: create, version, and map concepts onto curriculum nodes of the page-owned
	// exam. No requirement ID cited inline; e2e: core10-concepts.spec.ts.
	import { onMount } from 'svelte';
	import { Api, ApiError, type AdminConcept, type AdminCurriculumNode } from '$lib/api';

	let { examId, refreshAudit }: { examId: string; refreshAudit: () => Promise<void> } = $props();

	let nodes = $state<AdminCurriculumNode[]>([]);
	let concepts = $state<AdminConcept[]>([]);
	let conceptBusy = $state(false);
	let hierarchyBusy = $state(false);
	let conceptMessage = $state('');
	let conceptError = $state('');
	let conceptKey = $state('');
	let conceptName = $state('');
	let conceptDefinition = $state('');
	let versionTarget = $state('');
	let versionName = $state('');
	let versionDefinition = $state('');
	let selectedNodeId = $state('');
	let mappedConceptIds = $state<string[]>([]);
	let mappingBusy = $state(false);

	async function loadConcepts() {
		try {
			concepts = (await Api.adminConcepts()).concepts;
			if (!versionTarget || !concepts.some((concept) => concept.concept_id === versionTarget)) {
				versionTarget = concepts[0]?.concept_id ?? '';
			}
			loadVersionDraft();
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Concept identities could not be loaded.';
		}
	}

	function loadVersionDraft() {
		const concept = concepts.find((item) => item.concept_id === versionTarget);
		versionName = concept?.display_name ?? '';
		versionDefinition = concept?.definition ?? '';
	}

	async function loadHierarchy() {
		if (!examId.trim() || hierarchyBusy) return;
		hierarchyBusy = true;
		conceptError = '';
		conceptMessage = '';
		try {
			nodes = (await Api.adminHierarchy(examId.trim())).nodes;
			selectedNodeId = '';
			mappedConceptIds = [];
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Curriculum nodes could not be loaded.';
		} finally {
			hierarchyBusy = false;
		}
	}

	async function loadNodeConcepts() {
		if (!selectedNodeId) {
			mappedConceptIds = [];
			return;
		}
		mappingBusy = true;
		conceptError = '';
		try {
			mappedConceptIds = (await Api.adminNodeConcepts(selectedNodeId)).concepts.map(
				(concept) => concept.concept_id
			);
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Node mappings could not be loaded.';
		} finally {
			mappingBusy = false;
		}
	}

	async function createConcept(event: Event) {
		event.preventDefault();
		if (conceptBusy) return;
		conceptBusy = true;
		conceptError = '';
		conceptMessage = '';
		try {
			const created = await Api.createConcept({
				canonical_key: conceptKey,
				display_name: conceptName,
				definition: conceptDefinition
			});
			await loadConcepts();
			versionTarget = created.concept_id;
			loadVersionDraft();
			conceptKey = '';
			conceptName = '';
			conceptDefinition = '';
			conceptMessage = `Created concept identity at version ${created.current_version}.`;
			await refreshAudit();
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Concept identity could not be created.';
		} finally {
			conceptBusy = false;
		}
	}

	async function createConceptVersion(event: Event) {
		event.preventDefault();
		if (!versionTarget || conceptBusy) return;
		conceptBusy = true;
		conceptError = '';
		conceptMessage = '';
		try {
			const version = await Api.createConceptVersion(versionTarget, {
				display_name: versionName,
				definition: versionDefinition
			});
			await loadConcepts();
			conceptMessage = `Version ${version.current_version} saved.`;
			await refreshAudit();
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Concept version could not be saved.';
		} finally {
			conceptBusy = false;
		}
	}

	function toggleMappedConcept(conceptId: string, checked: boolean) {
		mappedConceptIds = checked
			? [...new Set([...mappedConceptIds, conceptId])]
			: mappedConceptIds.filter((id) => id !== conceptId);
	}

	async function saveNodeConcepts() {
		if (!selectedNodeId || mappingBusy) return;
		mappingBusy = true;
		conceptError = '';
		conceptMessage = '';
		try {
			await Api.setAdminNodeConcepts(selectedNodeId, mappedConceptIds);
			conceptMessage = 'Mapping saved.';
			await refreshAudit();
		} catch (err) {
			conceptError = err instanceof ApiError ? err.message : 'Node mapping could not be saved.';
		} finally {
			mappingBusy = false;
		}
	}

	onMount(loadConcepts);
</script>

<div class="card" data-testid="concept-editor">
	<h2>Concept identities</h2>
	<p class="muted">
		Concepts keep a stable identity across curriculum nodes. Each edit creates a new numbered
		version; learner note tags remain separate.
	</p>
	<button
		class="btn"
		type="button"
		disabled={hierarchyBusy || !examId.trim()}
		data-testid="concept-load-hierarchy"
		onclick={loadHierarchy}
	>
		{hierarchyBusy ? 'Loading curriculum…' : 'Load curriculum nodes'}
	</button>

	<form onsubmit={createConcept}>
		<h3>Create identity</h3>
		<label class="field" for="concept-key">
			<span>Canonical key</span>
			<input id="concept-key" bind:value={conceptKey} maxlength="100" required data-testid="concept-key" />
		</label>
		<label class="field" for="concept-name">
			<span>Display name</span>
			<input id="concept-name" bind:value={conceptName} maxlength="200" required data-testid="concept-name" />
		</label>
		<label class="field" for="concept-definition">
			<span>Definition</span>
			<textarea id="concept-definition" bind:value={conceptDefinition} maxlength="4000" rows="3" required data-testid="concept-definition"></textarea>
		</label>
		<button class="btn primary" type="submit" disabled={conceptBusy} data-testid="concept-create">
			{conceptBusy ? 'Saving…' : 'Create concept'}
		</button>
	</form>

	{#if nodes.length > 0}
		<h3>Map concepts to a curriculum node</h3>
		<label class="field" for="concept-map-node">
			<span>Curriculum node</span>
			<select
				id="concept-map-node"
				bind:value={selectedNodeId}
				onchange={loadNodeConcepts}
				data-testid="concept-map-node"
			>
				<option value="">Choose a node</option>
				{#each nodes as node (node.id)}
					<option value={node.id}>{node.kind}: {node.name}</option>
				{/each}
			</select>
		</label>
		{#if selectedNodeId}
			{#if concepts.length === 0}
				<p class="muted">Create a concept identity before mapping this node.</p>
			{:else}
			<fieldset class="plain-fieldset" disabled={mappingBusy}>
				<legend>Mapped concepts</legend>
				{#each concepts as concept (concept.concept_id)}
					<label class="field" for={`concept-map-${concept.concept_id}`}>
						<span>{concept.display_name} · {concept.canonical_key} · v{concept.current_version}</span>
						<input
							id={`concept-map-${concept.concept_id}`}
							type="checkbox"
							checked={mappedConceptIds.includes(concept.concept_id)}
							onchange={(event) => toggleMappedConcept(concept.concept_id, event.currentTarget.checked)}
							data-testid={`concept-map-${concept.concept_id}`}
						/>
					</label>
				{/each}
			</fieldset>
			<button
				class="btn"
				type="button"
				disabled={mappingBusy}
				data-testid="concept-map-save"
				onclick={saveNodeConcepts}
			>
				{mappingBusy ? 'Saving…' : 'Save mapping'}
			</button>
		{/if}
		{/if}
	{/if}

	<h3>Add a version</h3>
	<label class="field" for="concept-version-target">
		<span>Concept identity</span>
		<select
			id="concept-version-target"
			bind:value={versionTarget}
			onchange={loadVersionDraft}
			disabled={concepts.length === 0}
			data-testid="concept-version-target"
		>
			{#each concepts as concept (concept.concept_id)}
				<option value={concept.concept_id}>{concept.display_name} · {concept.canonical_key} · v{concept.current_version}</option>
			{/each}
		</select>
	</label>
	<form onsubmit={createConceptVersion}>
		<label class="field" for="concept-version-name">
			<span>New display name</span>
			<input id="concept-version-name" bind:value={versionName} maxlength="200" required disabled={!versionTarget} data-testid="concept-version-name" />
		</label>
		<label class="field" for="concept-version-definition">
			<span>New definition</span>
			<textarea id="concept-version-definition" bind:value={versionDefinition} maxlength="4000" rows="3" required disabled={!versionTarget} data-testid="concept-version-definition"></textarea>
		</label>
		<button class="btn" type="submit" disabled={conceptBusy || !versionTarget} data-testid="concept-version-save">
			{conceptBusy ? 'Saving…' : 'Save new version'}
		</button>
	</form>
	{#if conceptMessage}<p class="muted" role="status" data-testid="concept-message">{conceptMessage}</p>{/if}
	{#if conceptError}<p class="error-text" role="alert">{conceptError}</p>{/if}
</div>
