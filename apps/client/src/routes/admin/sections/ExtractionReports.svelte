<script lang="ts">
	// Document extraction QA: record parser manifests and review critical or uncertain regions.
	// Reads the page-owned content rights for its grant picker. No requirement ID cited inline;
	// e2e: lib07-extraction-reports.spec.ts.
	import { onMount } from 'svelte';
	import { Api, ApiError, type AdminContentRight, type AdminExtractionReport } from '$lib/api';

	let {
		contentRights,
		refreshAudit
	}: { contentRights: AdminContentRight[]; refreshAudit: () => Promise<void> } = $props();

	let extractionReports = $state<AdminExtractionReport[]>([]);
	let extractionBusy = $state(false);
	let extractionError = $state('');
	let extractionMessage = $state('');
	let extractionSourceLabel = $state('');
	let extractionSourceSha256 = $state('');
	let extractionMediaType = $state('application/pdf');
	let extractionParserVersion = $state('');
	let extractionRightsRef = $state('');
	let extractionScanStatus = $state<'clean' | 'blocked' | 'not_scanned'>('not_scanned');
	let extractionExpectedRegions = $state('');
	let extractionExtractedRegions = $state('');
	let extractionUncertainRegions = $state('');
	let extractionCriticalRegions = $state('');
	let extractionSelections = $state<Record<string, string[]>>({});
	let extractionDecisions = $state<Record<string, 'approved' | 'rejected'>>({});
	let extractionNotes = $state<Record<string, string>>({});

	async function loadExtractionReports() {
		extractionBusy = true;
		extractionError = '';
		try {
			extractionReports = (await Api.listExtractionReports()).reports;
		} catch (err) {
			extractionError = err instanceof ApiError ? err.message : 'Extraction reports could not be loaded.';
		} finally {
			extractionBusy = false;
		}
	}

	function reviewRegions(report: AdminExtractionReport) {
		return [...new Set([...report.critical_regions, ...report.uncertain_regions])].sort();
	}

	function toggleExtractionReview(reportId: string, region: string, checked: boolean) {
		const selected = new Set(extractionSelections[reportId] ?? []);
		if (checked) selected.add(region);
		else selected.delete(region);
		extractionSelections = { ...extractionSelections, [reportId]: [...selected].sort() };
	}

	async function createExtractionReport(event: Event) {
		event.preventDefault();
		if (extractionBusy) return;
		extractionBusy = true;
		extractionError = '';
		extractionMessage = '';
		try {
			const result = await Api.createExtractionReport({
				source_label: extractionSourceLabel.trim(),
				source_sha256: extractionSourceSha256.trim(),
				media_type: extractionMediaType,
				parser_version: extractionParserVersion.trim(),
				rights_ref: extractionRightsRef,
				malware_scan_status: extractionScanStatus,
				expected_regions: splitLines(extractionExpectedRegions),
				extracted_regions: splitLines(extractionExtractedRegions),
				uncertain_regions: splitLines(extractionUncertainRegions),
				critical_regions: splitLines(extractionCriticalRegions)
			});
			extractionSourceLabel = '';
			extractionSourceSha256 = '';
			extractionParserVersion = '';
			extractionExpectedRegions = '';
			extractionExtractedRegions = '';
			extractionUncertainRegions = '';
			extractionCriticalRegions = '';
			extractionMessage = `Report recorded with status: ${result.status}.`;
			await Promise.all([loadExtractionReports(), refreshAudit()]);
		} catch (err) {
			extractionError = err instanceof ApiError ? err.message : 'Extraction report could not be recorded.';
		} finally {
			extractionBusy = false;
		}
	}

	async function submitExtractionReview(report: AdminExtractionReport) {
		const decision = extractionDecisions[report.report_id] ?? 'approved';
		const verifiedRegions = extractionSelections[report.report_id] ?? [];
		const note = extractionNotes[report.report_id]?.trim() ?? '';
		if (extractionBusy || !note) return;
		extractionBusy = true;
		extractionError = '';
		extractionMessage = '';
		try {
			const result = await Api.reviewExtractionReport(report.report_id, {
				decision,
				verified_regions: verifiedRegions,
				note
			});
			extractionMessage = `Review recorded with status: ${result.status}.`;
			await Promise.all([loadExtractionReports(), refreshAudit()]);
		} catch (err) {
			extractionError = err instanceof ApiError ? err.message : 'Extraction review could not be recorded.';
		} finally {
			extractionBusy = false;
		}
	}

	function splitLines(value: string) {
		return value
			.split(/\r?\n/)
			.map((item) => item.trim())
			.filter(Boolean);
	}

	onMount(loadExtractionReports);
</script>

<section class="card" aria-labelledby="extraction-reports-heading" data-testid="extraction-reports">
	<h2 id="extraction-reports-heading">Document extraction QA</h2>
	<p class="muted">
		Record a parser manifest and its scan state. This form does not upload files, extract text,
		or run a malware scan. A complete status reflects only the recorded coverage and review.
	</p>
	<form onsubmit={createExtractionReport}>
		<label class="field" for="extraction-source-label">
			<span>Source label</span>
			<input id="extraction-source-label" bind:value={extractionSourceLabel} maxlength="240" required data-testid="extraction-source-label" />
		</label>
		<p class="muted">Use a generic filename label only; do not enter a path or identifying details.</p>
		<label class="field" for="extraction-source-checksum">
			<span>Source SHA-256</span>
			<input id="extraction-source-checksum" bind:value={extractionSourceSha256} maxlength="64" minlength="64" required data-testid="extraction-source-checksum" />
		</label>
		<label class="field" for="extraction-format">
			<span>Document format</span>
			<select id="extraction-format" bind:value={extractionMediaType}>
				<option value="application/pdf">PDF</option>
				<option value="application/vnd.openxmlformats-officedocument.wordprocessingml.document">DOCX</option>
				<option value="application/vnd.openxmlformats-officedocument.presentationml.presentation">PPTX</option>
				<option value="application/epub+zip">EPUB</option>
				<option value="text/html">Structured web export</option>
				<option value="image/jpeg">JPEG image</option>
				<option value="image/png">PNG image</option>
				<option value="image/tiff">TIFF image</option>
				<option value="image/webp">WebP image</option>
				<option value="text/plain">Transcript or plain text</option>
			</select>
		</label>
		<label class="field" for="extraction-parser-version">
			<span>Parser and version</span>
			<input id="extraction-parser-version" bind:value={extractionParserVersion} maxlength="100" required data-testid="extraction-parser-version" />
		</label>
		<label class="field" for="extraction-rights-ref">
			<span>Document-extraction rights</span>
			<select id="extraction-rights-ref" bind:value={extractionRightsRef} required>
				<option value="">Choose an active extraction grant</option>
				{#each contentRights.filter((right) => right.status === 'active' && right.permitted_uses.includes('document_extraction')) as right (right.rights_id)}
					<option value={right.ref_code}>{right.ref_code} · {right.licensor}</option>
				{/each}
			</select>
		</label>
		<label class="field" for="extraction-scan-status">
			<span>Reported malware scan state</span>
			<select id="extraction-scan-status" bind:value={extractionScanStatus}>
				<option value="not_scanned">Not scanned</option>
				<option value="clean">Clean</option>
				<option value="blocked">Blocked</option>
			</select>
		</label>
		<label class="field" for="extraction-expected-regions">
			<span>Expected region references (one per line)</span>
			<textarea id="extraction-expected-regions" bind:value={extractionExpectedRegions} rows="3" required data-testid="extraction-expected-regions"></textarea>
		</label>
		<label class="field" for="extraction-extracted-regions">
			<span>Extracted region references (one per line)</span>
			<textarea id="extraction-extracted-regions" bind:value={extractionExtractedRegions} rows="3" data-testid="extraction-extracted-regions"></textarea>
		</label>
		<label class="field" for="extraction-uncertain-regions">
			<span>Uncertain region references (one per line)</span>
			<textarea id="extraction-uncertain-regions" bind:value={extractionUncertainRegions} rows="2"></textarea>
		</label>
		<label class="field" for="extraction-critical-regions">
			<span>Critical tables or medical quantities (one reference per line)</span>
			<textarea id="extraction-critical-regions" bind:value={extractionCriticalRegions} rows="2"></textarea>
		</label>
		<button class="btn primary" type="submit" disabled={extractionBusy || !extractionRightsRef || !extractionExpectedRegions.trim()} data-testid="extraction-report-create">
			{extractionBusy ? 'Recording…' : 'Record extraction report'}
		</button>
	</form>
	{#if extractionMessage}
		<p class="muted" role="status" data-testid="extraction-report-message">{extractionMessage}</p>
	{/if}
	{#if extractionError}
		<p class="error-text" role="alert">{extractionError}</p>
	{/if}

	<h3>Extraction reports</h3>
	{#if extractionBusy && extractionReports.length === 0}
		<p class="muted is-loading" role="status">Loading extraction reports…</p>
	{:else if extractionReports.length === 0}
		<p class="muted">No extraction reports recorded.</p>
	{:else}
		<ul class="bare-list">
			{#each extractionReports as report (report.report_id)}
				<li class="extraction-report" data-testid={'extraction-report-' + report.report_id}>
					<strong>{report.source_label}</strong>
					<span class="chip">{report.status.replaceAll('_', ' ')}</span>
					<p class="muted">SHA-256: <code>{report.source_sha256}</code></p>
					<p class="muted">{report.media_type} · {report.parser_version} · rights {report.rights_ref}</p>
					<p>Scan: {report.malware_scan_status.replaceAll('_', ' ')}</p>
					<p>Expected: {report.expected_regions.join(', ') || 'none'}</p>
					<p>Extracted: {report.extracted_regions.join(', ') || 'none'}</p>
					<p>Missing: {report.missing_regions.join(', ') || 'none'}</p>
					<p>Uncertain: {report.uncertain_regions.join(', ') || 'none'}</p>
					<p>Critical: {report.critical_regions.join(', ') || 'none'}</p>
					{#if report.review}
						<p>Review: {report.review.decision} · {report.review.note}</p>
					{/if}
					{#if report.status === 'review_required' && report.rights_available && report.malware_scan_status === 'clean' && report.missing_regions.length === 0 && reviewRegions(report).length > 0}
						<fieldset class="region-review" disabled={extractionBusy}>
							<legend>Review critical and uncertain regions</legend>
							{#each reviewRegions(report) as region (region)}
								<label class="field" for={`extraction-verify-${report.report_id}-${region}`}>
									<input id={`extraction-verify-${report.report_id}-${region}`} type="checkbox" checked={extractionSelections[report.report_id]?.includes(region) ?? false} onchange={(event) => toggleExtractionReview(report.report_id, region, event.currentTarget.checked)} />
									<span>Verified: {region}</span>
								</label>
							{/each}
							<label class="field" for={`extraction-decision-${report.report_id}`}>
								<span>Decision</span>
								<select id={`extraction-decision-${report.report_id}`} value={extractionDecisions[report.report_id] ?? 'approved'} onchange={(event) => extractionDecisions = { ...extractionDecisions, [report.report_id]: event.currentTarget.value as 'approved' | 'rejected' }}>
									<option value="approved">Approve</option>
									<option value="rejected">Reject</option>
								</select>
							</label>
							<label class="field" for={`extraction-review-note-${report.report_id}`}>
								<span>Review note</span>
								<textarea id={`extraction-review-note-${report.report_id}`} bind:value={extractionNotes[report.report_id]} minlength="10" maxlength="2000" rows="2" required></textarea>
							</label>
							<button class="btn" type="button" disabled={extractionBusy || !extractionNotes[report.report_id]?.trim() || ((extractionDecisions[report.report_id] ?? 'approved') === 'approved' && !reviewRegions(report).every((region) => extractionSelections[report.report_id]?.includes(region)))} onclick={() => submitExtractionReview(report)} data-testid={`extraction-report-review-${report.report_id}`}>
								Record review
							</button>
						</fieldset>
					{:else if report.malware_scan_status === 'not_scanned'}
						<p class="muted">A clean scan result is required before approval.</p>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
	<button class="btn" type="button" disabled={extractionBusy} onclick={loadExtractionReports}>Refresh extraction reports</button>
</section>

<style>
	.extraction-report {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.extraction-report code {
		overflow-wrap: anywhere;
	}

	.region-review {
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}
</style>
