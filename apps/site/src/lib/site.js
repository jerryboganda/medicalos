/*
 * Single source of truth for the marketing site's external facts.
 * Every claim, link, and identifier used across pages comes from here so
 * nothing can drift out of sync with the product (TRUST-01: no invented
 * metrics, prices, or destinations).
 */

// The one live production origin. Marketing pages link here for every
// account/trial action; the site's own deployment origin is undecided.
export const appUrl = 'https://medicalos.polytronx.com';

export const siteName = 'Medical Learning OS';
export const tagline = 'Evidence-first exam preparation';

/*
 * Deep-link app association (GROW-02). The Tauri shells have not produced a
 * signed build yet, so the package identifier and SHA-256 cert fingerprints
 * are pending. The emitted files carry empty placeholders — operating systems
 * ignore statements that do not match a real signed app, so these files make
 * no claim until the values below are filled from the first signed beta.
 */
export const deepLinks = {
	android: {
			// Pending the first signed Tauri Android beta: applicationId from
		// tauri.conf.json and the SHA-256 from `keytool -list -v`.
		packageName: '',
		sha256CertFingerprints: []
	},
	apple: {
		// Pending the first signed Tauri iOS beta: appID (TEAMID.bundle-id)
		// from the signed build settings.
		appIds: []
	}
};

/*
 * Approved tier vocabulary (owner decision 7C). Prices are NOT approved —
 * free allowance and regional prices await owner decisions (§26.1, §32), so
 * every page must show "to be announced" rather than a number. Feature
 * differences below follow §26.1's upgrade triggers, labelled as plans.
 */
export const tiers = [
	{
		name: 'Free',
		status: 'available',
		blurb: 'Start real practice today.',
		points: [
			'Daily question allowance with full explanations',
			"Today's plan, goals, and streaks",
			'Five-question guest trial, no account needed'
		]
	},
	{
		name: 'Student',
		status: 'planned',
		blurb: 'For full-time exam candidates.',
		points: [
			'Everything in Free',
			'Planned: verified student pricing (regional tiers pending)'
		]
	},
	{
		name: 'Pro',
		status: 'planned',
		blurb: 'The complete practice loop.',
		points: [
			'Everything in Student',
			'Planned: full mock exams and offline packs',
			'Planned: chapter-level analytics and adaptive planning'
		]
	},
	{
		name: 'Ultimate',
		status: 'planned',
		blurb: 'The widest ceilings.',
		points: [
			'Everything in Pro',
			'Planned: extended AI tutoring and library allowances'
		]
	},
	{
		name: 'Institutional',
		status: 'planned',
		blurb: 'Programs, cohorts, faculty.',
		points: [
			'Institutional sign-in (OIDC), cohorts and assignments',
			'Faculty authoring and analytics with learner privacy',
			'Licensing and rollout: to be announced'
		]
	}
];

/*
 * Exam registry families (master plan §2 registry list). `status` reflects
 * reality: no exam content pack is published yet; the live app serves a
 * synthetic pilot fixture for testing. Family descriptions are deliberately
 * format-free — exam formats change and the registry must cite official
 * sources (EX-01) rather than copy historical labels.
 */
export const examFamilies = [
	{
		slug: 'usmle',
		name: 'USMLE',
		region: 'United States',
		status: 'in-development',
		summary:
			'The United States Medical Licensing Examination steps, prepared against the official blueprint rather than recycled question banks.',
		coverage: [
			'A blueprint-mapped question hierarchy you can drill by system and chapter',
			'Timed and tutor practice with per-question time budgets',
			'Source-anchored explanations and key learning points'
		]
	},
	{
		slug: 'uk-mla-plab',
		name: 'UK MLA / PLAB',
		region: 'United Kingdom',
		status: 'in-development',
		summary:
			'The UK Medical Licensing Assessment and PLAB pathway, mapped to the MLA content map with the same evidence loop as every other pack.',
		coverage: [
			'MLA content-map aligned curriculum browsing',
			'Question families and variants so retests serve unseen siblings',
			'Honest analytics built only from your own attempts'
		]
	},
	{
		slug: 'amc',
		name: 'AMC',
		region: 'Australia',
		status: 'planned',
		summary:
			'The Australian Medical Council examination pathway, planned as a Phase 6 content pack on the same curriculum and evidence foundations.',
		coverage: [
			'Curriculum browsing mapped to the AMC blueprint',
			'The same practice, evidence, and coach loop as every pack'
		]
	},
	{
		slug: 'neet-pg',
		name: 'NEET-PG / INI-CET',
		region: 'India',
		status: 'planned',
		summary:
			'India’s postgraduate medical entrance examinations, planned with regional pricing so payment routes match local wallets.',
		coverage: [
			'Exam-scoped curriculum browsing and practice pools',
			'Regional price tiers and local wallets (planned, pending payment approvals)'
		]
	}
];

export const examStatusLabels = {
	'in-development': 'In development',
	planned: 'Planned'
};

export const examRegistryNote =
	'The registry is also designed to carry FCPS, JCAT/PGET, Saudi and UAE ' +
	'licensing examinations, FMGE, MRCP/MRCS, MCCQE, and COMLEX. Registry ' +
	'coverage is an architectural target, not a claim that packs exist.';

export const productStages = [
	{
		label: '1.0',
		name: 'Aim',
		text:
			'Set your exam date, pick your goals, and mark the commitments that ' +
			'must not be dropped. Plans protect what you protect — replans ' +
			'preserve protected work instead of silently discarding it.'
	},
	{
		label: '2.0',
		name: 'Plan',
		text:
			'Each morning, Today gives you a time-budgeted next action, not a ' +
			'todo list. The coach proposes changes with a receipt and an undo — ' +
			'and never nags you to upgrade.'
	},
	{
		label: '3.0',
		name: 'Practice',
		text:
			'Build a session from the curriculum, or take a tutor, timed, or ' +
			'revision preset with server-enforced deadlines. Calculators, ' +
			'notes, question marks, and autosave travel with the session — ' +
			'online or off.'
	},
	{
		label: '4.0',
		name: 'Evidence',
		text:
			'Every attempt becomes evidence: per-chapter mastery state, ' +
			'drill-down analytics, and answer-change analysis. Numbers come ' +
			'only from your own attempts — when there is no data, the app ' +
			'says so instead of guessing.'
	}
];

// Payment routes under discussion (§26.1). Naming them is honest; claiming
// any of them works today would not be.
export const paymentNote =
	'Web checkout will support cards and local wallets such as JazzCash and ' +
	'Easypaisa, alongside store billing in the mobile apps. Every route feeds ' +
	'the same entitlement service. Checkout opens only after payment-provider ' +
	'rules are confirmed per market — no payment form is shown before that.';
