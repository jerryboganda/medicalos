import { deepLinks } from '../../lib/site.js';

// Apple app-site association. The appID (TEAMID.bundle-id) arrives with the
// first signed Tauri iOS beta (see src/lib/site.js); until then `details`
// is empty, which is a valid document that makes no claim.
export function GET() {
	const association = {
		applinks: {
			apps: [],
			details: deepLinks.apple.appIds.map((appID) => ({
				appID,
				paths: ['/duel/*', '/community/*']
			}))
		}
	};

	return new Response(JSON.stringify(association, null, '\t'), {
		headers: { 'Content-Type': 'application/json; charset=utf-8' }
	});
}
