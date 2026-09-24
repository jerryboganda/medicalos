#!/usr/bin/env node
// TRUST-01 enforcement (§2.3): fails the build on mechanically-detectable
// fabrication vectors in shipped UI sources.
//
// Scope note — the four TRUST-01 classes split by decidability:
//   1. Fabricated values (Math.random, placeholder copy, TODO/FIXME) are
//      statically decidable — checked here.
//   2. Dead controls and active-agent claims need behavioral context; they
//      are covered by the recorded manual audit
//      (.scratch/trust01-audit/findings.md) and by the Playwright suite,
//      which exercises every control against the real API.

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const roots = ['apps/client/src', 'apps/site/src'];
const exts = new Set(['.svelte', '.ts', '.astro', '.js', '.css']);
const checks = [
	{ name: 'Math.random (fabricated values)', re: /\bMath\.random\b/ },
	{
		name: 'placeholder copy (lorem / coming soon / sample data / dummy)',
		re: /lorem ipsum|coming soon|sample data|dummy data/i
	},
	{ name: 'TODO/FIXME/XXX markers', re: /\b(TODO|FIXME|XXX)\b/ }
];

function* files(dir) {
	for (const entry of readdirSync(dir)) {
		const full = join(dir, entry);
		if (statSync(full).isDirectory()) yield* files(full);
		else if (exts.has(entry.slice(entry.lastIndexOf('.')))) yield full;
	}
}

let failures = 0;
for (const root of roots) {
	for (const file of files(root)) {
		const lines = readFileSync(file, 'utf8').split(/\r?\n/);
		lines.forEach((line, i) => {
			for (const check of checks) {
				if (check.re.test(line)) {
					failures += 1;
					console.error(`TRUST-01 ${file}:${i + 1}: ${check.name}`);
					console.error(`    ${line.trim()}`);
				}
			}
		});
	}
}

if (failures > 0) {
	console.error(`\nTRUST-01 audit: ${failures} finding(s). Shipped UI must not fabricate.`);
	process.exit(1);
}
console.log('TRUST-01 audit: clean.');
