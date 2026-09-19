const API = 'http://127.0.0.1:8080';

export async function createVerifiedSession(email: string, password: string, deviceId: string) {
	const register = await fetch(`${API}/v1/auth/register`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ email, password })
	});
	if (!register.ok) throw new Error(`register failed (${register.status})`);
	const registered = await register.json();
	if (!registered.verification_token) throw new Error('verification token missing from E2E server');

	const verify = await fetch(`${API}/v1/auth/verify-email`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ token: registered.verification_token })
	});
	if (!verify.ok) throw new Error(`verification failed (${verify.status})`);

	const login = await fetch(`${API}/v1/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			email,
			password,
			device_id: deviceId,
			device_name: `E2E ${deviceId}`
		})
	});
	if (!login.ok) throw new Error(`login failed (${login.status})`);
	return login.json();
}
