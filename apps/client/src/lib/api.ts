import { auth, clearToken } from './auth.svelte';

// Until ARCH-02 contract generation lands, this is the single hand-written
// client for the endpoints the client app uses. It mirrors the API contract;
// when the generated client arrives this file is replaced, not edited.
// Production reaches the API same-origin through the /api/ prefix
// (medicalos.polytronx.com/api/* -> the API container); local dev keeps
// talking straight to the API host.
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

export function adminToken(): string {
	try {
		return localStorage.getItem('mlos_admin') ?? '';
	} catch {
		return '';
	}
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers: {
			'content-type': 'application/json',
			'x-admin-token': adminToken(),
			...(auth.token ? { authorization: `Bearer ${auth.token}` } : {})
		},
		body: body === undefined ? undefined : JSON.stringify(body)
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

export interface TodayTask {
	id: string;
	kind: string;
	title: string;
	chapter_id: string | null;
	source_session_id: string | null;
	question_count: number;
	status: string;
}

export interface TodayRevision {
	id: string;
	to_version: number;
	reason_code: string;
	explanation: string;
	automatic: boolean;
	undone: boolean;
}

export interface LearnerChapter {
	chapter_id: string;
	chapter_name: string;
	mastery_index: number | null;
	evidence_level: string;
	independent_count: number;
}

export interface Today {
	plan_id: string;
	version: number;
	tasks: TodayTask[];
	revisions: TodayRevision[];
	learner: LearnerChapter[];
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
	/** QB-08: honest flag state — null when unflagged. */
	report_status: 'open' | 'quarantined' | 'resolved_fixed' | null;
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
	backlinks: { note_id: string; title: string }[];
}

export interface MockTest {
	mock_id: string;
	title: string;
	pass_mark_percent: number;
	attempts_allowed: number;
	attempts_used: number;
	time_limit_seconds: number | null;
}

export interface MockResult {
	score_percent: number;
	passed: boolean;
	pass_mark_percent: number;
	percentile: number | null;
	takers: number;
	breakdown: { chapter: string; total: number; correct: number }[];
}

export interface EngagementQotd {
	enabled: boolean;
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
	daily_goal: { enabled: boolean; target: number; answered_today: number; met: boolean };
	streak: { enabled: boolean; count: number; freezes: number };
	qotd: EngagementQotd;
}

export interface EngagementSettings {
	daily_goal_questions?: number;
	daily_goal_enabled?: boolean;
	streak_enabled?: boolean;
	qotd_enabled?: boolean;
}

export const Api = {
	register: (email: string, password: string) =>
		call<{ user_id: string }>('POST', '/v1/auth/register', { email, password }),
	login: (email: string, password: string) =>
		call<{ token: string }>('POST', '/v1/auth/login', { email, password }),
	today: () => call<Today>('GET', '/v1/me/today'),
	engagement: () => call<Engagement>('GET', '/v1/me/engagement'),
	updateEngagementSettings: (body: EngagementSettings) =>
		call<{
			daily_goal_questions: number;
			daily_goal_enabled: boolean;
			streak_enabled: boolean;
			qotd_enabled: boolean;
			freezes: number;
		}>('PUT', '/v1/me/engagement/settings', body),
	answerQotd: (questionVersionId: string, chosenIndex: number) =>
		call<{
			correct: boolean;
			correct_index: number;
			community_split: { chosen_index: number; count: number }[];
			community_total: number;
		}>('POST', '/v1/me/qotd/answers', {
			question_version_id: questionVersionId,
			chosen_index: chosenIndex
		}),
	createSession: (body: {
		preset: string;
		chapter_id?: string;
		question_count?: number;
		source_session_id?: string;
		time_limit_seconds?: number;
	}) => call<{ session_id: string }>('POST', '/v1/practice/sessions', body),
	getSession: (sid: string) => call<PracticeSession>('GET', `/v1/practice/sessions/${sid}`),
	answer: (
		sid: string,
		body: { item_index: number; chosen_index: number | null; idempotency_key: string }
	) => call<AnswerResult>('POST', `/v1/practice/sessions/${sid}/answers`, body),
	submit: (sid: string) => call<SubmitResult>('POST', `/v1/practice/sessions/${sid}/submit`),
	undo: (planId: string, revisionId: string) =>
		call<{ plan_version: number }>(
			'POST',
			`/v1/plans/${planId}/revisions/${revisionId}/undo`
		),
	reportQuestion: (versionId: string, body: { category: string; note?: string }) =>
		call<{ report_id: string; already_recorded: boolean; quarantined: boolean }>(
			'POST',
			`/v1/questions/versions/${versionId}/reports`,
			body
		),
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
	startMock: (mockId: string) =>
		call<{ session_id: string }>('POST', `/v1/mocks/${mockId}/start`),
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
	createNote: (title: string, body: string) =>
		call<{ note_id: string }>('POST', '/v1/notes', { title, body }),
	listNotes: () => call<{ notes: Note[] }>('GET', '/v1/notes'),
	deleteNote: (noteId: string) =>
		call<{ deleted: boolean }>('DELETE', `/v1/notes/${noteId}`),
	librarySearch: (q: string) =>
		call<{ results: unknown[] }>(
			'GET',
			`/v1/library/search?q=${encodeURIComponent(q)}`
		),
	inbox: () =>
		call<{
			notifications: {
				id: string;
				category: string;
				title: string;
				body: string;
				read: boolean;
			}[];
		}>('GET', '/v1/me/notifications'),
	updateNotificationPrefs: (body: Record<string, unknown>) =>
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
	importQuestions: (body: {
		exam_id: string;
		dry_run: boolean;
		rows: unknown[];
	}) =>
		call<{
			batch_id: string;
			status: string;
			valid?: number;
			issues?: { row: number; code: string; message: string }[];
			created?: unknown[];
		}>('POST', '/v1/admin/import', body),
	rollbackImport: (batchId: string) =>
		call<{ removed_questions: number }>(
			'POST',
			`/v1/admin/import/${batchId}/rollback`
		),
	reviewEvent: (cardId: string, rating: string, idempotencyKey: string) =>
		call<{ already_recorded: boolean; due: string }>(
			'POST',
			'/v1/reviews/events',
			{ card_id: cardId, rating, idempotency_key: idempotencyKey }
		)
};
