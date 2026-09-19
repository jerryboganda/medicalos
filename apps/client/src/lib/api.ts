import {
	auth,
	clearAuth,
	getDeviceIdentity,
	setAuthSession,
	type AuthSession
} from './auth.svelte';

// Until ARCH-02 contract generation lands, this is the single hand-written
// client for the endpoints the client app uses. It mirrors the API contract;
// when the generated client arrives this file is replaced, not edited.
const BASE: string = import.meta.env.VITE_API_BASE ?? '';

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

let refreshInFlight: Promise<boolean> | null = null;

async function refreshSession(): Promise<boolean> {
	if (!auth.refreshToken) return false;
	if (refreshInFlight) return refreshInFlight;
	refreshInFlight = (async () => {
		try {
			const res = await fetch(`${BASE}/v1/auth/refresh`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ refresh_token: auth.refreshToken })
			});
			if (!res.ok) {
				clearAuth();
				return false;
			}
			setAuthSession((await res.json()) as AuthSession);
			return true;
		} catch {
			clearAuth();
			return false;
		} finally {
			refreshInFlight = null;
		}
	})();
	return refreshInFlight;
}

async function call<T>(
	method: string,
	path: string,
	body?: unknown,
	retryAfterRefresh = true
): Promise<T> {
	const protectedRequest = !path.startsWith('/v1/auth/') && !path.startsWith('/v1/guest-trial/');
	const accessToken = protectedRequest ? auth.token : '';
	const sentAccessToken = Boolean(accessToken);
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers: {
			'content-type': 'application/json',
			...(sentAccessToken ? { authorization: `Bearer ${accessToken}` } : {})
		},
		body: body === undefined ? undefined : JSON.stringify(body)
	});
	if (res.status === 401 && retryAfterRefresh && sentAccessToken) {
		// Another request may already have rotated the session while this request
		// was in flight. Reuse the newer access token instead of rotating again.
		if (auth.token && auth.token !== accessToken) return call<T>(method, path, body, false);
		if (auth.refreshToken && (await refreshSession())) {
			return call<T>(method, path, body, false);
		}
	}
	if (!res.ok) {
		if (res.status === 401 && sentAccessToken) clearAuth();
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

export interface GoalCommitment {
	title: string;
	date: string;
}

export interface LearnerGoals {
	version: number;
	daily_minutes: number | null;
	exam_date: string | null;
	protected_commitments: GoalCommitment[];
	created_at: string | null;
	can_undo: boolean;
	changed: boolean;
}

export interface GuestTrialItem {
	item_index: number;
	question_version_id: string;
	vignette: string;
	lead_in: string;
	difficulty: string;
	options: { text: string }[];
	correct_index: null;
	key_learning_point: null;
	exam_tip: null;
}

export interface GuestTrialStart {
	trial_token: string;
	expires_at: string;
	items: GuestTrialItem[];
}

export interface EngagementPreferences {
	daily_goal_enabled: boolean;
	streak_enabled: boolean;
	qotd_enabled: boolean;
	qotd_time: string | null;
}

export interface EngagementCommunityOption {
	option_index: number;
	count: number;
	percentage: number;
}

export interface EngagementQotd {
	exam_id: string;
	exam_code: string;
	exam_name: string;
	question_version_id: string;
	answered: boolean;
	due: boolean;
	community_split: {
		total_answers: number;
		options: EngagementCommunityOption[];
	} | null;
}

export interface Engagement {
	timezone: string;
	local_date: string;
	preferences: EngagementPreferences;
	daily_goal: {
		enabled: boolean;
		target_minutes: number | null;
		completed_minutes: number;
		met: boolean;
	};
	streak: {
		enabled: boolean;
		length: number;
		freezes_held: number;
		goal_met_days: number;
		next_freeze_in: number | null;
	};
	qotd: EngagementQotd[];
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
	marked: boolean;
}

export interface BuilderNode {
	id: string;
	exam_id: string;
	parent_id: string | null;
	kind: string;
	name: string;
	display_order: number;
	available: number;
	attempted: number;
	unattempted: number;
}

export interface BuilderSelection {
	all: number;
	attempted: number;
	unattempted: number;
	incorrect_skipped: number;
	marked: number;
	matching: number;
}

export interface BuilderResponse {
	nodes: BuilderNode[];
	selection: BuilderSelection;
}

export interface CreateSessionResponse {
	session_id: string;
	requested_count?: number | null;
	available_count?: number;
	question_count?: number;
	availability_message?: string | null;
}

export interface PracticeSession {
	session_id: string;
	preset: string;
	chapter_id: string | null;
	source_session_id: string | null;
	status: string;
	time_limit_seconds: number | null;
	/** EX-08: server-issued; the countdown derives from deadline − server_now. */
	deadline: string | null;
	server_now: string | null;
	items: SessionItem[];
	result: SubmitResult | null;
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
	assisted: number;
	time_taken_seconds: number;
}

export interface DeviceSession {
	session_id: string;
	device_id: string;
	device_name: string;
	created_at: string;
	last_seen_at: string;
	expires_at: string;
	refresh_expires_at: string;
	current: boolean;
}

export interface NotificationCategories {
	plan_review_reminders: boolean;
	mock_assignment: boolean;
	competition: boolean;
	duel_invitation: boolean;
	report_resolved: boolean;
	subscription_events: boolean;
}

export interface NotificationPreferences {
	timezone: string;
	quiet_start: string | null;
	quiet_end: string | null;
	categories: NotificationCategories;
}

export interface InboxNotification {
	id: string;
	category: keyof NotificationCategories;
	title: string;
	body: string;
	deep_link: string;
	campaign_key: string | null;
	promotional: boolean;
	created_at: string;
	read_at: string | null;
}

export interface LibraryItem {
	item_id: string;
	kind: 'article' | 'reference';
	version: number;
	title: string;
	provenance_class: string;
	source_label: string;
	source_url: string | null;
	effective_date: string;
	jurisdiction: string;
}

export interface LibraryVersion extends LibraryItem {
	body: string;
	concept_version_ids: string[];
	question_version_ids: string[];
}

export const Api = {
	register: (email: string, password: string, guestTrialToken?: string) =>
		call<{
			user_id: string;
			verification_required: boolean;
			verification_token?: string;
			guest_answers_migrated: number;
		}>(
			'POST',
			'/v1/auth/register',
			{ email, password, guest_trial_token: guestTrialToken }
		),
	guestTrialStart: () => call<GuestTrialStart>('POST', '/v1/guest-trial/start'),
	guestTrialAnswer: (body: {
		trial_token: string;
		item_index: number;
		chosen_index: number;
		idempotency_key: string;
	}) => call<AnswerResult>('POST', '/v1/guest-trial/answer', body),
	verifyEmail: (token: string) =>
		call<{ verified: boolean }>('POST', '/v1/auth/verify-email', { token }),
	login: (email: string, password: string) => {
		const { deviceId, deviceName } = getDeviceIdentity();
		return call<AuthSession>('POST', '/v1/auth/login', {
			email,
			password,
			device_id: deviceId,
			device_name: deviceName
		});
	},
	forgotPassword: (email: string) =>
		call<{ accepted: boolean; reset_token?: string }>('POST', '/v1/auth/forgot-password', {
			email
		}),
	resetPassword: (token: string, password: string) =>
		call<{ reset: boolean }>('POST', '/v1/auth/reset-password', { token, password }),
	sessions: () => call<{ sessions: DeviceSession[] }>('GET', '/v1/me/sessions'),
	signOutOthers: () => call<{ revoked: number }>('POST', '/v1/me/sessions/sign-out-others'),
	logout: () => call<{ signed_out: boolean }>('POST', '/v1/me/sessions/logout'),
	requestAccountDeletion: (password: string) =>
		call<{ status: 'pending' }>('POST', '/v1/me/account/deletion', { password }),
	notificationPreferences: () =>
		call<NotificationPreferences>('GET', '/v1/me/notification-preferences'),
	updateNotificationPreferences: (body: NotificationPreferences) =>
		call<NotificationPreferences>('PUT', '/v1/notification-preferences', body),
	registerPushToken: (body: { device_id: string; platform: 'ios' | 'android'; token: string }) =>
		call<{ registered: boolean }>('POST', '/v1/push-tokens', body),
	notifications: () => call<{ notifications: InboxNotification[] }>('GET', '/v1/notifications'),
	markNotificationRead: (id: string) =>
		call<{ read: boolean; read_at: string }>('POST', `/v1/notifications/${id}/read`),
	library: () => call<{ items: LibraryItem[] }>('GET', '/v1/library'),
	libraryVersion: (itemId: string, version: number) =>
		call<LibraryVersion>('GET', `/v1/library/${itemId}/versions/${version}`),
	today: () => call<Today>('GET', '/v1/me/today'),
	goals: () => call<LearnerGoals>('GET', '/v1/me/goals'),
	engagement: () => call<Engagement>('GET', '/v1/me/engagement'),
	updateEngagementPreferences: (body: EngagementPreferences) =>
		call<EngagementPreferences>('PUT', '/v1/me/engagement/preferences', body),
	startQotd: (examId: string, takeover = false) =>
		call<CreateSessionResponse & { exam_id: string; question_version_id: string; local_date: string }>(
			'POST',
			`/v1/me/engagement/qotd/${examId}/session`,
			{ takeover }
		),
	updateGoals: (body: {
		expected_version: number;
		daily_minutes: number | null;
		exam_date: string | null;
		protected_commitments: GoalCommitment[];
	}) => call<LearnerGoals>('PUT', '/v1/me/goals', body),
	undoGoals: (expectedVersion: number) =>
		call<LearnerGoals>('POST', '/v1/me/goals/undo', { expected_version: expectedVersion }),
	createSession: (body: {
		preset: string;
		chapter_id?: string;
		chapter_ids?: string[];
		question_count?: number;
		pool?: 'all' | 'incorrect_skipped' | 'unattempted' | 'marked';
		difficulties?: ('easy' | 'medium' | 'hard')[];
		high_yield?: boolean;
		all_available?: boolean;
		source_session_id?: string;
		time_limit_seconds?: number;
		takeover?: boolean;
	}) => call<CreateSessionResponse>('POST', '/v1/practice/sessions', body),
	builder: (filters: {
		chapter_ids?: string[];
		pool?: 'all' | 'incorrect_skipped' | 'unattempted' | 'marked';
		difficulties?: ('easy' | 'medium' | 'hard')[];
		high_yield?: boolean;
	} = {}) => {
		const params = new URLSearchParams();
		if (filters.chapter_ids?.length) params.set('chapter_ids', filters.chapter_ids.join(','));
		if (filters.pool) params.set('pool', filters.pool);
		if (filters.difficulties?.length) params.set('difficulties', filters.difficulties.join(','));
		if (filters.high_yield) params.set('high_yield', 'true');
		const query = params.toString();
		return call<BuilderResponse>('GET', `/v1/practice/builder${query ? `?${query}` : ''}`);
	},
	setQuestionMark: (versionId: string, marked: boolean) =>
		call<{ marked: boolean }>('PUT', `/v1/questions/versions/${versionId}/mark`, { marked }),
	getSession: (sid: string) => call<PracticeSession>('GET', `/v1/practice/sessions/${sid}`),
	answer: (
		sid: string,
		body: { item_index: number; chosen_index: number | null; idempotency_key: string }
	) => call<AnswerResult>('POST', `/v1/practice/sessions/${sid}/answers`, body),
	hint: (sid: string, itemIndex: number) =>
		call<{ hint: string }>('POST', `/v1/practice/sessions/${sid}/items/${itemIndex}/hint`),
	calculate: (body: { calculator: string; inputs: Record<string, number | boolean> }) =>
		call<{ value: number; unit: string }>('POST', '/v1/tools/calculate', body),
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
	reviewEvent: (cardId: string, rating: string, idempotencyKey: string) =>
		call<{ already_recorded: boolean; due: string }>(
			'POST',
			'/v1/reviews/events',
			{ card_id: cardId, rating, idempotency_key: idempotencyKey }
		)
};
