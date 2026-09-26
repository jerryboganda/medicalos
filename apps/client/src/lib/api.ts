import { auth, clearToken } from './auth.svelte';
import type { AdminDashboard } from './generated/admin/AdminDashboard';
import type { AdminArticleListResponse } from './generated/library/AdminArticleListResponse';
import type { AdminArticleVersion } from './generated/library/AdminArticleVersion';
import type { CreateArticleRequest } from './generated/library/CreateArticleRequest';
import type { UpdateArticleDraftRequest } from './generated/library/UpdateArticleDraftRequest';
import type { PublishArticleResponse } from './generated/library/PublishArticleResponse';
import type { LibraryArticleResponse } from './generated/library/LibraryArticleResponse';
import type { LibrarySearchResponse } from './generated/library/LibrarySearchResponse';
import type { PrivateDocumentSearchResult } from './generated/library/PrivateDocumentSearchResult';
import type { AdminImageAnnotationListResponse } from './generated/image/AdminImageAnnotationListResponse';
import type { ImageAnnotationCreatedResponse } from './generated/image/ImageAnnotationCreatedResponse';
import type { ImageAnnotationReviewResponse } from './generated/image/ImageAnnotationReviewResponse';
import type { ImageCaseConceptsResponse } from './generated/image/ImageCaseConceptsResponse';
import type { ImageCaseDetail } from './generated/image/ImageCaseDetail';
import type { ImageCaseListResponse } from './generated/image/ImageCaseListResponse';
import type { ScenarioHandover } from './generated/scenario/ScenarioHandover';
import type { ScenarioHandoverAcknowledgedResponse } from './generated/scenario/ScenarioHandoverAcknowledgedResponse';
import type { ScenarioHandoverCreatedResponse } from './generated/scenario/ScenarioHandoverCreatedResponse';
import type { ScenarioHandoversResponse } from './generated/scenario/ScenarioHandoversResponse';
import type { ScenarioCounterfactualReplay } from './generated/scenario/ScenarioCounterfactualReplay';
import type { ScenarioAssessmentAppeal } from './generated/scenario/ScenarioAssessmentAppeal';
import type { ScenarioAssessmentAppealCreatedResponse } from './generated/scenario/ScenarioAssessmentAppealCreatedResponse';
import type { ScenarioAssessmentAppealQueueResponse } from './generated/scenario/ScenarioAssessmentAppealQueueResponse';
import type { ScenarioAssessmentAppealRequest } from './generated/scenario/ScenarioAssessmentAppealRequest';
import type { ScenarioAssessmentAppealReviewRequest } from './generated/scenario/ScenarioAssessmentAppealReviewRequest';
import type { ScenarioAssessmentAppealReviewResponse } from './generated/scenario/ScenarioAssessmentAppealReviewResponse';
import type { ScenarioDebrief } from './generated/scenario/ScenarioDebrief';
import type { ScenarioEventResponse } from './generated/scenario/ScenarioEventResponse';
import type { ScenarioListResponse } from './generated/scenario/ScenarioListResponse';
import type { ScenarioRubricResult } from './generated/scenario/ScenarioRubricResult';
import type { ScenarioRun } from './generated/scenario/ScenarioRun';
import type { ScenarioStartResponse } from './generated/scenario/ScenarioStartResponse';
import type { ScenarioTimelineEvent } from './generated/scenario/ScenarioTimelineEvent';
import type { ScenarioTeam } from './generated/scenario/ScenarioTeam';
import type { ScenarioTeamInviteCreatedResponse } from './generated/scenario/ScenarioTeamInviteCreatedResponse';
import type { ScenarioTeamJoinResponse } from './generated/scenario/ScenarioTeamJoinResponse';
import type { ScenarioTeamRole } from './generated/scenario/ScenarioTeamRole';
import type { Today } from './generated/today/Today';
import type { PackLeaseListResponse } from './generated/packs/PackLeaseListResponse';
import type { PackLeaseRequest } from './generated/packs/PackLeaseRequest';
import type { PackLeaseResponse } from './generated/packs/PackLeaseResponse';
import type { PackManifest } from './generated/packs/PackManifest';
import type { PackQuestionResource } from './generated/packs/PackQuestionResource';
import type { PackResourcesRequest } from './generated/packs/PackResourcesRequest';
import type { PackResourcesResponse } from './generated/packs/PackResourcesResponse';
import type { TutoringCard } from './generated/packs/TutoringCard';

// Keep transport and authentication here. Data contracts are exported from
// Rust DTOs incrementally as ARCH-02 bindings are generated.
// Production reaches the API same-origin through the /api/ prefix
// (medicalos.polytronx.com/api/* -> the API container); local dev keeps
// talking straight to the API host.
export type { Today } from './generated/today/Today';
export type { TodayTask } from './generated/today/TodayTask';
export type { TodayRevision } from './generated/today/TodayRevision';
export type { LearnerChapter } from './generated/today/LearnerChapter';
export type { TodayRevisionBudget } from './generated/today/TodayRevisionBudget';
export type { AdminDashboard } from './generated/admin/AdminDashboard';
export type { AdminArticleListResponse } from './generated/library/AdminArticleListResponse';
export type { AdminArticleSummary } from './generated/library/AdminArticleSummary';
export type { AdminArticleVersion } from './generated/library/AdminArticleVersion';
export type { ArticleCitation } from './generated/library/ArticleCitation';
export type { CreateArticleRequest } from './generated/library/CreateArticleRequest';
export type { LibraryArticleResponse } from './generated/library/LibraryArticleResponse';
export type { LibrarySearchResponse } from './generated/library/LibrarySearchResponse';
export type { LibrarySearchResult } from './generated/library/LibrarySearchResult';
export type { UpdateArticleDraftRequest } from './generated/library/UpdateArticleDraftRequest';
export type { PublishArticleResponse } from './generated/library/PublishArticleResponse';
export type { PrivateDocumentSearchResult } from './generated/library/PrivateDocumentSearchResult';
export type { ArticleMedia } from './generated/library/ArticleMedia';
export type { MediaCaptionCue } from './generated/library/MediaCaptionCue';
export type { MediaChapterMarker } from './generated/library/MediaChapterMarker';
export type { ImageConceptLink } from './generated/image/ImageConceptLink';
export type { AdminImageAnnotation } from './generated/image/AdminImageAnnotation';
export type { AdminImageAnnotationListResponse } from './generated/image/AdminImageAnnotationListResponse';
export type { ImageAnnotationCreatedResponse } from './generated/image/ImageAnnotationCreatedResponse';
export type { ImageAnnotationDecision } from './generated/image/ImageAnnotationDecision';
export type { ImageAnnotationReviewResponse } from './generated/image/ImageAnnotationReviewResponse';
export type { ImageCaseAnnotation } from './generated/image/ImageCaseAnnotation';
export type { ImageCaseConceptsResponse } from './generated/image/ImageCaseConceptsResponse';
export type { ImageCaseDetail } from './generated/image/ImageCaseDetail';
export type { ImageCaseKind } from './generated/image/ImageCaseKind';
export type { ImageCaseListResponse } from './generated/image/ImageCaseListResponse';
export type { ImageCaseSummary } from './generated/image/ImageCaseSummary';
export type { ImageFinding } from './generated/image/ImageFinding';
export type { ImageRef } from './generated/image/ImageRef';
export type { PendingImageAnnotationStatus } from './generated/image/PendingImageAnnotationStatus';
export type { ScenarioListResponse } from './generated/scenario/ScenarioListResponse';
export type { ScenarioAppealSummary } from './generated/scenario/ScenarioAppealSummary';
export type { ScenarioAppealDecision } from './generated/scenario/ScenarioAppealDecision';
export type { ScenarioAppealStatus } from './generated/scenario/ScenarioAppealStatus';
export type { ScenarioAssessmentAppeal } from './generated/scenario/ScenarioAssessmentAppeal';
export type { ScenarioAssessmentAppealCreatedResponse } from './generated/scenario/ScenarioAssessmentAppealCreatedResponse';
export type { ScenarioAssessmentAppealQueueItem } from './generated/scenario/ScenarioAssessmentAppealQueueItem';
export type { ScenarioAssessmentAppealQueueResponse } from './generated/scenario/ScenarioAssessmentAppealQueueResponse';
export type { ScenarioAssessmentAppealRequest } from './generated/scenario/ScenarioAssessmentAppealRequest';
export type { ScenarioAssessmentAppealReviewRequest } from './generated/scenario/ScenarioAssessmentAppealReviewRequest';
export type { ScenarioAssessmentAppealReviewResponse } from './generated/scenario/ScenarioAssessmentAppealReviewResponse';
export type { ScenarioCounterfactualEvent } from './generated/scenario/ScenarioCounterfactualEvent';
export type { ScenarioCounterfactualReplay } from './generated/scenario/ScenarioCounterfactualReplay';
export type { ScenarioConsequentialUseStatus } from './generated/scenario/ScenarioConsequentialUseStatus';
export type { ScenarioDebrief } from './generated/scenario/ScenarioDebrief';
export type { ScenarioEventResponse } from './generated/scenario/ScenarioEventResponse';
export type { ScenarioRubricResult } from './generated/scenario/ScenarioRubricResult';
export type { ScenarioRubricStatus } from './generated/scenario/ScenarioRubricStatus';
export type { ScenarioRun } from './generated/scenario/ScenarioRun';
export type { ScenarioStartResponse } from './generated/scenario/ScenarioStartResponse';
export type { ScenarioTimelineEvent } from './generated/scenario/ScenarioTimelineEvent';
export type { ScenarioTranscriptCorrection } from './generated/scenario/ScenarioTranscriptCorrection';
export type { ScenarioSummary } from './generated/scenario/ScenarioSummary';
export type { ScenarioHandover } from './generated/scenario/ScenarioHandover';
export type { ScenarioHandoverAcknowledgedResponse } from './generated/scenario/ScenarioHandoverAcknowledgedResponse';
export type { ScenarioHandoverCreatedResponse } from './generated/scenario/ScenarioHandoverCreatedResponse';
export type { ScenarioHandoversResponse } from './generated/scenario/ScenarioHandoversResponse';
export type { ScenarioTeam } from './generated/scenario/ScenarioTeam';
export type { ScenarioTeamInviteCreatedResponse } from './generated/scenario/ScenarioTeamInviteCreatedResponse';
export type { ScenarioTeamJoinResponse } from './generated/scenario/ScenarioTeamJoinResponse';
export type { ScenarioTeamMember } from './generated/scenario/ScenarioTeamMember';
export type { ScenarioTeamRole } from './generated/scenario/ScenarioTeamRole';
export type { PackLeaseListResponse } from './generated/packs/PackLeaseListResponse';
export type { PackLeaseRequest } from './generated/packs/PackLeaseRequest';
export type { PackLeaseResponse } from './generated/packs/PackLeaseResponse';
export type { PackLeaseSummary } from './generated/packs/PackLeaseSummary';
export type { PackManifest } from './generated/packs/PackManifest';
export type { PackManifestItem } from './generated/packs/PackManifestItem';
export type { PackQuestionResource } from './generated/packs/PackQuestionResource';
export type { PackResourcesRequest } from './generated/packs/PackResourcesRequest';
export type { PackResourcesResponse } from './generated/packs/PackResourcesResponse';
export type { QuestionOption } from './generated/packs/QuestionOption';
export type { TutoringCard } from './generated/packs/TutoringCard';
const BASE: string =
	import.meta.env.VITE_API_BASE ?? (import.meta.env.PROD ? '/api' : '');

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

export function apiErrorMessage(value: unknown, fallback: string): string {
	return value instanceof Error ? value.message : fallback;
}

export function adminToken(): string {
	try {
		return localStorage.getItem('mlos_admin') ?? '';
	} catch {
		return '';
	}
}

async function call<T>(
	method: string,
	path: string,
	body?: unknown,
	contentType = 'application/json'
): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers: {
			'content-type': contentType,
			'x-admin-token': adminToken(),
			...(auth.token ? { authorization: `Bearer ${auth.token}` } : {})
		},
		body:
			body === undefined
				? undefined
				: contentType === 'application/json'
					? JSON.stringify(body)
					: (body as BodyInit)
	});
	if (!res.ok) {
		if (res.status === 401) {
			clearToken();
		}
		let code = 'error';
		let message = `Request failed (${res.status})`;
		try {
			const parsed = await res.json();
			code = parsed?.error?.code ?? code;
			message = parsed?.error?.message ?? message;
		} catch {
			/* non-json error body */
		}
		throw new ApiError(res.status, code, message);
	}
	return (await res.json()) as T;
}

export interface AdminImportResult {
	batch_id: string;
	status: string;
	rows: number;
	valid?: number;
	issues?: { row: number; code: string; message: string }[];
	created?: unknown[];
}

export interface SessionItem {
	item_index: number;
	question_version_id: string;
	vignette: string;
	lead_in: string;
	difficulty: string;
	options: { text: string; rationale?: string }[];
	answered: boolean;
	chosen_index: number | null;
	correct: boolean | null;
	correct_index: number | null;
	key_learning_point: string | null;
	exam_tip: string | null;
	hint_available?: boolean;
	/** QB-08: honest flag state — null when unflagged. */
	report_status: 'open' | 'quarantined' | 'resolved_fixed' | 'resolved_rejected' | null;
	corrected_version_id?: string | null;
	/** This is a newly published version created to correct an earlier report. */
	corrected?: boolean;
	correction_note?: string | null;
	my_report?: {
		status: string;
		resolution_note: string | null;
		correction_note: string | null;
		resolved_at: string | null;
		corrected_version_id: string | null;
		corrected_version_number: number | null;
		acknowledged_at: string;
		acknowledgement_due_at: string;
		resolution_due_at: string;
		resolution_overdue: boolean;
	} | null;
	tutoring_cards?: TutoringCard[];
}

export interface RecommendedAction {
	task_id: string;
	task_key: string;
	kind: string;
	title: string;
	chapter_id: string | null;
	source_session_id: string | null;
	question_count: number;
	estimated_minutes: number;
	adjusted_estimated_minutes: number;
	protected: boolean;
	reason_code: string;
	independent_count: number;
}

export interface NextActionRecommendation {
	available_minutes: number;
	activity_preference: 'any' | 'practice' | 'revision';
	time_multiplier: number;
	exam_date: string | null;
	plan_id: string | null;
	plan_version: number | null;
	reason_code: string;
	recommended_action: RecommendedAction | null;
	allowance?: {
		limit: number;
		used: number;
		remaining: number;
		required?: number;
	} | null;
}

export interface PrivateImportRight {
	rights_id: string;
	ref_code: string;
	licensor: string;
	valid_to: string | null;
	search_allowed: boolean;
}

export interface PrivateImportSummary {
	document_id: string;
	title: string;
	media_type: 'text/plain' | 'text/markdown';
	rights_ref: string;
	sha256: string;
	created_at: string;
	available: boolean;
}

export interface PrivateImport extends PrivateImportSummary {
	content: string;
}

export interface AdminContentRight {
	rights_id: string;
	ref_code: string;
	licensor: string;
	territory: string;
	permitted_uses: string[];
	valid_from: string;
	valid_to: string | null;
	notes: string | null;
	revoked_at: string | null;
	revoked_by: string | null;
	revocation_note: string | null;
	contract_ref: string | null;
	contract_version: string | null;
	asset_refs: string[];
	audiences: string[];
	seat_limit: number | null;
	offline_terms: string | null;
	quotation_limit_words: number | null;
	ai_terms: string | null;
	derivative_terms: string | null;
	attribution: string | null;
	royalty_terms: string | null;
	status: 'active' | 'scheduled' | 'expired' | 'revoked';
}

export interface AdminExtractionReport {
	report_id: string;
	source_label: string;
	source_sha256: string;
	media_type: string;
	parser_version: string;
	rights_ref: string;
	rights_available: boolean;
	malware_scan_status: 'clean' | 'blocked' | 'not_scanned';
	expected_regions: string[];
	extracted_regions: string[];
	missing_regions: string[];
	uncertain_regions: string[];
	critical_regions: string[];
	status: 'blocked' | 'incomplete' | 'review_required' | 'rejected' | 'complete' | 'rights_unavailable';
	created_by: string | null;
	created_at: string;
	review: {
		decision: 'approved' | 'rejected';
		reviewer_id: string | null;
		verified_regions: string[] | null;
		note: string | null;
		reviewed_at: string | null;
	} | null;
}

export interface PendingScenarioAssessment {
	run_id: string;
	scenario: string;
	scenario_version: number;
	finished_at: string;
	criterion_count: number;
}

export interface AdminScenarioAssessment {
	run_id: string;
	scenario: string;
	scenario_version: number;
	transcript: { from: string; on: string; to: string; actor_role?: ScenarioTeamRole | null }[];
	started_at: string;
	finished_at: string;
	rubric: ScenarioRubricResult[];
}

export interface AdminConcept {
	concept_id: string;
	canonical_key: string;
	current_version: number;
	display_name: string;
	definition: string;
}

export interface AdminCurriculumNode {
	id: string;
	kind: string;
	name: string;
	parent_id: string | null;
	display_order: number;
	status: string;
}

export interface AdminReport {
	report_id: string;
	question_version_id: string;
	question_id: string;
	version: number;
	vignette: string;
	lead_in: string;
	category: string;
	reporter_feedback: { category: string; note: string }[];
	feedback_truncated: boolean;
	report_count: number;
	first_reported_at: string;
	acknowledgement_due_at: string;
	resolution_due_at: string;
	acknowledgements_on_time: boolean;
	resolution_overdue: boolean;
	quarantined: boolean;
}

export interface PracticeSession {
	session_id: string;
	preset: string;
	status: string;
	time_limit_seconds: number | null;
	/** EX-08: server-issued; the countdown derives from deadline − server_now. */
	deadline: string | null;
	server_now: string | null;
	items: SessionItem[];
}

export interface AnswerResult {
	already_recorded: boolean;
	correct: boolean | null;
	correct_index: number;
	options: { text: string; rationale: string }[];
	key_learning_point: string;
	exam_tip: string | null;
	tutoring_cards?: TutoringCard[];
}

export interface SubmitResult {
	total: number;
	correct: number;
	incorrect: number;
	skipped: number;
	score: number;
	expected_score: number | null;
	mock: MockResult | null;
}

export interface Note {
	note_id: string;
	title: string;
	body: string;
	source_question_version_id: string | null;
	updated_at: string;
	backlinks: { note_id: string; title: string }[];
}

export interface MockTest {
	mock_id: string;
	title: string;
	pass_mark_percent: number;
	attempts_allowed: number;
	attempts_used: number;
	time_limit_seconds: number | null;
	late_sync_grace_seconds: number;
	integrity_policy: 'log_only' | 'warn' | 'auto_submit';
	away_timeout_seconds: number | null;
}

export interface MockResult {
	score_percent: number;
	passed: boolean;
	pass_mark_percent: number;
	percentile: number | null;
	takers: number;
	ranked: boolean;
	late_sync_answers: number;
	breakdown: { chapter: string; total: number; correct: number }[];
}

export interface EngagementQotd {
	enabled: boolean;
	exam_id?: string | null;
	needs_exam_selection?: boolean;
	answered?: boolean;
	available?: boolean;
	question_version_id?: string;
	vignette?: string;
	options?: { text: string }[];
	community_split?: { chosen_index: number; count: number }[];
	community_total?: number;
}

export interface Engagement {
	enabled: boolean;
	available_minutes: number;
	daily_goal: {
		enabled: boolean;
		mode: 'questions' | 'minutes';
		unit: 'questions' | 'minutes';
		target: number;
		answered_today: number;
		minutes_today: number;
		met: boolean;
	};
	streak: { enabled: boolean; count: number; freezes: number };
	qotd: EngagementQotd;
}

export interface EngagementSettings {
	daily_goal_questions?: number;
	daily_goal_mode?: 'questions' | 'minutes';
	available_minutes?: number;
	daily_goal_enabled?: boolean;
	streak_enabled?: boolean;
	qotd_enabled?: boolean;
	qotd_exam_id?: string | null;
}

export interface OidcProviderView {
	issuer: string;
	client_id: string;
	enabled: boolean;
	client_secret_configured: boolean;
}

export interface OidcProviderUpdate {
	issuer: string;
	client_id: string;
	client_secret?: string;
	clear_client_secret?: boolean;
	enabled: boolean;
}

export interface AdminSettings {
	mastery_bands: number[];
	community_min_sample: number;
	free_daily_questions: number;
	free_daily_coach_turns: number;
	retest_intervals_days: number[];
	offline_lease_days: number;
	max_reviews_per_day: number;
	max_new_cards_per_day: number;
	competition_difficulty_points: number[];
}

export interface CompetitionSummary {
	competition_id: string;
	title: string;
	exam_id: string;
	exam: string;
	cadence: 'one_off' | 'daily' | 'weekly' | 'monthly' | 'live';
	series_id: string | null;
	starts_at: string;
	ends_at: string;
	status: string;
	entered: boolean;
	attempt_status: 'in_progress' | 'submitted' | null;
}

export interface CompetitionLeagueStanding {
	rank: number;
	handle: string;
	points: number;
	accuracy: number;
	total_time_ms: number;
	is_me: boolean;
}

export type CompetitionLeagueState =
	| { joined: false }
	| {
			joined: true;
			exam_id: string;
			cohort_id: string;
			week_start: string;
			week_end: string;
			division: number;
			cohort_number: number;
			standings: CompetitionLeagueStanding[];
	  };

export interface CompetitionQuestion {
	question_version_id: string;
	question_number: number;
	total_questions: number;
	vignette: string;
	lead_in: string;
	options: { text: string }[];
}

export type CompetitionAttemptStep =
	| { attempt_id: string; submitted: false; question: CompetitionQuestion }
	| {
			attempt_id: string;
			submitted: true;
			entry_id: string;
			score: number;
			questions: number;
			total_time_ms: number;
		};

export interface CompetitionLeaderboard {
	prize_reviewed: boolean;
	status: string;
	entries: {
		rank: number;
		handle: string;
		score: number;
		accuracy: number;
		questions_attempted: number;
		average_response_time_ms: number;
		total_time_ms: number;
		is_me: boolean;
		prize_eligible: boolean;
	}[];
}

export const Api = {
	register: (email: string, password: string) =>
		call<{ user_id: string }>('POST', '/v1/auth/register', { email, password }),
	login: (email: string, password: string) =>
		call<{ token: string }>('POST', '/v1/auth/login', { email, password }),
	getInstitutionOidc: (institutionId: string) =>
		call<OidcProviderView>(
			'GET',
			`/v1/admin/institutions/${encodeURIComponent(institutionId)}/sso/oidc`
		),
	configureInstitutionOidc: (institutionId: string, body: OidcProviderUpdate) =>
		call<OidcProviderView>(
			'PUT',
			`/v1/admin/institutions/${encodeURIComponent(institutionId)}/sso/oidc`,
			body
		),
	startInstitutionSso: (institutionId: string) =>
		call<{ authorization_url: string }>(
			'GET',
			`/v1/institutions/${encodeURIComponent(institutionId)}/sso/oidc/start`
		),
	completeInstitutionSso: (ticket: string) =>
		call<{ token: string }>('POST', '/v1/auth/oidc/complete', { ticket }),
	today: () => call<Today>('GET', '/v1/me/today'),
	recommendNextAction: (
		availableMinutes: number,
		activityPreference: 'any' | 'practice' | 'revision' = 'any',
		timeMultiplier = 1
	) => {
		const query = new URLSearchParams({
			available_minutes: String(availableMinutes),
			activity_preference: activityPreference,
			time_multiplier: String(timeMultiplier)
		});
		return call<NextActionRecommendation>('GET', `/v1/me/plan/next-action?${query}`);
	},
	protectPlanTask: (planId: string, taskId: string, isProtected: boolean) =>
		call<{ task_id: string; protected: boolean }>(
			'PUT',
			`/v1/plans/${planId}/tasks/${taskId}/protection`,
			{ protected: isProtected }
		),
	replan: (dailyMinutes: number, expectedVersion: number) =>
		call<{
			replanned: boolean;
			version?: number;
			deferred_tasks?: number;
			deferred_task_ids?: string[];
		}>(
			'POST',
			'/v1/me/plan/replan',
			{ daily_minutes: dailyMinutes, expected_version: expectedVersion }
		),
	engagement: () => call<Engagement>('GET', '/v1/me/engagement'),
	updateEngagementSettings: (body: EngagementSettings) =>
		call<{
			daily_goal_questions: number;
		daily_goal_mode: 'questions' | 'minutes';
		available_minutes: number;
			daily_goal_enabled: boolean;
			streak_enabled: boolean;
			qotd_enabled: boolean;
			freezes: number;
			qotd_exam_id: string | null;
		}>('PUT', '/v1/me/engagement/settings', body),
	answerQotd: (questionVersionId: string, chosenIndex: number, elapsedMs = 0) =>
		call<{
			correct: boolean;
			correct_index: number;
			community_split: { chosen_index: number; count: number }[];
			community_total: number;
		}>('POST', '/v1/me/qotd/answers', {
			question_version_id: questionVersionId,
			chosen_index: chosenIndex,
			elapsed_ms: elapsedMs
		}),
	createSession: (body: {
		preset: string;
		chapter_id?: string;
		chapter_ids?: string[];
		blueprint?: { chapter_id: string; count: number }[];
		source?: 'any' | 'unseen' | 'incorrect' | 'marked';
		question_count?: number;
		source_session_id?: string;
		plan_task_key?: string;
		time_limit_seconds?: number;
	}) => call<{ session_id: string }>('POST', '/v1/practice/sessions', body),
	createPackLease: (body: PackLeaseRequest) =>
		call<PackLeaseResponse>('POST', '/v1/packs/lease', body),
	packManifest: (examId: string, chapters: string[], deviceId: string) => {
		const query = new URLSearchParams({ chapters: chapters.join(','), device_id: deviceId });
		return call<PackManifest>(
			'GET',
			`/v2/packs/${encodeURIComponent(examId)}/manifest?${query.toString()}`
		);
	},
	packResources: (
		examId: string,
		body: PackResourcesRequest
	) =>
		call<PackResourcesResponse>(
			'POST',
			`/v2/packs/${encodeURIComponent(examId)}/resources`,
			body
		),
	revokePackLease: (leaseId: string) =>
		call<{ revoked: boolean }>('DELETE', `/v1/packs/lease/${encodeURIComponent(leaseId)}`),
	listPackLeases: () => call<PackLeaseListResponse>('GET', '/v1/me/packs'),
	listScenarios: () => call<ScenarioListResponse>('GET', '/v1/scenarios'),
	imageCases: () => call<ImageCaseListResponse>('GET', '/v1/me/image-cases'),
	imageCase: (caseId: string) =>
		call<ImageCaseDetail>('GET', `/v1/me/image-cases/${encodeURIComponent(caseId)}`),
	adminImageCaseConcepts: (caseId: string) =>
		call<ImageCaseConceptsResponse>(
			'GET',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/concepts`
		),
	setAdminImageCaseConcepts: (caseId: string, conceptIds: string[]) =>
		call<ImageCaseConceptsResponse>(
			'PUT',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/concepts`,
			{ concept_ids: conceptIds }
		),
	adminImageAnnotations: () =>
		call<AdminImageAnnotationListResponse>('GET', '/v1/admin/image-annotations'),
	createImageAnnotation: (
		caseId: string,
		body: { image_index: number; x_percent: number; y_percent: number; body: string }
	) =>
		call<ImageAnnotationCreatedResponse>(
			'POST',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/annotations`,
			body
		),
	reviewImageAnnotation: (
		annotationId: string,
		decision: 'approved' | 'rejected',
		note: string
	) =>
		call<ImageAnnotationReviewResponse>(
			'POST',
			`/v1/admin/image-annotations/${encodeURIComponent(annotationId)}/review`,
			{ decision, note }
		),
	startScenario: (scenarioSlug: string) =>
		call<ScenarioStartResponse>('POST', '/v1/scenarios/runs', { scenario_slug: scenarioSlug }),
	getScenarioRun: (runId: string) =>
		call<ScenarioRun>('GET', `/v1/scenarios/runs/${encodeURIComponent(runId)}`),
	getScenarioTeam: (runId: string) =>
		call<ScenarioTeam>('GET', `/v1/scenarios/runs/${encodeURIComponent(runId)}/team`),
	createScenarioTeamInvite: (runId: string, role: Exclude<ScenarioTeamRole, 'team_lead'>) =>
		call<ScenarioTeamInviteCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/team/invites`,
			{ role }
		),
	joinScenarioTeam: (inviteCode: string) =>
		call<ScenarioTeamJoinResponse>(
			'POST',
			'/v1/scenario-team-invites/join',
			{ invite_code: inviteCode }
		),
	listScenarioHandovers: (runId: string) =>
		call<ScenarioHandoversResponse>(
			'GET',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers`
		),
	createScenarioHandover: (
		runId: string,
		body: Pick<ScenarioHandover, 'situation' | 'background' | 'assessment' | 'recommendation'> & {
			recipient_member_id: string;
		}
	) =>
		call<ScenarioHandoverCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers`,
			body
		),
	acknowledgeScenarioHandover: (runId: string, handoverId: string) =>
		call<ScenarioHandoverAcknowledgedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers/${encodeURIComponent(handoverId)}/ack`
		),
	advanceScenario: (runId: string, event: string) =>
		call<ScenarioEventResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/events`,
			{ event }
		),
	getScenarioDebrief: (runId: string) =>
		call<ScenarioDebrief>(
			'GET',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/debrief`
		),
	appealScenarioAssessment: (runId: string, body: ScenarioAssessmentAppealRequest) =>
		call<ScenarioAssessmentAppealCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/appeals`,
			body
		),
	replayScenario: (runId: string, events: string[]) =>
		call<ScenarioCounterfactualReplay>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/counterfactual`,
			{ events }
		),
	listScenarioAssessmentAppeals: () =>
		call<ScenarioAssessmentAppealQueueResponse>(
			'GET',
			'/v1/admin/scenario-assessment-appeals'
		),
	getScenarioAssessmentAppeal: (appealId: string) =>
		call<ScenarioAssessmentAppeal>(
			'GET',
			`/v1/admin/scenario-assessment-appeals/${encodeURIComponent(appealId)}`
		),
	reviewScenarioAssessmentAppeal: (
		appealId: string,
		body: ScenarioAssessmentAppealReviewRequest
	) =>
		call<ScenarioAssessmentAppealReviewResponse>(
			'POST',
			`/v1/admin/scenario-assessment-appeals/${encodeURIComponent(appealId)}/review`,
			body
		),
	getSession: (sid: string) => call<PracticeSession>('GET', `/v1/practice/sessions/${sid}`),
	recordIntegrityEvent: (body: {
		session_id: string;
		signal_type:
			| 'background'
			| 'foreground'
			| 'window_blur'
			| 'fullscreen_exit'
			| 'clock_change';
		detail?: Record<string, number>;
		client_time: string;
	}) =>
		call<{
			recorded: true;
			event_id: string;
			action: 'none' | 'warn' | 'auto_submitted';
			away_seconds?: number;
			receipt?: SubmitResult;
		}>('POST', '/v1/integrity-events', body),
	getSessionHint: (sid: string, itemIndex: number) =>
		call<{ hint: string; assisted: true }>(
			'GET',
			`/v1/practice/sessions/${sid}/items/${itemIndex}/hint`
		),
	calculate: (kind: string, inputs: Record<string, number | boolean>) =>
		call<{ calculator: string; value: number; unit: string; disclaimer: string }>(
			'POST',
			`/v1/calculators/${encodeURIComponent(kind)}`,
			inputs
		),
	convertUnits: (body: { value: number; analyte?: string; from: string; to: string }) =>
		call<{ value: number; unit: string; disclaimer: string }>(
			'POST',
			'/v1/calculators/convert',
			body
		),
	answer: (
		sid: string,
		body: {
			item_index: number;
			chosen_index: number | null;
			idempotency_key: string;
			elapsed_ms?: number;
			assisted?: boolean;
			client_recorded_at?: string;
		}
	) => call<AnswerResult>('POST', `/v1/practice/sessions/${sid}/answers`, body),
	submit: (sid: string) => call<SubmitResult>('POST', `/v1/practice/sessions/${sid}/submit`),
	undo: (planId: string, revisionId: string) =>
		call<{ plan_version: number }>(
			'POST',
			`/v1/plans/${planId}/revisions/${revisionId}/undo`
		),
	reportQuestion: (versionId: string, body: { category: string; note?: string }) =>
		call<{
			report_id: string;
			already_recorded: boolean;
			quarantined: boolean;
			status: 'open' | 'quarantined' | 'resolved_fixed' | 'resolved_rejected';
			created_at: string;
			acknowledged_at: string;
			acknowledgement_due_at: string;
			resolution_due_at: string;
			resolution_note: string | null;
			resolved_at: string | null;
			corrected_version_id: string | null;
			corrected_version_number: number | null;
			correction_note: string | null;
		}>(
			'POST',
			`/v1/questions/versions/${versionId}/reports`,
			body
		),
	adminReports: () =>
		call<{ reports: AdminReport[] }>('GET', '/v1/admin/reports?limit=100'),
	adminDashboard: () => call<AdminDashboard>('GET', '/v1/admin/dashboard'),
	adminArticles: () => call<AdminArticleListResponse>('GET', '/v1/admin/articles'),
	createAdminArticle: (body: CreateArticleRequest) =>
		call<AdminArticleVersion>('POST', '/v1/admin/articles', body),
	createAdminArticleVersion: (articleId: string) =>
		call<AdminArticleVersion>(
			'POST',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions`
		),
	adminArticleVersion: (articleId: string, versionId: string) =>
		call<AdminArticleVersion>(
			'GET',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}`
		),
	updateAdminArticleDraft: (
		articleId: string,
		versionId: string,
		body: UpdateArticleDraftRequest
	) =>
		call<AdminArticleVersion>(
			'PATCH',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}`,
			body
		),
	publishAdminArticleDraft: (articleId: string, versionId: string) =>
		call<PublishArticleResponse>(
			'POST',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}/publish`
		),
	adminSettings: () => call<{ settings: AdminSettings }>('GET', '/v1/admin/settings'),
	updateAdminSettings: (settings: AdminSettings) =>
		call<{ updated: (keyof AdminSettings)[] }>('PATCH', '/v1/admin/settings', settings),
	listContentRights: () =>
		call<{ rights: AdminContentRight[] }>('GET', '/v1/admin/content-rights'),
	listExtractionReports: () =>
		call<{ reports: AdminExtractionReport[] }>('GET', '/v1/admin/library/extraction-reports'),
	pendingScenarioAssessments: () =>
		call<{ runs: PendingScenarioAssessment[] }>(
			'GET',
			'/v1/admin/scenarios/runs/pending-assessment'
		),
	getScenarioAssessment: (runId: string) =>
		call<AdminScenarioAssessment>(
			'GET',
			`/v1/admin/scenarios/runs/${encodeURIComponent(runId)}/assessment`
		),
	recordScenarioAssessment: (
		runId: string,
		body: {
			criteria: {
				criterion_key: string;
				assessment_status: 'assessed' | 'not_assessed';
				score: number | null;
				evidence: string;
				transcript_event_indexes: number[];
				transcript_uncertain: boolean;
			}[];
		}
	) =>
		call<{ recorded_criteria: number; not_assessed: number }>(
			'POST',
			`/v1/admin/scenarios/runs/${encodeURIComponent(runId)}/assessment`,
			body
		),
	createExtractionReport: (body: {
		source_label: string;
		source_sha256: string;
		media_type: string;
		parser_version: string;
		rights_ref: string;
		malware_scan_status: AdminExtractionReport['malware_scan_status'];
		expected_regions: string[];
		extracted_regions: string[];
		uncertain_regions: string[];
		critical_regions: string[];
	}) => call<AdminExtractionReport>('POST', '/v1/admin/library/extraction-reports', body),
	reviewExtractionReport: (
		reportId: string,
		body: {
			decision: 'approved' | 'rejected';
			verified_regions: string[];
			note: string;
		}
	) =>
		call<AdminExtractionReport>(
			'POST',
			`/v1/admin/library/extraction-reports/${encodeURIComponent(reportId)}/review`,
			body
		),
	createContentRights: (body: {
		ref_code: string;
		licensor: string;
		territory: string;
		permitted_uses: string[];
		valid_from: string;
		valid_to?: string;
		notes?: string;
		contract_ref?: string;
		contract_version?: string;
		asset_refs?: string[];
		audiences?: string[];
		seat_limit?: number;
		offline_terms?: string;
		quotation_limit_words?: number;
		ai_terms?: string;
		derivative_terms?: string;
		attribution?: string;
		royalty_terms?: string;
	}) => call<{ rights_id: string; ref_code: string }>('POST', '/v1/admin/content-rights', body),
	revokeContentRights: (rightsId: string, reason: string) =>
		call<{ revoked: boolean; already_revoked: boolean }>(
			'PATCH',
			`/v1/admin/content-rights/${encodeURIComponent(rightsId)}/revoke`,
			{ reason }
		),
	resolveReport: (
		reportId: string,
		status: 'resolved_fixed' | 'resolved_rejected',
		resolutionNote: string,
		correctionNote?: string
	) =>
		call<{
			status: string;
			question_version_id: string;
			corrected_version_id: string | null;
			resolved_reports: number;
			notified_reporters: number;
		}>('POST', `/v1/reports/${encodeURIComponent(reportId)}/resolve`, {
			status,
			resolution_note: resolutionNote,
			...(correctionNote ? { correction_note: correctionNote } : {})
		}),
	createDeck: (name: string) =>
		call<{ deck_id: string }>('POST', '/v1/decks', { name }),
	addCard: (deckId: string, front: string, back: string) =>
		call<{ card_id: string }>('POST', `/v1/decks/${deckId}/cards`, {
			front,
			back
		}),
	reviewQueue: () =>
		call<{
			due: { card_id: string; front: string; back: string }[];
			new: { card_id: string; front: string; back: string }[];
			backlog_remaining: number;
		}>('GET', '/v1/reviews/queue'),
	listMocks: () => call<{ mocks: MockTest[] }>('GET', '/v1/mocks'),
	listCompetitions: () =>
		call<{ competitions: CompetitionSummary[] }>('GET', '/v1/competitions'),
	competitionLeague: (examId: string) =>
		call<CompetitionLeagueState>(
			'GET',
			`/v1/leagues/${encodeURIComponent(examId)}`
		),
	joinCompetitionLeague: (examId: string) =>
		call<Extract<CompetitionLeagueState, { joined: true }>>(
			'POST',
			`/v1/leagues/${encodeURIComponent(examId)}/join`
		),
	leaveCompetitionLeague: (examId: string) =>
		call<{ left: boolean }>(
			'DELETE',
			`/v1/leagues/${encodeURIComponent(examId)}/membership`
		),
	startCompetitionEntry: (competitionId: string, handle: string) =>
		call<CompetitionAttemptStep>(
			'POST',
			`/v1/competitions/${encodeURIComponent(competitionId)}/entry`,
			{ handle }
		),
	answerCompetitionQuestion: (
		competitionId: string,
		questionVersionId: string,
		chosenIndex: number,
		idempotencyKey: string
	) =>
		call<CompetitionAttemptStep>(
			'POST',
			`/v1/competitions/${encodeURIComponent(competitionId)}/entry/answer`,
			{
				question_version_id: questionVersionId,
				chosen_index: chosenIndex,
				idempotency_key: idempotencyKey
			}
		),
	competitionLeaderboard: (competitionId: string) =>
		call<CompetitionLeaderboard>(
			'GET',
			`/v1/competitions/${encodeURIComponent(competitionId)}/leaderboard`
		),
	startMock: (mockId: string) =>
		call<{ session_id: string; late_sync_grace_seconds: number }>(
			'POST',
			`/v1/mocks/${mockId}/start`
		),
	answerableQuestions: () =>
		call<{
			questions: {
				question_version_id: string;
				vignette: string;
				chapter: string;
			}[];
		}>('GET', '/v1/coach/answerable-questions'),
	coachTurn: (
		vid: string,
		promptType: string,
		message: string,
		idempotencyKey: string
	) =>
		call<{
			already_recorded: boolean;
			answer: string;
			adapter: string;
		}>('POST', '/v1/coach/turns', {
			question_version_id: vid,
			prompt_type: promptType,
			message,
			idempotency_key: idempotencyKey
		}),
	coachHistory: (vid: string) =>
		call<{ turns: unknown[] }>(
			'GET',
			`/v1/coach/history?question_version_id=${vid}`
		),
	createNote: (title: string, body: string, sourceQuestionVersionId?: string) =>
		call<{ note_id: string; updated_at: string }>('POST', '/v1/notes', {
			title,
			body,
			...(sourceQuestionVersionId
				? { source_question_version_id: sourceQuestionVersionId }
				: {})
		}),
	updateNote: (noteId: string, body: { title?: string; body?: string; base_updated_at?: string }) =>
		call<{ updated_at: string }>('PATCH', `/v1/notes/${encodeURIComponent(noteId)}`, body),
	listNotes: () => call<{ notes: Note[] }>('GET', '/v1/notes'),
	myMarks: () =>
		call<{ marks: { question_version_id: string; marked_at: string }[] }>(
			'GET',
			'/v1/me/marks'
		),
	markQuestion: (versionId: string) =>
		call<{ marked: boolean }>('POST', `/v1/questions/${encodeURIComponent(versionId)}/mark`),
	unmarkQuestion: (versionId: string) =>
		call<{ marked: boolean }>('DELETE', `/v1/questions/${encodeURIComponent(versionId)}/mark`),
	deleteNote: (noteId: string) =>
		call<{ deleted: boolean }>('DELETE', `/v1/notes/${noteId}`),
	librarySearch: (q: string, scope: { jurisdiction?: string; as_of?: string } = {}) => {
		const params = new URLSearchParams({ q });
		if (scope.jurisdiction) params.set('jurisdiction', scope.jurisdiction);
		if (scope.as_of) params.set('as_of', scope.as_of);
		return call<LibrarySearchResponse>('GET', `/v1/library/search?${params.toString()}`);
	},
	libraryArticle: (slug: string, scope: { jurisdiction?: string; as_of?: string } = {}) => {
		const params = new URLSearchParams();
		if (scope.jurisdiction) params.set('jurisdiction', scope.jurisdiction);
		if (scope.as_of) params.set('as_of', scope.as_of);
		const suffix = params.toString();
		return call<LibraryArticleResponse>(
			'GET',
			`/v1/library/articles/${encodeURIComponent(slug)}${suffix ? `?${suffix}` : ''}`
		);
	},
	privateImportRights: () => call<{ rights: PrivateImportRight[] }>('GET', '/v1/me/library/import-rights'),
	listPrivateImports: () => call<{ documents: PrivateImportSummary[] }>('GET', '/v1/me/library/imports'),
	createPrivateImport: (body: {
		title: string;
		media_type: PrivateImport['media_type'];
		content: string;
		rights_ref: string;
	}) => call<PrivateImportSummary>('POST', '/v1/me/library/imports', body),
	getPrivateImport: (documentId: string) =>
		call<PrivateImport>('GET', `/v1/me/library/imports/${encodeURIComponent(documentId)}`),
	deletePrivateImport: (documentId: string) =>
		call<{ deleted: boolean }>('DELETE', `/v1/me/library/imports/${encodeURIComponent(documentId)}`),
	inbox: () =>
		call<{
			preferences: {
				plan_reminders: boolean;
				mock_results: boolean;
				reports: boolean;
				content_updates: boolean;
				quiet_hours_start: number;
				quiet_hours_end: number;
			};
			notifications: {
				id: string;
				category: string;
				title: string;
				body: string;
				read: boolean;
			}[];
		}>('GET', '/v1/me/notifications'),
	updateNotificationPrefs: (body: Partial<{
		plan_reminders: boolean;
		mock_results: boolean;
		reports: boolean;
		content_updates: boolean;
		quiet_hours_start: number;
		quiet_hours_end: number;
	}>) =>
		call<{ updated: boolean }>('PATCH', '/v1/me/notifications', body),
	addPortfolioEntry: (body: Record<string, unknown>) =>
		call<{ entry_id: string }>('POST', '/v1/me/portfolio', body),
	listPortfolio: () => call<{ entries: unknown[] }>('GET', '/v1/me/portfolio'),
	addCeActivity: (activity: string, hours: number) =>
		call<{ activity_id: string; note: string }>('POST', '/v1/me/ce-activities', {
			activity,
			hours
		}),
	listAdminAudit: () =>
		call<{ events: unknown[] }>('GET', '/v1/admin/audit'),
	createNode: (body: {
		exam_id: string;
		kind: string;
		name: string;
		parent_id?: string;
	}) => call<{ node_id: string }>('POST', '/v1/admin/hierarchy', body),
	adminHierarchy: (examId: string) =>
		call<{ nodes: AdminCurriculumNode[] }>(
			'GET',
			`/v1/admin/hierarchy?exam_id=${encodeURIComponent(examId)}`
		),
	adminConcepts: () => call<{ concepts: AdminConcept[] }>('GET', '/v1/admin/concepts'),
	createConcept: (body: {
		canonical_key: string;
		display_name: string;
		definition: string;
	}) => call<{ concept_id: string; current_version: number }>('POST', '/v1/admin/concepts', body),
	createConceptVersion: (
		conceptId: string,
		body: { display_name: string; definition: string }
	) =>
		call<{ concept_id: string; current_version: number }>(
			'POST',
			`/v1/admin/concepts/${encodeURIComponent(conceptId)}/versions`,
			body
		),
	adminNodeConcepts: (nodeId: string) =>
		call<{ concepts: (Omit<AdminConcept, 'current_version'> & { version: number })[] }>(
			'GET',
			`/v1/admin/hierarchy/${encodeURIComponent(nodeId)}/concepts`
		),
	setAdminNodeConcepts: (nodeId: string, conceptIds: string[]) =>
		call<{ node_id: string; mapped: number }>(
			'PUT',
			`/v1/admin/hierarchy/${encodeURIComponent(nodeId)}/concepts`,
			{ concept_ids: conceptIds }
		),
	importQuestions: (body: {
		exam_id: string;
		dry_run: boolean;
		rows: unknown[];
	}) => call<AdminImportResult>('POST', '/v1/admin/import', body),
	importQuestionFile: (examId: string, dryRun: boolean, file: File, contentType: string) => {
		const query = new URLSearchParams({ exam_id: examId, dry_run: String(dryRun) });
		return call<AdminImportResult>(
			'POST',
			`/v1/admin/import-file?${query.toString()}`,
			file,
			contentType
		);
	},
	rollbackImport: (batchId: string) =>
		call<{ removed_questions: number }>(
			'POST',
			`/v1/admin/import/${batchId}/rollback`
		),
	assessmentWorkflow: (
		action: 'submit' | 'approve' | 'reject' | 'publish',
		versionIds: string[]
	) =>
		call<{
			results: {
				version_id: string;
				status?: string;
				error?: { code: string; message: string };
			}[];
		}>('POST', '/v1/admin/assessment-workflow', {
			action,
			version_ids: versionIds
		}),
	institutionAnalytics: (institutionId: string, cohortId: string) =>
		call<{
			cohort_id: string;
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
		}>(
			'GET',
			`/v1/institutions/${institutionId}/analytics?cohort_id=${cohortId}`
		),
	institutionAudit: (institutionId: string) =>
		call<{ events: unknown[] }>(
			'GET',
			`/v1/institutions/${institutionId}/audit`
		),
	reviewEvent: (cardId: string, rating: string, idempotencyKey: string) =>
		call<{ already_recorded: boolean; due: string }>(
			'POST',
			'/v1/reviews/events',
			{ card_id: cardId, rating, idempotency_key: idempotencyKey }
		),
	reviewDebt: () =>
		call<{
			due_now: number;
			completed_last_7_days: number;
			daily_rate: number | null;
			projected_backlog_days: number | null;
			note: string;
		}>('GET', '/v1/me/review-debt'),
	myCurriculum: () =>
		call<{
			chapters: {
				chapter_id: string;
				chapter_name: string;
				system: string;
				subject: string;
				exam_id: string;
				exam: string;
				published_questions: number;
			}[];
		}>('GET', '/v1/me/curriculum'),
	masteryHeatmap: (params?: {
		system_id?: string;
		difficulty?: string;
		trend_days?: number;
	}) => {
		const qs = new URLSearchParams();
		if (params?.system_id) qs.set('system_id', params.system_id);
		if (params?.difficulty) qs.set('difficulty', params.difficulty);
		if (params?.trend_days) qs.set('trend_days', String(params.trend_days));
		const suffix = qs.toString() ? `?${qs.toString()}` : '';
		return call<{
			systems: {
				system_id: string;
				system_name: string;
				chapters: {
					chapter_id: string;
					chapter_name: string;
					ability: number | null;
					evidence_count: number | null;
					band: string;
					filtered_accuracy?: number | null;
					recent_answered?: number;
					recent_correct?: number;
				}[];
			}[];
			bands: { weak_below: number; strong_at: number };
		}>('GET', `/v1/me/heatmap${suffix}`);
	},
	myInstitutions: () =>
		call<{
			memberships: { institution_id: string; name: string; role: string }[];
		}>('GET', '/v1/me/institutions'),
	createInstitution: (name: string) =>
		call<{ institution_id: string }>('POST', '/v1/institutions', { name }),
	addInstitutionMember: (
		institutionId: string,
		userId: string,
		role: string
	) =>
		call<{ member: string; role: string }>(
			'POST',
			`/v1/institutions/${institutionId}/members`,
			{ user_id: userId, role }
		),
	institutionCohorts: (institutionId: string) =>
		call<{
			cohorts: {
				cohort_id: string;
				name: string;
				program_id: string | null;
				members: number;
			}[];
		}>('GET', `/v1/institutions/${institutionId}/cohorts`),
	institutionPrograms: (institutionId: string) =>
		call<{
			programs: {
				program_id: string;
				name: string;
				chapter_ids: string[];
			}[];
		}>('GET', `/v1/institutions/${institutionId}/programs`),
	createInstitutionProgram: (institutionId: string, name: string) =>
		call<{ program_id: string }>(
			'POST',
			`/v1/institutions/${institutionId}/programs`,
			{ name }
		),
	setProgramCurriculum: (institutionId: string, programId: string, chapterIds: string[]) =>
		call<{ program_id: string; chapter_ids: string[]; chapter_count: number }>(
			'PUT',
			`/v1/institutions/${institutionId}/programs/${programId}/curriculum`,
			{ chapter_ids: chapterIds }
		),
	programCurriculumCoverage: (institutionId: string, programId: string) =>
		call<{
			program_id: string;
			cohort_size: number;
			minimum_group_size: number;
			suppressed: boolean;
			chapters: {
				chapter_id: string;
				chapter: string;
				learners_with_evidence: number | null;
				attempts: number | null;
				coverage_percent: number | null;
			}[];
		}>(
			'GET',
			`/v1/institutions/${institutionId}/programs/${programId}/coverage`
		),
	createCohort: (
		institutionId: string,
		name: string,
		memberIds: string[],
		programId?: string
	) =>
		call<{ cohort_id: string }>(
			'POST',
			`/v1/institutions/${institutionId}/cohorts`,
			{ name, member_ids: memberIds, ...(programId ? { program_id: programId } : {}) }
		),
	createAssignment: (cohortId: string, title: string, dueAt?: string) =>
		call<{ assignment_id: string }>(
			'POST',
			`/v1/cohorts/${cohortId}/assignments`,
			{ title, due_at: dueAt }
		),
	communityProfile: () =>
		call<{ opted_in: boolean; handle?: string }>('GET', '/v1/community/me'),
	createCommunityProfile: (handle: string) =>
		call<{ handle: string }>('POST', '/v1/community/profile', { handle }),
	profileByHandle: (handle: string) =>
		call<{ user_id: string; handle: string }>(
			'GET',
			`/v1/community/profiles/${handle}`
		),
	createDuel: (opponent: string, examId: string, questionCount: number, chapterId?: string) =>
		call<{ duel_id: string; share_token: string }>(
			'POST',
			'/v1/community/duels',
			{
				opponent,
				exam_id: examId,
				question_count: questionCount,
				...(chapterId ? { chapter_id: chapterId } : {})
			}
		),
	myDuels: () => call<{ duels: unknown[] }>('GET', '/v1/me/duels'),
	shareCards: () =>
		call<{
			cards: {
				kind: string;
				headline: string;
				subline: string;
				detail: string;
				share_text: string;
			}[];
			unavailable: { kind: string; reason: string }[];
		}>('GET', '/v1/me/share-cards'),
	duelState: (duelId: string) =>
		call<{ status: string; winner: string | null; sides: unknown[] }>(
			'GET',
			`/v1/community/duels/${duelId}`
		),
	acceptDuel: (duelId: string) =>
		call<{ accepted: boolean; your_session_id: string; question_count: number }>(
			'POST',
			`/v1/community/duels/${duelId}/accept`
		),
	declineDuel: (duelId: string) =>
		call<{ declined: boolean }>(
			'POST',
			`/v1/community/duels/${duelId}/decline`
		),
	createCommunityGroup: (name: string) =>
		call<{ group_id: string }>('POST', '/v1/community/groups', { name }),
	joinGroup: (groupId: string) =>
		call<{ joined: boolean }>('POST', `/v1/community/groups/${groupId}/join`),
	listGroupPosts: (groupId: string) =>
		call<{
			posts: {
				post_id: string;
				body: string;
				status: string;
				handle: string;
				at: string;
			}[];
			is_moderator: boolean;
		}>(
			'GET',
			`/v1/community/groups/${groupId}/posts`
		),
	createGroupPost: (groupId: string, body: string) =>
		call<{ post_id: string }>(
			'POST',
			`/v1/community/groups/${groupId}/posts`,
			{ body }
		),
	removeGroupPost: (groupId: string, postId: string) =>
		call<{ removed: boolean }>(
			'DELETE',
			`/v1/community/groups/${groupId}/posts/${postId}`
		),
	reportGroupPost: (
		groupId: string,
		postId: string,
		reason: 'spam' | 'harassment' | 'medical_misinformation' | 'other',
		note: string
	) =>
		call<{ report_id: string; status: string; created_at: string }>(
			'POST',
			`/v1/community/groups/${groupId}/posts/${postId}/reports`,
			{ reason, note: note.trim() || null }
		),
	myCommunityPostReports: () =>
		call<{
			reports: {
				report_id: string;
				group_id: string;
				group_name: string;
				post_id: string;
				reason: string;
				status: string;
				created_at: string;
			}[];
		}>('GET', '/v1/community/me/reports'),
	groupPostReportQueue: (groupId: string) =>
		call<{
			reports: {
				report_id: string;
				post_id: string;
				reason: string;
				note: string | null;
				created_at: string;
				post_body: string;
				author_handle: string;
			}[];
		}>('GET', `/v1/community/groups/${groupId}/reports`),
	resolveGroupPostReport: (
		groupId: string,
		reportId: string,
		action: 'dismiss' | 'remove'
	) =>
		call<{ resolved_reports: number }>(
			'POST',
			`/v1/community/groups/${groupId}/reports/${reportId}/resolve`,
			{ action }
		),
	listGroups: () =>
		call<{
			groups: { group_id: string; name: string; members: number }[];
		}>('GET', '/v1/community/groups'),
	selectionPolicy: () =>
		call<{
			estimator: {
				model: string;
				base: number;
				difficulty_anchors: Record<string, number>;
				k_rule: string;
				counted_evidence: string;
			};
			selection_rules: string[];
			your_chapters: {
				chapter: string;
				ability: number | null;
				current_k: number;
				evidence_count: number;
			}[];
		}>('GET', '/v1/me/selection-policy')
};
