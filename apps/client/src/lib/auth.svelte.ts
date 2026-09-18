export const auth = $state({ token: '' });

const KEY = 'mlos_token';

export function loadAuth(): string {
	if (!auth.token && typeof localStorage !== 'undefined') {
		auth.token = localStorage.getItem(KEY) ?? '';
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
