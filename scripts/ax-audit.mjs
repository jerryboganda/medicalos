#!/usr/bin/env node
// AX protocol enforcement (docs/agents/agent-experience.md): fails the build
// on mechanically-detectable evidence-free claims.
//
//   1. Ledger rows claiming tested/resolved with an empty Evidence cell
//      (docs/requirements/traceability.md is the per-ID source of truth).
//   2. STATE.md gate rows marked PASS without raw-output evidence.
//   3. STATE.md files missing required section headers.
//
// Everything else (prose quality, run-URL validity, NOT RUN states) stays a
// review concern — the audit only rejects claims of verification without
// evidence, the same honesty line TRUST-01 draws for shipped UI.

import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const LEDGER = 'docs/requirements/traceability.md';
const SCRATCH = '.scratch';
// ponytail: kept in sync with the STATE.md template in
// docs/agents/agent-experience.md — regenerate both if one changes.
const REQUIRED_SECTIONS = [
	'## Objective',
	'## Status',
	'## Decided',
	'## Do-not-touch',
	'## Completed',
	'## Active diffs',
	'## Verification gates',
	'## Blockers / owner inputs',
	'## Next action',
	'## Log'
];

let failures = 0;
const fail = (msg) => {
	failures += 1;
	console.error(`AX ${msg}`);
};

function cells(line) {
	// "| a | b | c | d |" -> ['a', 'b', 'c', 'd']; null for non-4-col rows.
	const trimmed = line.trim();
	if (!trimmed.startsWith('|') || !trimmed.endsWith('|')) return null;
	const parts = trimmed.slice(1, -1).split('|').map((c) => c.trim());
	return parts.length === 4 ? parts : null;
}

function isDataRow(row) {
	return row && row[0] !== '' && !/^-+$/.test(row[0]) && row.some((c) => c !== '');
}

let ledgerRows = 0;
for (const line of readFileSync(LEDGER, 'utf8').split(/\r?\n/)) {
	const row = cells(line);
	if (!isDataRow(row) || row[0] === 'ID') continue;
	ledgerRows += 1;
	if (/^(tested|resolved)\b/i.test(row[2]) && row[3] === '') {
		fail(`${LEDGER}: row ${row[0]} claims ${row[2].split(' ')[0]} with an empty Evidence cell`);
	}
}

function* stateFiles(dir) {
	for (const entry of readdirSync(dir)) {
		const full = join(dir, entry);
		if (!statSync(full).isDirectory()) continue;
		if (entry === 'evidence') continue;
		const state = join(full, 'STATE.md');
		// A missing STATE.md is the working agent's to add (protocol: created
		// at first edit); absence stays a review concern, not a CI failure.
		if (existsSync(state)) yield state;
	}
}

let stateCount = 0;
for (const file of stateFiles(SCRATCH)) {
	stateCount += 1;
	const lines = readFileSync(file, 'utf8').split(/\r?\n/);
	for (const section of REQUIRED_SECTIONS) {
		if (!lines.some((l) => l.trim() === section)) {
			fail(`${file}: missing required section ${section}`);
		}
	}
	let inGates = false;
	for (const line of lines) {
		const heading = line.trim();
		if (heading.startsWith('## ')) inGates = heading === '## Verification gates';
		if (!inGates) continue;
		const row = cells(line);
		if (!isDataRow(row) || row[0] === 'Gate') continue;
		if (/\bPASS\b/.test(row[2]) && row[3] === '') {
			fail(`${file}: gate "${row[0]}" marked PASS with empty raw-output evidence`);
		}
	}
}

if (failures > 0) {
	console.error(`\nAX audit: ${failures} finding(s). No gate passes without raw output evidence.`);
	process.exit(1);
}
console.log(`AX audit: clean (${ledgerRows} ledger rows, ${stateCount} STATE files checked).`);
