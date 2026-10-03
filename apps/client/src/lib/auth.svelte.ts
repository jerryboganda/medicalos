export const auth = $state({ token: '' });

const KEY = 'mlos_token';

export function loadAuth(): string {
	if (!auth.token) {
		try {
			auth.token = localStorage.getItem(KEY) ?? '';
		} catch {
			// Keep an in-memory bearer when browser storage is unavailable.
		}
	}
	return auth.token;
}

export function setToken(t: string): void {
	auth.token = t;
	try {
		localStorage.setItem(KEY, t);
	} catch {
		/* storage unavailable (private mode) — session-only auth */
	}
}

export function clearToken(): void {
	auth.token = '';
	try {
		localStorage.removeItem(KEY);
	} catch {
		/* ignore */
	}
}
