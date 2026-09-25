<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E4 S4 R4 V4 */
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import {
		Api,
		ApiError,
		adminToken,
		type AdminSettings,
		type AdminConcept,
		type AdminContentRight,
		type AdminExtractionReport,
		type AdminCurriculumNode,
		type AdminScenarioAssessment,
		type PendingScenarioAssessment,
		type ScenarioAssessmentAppeal,
		type ScenarioAssessmentAppealQueueItem
	} from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	// ADMIN-06 baseline console: hierarchy management, question creation, and
	// CSV/XLSX plus JSON bulk import with dry-run/rollback. Gated by the admin token —
	// role-aware accounts (§18.1) replace this gate later.
	let token = $state('');
	let unlocked = $state(false);
	let busy = $state(false);
	let message = $state('');
	let error = $state('');
	let audit = $state<unknown[]>([]);
	let reportQueue = $state<Awaited<ReturnType<typeof Api.adminReports>>['reports']>([]);
	let reportQueueBusy = $state(false);
	let reportQueueError = $state('');
	let reportQueueMessage = $state('');
	let resolvingReport = $state('');
	let resolutionNotes = $state<Record<string, string>>({});
	let correctionNotes = $state<Record<string, string>>({});
	let contentRights = $state<AdminContentRight[]>([]);
	let rightsBusy = $state(false);
	let rightsError = $state('');
	let rightsMessage = $state('');
	let rightsRefCode = $state('');
	let rightsLicensor = $state('');
	let rightsTerritory = $state('worldwide');
	let rightsPermittedUses = $state<string[]>(['display']);
	let rightsValidFrom = $state('');
	let rightsValidTo = $state('');
	let rightsNotes = $state('');
	let rightsContractRef = $state('');
	let rightsContractVersion = $state('');
	let rightsAssetRefs = $state('');
	let rightsAudiences = $state('');
	let rightsSeatLimit = $state<number | undefined>();
	let rightsOfflineTerms = $state('');
	let rightsQuotationLimit = $state<number | undefined>();
	let rightsAiTerms = $state('');
	let rightsDerivativeTerms = $state('');
	let rightsAttribution = $state('');
	let rightsRoyaltyTerms = $state('');
	let revocationReasons = $state<Record<string, string>>({});
	let revokingRightsId = $state('');
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
	let pendingAssessments = $state<PendingScenarioAssessment[]>([]);
	let pendingAssessmentBusy = $state(false);
	let pendingAssessmentError = $state('');
	let pendingAssessmentMessage = $state('');
	let selectedAssessment = $state<AdminScenarioAssessment | null>(null);
	let assessmentDrafts = $state<Record<string, ScenarioAssessmentDraft>>({});
	let assessmentSaving = $state(false);
	let assessmentAppeals = $state<ScenarioAssessmentAppealQueueItem[]>([]);
	let appealQueueBusy = $state(false);
	let appealQueueError = $state('');
	let appealQueueMessage = $state('');
	let selectedAssessmentAppeal = $state<ScenarioAssessmentAppeal | null>(null);
	let appealDecision = $state<'confirmed' | 'reassessment_required'>('confirmed');
	let appealRationale = $state('');
	let settingsDraft = $state({
		masteryLow: 1400,
		masteryHigh: 1600,
		communityMinSample: 20,
		freeDailyQuestions: 10,
		freeDailyCoachTurns: 1,
		offlineLeaseDays: 14,
		maxReviewsPerDay: 30,
		maxNewCardsPerDay: 10,
		competitionEasyPoints: 5,
		competitionMediumPoints: 10,
		competitionHardPoints: 15
	});
	let settingsIntervals = $state('1, 3, 7, 14');
	let settingsLoaded = $state(false);
	let settingsBusy = $state(false);
	let settingsMessage = $state('');
	let settingsError = $state('');
	type ScenarioAssessmentDraft = {
		status: 'assessed' | 'not_assessed';
		score: string;
		evidence: string;
		eventIndexes: number[];
		uncertain: boolean;
	};

	const contentRightUses = [
		{ value: 'display', label: 'Display' },
		{ value: 'search', label: 'Search' },
		{ value: 'offline', label: 'Offline use' },
		{ value: 'embeddings', label: 'Embeddings' },
		{ value: 'ai', label: 'AI use' },
		{ value: 'derivatives', label: 'Derivatives' },
		{ value: 'translation', label: 'Translation' },
		{ value: 'private_import', label: 'Private learner imports' },
		{ value: 'document_extraction', label: 'Document extraction' }
	];

	// hierarchy
	let examId = $state('');
	let nodeKind = $state('chapter');
	let nodeName = $state('');
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

	// import
	let importJson = $state('');
	let importFile = $state<File | null>(null);
	let fileImportBusy = $state<'preview' | 'apply' | null>(null);
	let importReport = $state('');
	let lastBatch = $state('');

	// editorial workflow (INST-05)
	let workflowIds = $state('');
	let workflowReport = $state<unknown[]>([]);

	// INST-03: provider config is global-admin gated; secrets are write-only.
	let oidcInstitutionId = $state('');
	let oidcIssuer = $state('');
	let oidcClientId = $state('');
	let oidcClientSecret = $state('');
	let oidcEnabled = $state('false');
	let oidcSecretConfigured = $state(false);
	let oidcClearSecret = $state(false);
	let oidcLoaded = $state(false);
	let oidcBusy = $state(false);
	let oidcMessage = $state('');
	let oidcError = $state('');
	let oidcSignInUrl = $state('');

	async function unlock() {
		unlocked = adminToken().length > 0;
		if (unlocked) {
			await Promise.all([
				refreshAudit(),
				loadReportQueue(),
				loadConcepts(),
				loadContentRights(),
				loadExtractionReports(),
				loadPendingAssessments(),
				loadScenarioAssessmentAppeals(),
				loadRuntimeSettings()
			]);
		}
	}

	async function loadRuntimeSettings() {
		settingsBusy = true;
		settingsError = '';
		try {
			const { settings } = await Api.adminSettings();
			settingsDraft = {
				masteryLow: settings.mastery_bands[0],
				masteryHigh: settings.mastery_bands[1],
				communityMinSample: settings.community_min_sample,
				freeDailyQuestions: settings.free_daily_questions,
				freeDailyCoachTurns: settings.free_daily_coach_turns,
				offlineLeaseDays: settings.offline_lease_days,
				maxReviewsPerDay: settings.max_reviews_per_day,
				maxNewCardsPerDay: settings.max_new_cards_per_day,
				competitionEasyPoints: settings.competition_difficulty_points[0],
				competitionMediumPoints: settings.competition_difficulty_points[1],
				competitionHardPoints: settings.competition_difficulty_points[2]
			};
			settingsIntervals = settings.retest_intervals_days.join(', ');
			settingsLoaded = true;
		} catch (err) {
			settingsError = err instanceof ApiError ? err.message : 'Runtime settings could not be loaded.';
		} finally {
			settingsBusy = false;
		}
	}

	async function saveRuntimeSettings(event: Event) {
		event.preventDefault();
		if (settingsBusy || !settingsLoaded) return;
		const intervals = settingsIntervals
			.split(',')
			.map((value) => value.trim())
			.filter(Boolean)
			.map(Number);
		if (intervals.some((value) => !Number.isSafeInteger(value))) {
			settingsError = 'Re-test intervals must be comma-separated whole numbers.';
			return;
		}
		settingsBusy = true;
		settingsError = '';
		settingsMessage = '';
		try {
			const settings: AdminSettings = {
				mastery_bands: [settingsDraft.masteryLow, settingsDraft.masteryHigh],
				community_min_sample: settingsDraft.communityMinSample,
				free_daily_questions: settingsDraft.freeDailyQuestions,
				free_daily_coach_turns: settingsDraft.freeDailyCoachTurns,
				retest_intervals_days: intervals,
				offline_lease_days: settingsDraft.offlineLeaseDays,
				max_reviews_per_day: settingsDraft.maxReviewsPerDay,
				max_new_cards_per_day: settingsDraft.maxNewCardsPerDay,
				competition_difficulty_points: [
					settingsDraft.competitionEasyPoints,
					settingsDraft.competitionMediumPoints,
					settingsDraft.competitionHardPoints
				]
			};
			await Api.updateAdminSettings(settings);
			settingsMessage = 'Runtime settings saved.';
			await refreshAudit();
		} catch (err) {
			settingsError = err instanceof ApiError ? err.message : 'Runtime settings could not be saved.';
		} finally {
			settingsBusy = false;
		}
	}

	async function loadPendingAssessments() {
		pendingAssessmentBusy = true;
		pendingAssessmentError = '';
		try {
			pendingAssessments = (await Api.pendingScenarioAssessments()).runs;
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Assessment queue could not be loaded.';
		} finally {
			pendingAssessmentBusy = false;
		}
	}

	async function loadScenarioAssessmentAppeals() {
		appealQueueBusy = true;
		appealQueueError = '';
		try {
			assessmentAppeals = (await Api.listScenarioAssessmentAppeals()).appeals;
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Assessment appeals could not be loaded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	async function openScenarioAssessmentAppeal(appealId: string) {
		appealQueueBusy = true;
		appealQueueError = '';
		appealQueueMessage = '';
		try {
			selectedAssessmentAppeal = await Api.getScenarioAssessmentAppeal(appealId);
			appealDecision = 'confirmed';
			appealRationale = '';
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Appeal details could not be loaded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	async function reviewScenarioAssessmentAppeal(event: Event) {
		event.preventDefault();
		if (!selectedAssessmentAppeal || appealQueueBusy || !appealRationale.trim()) return;
		appealQueueBusy = true;
		appealQueueError = '';
		appealQueueMessage = '';
		try {
			const result = await Api.reviewScenarioAssessmentAppeal(
				selectedAssessmentAppeal.appeal_id,
				{ decision: appealDecision, rationale: appealRationale.trim() }
			);
			appealQueueMessage = `Appeal reviewed: ${result.decision.replaceAll('_', ' ')}.`;
			selectedAssessmentAppeal = null;
			await Promise.all([loadScenarioAssessmentAppeals(), refreshAudit()]);
		} catch (err) {
			appealQueueError = err instanceof ApiError ? err.message : 'Appeal decision could not be recorded.';
		} finally {
			appealQueueBusy = false;
		}
	}

	async function openScenarioAssessment(runId: string) {
		pendingAssessmentBusy = true;
		pendingAssessmentError = '';
		pendingAssessmentMessage = '';
		selectedAssessment = null;
		try {
			const assessment = await Api.getScenarioAssessment(runId);
			selectedAssessment = assessment;
			assessmentDrafts = Object.fromEntries(
				assessment.rubric.map((criterion) => [criterion.criterion_key, {
					status: 'not_assessed',
					score: '',
					evidence: '',
					eventIndexes: [],
					uncertain: false
				} satisfies ScenarioAssessmentDraft])
			);
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Scenario assessment could not be opened.';
		} finally {
			pendingAssessmentBusy = false;
		}
	}

	function updateScenarioAssessmentDraft(
		criterionKey: string,
		patch: Partial<ScenarioAssessmentDraft>
	) {
		const current = assessmentDrafts[criterionKey];
		if (!current) return;
		assessmentDrafts = {
			...assessmentDrafts,
			[criterionKey]: { ...current, ...patch }
		};
	}

	function toggleScenarioAssessmentEvent(criterionKey: string, index: number, checked: boolean) {
		const current = assessmentDrafts[criterionKey];
		if (!current) return;
		const indexes = new Set(current.eventIndexes);
		if (checked) indexes.add(index);
		else indexes.delete(index);
		updateScenarioAssessmentDraft(criterionKey, { eventIndexes: [...indexes].sort((a, b) => a - b) });
	}

	async function recordScenarioAssessment(event: Event) {
		event.preventDefault();
		if (!selectedAssessment || assessmentSaving) return;
		assessmentSaving = true;
		pendingAssessmentError = '';
		pendingAssessmentMessage = '';
		try {
			const result = await Api.recordScenarioAssessment(selectedAssessment.run_id, {
				criteria: selectedAssessment.rubric.map((criterion) => {
					const draft = assessmentDrafts[criterion.criterion_key];
					const assessed = draft.status === 'assessed';
					return {
						criterion_key: criterion.criterion_key,
						assessment_status: draft.status,
						score: assessed && draft.score.trim() ? Number(draft.score) : null,
						evidence: draft.evidence.trim(),
						transcript_event_indexes: assessed ? draft.eventIndexes : [],
						transcript_uncertain: assessed && draft.uncertain
					};
				})
			});
			pendingAssessmentMessage = `Recorded ${result.recorded_criteria} criterion results (${result.not_assessed} not assessed).`;
			selectedAssessment = null;
			assessmentDrafts = {};
			await Promise.all([loadPendingAssessments(), refreshAudit()]);
		} catch (err) {
			pendingAssessmentError = err instanceof ApiError ? err.message : 'Scenario assessment could not be recorded.';
		} finally {
			assessmentSaving = false;
		}
	}

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

	async function loadContentRights() {
		rightsBusy = true;
		rightsError = '';
		try {
			contentRights = (await Api.listContentRights()).rights;
		} catch (err) {
			rightsError = err instanceof ApiError ? err.message : 'Content rights could not be loaded.';
		} finally {
			rightsBusy = false;
		}
	}

	function toggleContentRightUse(value: string, checked: boolean) {
		rightsPermittedUses = checked
			? [...new Set([...rightsPermittedUses, value])]
			: rightsPermittedUses.filter((use) => use !== value);
	}

	function splitLines(value: string) {
		return value
			.split(/\r?\n/)
			.map((item) => item.trim())
			.filter(Boolean);
	}

	async function createContentRight(event: Event) {
		event.preventDefault();
		if (rightsBusy || rightsPermittedUses.length === 0) return;
		rightsBusy = true;
		rightsError = '';
		rightsMessage = '';
		try {
			const result = await Api.createContentRights({
				ref_code: rightsRefCode.trim(),
				licensor: rightsLicensor.trim(),
				territory: rightsTerritory.trim() || 'worldwide',
				permitted_uses: rightsPermittedUses,
				valid_from: rightsValidFrom,
				...(rightsValidTo ? { valid_to: rightsValidTo } : {}),
				...(rightsNotes.trim() ? { notes: rightsNotes.trim() } : {}),
				...(rightsContractRef.trim() ? { contract_ref: rightsContractRef.trim() } : {}),
				...(rightsContractVersion.trim()
					? { contract_version: rightsContractVersion.trim() }
					: {}),
				asset_refs: splitLines(rightsAssetRefs),
				audiences: splitLines(rightsAudiences),
				...(rightsSeatLimit !== undefined ? { seat_limit: rightsSeatLimit } : {}),
				...(rightsOfflineTerms.trim() ? { offline_terms: rightsOfflineTerms.trim() } : {}),
				...(rightsQuotationLimit !== undefined
					? { quotation_limit_words: rightsQuotationLimit }
					: {}),
				...(rightsAiTerms.trim() ? { ai_terms: rightsAiTerms.trim() } : {}),
				...(rightsDerivativeTerms.trim()
					? { derivative_terms: rightsDerivativeTerms.trim() }
					: {}),
				...(rightsAttribution.trim() ? { attribution: rightsAttribution.trim() } : {}),
				...(rightsRoyaltyTerms.trim() ? { royalty_terms: rightsRoyaltyTerms.trim() } : {})
			});
			rightsRefCode = '';
			rightsLicensor = '';
			rightsTerritory = 'worldwide';
			rightsValidFrom = '';
			rightsValidTo = '';
			rightsNotes = '';
			rightsContractRef = '';
			rightsContractVersion = '';
			rightsAssetRefs = '';
			rightsAudiences = '';
			rightsSeatLimit = undefined;
			rightsOfflineTerms = '';
			rightsQuotationLimit = undefined;
			rightsAiTerms = '';
			rightsDerivativeTerms = '';
			rightsAttribution = '';
			rightsRoyaltyTerms = '';
			rightsMessage = 'Rights record ' + result.ref_code + ' created.';
			await Promise.all([loadContentRights(), refreshAudit()]);
		} catch (err) {
			rightsError =
				err instanceof ApiError ? err.message : 'Content rights could not be created.';
		} finally {
			rightsBusy = false;
		}
	}

	async function revokeContentRight(rightsId: string) {
		const reason = revocationReasons[rightsId]?.trim() ?? '';
		if (!reason || rightsBusy || revokingRightsId) return;
		revokingRightsId = rightsId;
		rightsError = '';
		rightsMessage = '';
		try {
			await Api.revokeContentRights(rightsId, reason);
			revocationReasons = { ...revocationReasons, [rightsId]: '' };
			rightsMessage = 'Rights record revoked. New reads and imports are blocked.';
			await Promise.all([loadContentRights(), refreshAudit()]);
		} catch (err) {
			rightsError = err instanceof ApiError ? err.message : 'Content rights could not be revoked.';
		} finally {
			revokingRightsId = '';
		}
	}

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

	async function refreshAudit() {
		try {
			audit = (await Api.listAdminAudit()).events;
		} catch {
			audit = [];
		}
	}

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

	async function loadOidcProvider() {
		if (oidcBusy || !oidcInstitutionId.trim()) return;
		oidcBusy = true;
		oidcError = '';
		oidcMessage = '';
		oidcSignInUrl = '';
		try {
			const provider = await Api.getInstitutionOidc(oidcInstitutionId.trim());
			oidcIssuer = provider.issuer;
			oidcClientId = provider.client_id;
			oidcEnabled = String(provider.enabled);
			oidcSecretConfigured = provider.client_secret_configured;
			oidcClearSecret = false;
			oidcLoaded = true;
			oidcMessage = 'Provider settings loaded.';
		} catch (err) {
			if (err instanceof ApiError && err.code === 'oidc_provider_not_found') {
				oidcIssuer = '';
				oidcClientId = '';
				oidcEnabled = 'false';
				oidcSecretConfigured = false;
				oidcClearSecret = false;
				oidcLoaded = true;
				oidcMessage = 'No provider is configured yet.';
			} else {
				oidcError = err instanceof ApiError ? err.message : 'Provider settings could not be loaded.';
			}
		} finally {
			oidcBusy = false;
		}
	}

	async function saveOidcProvider(event: Event) {
		event.preventDefault();
		if (oidcBusy) return;
		oidcBusy = true;
		oidcError = '';
		oidcMessage = '';
		oidcSignInUrl = '';
		try {
			const provider = await Api.configureInstitutionOidc(oidcInstitutionId.trim(), {
				issuer: oidcIssuer.trim(),
				client_id: oidcClientId.trim(),
				...(oidcClientSecret ? { client_secret: oidcClientSecret } : {}),
				clear_client_secret: oidcClearSecret,
				enabled: oidcEnabled === 'true'
			});
			oidcSecretConfigured = provider.client_secret_configured;
			oidcEnabled = String(provider.enabled);
			oidcClientSecret = '';
			oidcClearSecret = false;
			oidcMessage = 'Provider settings saved.';
			if (provider.enabled) {
				oidcSignInUrl = `${window.location.origin}${base}/login?institution=${encodeURIComponent(oidcInstitutionId.trim())}`;
			}
			await refreshAudit();
		} catch (err) {
			oidcError = err instanceof ApiError ? err.message : 'Provider settings could not be saved.';
		} finally {
			oidcBusy = false;
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
	<p><a class="btn" href={`${base}/admin/image-annotations`}>Image annotation review</a></p>
	<section class="card" aria-labelledby="runtime-settings-heading" data-testid="runtime-settings">
		<h2 id="runtime-settings-heading">Runtime settings</h2>
		<p class="muted">
			Changes apply to new requests immediately. Active offline leases keep their expiry; new and renewed
			leases use this window. Review caps affect the next queue request; due overflow remains scheduled.
			Community disclosure keeps a positive minimum sample; changes are audited. Competition points apply
			to newly created competitions; ties use accuracy, total time, then submission time.
		</p>
		{#if settingsError}<p class="error-text" role="alert">{settingsError}</p>{/if}
		{#if !settingsLoaded}
			<p class="muted" role="status">{settingsBusy ? 'Loading runtime settings…' : 'Runtime settings are unavailable.'}</p>
		{:else}
			<form onsubmit={saveRuntimeSettings}>
				<div class="runtime-settings-fields">
					<label class="field" for="settings-mastery-low">
						<span>Lower mastery boundary</span>
						<input id="settings-mastery-low" type="number" min="0" max="2999" step="1" bind:value={settingsDraft.masteryLow} required />
					</label>
					<label class="field" for="settings-mastery-high">
						<span>Higher mastery boundary</span>
						<input id="settings-mastery-high" type="number" min="1" max="3000" step="1" bind:value={settingsDraft.masteryHigh} required />
					</label>
					<label class="field" for="settings-community-sample">
						<span>Minimum community sample</span>
						<input id="settings-community-sample" aria-label="Minimum community sample" type="number" min="1" max="1000000" step="1" bind:value={settingsDraft.communityMinSample} required />
					</label>
					<label class="field" for="settings-free-questions">
						<span>Daily free questions</span>
						<input id="settings-free-questions" aria-label="Daily free questions" type="number" min="0" max="5000" step="1" bind:value={settingsDraft.freeDailyQuestions} required />
					</label>
					<label class="field" for="settings-free-coach-turns">
						<span>Daily free Coach turns</span>
						<input id="settings-free-coach-turns" aria-label="Daily free Coach turns" type="number" min="0" max="1000" step="1" bind:value={settingsDraft.freeDailyCoachTurns} required />
					</label>
					<label class="field" for="settings-retest-intervals">
						<span>Re-test intervals in days</span>
						<input id="settings-retest-intervals" aria-label="Re-test intervals in days" bind:value={settingsIntervals} maxlength="100" required />
					</label>
					<label class="field" for="settings-offline-lease-days">
						<span>Offline lease length in days</span>
						<input id="settings-offline-lease-days" type="number" min="1" max="30" step="1" bind:value={settingsDraft.offlineLeaseDays} required />
					</label>
					<label class="field" for="settings-due-review-cap">
						<span>Daily due-review cap</span>
						<input id="settings-due-review-cap" type="number" min="0" max="5000" step="1" bind:value={settingsDraft.maxReviewsPerDay} required />
					</label>
					<label class="field" for="settings-new-card-cap">
						<span>Daily new-card cap</span>
						<input id="settings-new-card-cap" type="number" min="0" max="1000" step="1" bind:value={settingsDraft.maxNewCardsPerDay} required />
					</label>
					<label class="field" for="settings-competition-easy-points">
						<span>Easy competition points</span>
						<input id="settings-competition-easy-points" type="number" min="1" max="1000" step="1" bind:value={settingsDraft.competitionEasyPoints} required />
					</label>
					<label class="field" for="settings-competition-medium-points">
						<span>Medium competition points</span>
						<input id="settings-competition-medium-points" type="number" min="2" max="1000" step="1" bind:value={settingsDraft.competitionMediumPoints} required />
					</label>
					<label class="field" for="settings-competition-hard-points">
						<span>Hard competition points</span>
						<input id="settings-competition-hard-points" type="number" min="3" max="1000" step="1" bind:value={settingsDraft.competitionHardPoints} required />
					</label>
				</div>
				<button class="btn primary" type="submit" disabled={settingsBusy} data-testid="settings-save">
					{settingsBusy ? 'Saving…' : 'Save runtime settings'}
				</button>
			</form>
		{/if}
		{#if settingsMessage}<p class="muted" role="status" data-testid="settings-message">{settingsMessage}</p>{/if}
	</section>
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

	<section class="card" aria-labelledby="scenario-assessments-heading" data-testid="scenario-assessments">
		<h2 id="scenario-assessments-heading">Clinical scenario assessments</h2>
		<p class="muted">
			Review finished stations against their versioned rubric. Scores must cite observed transcript
			events; mark a criterion not assessed when the evidence is absent. This records examiner
			judgment and does not validate clinical competence.
		</p>
		{#if pendingAssessmentBusy && pendingAssessments.length === 0}
			<p class="muted" role="status">Loading finished stations…</p>
		{:else if pendingAssessmentError}
			<p class="error-text" role="alert">{pendingAssessmentError}</p>
		{:else if pendingAssessments.length === 0}
			<p class="muted">No finished stations are waiting for assessment.</p>
		{:else}
			<ul class="scenario-assessment-list">
				{#each pendingAssessments as run (run.run_id)}
					<li class="scenario-assessment-row" data-testid={`pending-scenario-${run.run_id}`}>
						<div>
							<strong>{run.scenario}</strong>
							<p class="muted">Version {run.scenario_version} · {run.criterion_count} criteria · finished {new Date(run.finished_at).toLocaleString()}</p>
						</div>
						<button class="btn" type="button" disabled={pendingAssessmentBusy} onclick={() => openScenarioAssessment(run.run_id)} data-testid={`scenario-assessment-open-${run.run_id}`}>
							{pendingAssessmentBusy ? 'Opening…' : 'Review station'}
						</button>
					</li>
				{/each}
			</ul>
		{/if}
		{#if pendingAssessmentMessage}
			<p class="muted" role="status" data-testid="scenario-assessment-message">{pendingAssessmentMessage}</p>
		{/if}
		<button class="btn" type="button" disabled={pendingAssessmentBusy} onclick={loadPendingAssessments}>Refresh assessment queue</button>

		{#if selectedAssessment}
			<section class="scenario-review" aria-labelledby="scenario-review-heading">
				<h3 id="scenario-review-heading">{selectedAssessment.scenario} · version {selectedAssessment.scenario_version}</h3>
				<p class="muted">Run {selectedAssessment.run_id} · finished {new Date(selectedAssessment.finished_at).toLocaleString()}</p>
				<h4>Immutable station transcript</h4>
				<ol class="scenario-transcript">
					{#each selectedAssessment.transcript as event, index (index)}
						<li><strong>Event {index + 1}: {event.on}</strong><span class="muted">{event.from} → {event.to}{event.actor_role ? ` · ${event.actor_role.replaceAll('_', ' ')}` : ''}</span></li>
					{/each}
				</ol>
				<form onsubmit={recordScenarioAssessment}>
					{#each selectedAssessment.rubric as criterion (criterion.criterion_key)}
						{@const draft = assessmentDrafts[criterion.criterion_key]}
						<fieldset class="scenario-assessment-criterion" disabled={assessmentSaving}>
							<legend>{criterion.label} · max {criterion.max_score}</legend>
							<label class="field" for={`scenario-status-${criterion.criterion_key}`}>
								<span>Assessment for {criterion.criterion_key}</span>
								<select id={`scenario-status-${criterion.criterion_key}`} value={draft.status} onchange={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { status: event.currentTarget.value as ScenarioAssessmentDraft['status'] })}>
									<option value="assessed">Assessed from transcript</option>
									<option value="not_assessed">Not assessed</option>
								</select>
							</label>
							{#if draft.status === 'assessed'}
								<label class="field" for={`scenario-score-${criterion.criterion_key}`}>
									<span>Score for {criterion.criterion_key}</span>
									<input id={`scenario-score-${criterion.criterion_key}`} type="number" min="0" max={criterion.max_score} step="any" value={draft.score} oninput={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { score: event.currentTarget.value })} required />
								</label>
								<p class="muted">Select the transcript events that support this score.</p>
								{#each selectedAssessment.transcript as transcriptEvent, index (index)}
									<label class="field" for={`scenario-event-${criterion.criterion_key}-${index}`}>
										<input id={`scenario-event-${criterion.criterion_key}-${index}`} type="checkbox" checked={draft.eventIndexes.includes(index)} onchange={(event) => toggleScenarioAssessmentEvent(criterion.criterion_key, index, event.currentTarget.checked)} />
										<span>Use event {index + 1}: {transcriptEvent.on}</span>
									</label>
								{/each}
								<label class="field" for={`scenario-uncertain-${criterion.criterion_key}`}>
									<input id={`scenario-uncertain-${criterion.criterion_key}`} type="checkbox" checked={draft.uncertain} onchange={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { uncertain: event.currentTarget.checked })} />
									<span>Transcript evidence is uncertain</span>
								</label>
							{/if}
							<label class="field" for={`scenario-evidence-${criterion.criterion_key}`}>
								<span>Evidence or not-assessed reason for {criterion.criterion_key}</span>
								<textarea id={`scenario-evidence-${criterion.criterion_key}`} value={draft.evidence} oninput={(event) => updateScenarioAssessmentDraft(criterion.criterion_key, { evidence: event.currentTarget.value })} minlength="1" maxlength="2000" rows="2" required></textarea>
							</label>
						</fieldset>
					{/each}
					<button class="btn primary" type="submit" disabled={assessmentSaving || selectedAssessment.rubric.some((criterion) => !assessmentDrafts[criterion.criterion_key]?.evidence.trim())} data-testid="scenario-assessment-record">
						{assessmentSaving ? 'Recording…' : 'Record examiner assessment'}
					</button>
				</form>
			</section>
		{/if}
	</section>

	<section class="card" aria-labelledby="scenario-appeals-heading" data-testid="scenario-assessment-appeals">
		<h2 id="scenario-appeals-heading">Independent assessment appeals</h2>
		<p class="muted">
			Review the learner’s reason and the original station evidence. The learner and every original
			examiner are barred from deciding the appeal. Decisions are permanent and do not rewrite the
			original assessment.
		</p>
		{#if appealQueueBusy && assessmentAppeals.length === 0}
			<p class="muted" role="status">Loading assessment appeals…</p>
		{:else if appealQueueError}
			<p class="error-text" role="alert">{appealQueueError}</p>
		{:else if assessmentAppeals.length === 0}
			<p class="muted">No assessment appeals are waiting for review.</p>
		{:else}
			<ul class="scenario-assessment-list">
				{#each assessmentAppeals as appeal (appeal.appeal_id)}
					<li class="scenario-assessment-row" data-testid={`scenario-appeal-${appeal.appeal_id}`}>
						<div>
							<strong>{appeal.scenario}</strong>
							<p class="muted">Version {appeal.scenario_version} · submitted {new Date(appeal.created_at).toLocaleString()}</p>
						</div>
						<button class="btn" type="button" disabled={appealQueueBusy} onclick={() => openScenarioAssessmentAppeal(appeal.appeal_id)} data-testid={`scenario-appeal-open-${appeal.appeal_id}`}>
							Review appeal
						</button>
					</li>
				{/each}
			</ul>
		{/if}
		{#if appealQueueMessage}
			<p class="muted" role="status" data-testid="scenario-appeal-message">{appealQueueMessage}</p>
		{/if}
		<button class="btn" type="button" disabled={appealQueueBusy} onclick={loadScenarioAssessmentAppeals}>Refresh appeal queue</button>

		{#if selectedAssessmentAppeal}
			<section class="scenario-review" aria-labelledby="scenario-appeal-review-heading">
				<h3 id="scenario-appeal-review-heading">{selectedAssessmentAppeal.scenario} · version {selectedAssessmentAppeal.scenario_version}</h3>
				<p class="muted">Run {selectedAssessmentAppeal.run_id} · submitted {new Date(selectedAssessmentAppeal.created_at).toLocaleString()}</p>
				<h4>Learner’s appeal reason</h4>
				<p class="appeal-reason" data-testid="scenario-appeal-reason">{selectedAssessmentAppeal.reason}</p>
				<h4>Original station transcript</h4>
				<ol class="scenario-transcript">
					{#each selectedAssessmentAppeal.timeline as event, index (index)}
						<li><strong>Event {index + 1}: {event.on}</strong><span class="muted">{event.from} → {event.to}</span></li>
					{/each}
				</ol>
				<h4>Original rubric</h4>
				<ul class="scenario-assessment-list">
					{#each selectedAssessmentAppeal.rubric as criterion (criterion.criterion_key)}
						<li class="rights-record">
							<strong>{criterion.label}</strong>
							<p class="muted">{criterion.assessment_status.replaceAll('_', ' ')}{criterion.score === null ? '' : ` · ${criterion.score} / ${criterion.max_score}`}</p>
							<p>{criterion.evidence || 'No examiner evidence recorded.'}</p>
						</li>
					{/each}
				</ul>
				<form onsubmit={reviewScenarioAssessmentAppeal}>
					<label class="field" for="scenario-appeal-decision">
						<span>Independent decision</span>
						<select id="scenario-appeal-decision" bind:value={appealDecision} disabled={appealQueueBusy}>
							<option value="confirmed">Confirm original assessment</option>
							<option value="reassessment_required">Require a new independent assessment</option>
						</select>
					</label>
					{#if appealDecision === 'reassessment_required'}
						<p class="muted">The original score remains visible and is flagged as unsuitable for consequential use pending a new independent assessment.</p>
					{/if}
					<label class="field" for="scenario-appeal-rationale">
						<span>Decision rationale</span>
						<textarea id="scenario-appeal-rationale" bind:value={appealRationale} minlength="10" maxlength="2000" rows="3" required disabled={appealQueueBusy} data-testid="scenario-appeal-rationale"></textarea>
					</label>
					<button class="btn primary" type="submit" disabled={appealQueueBusy || appealRationale.trim().length < 10} data-testid="scenario-appeal-submit">
						{appealQueueBusy ? 'Recording…' : 'Record independent decision'}
					</button>
				</form>
			</section>
		{/if}
	</section>

	<section class="card" aria-labelledby="content-rights-heading" data-testid="content-rights">
		<h2 id="content-rights-heading">Content rights</h2>
		<p class="muted">
			Record the permitted uses from the source agreement. This ledger records operator-supplied
			information; it does not verify or replace the underlying license.
		</p>
		<form onsubmit={createContentRight}>
			<label class="field" for="rights-ref-code">
				<span>Reference code</span>
				<input id="rights-ref-code" bind:value={rightsRefCode} maxlength="60" required />
			</label>
			<label class="field" for="rights-licensor">
				<span>Licensor</span>
				<input id="rights-licensor" bind:value={rightsLicensor} required />
			</label>
			<label class="field" for="rights-territory">
				<span>Territory</span>
				<input id="rights-territory" bind:value={rightsTerritory} placeholder="worldwide" />
			</label>
			<fieldset
				disabled={rightsBusy}
				style="display:flex; flex-wrap:wrap; gap:var(--space-md); border:1px solid var(--color-surface-elevated); border-radius:var(--radius-control); padding:var(--space-md); margin:var(--space-md) 0;"
			>
				<legend>Permitted uses</legend>
				{#each contentRightUses as use (use.value)}
					<label style="display:flex; align-items:center; gap:var(--space-sm);">
						<input
							type="checkbox"
							style="width:auto;"
							checked={rightsPermittedUses.includes(use.value)}
							onchange={(event) =>
								toggleContentRightUse(use.value, event.currentTarget.checked)}
						/>
						<span>{use.label}</span>
					</label>
				{/each}
			</fieldset>
			<label class="field" for="rights-valid-from">
				<span>Valid from</span>
				<input id="rights-valid-from" type="date" bind:value={rightsValidFrom} required />
			</label>
			<label class="field" for="rights-valid-to">
				<span>Valid through (optional)</span>
				<input
					id="rights-valid-to"
					type="date"
					bind:value={rightsValidTo}
					min={rightsValidFrom || undefined}
				/>
			</label>
			<label class="field" for="rights-notes">
				<span>Notes (optional)</span>
				<textarea id="rights-notes" bind:value={rightsNotes} rows="2"></textarea>
			</label>
			<details>
				<summary>Contract and scope terms</summary>
				<label class="field" for="rights-contract-ref">
					<span>Contract reference</span>
					<input id="rights-contract-ref" bind:value={rightsContractRef} maxlength="2000" />
				</label>
				<label class="field" for="rights-contract-version">
					<span>Contract version</span>
					<input id="rights-contract-version" bind:value={rightsContractVersion} maxlength="2000" />
				</label>
				<label class="field" for="rights-assets">
					<span>Covered asset references (one per line)</span>
					<textarea id="rights-assets" bind:value={rightsAssetRefs} rows="3"></textarea>
				</label>
				<label class="field" for="rights-audiences">
					<span>Permitted audiences (one per line)</span>
					<textarea id="rights-audiences" bind:value={rightsAudiences} rows="2"></textarea>
				</label>
				<label class="field" for="rights-seat-limit">
					<span>Seat limit (optional)</span>
					<input id="rights-seat-limit" type="number" min="1" step="1" bind:value={rightsSeatLimit} />
				</label>
				<label class="field" for="rights-offline-terms">
					<span>Offline-use terms</span>
					<textarea id="rights-offline-terms" bind:value={rightsOfflineTerms} maxlength="2000" rows="2"></textarea>
				</label>
				<label class="field" for="rights-quotation-limit">
					<span>Quotation limit in words</span>
					<input id="rights-quotation-limit" type="number" min="0" max="1000000" step="1" bind:value={rightsQuotationLimit} />
				</label>
				<label class="field" for="rights-ai-terms">
					<span>AI processing permissions</span>
					<textarea id="rights-ai-terms" bind:value={rightsAiTerms} maxlength="2000" rows="2"></textarea>
				</label>
				<label class="field" for="rights-derivative-terms">
					<span>Derivative terms</span>
					<textarea id="rights-derivative-terms" bind:value={rightsDerivativeTerms} maxlength="2000" rows="2"></textarea>
				</label>
				<label class="field" for="rights-attribution">
					<span>Required attribution</span>
					<textarea id="rights-attribution" bind:value={rightsAttribution} maxlength="2000" rows="2"></textarea>
				</label>
				<label class="field" for="rights-royalty-terms">
					<span>Royalty terms</span>
					<textarea id="rights-royalty-terms" bind:value={rightsRoyaltyTerms} maxlength="2000" rows="2"></textarea>
				</label>
			</details>
			<button
				class="btn primary"
				type="submit"
				disabled={rightsBusy || rightsPermittedUses.length === 0 || !rightsValidFrom}
				data-testid="content-rights-create"
			>
				{rightsBusy ? 'Saving…' : 'Record rights'}
			</button>
		</form>
		{#if rightsMessage}
			<p class="muted" role="status" data-testid="content-rights-message">{rightsMessage}</p>
		{/if}
		{#if rightsError}
			<p class="error-text" role="alert">{rightsError}</p>
			<button class="btn" type="button" disabled={rightsBusy} onclick={loadContentRights}>
				Retry loading rights
			</button>
		{/if}

		<h3>Recorded grants</h3>
		{#if rightsBusy && contentRights.length === 0}
			<p class="muted" role="status">Loading rights records…</p>
		{:else if contentRights.length === 0}
			<p class="muted">No content rights records yet.</p>
		{:else}
			<ul style="list-style:none; padding:0;">
				{#each contentRights as right (right.rights_id)}
					<li
						class="rights-record"
						data-testid={'content-right-' + right.rights_id}
					>
						<strong>{right.ref_code}</strong>
						<span class="chip">{right.status.charAt(0).toUpperCase() + right.status.slice(1)}</span>
						<p class="muted">{right.licensor} · {right.territory}</p>
							<p class="muted">Permitted: {right.permitted_uses.join(', ')}</p>
						<p class="muted">
							{right.valid_from}{right.valid_to ? ' – ' + right.valid_to : ' – no end date'}
						</p>
							{#if right.notes}
								<p class="muted">Notes: {right.notes}</p>
							{/if}
							{#if right.contract_ref || right.contract_version}
								<p class="muted">
									Contract: {right.contract_ref || 'reference not recorded'}
									{right.contract_version ? ' · ' + right.contract_version : ''}
								</p>
							{/if}
							{#if right.asset_refs.length > 0}
								<p class="muted">Assets: {right.asset_refs.join(', ')}</p>
							{/if}
							{#if right.audiences.length > 0}
								<p class="muted">Audiences: {right.audiences.join(', ')}</p>
							{/if}
							{#if right.seat_limit || right.quotation_limit_words !== null}
								<p class="muted">
									{right.seat_limit ? 'Seats: ' + right.seat_limit : ''}
									{right.quotation_limit_words !== null
										? ' · Quotation words: ' + right.quotation_limit_words
										: ''}
								</p>
							{/if}
							{#if right.offline_terms}<p class="muted">Offline: {right.offline_terms}</p>{/if}
							{#if right.ai_terms}<p class="muted">AI permissions: {right.ai_terms}</p>{/if}
							{#if right.derivative_terms}<p class="muted">Derivatives: {right.derivative_terms}</p>{/if}
							{#if right.attribution}<p class="muted">Attribution: {right.attribution}</p>{/if}
							{#if right.royalty_terms}<p class="muted">Royalties: {right.royalty_terms}</p>{/if}
						{#if right.revocation_note}
							<p class="muted">Revocation reason: {right.revocation_note}</p>
						{/if}
						{#if !right.revoked_at}
							<label class="field" for={'rights-revoke-' + right.rights_id}>
								<span>Reason to revoke {right.ref_code}</span>
								<input
									id={'rights-revoke-' + right.rights_id}
									bind:value={revocationReasons[right.rights_id]}
									maxlength="500"
									required
								/>
							</label>
							<button
								class="btn danger-text"
								type="button"
								disabled={rightsBusy || !!revokingRightsId || !revocationReasons[right.rights_id]?.trim()}
								onclick={() => revokeContentRight(right.rights_id)}
							>
								{revokingRightsId === right.rights_id ? 'Revoking…' : 'Revoke rights'}
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		<button class="btn" type="button" disabled={rightsBusy} onclick={loadContentRights}>
			Refresh rights
		</button>
	</section>

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
			<p class="muted" role="status">Loading extraction reports…</p>
		{:else if extractionReports.length === 0}
			<p class="muted">No extraction reports recorded.</p>
		{:else}
			<ul style="list-style:none; padding:0;">
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
							<fieldset disabled={extractionBusy} style="border:1px solid var(--color-surface-elevated); border-radius:var(--radius-control); padding:var(--space-md);">
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
				<fieldset disabled={mappingBusy} style="border:0; padding:0; margin:var(--space-md) 0;">
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

	<div class="card" data-testid="oidc-provider-settings">
		<h2>Institution single sign-on</h2>
		<p class="muted">
			Configure an OIDC provider for an institution. Learners must already have an exact issuer and
			subject enrollment; accounts are never linked by email.
		</p>
		<label class="field" for="oidc-institution-id">
			<span>Institution ID</span>
			<input id="oidc-institution-id" bind:value={oidcInstitutionId} data-testid="oidc-institution-id" />
		</label>
		<button
			class="btn"
			type="button"
			disabled={oidcBusy || !oidcInstitutionId.trim()}
			data-testid="oidc-load"
			onclick={loadOidcProvider}
		>
			{oidcBusy ? 'Loading…' : 'Load provider settings'}
		</button>
		{#if oidcMessage}<p class="muted" role="status">{oidcMessage}</p>{/if}
		{#if oidcError}<p class="error-text" role="alert">{oidcError}</p>{/if}
		{#if oidcLoaded}
			<form onsubmit={saveOidcProvider}>
				<label class="field" for="oidc-issuer">
					<span>Issuer URL</span>
					<input id="oidc-issuer" type="url" bind:value={oidcIssuer} required data-testid="oidc-issuer" />
				</label>
				<label class="field" for="oidc-client-id">
					<span>Client ID</span>
					<input id="oidc-client-id" bind:value={oidcClientId} required data-testid="oidc-client-id" />
				</label>
				<label class="field" for="oidc-client-secret">
					<span>Client secret (optional)</span>
					<input
						id="oidc-client-secret"
						type="password"
						autocomplete="new-password"
						bind:value={oidcClientSecret}
						data-testid="oidc-client-secret"
					/>
				</label>
			<p class="muted">
				{oidcSecretConfigured
					? 'A secret is stored. Leave this blank to keep it.'
					: 'No secret is stored. A public client can use PKCE.'}
			</p>
			{#if oidcSecretConfigured}
				<button
					class="linklike"
					type="button"
					disabled={oidcBusy || !!oidcClientSecret}
					onclick={() => (oidcClearSecret = !oidcClearSecret)}
				>
					{oidcClearSecret ? 'Keep stored secret' : 'Clear stored secret'}
				</button>
			{/if}
			<label class="field" for="oidc-enabled">
				<span>Sign-in status</span>
				<select id="oidc-enabled" bind:value={oidcEnabled} data-testid="oidc-enabled">
					<option value="false">Disabled</option>
					<option value="true">Enabled</option>
				</select>
			</label>
			<button
				class="btn primary"
				type="submit"
				disabled={oidcBusy || !oidcInstitutionId.trim() || !oidcIssuer.trim() || !oidcClientId.trim()}
				data-loading={oidcBusy}
				data-testid="oidc-save"
			>
				{oidcBusy ? 'Saving…' : 'Save provider settings'}
			</button>
			{#if oidcSignInUrl}
				<p class="muted" role="status">
					<a href={oidcSignInUrl}>Open learner sign-in</a>
				</p>
			{/if}
			</form>
		{/if}
	</div>

	<div class="card">
		<h2>Bulk question import</h2>
		<p class="muted" style="font-size: var(--text-sm);">
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
		<p id="import-file-help" class="muted" style="font-size: var(--text-sm);">
			{importFile ? importFile.name : 'CSV must be UTF-8. XLSX formulas are not accepted. Maximum file size: 3 MiB.'}
		</p>
		<div style="display:flex; gap:12px; flex-wrap:wrap; margin-bottom:var(--space-lg);">
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
			<p class="muted" style="font-size: var(--text-sm);">
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
		</details>
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

	<div class="card" data-testid="report-queue">
		<div style="display:flex; align-items:center; justify-content:space-between; gap:12px; flex-wrap:wrap;">
			<div>
				<h2>Question reports</h2>
				<p class="muted" style="margin:0;">Receipt is acknowledged immediately. Resolve within 72 hours; marking a report fixed requires a newer published question version.</p>
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
			<article class="card" data-testid="report-item" style="margin-top:var(--space-md);">
				<div style="display:flex; align-items:center; justify-content:space-between; gap:12px; flex-wrap:wrap;">
					<strong>Version {report.version} · {report.category.replaceAll('_', ' ')} · {report.report_count} report{report.report_count === 1 ? '' : 's'}</strong>
					<span class="chip">{report.quarantined ? 'Quarantined' : 'In review'}</span>
				</div>
				<p style="margin:var(--space-sm) 0;"><strong>{report.vignette}</strong><br />{report.lead_in}</p>
				<div aria-label="Learner report details">
					{#each report.reporter_feedback as feedback}
						<p class="muted" style="margin:4px 0;">
							<strong>{feedback.category.replaceAll('_', ' ')}:</strong>
							{feedback.note || 'No additional details provided.'}
						</p>
					{/each}
					{#if report.feedback_truncated}
						<p class="muted" style="margin:4px 0;">Showing the first 20 of {report.report_count} reports.</p>
					{/if}
				</div>
				<p class="muted" style="font-size:var(--text-sm);">
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
				<div style="display:flex; gap:8px; flex-wrap:wrap;">
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
	.runtime-settings-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr));
		gap: var(--space-md);
	}

	.rights-record,
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

	.scenario-assessment-list {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.scenario-assessment-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		padding: var(--space-md) 0;
		border-bottom: 1px solid var(--color-surface-elevated);
	}

	.scenario-review {
		min-width: 0;
		margin-top: var(--space-lg);
		padding-top: var(--space-lg);
		border-top: 1px solid var(--color-surface-elevated);
	}

	.scenario-transcript {
		padding-left: var(--space-lg);
	}

	.scenario-transcript li {
		display: grid;
		gap: var(--space-xs);
		margin: var(--space-sm) 0;
	}

	.scenario-assessment-criterion {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}
</style>
