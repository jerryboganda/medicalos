<script lang="ts">
	// Question report queue: resolve grouped learner reports with a private note and, when fixed,
	// a public correction. No requirement ID cited inline; e2e: report-flow.spec.ts.
	import { onMount } from 'svelte';
	import { Api, ApiError } from '$lib/api';

	let { refreshAudit }: { refreshAudit: () => Promise<void> } = $props();

	let reportQueue = $state<Awaited<ReturnType<typeof Api.adminReports>>['reports']>([]);
	let reportQueueBusy = $state(false);
	let reportQueueError = $state('');
	let reportQueueMessage = $state('');
	let resolvingReport = $state('');
	let resolutionNotes = $state<Record<string, string>>({});
	let correctionNotes = $state<Record<string, string>>({});

	async function loadReportQueue() {
		if (reportQueueBusy) return;
		reportQueueBusy = true;
		reportQueueError = '';
		try {
			reportQueue = (await Api.adminReports()).reports;
		} catch (err) {
			reportQueueError = err instanceof ApiError ? err.message : 'Could not load issue reports.';
		} finally {
			reportQueueBusy = false;
		}
	}

	async function resolveQuestionReport(
		reportId: string,
		status: 'resolved_fixed' | 'resolved_rejected'
	) {
		const resolutionNote = resolutionNotes[reportId]?.trim() ?? '';
		const correctionNote = correctionNotes[reportId]?.trim() ?? '';
		if (!resolutionNote || (status === 'resolved_fixed' && !correctionNote) || resolvingReport) return;
		resolvingReport = reportId;
		reportQueueError = '';
		reportQueueMessage = '';
		try {
			const result = await Api.resolveReport(
				reportId,
				status,
				resolutionNote,
				status === 'resolved_fixed' ? correctionNote : undefined
			);
			reportQueueMessage = `${result.resolved_reports} reports resolved; ${result.notified_reporters} reporters notified.`;
			delete resolutionNotes[reportId];
			delete correctionNotes[reportId];
			await Promise.all([loadReportQueue(), refreshAudit()]);
		} catch (err) {
			reportQueueError = err instanceof ApiError ? err.message : 'Could not resolve the report.';
		} finally {
			resolvingReport = '';
		}
	}

	onMount(loadReportQueue);
</script>

<div class="card" data-testid="report-queue">
	<div class="row">
		<div>
			<h2>Question reports</h2>
			<p class="muted flush">Receipt is acknowledged immediately. Resolve within 72 hours; marking a report fixed requires a newer published question version.</p>
		</div>
		<button class="btn" type="button" disabled={reportQueueBusy} onclick={loadReportQueue} data-testid="report-queue-refresh">
			{reportQueueBusy ? 'Refreshing…' : 'Refresh'}
		</button>
	</div>
	{#if reportQueueMessage}<p role="status" class="muted" data-testid="report-queue-success">{reportQueueMessage}</p>{/if}
	{#if reportQueueError}<p role="alert" class="error-text" data-testid="report-queue-error">{reportQueueError}</p>{/if}
	{#if reportQueue.length === 0 && !reportQueueBusy && !reportQueueError}
		<p class="muted" data-testid="report-queue-empty">No unresolved question reports.</p>
	{/if}
	{#each reportQueue as report (report.question_version_id)}
		<article class="card report-item" data-testid="report-item">
			<div class="row">
				<strong>Version {report.version} · {report.category.replaceAll('_', ' ')} · {report.report_count} report{report.report_count === 1 ? '' : 's'}</strong>
				<span class="chip">{report.quarantined ? 'Quarantined' : 'In review'}</span>
			</div>
			<p class="report-stem"><strong>{report.vignette}</strong><br />{report.lead_in}</p>
			<div aria-label="Learner report details">
				{#each report.reporter_feedback as feedback}
					<p class="muted tight">
						<strong>{feedback.category.replaceAll('_', ' ')}:</strong>
						{feedback.note || 'No additional details provided.'}
					</p>
				{/each}
				{#if report.feedback_truncated}
					<p class="muted tight">Showing the first 20 of {report.report_count} reports.</p>
				{/if}
			</div>
			<p class="muted small">
				First reported {new Date(report.first_reported_at).toLocaleString()} ·
				Acknowledgement due {new Date(report.acknowledgement_due_at).toLocaleString()} ·
				Acknowledged {report.acknowledgements_on_time ? 'within 24 hours' : 'late'} ·
				Resolution due {new Date(report.resolution_due_at).toLocaleString()}
				{#if report.resolution_overdue}<strong class="danger-text">Overdue</strong>{/if}
			</p>
			<label class="field" for={`resolution-note-${report.question_version_id}`}>
				<span>Private resolution note sent to reporters</span>
				<textarea
					id={`resolution-note-${report.question_version_id}`}
					value={resolutionNotes[report.report_id] ?? ''}
					oninput={(event) => (resolutionNotes[report.report_id] = event.currentTarget.value)}
					maxlength="2000"
					rows="2"
					placeholder="Explain the outcome to the reporters."
					data-testid="report-resolution-note"
				></textarea>
			</label>
			<label class="field" for={`correction-note-${report.question_version_id}`}>
				<span>Public correction changelog (required when marking corrected)</span>
				<textarea
					id={`correction-note-${report.question_version_id}`}
					value={correctionNotes[report.report_id] ?? ''}
					oninput={(event) => (correctionNotes[report.report_id] = event.currentTarget.value)}
					maxlength="2000"
					rows="2"
					placeholder="Describe the content change without reporter details."
					data-testid="report-correction-note"
				></textarea>
			</label>
			<div class="cluster">
				<button class="btn primary" type="button" disabled={reportQueueBusy || !!resolvingReport || !resolutionNotes[report.report_id]?.trim() || !correctionNotes[report.report_id]?.trim()} data-testid="report-resolve-fixed" onclick={() => resolveQuestionReport(report.report_id, 'resolved_fixed')}>
					{resolvingReport === report.report_id ? 'Saving…' : 'Mark corrected'}
				</button>
				<button class="btn" type="button" disabled={reportQueueBusy || !!resolvingReport || !resolutionNotes[report.report_id]?.trim()} data-testid="report-resolve-rejected" onclick={() => resolveQuestionReport(report.report_id, 'resolved_rejected')}>
					{resolvingReport === report.report_id ? 'Saving…' : 'Reject report'}
				</button>
			</div>
		</article>
	{/each}
</div>

<style>
	.report-item {
		margin-top: var(--space-md);
	}

	.report-stem {
		margin: var(--space-sm) 0;
	}
</style>
