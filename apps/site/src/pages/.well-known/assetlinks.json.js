import { deepLinks } from '../../lib/site.js';

// Android app-link association. The package name and SHA-256 fingerprints
// arrive with the first signed Tauri Android beta (see src/lib/site.js);
// until then the statement lists nothing and claims nothing.
export function GET() {
	const statements = deepLinks.android.packageName
		? [
				{
					relation: ['delegate_permission/common.handle_all_urls'],
					target: {
						namespace: 'android_app',
						package_name: deepLinks.android.packageName,
						sha256_cert_fingerprints: deepLinks.android.sha256CertFingerprints
					}
				}
			]
		: [];

	return new Response(JSON.stringify(statements, null, '\t'), {
		headers: { 'Content-Type': 'application/json; charset=utf-8' }
	});
}
