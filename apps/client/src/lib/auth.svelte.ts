export interface AuthSession {
	token: string;
	refresh_token: string;
	session_id: string;
	expires_at: string;
}

export const auth = $state({
	token: '',
	refreshToken: ''
});

const ACCESS_KEY = 'mlos_token';
const REFRESH_KEY = 'mlos_refresh_token';
const DEVICE_KEY = 'mlos_device_id';
let volatileDeviceId = '';

export function loadAuth(): string {
	if (!auth.token && typeof localStorage !== 'undefined') {
		try {
			auth.token = localStorage.getItem(ACCESS_KEY) ?? '';
			auth.refreshToken = localStorage.getItem(REFRESH_KEY) ?? '';
		} catch {
			/* storage unavailable (private mode) — session-only auth */
		}
	}
	return auth.token;
}

export function setAuthSession(session: AuthSession): void {
	auth.token = session.token;
	auth.refreshToken = session.refresh_token;
	try {
		localStorage.setItem(ACCESS_KEY, session.token);
		localStorage.setItem(REFRESH_KEY, session.refresh_token);
	} catch {
		/* storage unavailable (private mode) — session-only auth */
	}
}

export function clearAuth(): void {
	auth.token = '';
	auth.refreshToken = '';
	try {
		localStorage.removeItem(ACCESS_KEY);
		localStorage.removeItem(REFRESH_KEY);
	} catch {
		/* ignore */
	}
}

export function getDeviceIdentity(): { deviceId: string; deviceName: string } {
	let deviceId = volatileDeviceId;
	try {
		deviceId = localStorage.getItem(DEVICE_KEY) ?? deviceId;
	} catch {
		/* storage unavailable */
	}
	if (!deviceId) {
		deviceId =
				typeof crypto !== 'undefined' && 'randomUUID' in crypto
					? crypto.randomUUID()
					: `web-${Date.now()}-${Math.random().toString(36).slice(2)}`;
		volatileDeviceId = deviceId;
		try {
			localStorage.setItem(DEVICE_KEY, deviceId);
		} catch {
			/* session-only device identity */
		}
	} else {
		volatileDeviceId = deviceId;
	}
	const platform = typeof navigator !== 'undefined' && navigator.platform ? navigator.platform : 'Web';
	return {
		deviceId,
		deviceName: `Web browser on ${platform}`.slice(0, 120)
	};
}
