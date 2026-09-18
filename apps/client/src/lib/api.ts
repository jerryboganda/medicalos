import { auth, clearToken } from './auth.svelte';

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

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers: {
			'content-type': 'application/json',
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
}

export interface PracticeSession {
	session_id: string;
	preset: string;
	status: string;
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
}

export const Api = {
	register: (email: string, password: string) =>
		call<{ user_id: string }>('POST', '/v1/auth/register', { email, password }),
	login: (email: string, password: string) =>
		call<{ token: string }>('POST', '/v1/auth/login', { email, password }),
	today: () => call<Today>('GET', '/v1/me/today'),
	createSession: (body: {
		preset: string;
		chapter_id?: string;
		question_count?: number;
		source_session_id?: string;
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
		)
};
