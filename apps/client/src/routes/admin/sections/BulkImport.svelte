<script lang="ts">
	// Bulk question import (ADMIN-06): CSV/XLSX file and JSON rows with dry run and rollback into
	// the page-owned exam; holds the page-owned busy lock while a call runs.
	import { Api, ApiError } from '$lib/api';

	let {
		examId,
		busy = $bindable(),
		refreshAudit
	}: { examId: string; busy: boolean; refreshAudit: () => Promise<void> } = $props();

	// Written but never rendered, as before the split.
	let error = $state('');
	let importJson = $state('');
	let importFile = $state<File | null>(null);
	let fileImportBusy = $state<'preview' | 'apply' | null>(null);
	let importReport = $state('');
	let lastBatch = $state('');

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
			lastBatch = res.status === 'applied' ? res.batch_id : '';
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

	function downloadQuestionTemplate() {
		const optionColumns = Array.from({ length: 10 }, (_, index) => [
			`option_${index + 1}`,
			`rationale_${index + 1}`
		]).flat();
		const headers = [
			'chapter_id',
			'difficulty',
			'vignette',
			'lead_in',
			...optionColumns,
			'correct_option',
			'key_learning_point',
			'source_ref',
			'rights_ref',
			'exam_tip',
			'hint',
			'high_yield',
			'tags',
			'references',
			'media_refs'
		];
		const blob = new Blob([`${headers.join(',')}\r\n`], { type: 'text/csv;charset=utf-8' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = 'questions-template.csv';
		link.click();
		setTimeout(() => URL.revokeObjectURL(url), 0);
	}

	function selectImportFile(event: Event) {
		const input = event.currentTarget;
		if (input instanceof HTMLInputElement) importFile = input.files?.[0] ?? null;
	}

	async function runFileImport(dryRun: boolean) {
		if (!importFile || !examId.trim() || busy || fileImportBusy) return;
		const lowerName = importFile.name.toLowerCase();
		const contentType = lowerName.endsWith('.csv')
			? 'text/csv'
			: lowerName.endsWith('.xlsx')
				? 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
				: '';
		if (!contentType || importFile.size > 3 * 1024 * 1024) {
			error = 'Choose a CSV or XLSX file no larger than 3 MiB.';
			return;
		}
		busy = true;
		fileImportBusy = dryRun ? 'preview' : 'apply';
		error = '';
		importReport = '';
		try {
			const res = await Api.importQuestionFile(examId.trim(), dryRun, importFile, contentType);
			lastBatch = res.status === 'applied' ? res.batch_id : '';
			importReport = JSON.stringify(res, null, 2);
			await refreshAudit();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'File import failed.';
		} finally {
			fileImportBusy = null;
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
</script>

<div class="card">
	<h2>Bulk question import</h2>
	<p class="muted small">
		Import CSV or Excel .xlsx rows with 2-10 option/rationale pairs. Each row
		needs an active rights_ref allowing display and derivatives, scoped to all
		source and media references. correct_option is one-based; separate tags,
		references, and media references with |. Dry run creates no questions. Applied rows start as
		drafts and still require independent review before publication.
	</p>
	<button class="btn" type="button" onclick={downloadQuestionTemplate}>
		Download CSV template
	</button>
	<label class="field" for="import-file">
		<span>Question bank file</span>
		<input
			id="import-file"
			type="file"
			accept=".csv,.xlsx,text/csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
			aria-describedby="import-file-help"
			onchange={selectImportFile}
		/>
	</label>
	<p id="import-file-help" class="muted small">
		{importFile ? importFile.name : 'CSV must be UTF-8. XLSX formulas are not accepted. Maximum file size: 3 MiB.'}
	</p>
	<div class="cluster file-import-actions">
		<button
			class="btn"
			type="button"
			disabled={busy || !examId.trim() || !importFile}
			data-loading={fileImportBusy === 'preview'}
			data-testid="import-file-preview"
			onclick={() => runFileImport(true)}
		>
			{fileImportBusy === 'preview' ? 'Previewing…' : 'Preview file'}
		</button>
		<button
			class="btn primary"
			type="button"
			disabled={busy || !examId.trim() || !importFile}
			data-loading={fileImportBusy === 'apply'}
			data-testid="import-file-apply"
			onclick={() => runFileImport(false)}
		>
			{fileImportBusy === 'apply' ? 'Applying…' : 'Apply file'}
		</button>
	</div>
	<details>
		<summary>Import JSON rows</summary>
		<p class="muted small">
			JSON rows use chapter_id, difficulty, vignette, lead_in, options with
			text and rationale, correct_index (zero-based), key_learning_point,
		and source_ref. Every row also needs an active rights_ref allowing
		display and derivatives for its source and media assets. Optional tags,
		source_refs, and media_refs are retained.
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
	<div class="cluster">
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
	</details>
	{#if importReport}
		<pre
			class="pre-wrap small"
			data-testid="import-report">{importReport}</pre>
	{/if}
	{#if error}
		<p class="error-text" role="alert">{error}</p>
	{/if}
</div>

<style>
	.file-import-actions {
		margin-bottom: var(--space-lg);
	}
</style>
