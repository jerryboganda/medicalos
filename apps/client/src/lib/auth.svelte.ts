export interface AuthSession {
	token: string;
	refresh_token: string;
	session_id: string;
	expires_at: string;
}

export const auth = $state({
	token: '',
	refreshToken: '',
	sessionId: '',
	expiresAt: ''
});

const ACCESS_KEY = 'mlos_token';
const REFRESH_KEY = 'mlos_refresh_token';
const SESSION_KEY = 'mlos_session_id';
const EXPIRES_KEY = 'mlos_access_expires_at';
const DEVICE_KEY = 'mlos_device_id';

export function loadAuth(): string {
	if (!auth.token && typeof localStorage !== 'undefined') {
		try {
			auth.token = localStorage.getItem(ACCESS_KEY) ?? '';
			auth.refreshToken = localStorage.getItem(REFRESH_KEY) ?? '';
			auth.sessionId = localStorage.getItem(SESSION_KEY) ?? '';
			auth.expiresAt = localStorage.getItem(EXPIRES_KEY) ?? '';
		} catch {
			/* storage unavailable (private mode) — session-only auth */
		}
	}
	return auth.token;
}

export function setAuthSession(session: AuthSession): void {
	auth.token = session.token;
	auth.refreshToken = session.refresh_token;
	auth.sessionId = session.session_id;
	auth.expiresAt = session.expires_at;
	try {
		localStorage.setItem(ACCESS_KEY, session.token);
		localStorage.setItem(REFRESH_KEY, session.refresh_token);
		localStorage.setItem(SESSION_KEY, session.session_id);
		localStorage.setItem(EXPIRES_KEY, session.expires_at);
	} catch {
		/* storage unavailable (private mode) — session-only auth */
	}
}

export function clearAuth(): void {
	auth.token = '';
	auth.refreshToken = '';
	auth.sessionId = '';
	auth.expiresAt = '';
	try {
		localStorage.removeItem(ACCESS_KEY);
		localStorage.removeItem(REFRESH_KEY);
		localStorage.removeItem(SESSION_KEY);
		localStorage.removeItem(EXPIRES_KEY);
	} catch {
		/* ignore */
	}
}

export function getDeviceIdentity(): { deviceId: string; deviceName: string } {
	let deviceId = '';
	try {
		deviceId = localStorage.getItem(DEVICE_KEY) ?? '';
	} catch {
		/* storage unavailable */
	}
	if (!deviceId) {
		deviceId =
			typeof crypto !== 'undefined' && 'randomUUID' in crypto
				? crypto.randomUUID()
				: `web-${Date.now()}-${Math.random().toString(36).slice(2)}`;
		try {
			localStorage.setItem(DEVICE_KEY, deviceId);
		} catch {
			/* session-only device identity */
		}
	}
	const platform = typeof navigator !== 'undefined' && navigator.platform ? navigator.platform : 'Web';
	return {
		deviceId,
		deviceName: `Web browser on ${platform}`.slice(0, 120)
	};
}
