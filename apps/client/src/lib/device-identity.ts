const STORAGE_KEY = 'mlos_pack_device';
let sessionDeviceKey: string | undefined;

/** The same stable browser identity is used for API sessions and offline packs. */
export function browserDeviceId(): string {
	try {
		const stored = globalThis.localStorage?.getItem(STORAGE_KEY);
		if (stored) {
			sessionDeviceKey = stored;
			return stored;
		}
	} catch {
		// Storage can be unavailable in private or restricted browser contexts.
	}

	if (!sessionDeviceKey) sessionDeviceKey = globalThis.crypto.randomUUID();
	try {
		globalThis.localStorage?.setItem(STORAGE_KEY, sessionDeviceKey);
	} catch {
		// Keep one shared in-memory key for this page session.
	}
	return sessionDeviceKey;
}
