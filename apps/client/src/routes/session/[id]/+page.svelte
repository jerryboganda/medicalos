<script>
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { beforeNavigate } from '$app/navigation';
	import { Api, ApiError } from '$lib/api';

	let { data } = $props();

	const sid = data.id;
	let session = $state(null);
	let current = $state(0);
	let selected = $state(null);
	let busy = $state(false);
	let submitting = $state(false);
	let error = $state('');
	let result = $state(null);
	let loadFailed = $state('');
	let integrityWarning = $state('');
	let integrityAutoSubmitted = $state(false);
	let integrityActive = false;

	beforeNavigate(({ from, to }) => {
		if (to?.route.id !== from?.route.id || to?.params.id !== sid) integrityActive = false;
	});

	// QB-08: report-a-problem control on answered items.
	let reportOpen = $state(false);
	let reportCategory = $state('wrong_answer');
	let reportNote = $state('');
	let reportBusy = $state(false);
	let reportDone = $state('');
	let reportError = $state('');
	let toolsOpen = $state(false);
	let textSize = $state(0);
	let selectedCalculator = $state('bmi');
	let calculatorInputs = $state({});
	let calculatorResult = $state('');
	let calculatorError = $state('');
	let calculating = $state(false);
	let converterAnalyte = $state('general');
	let converterPair = $state('kg|lb');
	let converterValue = $state('');
	let converterResult = $state('');
	let converterError = $state('');
	let noteByVersion = $state({});
	let noteRecords = $state({});
	let noteSaveState = $state({});
	let markState = $state({});
	let pendingAnswers = $state({});
	let unacceptedAnswers = $state({});
	let pendingMarks = $state({});
	let pendingNotes = $state({});
	let highlights = $state({});
	let eliminated = $state({});
	let hintText = $state('');
	let hintByVersion = $state({});
	let hintBusy = $state(false);
	let activeTutoringType = $state('');
	let testAnswerRevealed = $state(false);
	let offlineMessage = $state('');
	let timerWarning = $state('');
	let pendingSubmission = $state(false);
	let syncingDraft = false;
	let draftReady = false;
	let eliminating = $state(false);
	let noteTimers = new Map();
	let noteInFlight = new Set();

	// UX-01: Focus Mode dims the surrounding chrome to the current item;
	// horizontal swipe gestures move between answered items on touch.
	// UX-02: the browser fullscreen toggle lives beside it.
	let focusMode = $state(false);
	let touchStartX = $state(null);

	function toggleFocus() {
		focusMode = !focusMode;
	}

	async function toggleFullscreen() {
		try {
			if (document.fullscreenElement) {
				await document.exitFullscreen();
			} else {
				await document.documentElement.requestFullscreen();
			}
		} catch {
			// Fullscreen can be refused (permissions/policy) — stay silent.
		}
	}

	function touchStart(event) {
		touchStartX = event.changedTouches[0]?.clientX ?? null;
	}

	function touchEnd(event) {
		if (touchStartX === null || result) return;
		const endX = event.changedTouches[0]?.clientX ?? null;
		if (endX === null) return;
		const dx = endX - touchStartX;
		touchStartX = null;
		if (Math.abs(dx) < 60 || !session || !item) return;
		if (dx < 0 && current < session.items.length - 1 && item.answered) {
			navigateTo(current + 1);
		} else if (dx > 0 && current > 0) {
			navigateTo(current - 1);
		}
	}

	const REPORT_CATEGORIES = [
		['wrong_answer', 'Wrong answer'],
		['bad_explanation', 'Bad explanation'],
		['typo', 'Typo'],
		['duplicate', 'Duplicate'],
		['outdated', 'Outdated'],
		['broken_image', 'Broken image'],
		['other', 'Other']
	];

	const CALCULATORS = {
		bmi: { label: 'BMI', fields: [['weight_kg', 'Weight (kg)'], ['height_m', 'Height (m)']] },
		bsa: { label: 'Body surface area', fields: [['weight_kg', 'Weight (kg)'], ['height_cm', 'Height (cm)']] },
		map: { label: 'Mean arterial pressure', fields: [['systolic', 'Systolic (mmHg)'], ['diastolic', 'Diastolic (mmHg)']] },
		gcs: { label: 'Glasgow Coma Scale', fields: [['eye', 'Eye (1–4)'], ['verbal', 'Verbal (1–5)'], ['motor', 'Motor (1–6)']] },
		'creatinine-clearance': { label: 'Creatinine clearance', fields: [['age_years', 'Age (years)'], ['weight_kg', 'Weight (kg)'], ['serum_creatinine_mg_dl', 'Creatinine (mg/dL)']], sex: true },
		egfr: { label: 'eGFR (CKD-EPI 2021)', fields: [['serum_creatinine_mg_dl', 'Creatinine (mg/dL)'], ['age_years', 'Age (years)']], sex: true },
		'anion-gap': { label: 'Anion gap', fields: [['sodium_mmol_l', 'Sodium (mmol/L)'], ['chloride_mmol_l', 'Chloride (mmol/L)'], ['bicarbonate_mmol_l', 'Bicarbonate (mmol/L)']] },
		'corrected-calcium': { label: 'Corrected calcium', fields: [['calcium_mg_dl', 'Calcium (mg/dL)'], ['albumin_g_dl', 'Albumin (g/dL)']] }
	};
	const UNIT_PAIRS = {
		general: [
			['kg|lb', 'kg → lb'], ['lb|kg', 'lb → kg'], ['m|cm', 'm → cm'],
			['cm|m', 'cm → m'], ['cm|in', 'cm → in'], ['in|cm', 'in → cm'],
			['L|mL', 'L → mL'], ['mL|L', 'mL → L'], ['°C|°F', '°C → °F'], ['°F|°C', '°F → °C']
		],
		glucose: [['mg/dL|mmol/L', 'mg/dL → mmol/L'], ['mmol/L|mg/dL', 'mmol/L → mg/dL']]
	};
	const TUTORING_PROMPTS = [
		['explain', 'Explain simply'],
		['why_wrong', 'Explain the tempting alternative'],
		['compare', 'Compare'],
		['mnemonic', 'Mnemonic'],
		['test_me', 'Test me']
	];
	const item = $derived(session?.items?.[current] ?? null);
	const activeTutoringCard = $derived(
		item?.tutoring_cards?.find((card) => card.prompt_type === activeTutoringType) ?? null
	);
	const activeTest = $derived(
		activeTutoringCard?.prompt_type === 'test_me'
			? splitTestCard(activeTutoringCard.content, item?.key_learning_point ?? '')
			: null
	);
	const allAnswered = $derived(
		session?.items ? session.items.every((i, index) => i.answered || pendingAnswers[index]) : false
	);
	const watermarkLabel = $derived(
		session?.user_id
			? `Account ${session.user_id}`
			: session?.session_id
				? `Session ${session.session_id}`
				: ''
	);
	const marked = $derived(item ? Boolean(markState[item.question_version_id]) : false);
	const questionNote = $derived(item ? (noteByVersion[item.question_version_id] ?? '') : '');
	const questionHighlights = $derived(item ? (highlights[item.question_version_id] ?? []) : []);
	const itemEliminated = $derived(item ? (eliminated[item.question_version_id] ?? {}) : {});
	const calculatorDefinition = $derived(CALCULATORS[selectedCalculator]);
	const deferredFeedback = $derived(session?.preset === 'mock' || session?.preset === 'timed');

	// EX-08: a monotonic browser clock counts down from the server-issued
	// deadline; changing the wall clock while a session is open has no effect.
	let remainingMs = $state(null);
	let autoSubmitted = $state(false);
	let timerBaseRemainingMs = null;
	let timerStartedAt = null;
	let currentItemStartedAt = null;
	let elapsedByItem = $state({});
	let lastPersistedSecond = null;
	const CLOCK_CHANGE_THRESHOLD_MS = 10_000;
	const MAX_CLOCK_SAMPLE_GAP_MS = 60_000;
	let clockOffsetBaselineMs = null;
	let clockSampleAtMs = null;

	/**
	 * @param {'background' | 'foreground' | 'window_blur' | 'fullscreen_exit' | 'clock_change'} signalType
	 * @param {Record<string, number>} [detail]
	 */
	async function recordIntegritySignal(signalType, detail = {}) {
		if (!integrityActive || (!session?.deadline && session?.preset !== 'mock') || session?.status !== 'open' || result) return;
		try {
			const response = await Api.recordIntegrityEvent({
				session_id: sid,
				signal_type: signalType,
				detail,
				client_time: new Date().toISOString()
			});
			if (!integrityActive) return;
			if (response.action === 'warn') {
				const seconds = Math.max(1, response.away_seconds ?? 0);
				integrityWarning = `You were away for ${seconds} seconds. This assessment remains open; check your connection before continuing.`;
			} else if (response.action === 'auto_submitted' && response.receipt) {
				integrityAutoSubmitted = true;
				session.status = 'submitted';
				result = response.receipt;
				pendingSubmission = false;
				persistDraft();
			}
		} catch {
			// Integrity logging must never interrupt answering or submission.
		}
	}

	function checkDeviceClock() {
		if (document.visibilityState !== 'visible') {
			clockOffsetBaselineMs = null;
			clockSampleAtMs = null;
			return;
		}
		const monotonicNow = performance.now();
		const clockOffsetMs = Date.now() - monotonicNow;
		if (
			clockOffsetBaselineMs === null ||
			clockSampleAtMs === null ||
			monotonicNow - clockSampleAtMs > MAX_CLOCK_SAMPLE_GAP_MS
		) {
			clockOffsetBaselineMs = clockOffsetMs;
			clockSampleAtMs = monotonicNow;
			return;
		}

		const clockDeltaMs = Math.round(clockOffsetMs - clockOffsetBaselineMs);
		if (Math.abs(clockDeltaMs) >= CLOCK_CHANGE_THRESHOLD_MS) {
			recordIntegritySignal('clock_change', { clock_delta_ms: clockDeltaMs });
			clockOffsetBaselineMs = clockOffsetMs;
		}
		clockSampleAtMs = monotonicNow;
	}

	function warnIfThresholdCrossed(previous, next) {
		const thresholds = [[600_000, '10 minutes'], [300_000, '5 minutes'], [60_000, '1 minute']];
		if (next <= 0) {
			timerWarning = 'Time expired. This session is submitting now.';
			return;
		}
		let warning = '';
		if (previous === null && next <= 600_000) {
			warning = [...thresholds].reverse().find(([threshold]) => next <= threshold)?.[1] ?? '';
		} else {
			for (const [threshold, label] of thresholds) {
				if (previous > threshold && next <= threshold) warning = label;
			}
		}
		if (warning) {
			timerWarning = `${warning} remaining. This session submits automatically when time expires.`;
			if (typeof navigator !== 'undefined' && navigator.vibrate) navigator.vibrate(45);
		}
	}

	function anchorTimer(remote) {
		if (!remote?.deadline || !remote?.server_now) return;
		const previous = remainingMs;
		const monotonicNow = performance.now();
		if (clockOffsetBaselineMs === null) {
			clockOffsetBaselineMs = Date.now() - monotonicNow;
			clockSampleAtMs = monotonicNow;
		}
		timerBaseRemainingMs = new Date(remote.deadline).getTime() - new Date(remote.server_now).getTime();
		timerStartedAt = monotonicNow;
		remainingMs = Math.max(0, timerBaseRemainingMs);
		lastPersistedSecond = Math.ceil(remainingMs / 1000);
		warnIfThresholdCrossed(previous, remainingMs);
	}

	function tick() {
		if (!session?.deadline) return;
		checkDeviceClock();
		if (timerStartedAt === null) anchorTimer(session);
		if (timerStartedAt === null) return;
		const previous = remainingMs;
		remainingMs = timerBaseRemainingMs - (performance.now() - timerStartedAt);
		warnIfThresholdCrossed(previous, remainingMs);
		const second = Math.ceil(Math.max(0, remainingMs) / 1000);
		if (second !== lastPersistedSecond) {
			lastPersistedSecond = second;
			if (draftReady) persistDraft();
		}
		if (remainingMs <= 0 && !autoSubmitted && !result && session.status === 'open') {
			autoSubmitted = true;
			void submitSession();
		}
	}

	function fmt(ms) {
		const total = Math.max(0, Math.floor(ms / 1000));
		const m = Math.floor(total / 60);
		const s = total % 60;
		return `${m}:${String(s).padStart(2, '0')}`;
	}

	function storageKey(index) {
		return `mlos_key_${sid}_${index}`;
	}

	function draftKey() {
		return `mlos_session_${sid}`;
	}

	function readDraft() {
		try {
			return JSON.parse(localStorage.getItem(draftKey()) ?? 'null');
		} catch {
			return null;
		}
	}

	function persistDraft() {
		if (!session) return;
		try {
			localStorage.setItem(
				draftKey(),
				JSON.stringify({
					session,
					current,
					selected,
					textSize,
					markState,
					noteByVersion,
					noteRecords,
					pendingAnswers,
					unacceptedAnswers,
					pendingMarks,
					pendingNotes,
					result,
					highlights,
					eliminated,
					hintByVersion,
					elapsedByItem,
					pendingSubmission,
					remainingMs
				})
			);
			localStorage.setItem('mlos_session_text_size', String(textSize));
		} catch {
			offlineMessage = 'This browser could not save the session draft on this device.';
		}
	}

	function idempotencyKey(index) {
		let v = localStorage.getItem(storageKey(index));
		if (!v) {
			v = crypto.randomUUID();
			localStorage.setItem(storageKey(index), v);
		}
		return v;
	}

	async function load() {
		const cached = readDraft();
		if (cached) {
			current = Number.isInteger(cached.current) ? cached.current : 0;
			selected = cached.selected ?? null;
			textSize = Number.isInteger(cached.textSize) ? Math.min(3, Math.max(0, cached.textSize)) : 0;
			markState = cached.markState ?? {};
			noteByVersion = cached.noteByVersion ?? {};
			noteRecords = cached.noteRecords ?? {};
			pendingAnswers = cached.pendingAnswers ?? {};
			unacceptedAnswers = cached.unacceptedAnswers ?? {};
			pendingMarks = cached.pendingMarks ?? {};
			pendingNotes = cached.pendingNotes ?? {};
			highlights = cached.highlights ?? {};
			eliminated = cached.eliminated ?? {};
			hintByVersion = cached.hintByVersion ?? {};
			elapsedByItem = cached.elapsedByItem ?? {};
			pendingSubmission = cached.pendingSubmission ?? false;
			result = cached.result ?? null;
			if (typeof cached.remainingMs === 'number') remainingMs = cached.remainingMs;
		}
		try {
			session = await Api.getSession(sid);
		} catch (err) {
			if (!navigator.onLine && cached?.session) {
				session = cached.session;
				offlineMessage = 'Working from the saved session on this device. Changes will sync when you reconnect.';
			} else {
				loadFailed = err instanceof ApiError ? err.message : 'Could not load the session.';
				return;
			}
		}
		for (const [index, queued] of Object.entries(pendingAnswers)) {
			const itemIndex = Number(index);
			if (session.items[itemIndex] && !session.items[itemIndex].answered) {
				session.items[itemIndex] = {
					...session.items[itemIndex],
					answered: true,
					chosen_index: queued.chosen_index
				};
			}
		}
		if (!cached || !Number.isInteger(cached.current)) {
			const firstUnanswered = session.items.findIndex((i) => !i.answered);
			current = firstUnanswered === -1 ? Math.max(0, session.items.length - 1) : firstUnanswered;
		}
		current = Math.min(Math.max(0, current), Math.max(0, session.items.length - 1));
		if (session.deadline && session.server_now) anchorTimer(session);
		else if (session.deadline && remainingMs !== null) {
			timerBaseRemainingMs = remainingMs;
			timerStartedAt = performance.now();
			lastPersistedSecond = Math.ceil(remainingMs / 1000);
			warnIfThresholdCrossed(null, remainingMs);
		}
		currentItemStartedAt = performance.now();
		draftReady = true;
		try {
			const [marksResult, notesResult] = await Promise.allSettled([Api.myMarks(), Api.listNotes()]);
			if (marksResult.status === 'fulfilled') {
				const fromServer = Object.fromEntries(
					marksResult.value.marks.map((mark) => [mark.question_version_id, true])
				);
				for (const versionId of Object.keys(pendingMarks)) {
					fromServer[versionId] = markState[versionId];
				}
				markState = fromServer;
			}
			if (notesResult.status === 'fulfilled') {
				const fromServer = {};
				const records = {};
				for (const note of notesResult.value.notes) {
					if (!note.source_question_version_id) continue;
					fromServer[note.source_question_version_id] = note.body;
					records[note.source_question_version_id] = {
						note_id: note.note_id,
						updated_at: note.updated_at
					};
				}
				for (const versionId of Object.keys(pendingNotes)) {
					fromServer[versionId] = noteByVersion[versionId];
					if (noteRecords[versionId]) records[versionId] = noteRecords[versionId];
				}
				noteByVersion = fromServer;
				noteRecords = records;
			}
			persistDraft();
		} catch {
			// Session progress remains usable when note or mark refresh is unavailable.
		}
		if (session.deadline) tick();
		if (!result && session.status === 'submitted' && navigator.onLine) {
			try {
				result = await Api.submit(sid);
				persistDraft();
			} catch (err) {
				loadFailed = err instanceof ApiError ? err.message : 'Could not recover the submitted result.';
				return;
			}
		}
		if (navigator.onLine) void syncPending();
	}

	function setAnswerResult(index, chosen, response) {
		if (deferredFeedback) {
			session.items[index] = { ...session.items[index], answered: true, chosen_index: chosen };
		} else {
			session.items[index] = {
				...session.items[index],
				answered: true,
				chosen_index: chosen,
				correct: response.correct,
				correct_index: response.correct_index,
				options: response.options,
				key_learning_point: response.key_learning_point,
				exam_tip: response.exam_tip,
				tutoring_cards: response.tutoring_cards ?? []
			};
		}
		const next = { ...pendingAnswers };
		delete next[index];
		pendingAnswers = next;
		selected = null;
		persistDraft();
	}

	function elapsedFor(index) {
		const active = index === current && currentItemStartedAt !== null
			? Math.max(0, performance.now() - currentItemStartedAt)
			: 0;
		return Math.min(3_600_000, Math.round((elapsedByItem[index] ?? 0) + active));
	}

	function navigateTo(index) {
		if (!session || index < 0 || index >= session.items.length) return;
		if (currentItemStartedAt !== null) {
			elapsedByItem = {
				...elapsedByItem,
				[current]: Math.min(3_600_000, (elapsedByItem[current] ?? 0) + performance.now() - currentItemStartedAt)
			};
		}
		current = index;
		selected = null;
		activeTutoringType = '';
		testAnswerRevealed = false;
		eliminating = false;
		currentItemStartedAt = performance.now();
		persistDraft();
	}

	function splitTestCard(content, fallbackAnswer) {
		const marker = '\nAnswer: ';
		const answerStart = content.indexOf(marker);
		if (answerStart < 0) {
			return {
				prompt: 'Recall: State the key learning point for this question.',
				answer: fallbackAnswer
			};
		}
		return {
			prompt: content.slice(0, answerStart),
			answer: content.slice(answerStart + marker.length)
		};
	}

	function selectOption(index) {
		if (item?.answered || busy || pendingAnswers[current]) return;
		if (eliminating) {
			toggleEliminated(index);
			return;
		}
		selected = selected === index ? null : index;
		persistDraft();
	}

	function toggleEliminated(index) {
		if (!item) return;
		const versionId = item.question_version_id;
		const choices = { ...(eliminated[versionId] ?? {}) };
		choices[index] = !choices[index];
		eliminated = { ...eliminated, [versionId]: choices };
		persistDraft();
	}

	async function answer(chosen) {
		if (!item || item.answered || busy || pendingAnswers[current]) return;
		const index = current;
		const pending = {
			item_index: index,
			chosen_index: chosen,
			idempotency_key: idempotencyKey(index),
			elapsed_ms: elapsedFor(index),
			assisted: Boolean(item.hint_used),
			client_recorded_at: new Date().toISOString()
		};
		pendingAnswers = { ...pendingAnswers, [index]: pending };
		session.items[index] = { ...session.items[index], answered: true, chosen_index: chosen };
		selected = null;
		error = '';
		persistDraft();
		if (!navigator.onLine) {
			offlineMessage = 'Answer saved on this device. It will sync when you reconnect.';
			return;
		}
		await syncPending();
	}

	async function syncPending() {
		if (syncingDraft || !session || !navigator.onLine) return;
		syncingDraft = true;
		try {
			for (const index of Object.keys(pendingAnswers)) {
				const queued = pendingAnswers[index];
				busy = true;
				try {
					const response = await Api.answer(sid, queued);
					setAnswerResult(Number(index), queued.chosen_index, response);
					offlineMessage = '';
				} catch (err) {
					if (err instanceof ApiError && err.code === 'session_expired') {
						const rejected = { ...unacceptedAnswers, [index]: queued };
						unacceptedAnswers = rejected;
						const next = { ...pendingAnswers };
						delete next[index];
						pendingAnswers = next;
						session.items[Number(index)] = {
							...session.items[Number(index)],
							answered: false,
							chosen_index: null
						};
						offlineMessage = 'A saved answer arrived after the 10-minute practice sync window. It stays on this device and counts as unanswered.';
						persistDraft();
						continue;
					}
					error = err instanceof ApiError ? err.message : 'Answer is saved on this device and will sync when the connection returns.';
					offlineMessage = err instanceof ApiError ? '' : 'Answer saved on this device. It will sync when you reconnect.';
					break;
				} finally {
					busy = false;
				}
			}
			for (const versionId of Object.keys(pendingMarks)) {
				try {
					if (pendingMarks[versionId]) await Api.markQuestion(versionId);
					else await Api.unmarkQuestion(versionId);
					const next = { ...pendingMarks };
					delete next[versionId];
					pendingMarks = next;
				} catch {
					break;
				}
			}
			for (const versionId of Object.keys(pendingNotes)) {
				await saveQuestionNote(versionId);
				if (pendingNotes[versionId]) break;
			}
			if (pendingSubmission && Object.keys(pendingAnswers).length === 0) {
				pendingSubmission = false;
				persistDraft();
				await sendSubmission();
			}
		} finally {
			syncingDraft = false;
		}
	}

	async function submitSession() {
		if (submitting || result) return;
		pendingSubmission = true;
		persistDraft();
		if (!navigator.onLine) {
			offlineMessage = 'Session closed on this device. Submission and scoring will sync when you reconnect.';
			return;
		}
		await syncPending();
	}

	async function sendSubmission() {
		if (submitting || !navigator.onLine) return;
		submitting = true;
		error = '';
		try {
			result = await Api.submit(sid);
			offlineMessage = '';
			pendingSubmission = false;
			persistDraft();
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
				pendingSubmission = false;
			} else {
				offlineMessage = 'Session closed on this device. Submission and scoring will sync when you reconnect.';
			}
		} finally {
			submitting = false;
			persistDraft();
		}
	}

	function editQuestionNote(value) {
		if (!item) return;
		const versionId = item.question_version_id;
		noteByVersion = { ...noteByVersion, [versionId]: value };
		pendingNotes = { ...pendingNotes, [versionId]: true };
		noteSaveState = { ...noteSaveState, [versionId]: 'Saved on this device' };
		persistDraft();
		scheduleNoteSave(versionId);
	}

	function scheduleNoteSave(versionId, delay = 450) {
		if (noteTimers.has(versionId)) clearTimeout(noteTimers.get(versionId));
		noteTimers.set(versionId, setTimeout(() => {
			noteTimers.delete(versionId);
			void saveQuestionNote(versionId);
		}, delay));
	}

	async function saveQuestionNote(versionId) {
		if (noteInFlight.has(versionId)) return;
		const body = noteByVersion[versionId] ?? '';
		const existing = noteRecords[versionId];
		if (!body.trim() && !existing) {
			const next = { ...pendingNotes };
			delete next[versionId];
			pendingNotes = next;
			noteSaveState = { ...noteSaveState, [versionId]: 'Saved' };
			persistDraft();
			return;
		}
		if (!navigator.onLine) {
			pendingNotes = { ...pendingNotes, [versionId]: true };
			noteSaveState = { ...noteSaveState, [versionId]: 'Saved on this device — waiting to sync' };
			persistDraft();
			return;
		}
		noteInFlight.add(versionId);
		try {
			let record;
			if (existing) {
				const updated = await Api.updateNote(existing.note_id, {
					body,
					base_updated_at: existing.updated_at
				});
				record = { ...existing, updated_at: updated.updated_at };
			} else {
				const created = await Api.createNote('', body, versionId);
				record = { note_id: created.note_id, updated_at: created.updated_at };
			}
			noteRecords = { ...noteRecords, [versionId]: record };
			if (noteByVersion[versionId] === body) {
				const next = { ...pendingNotes };
				delete next[versionId];
				pendingNotes = next;
				noteSaveState = { ...noteSaveState, [versionId]: 'Saved' };
			} else {
				pendingNotes = { ...pendingNotes, [versionId]: true };
				noteSaveState = { ...noteSaveState, [versionId]: 'Saving…' };
				scheduleNoteSave(versionId, 0);
			}
			persistDraft();
		} catch (err) {
			if (err instanceof ApiError && err.code === 'note_conflict') {
				const next = { ...pendingNotes };
				delete next[versionId];
				pendingNotes = next;
				noteSaveState = { ...noteSaveState, [versionId]: 'Conflict — review this note in your notebook' };
			} else {
				pendingNotes = { ...pendingNotes, [versionId]: true };
				noteSaveState = { ...noteSaveState, [versionId]: 'Saved on this device — waiting to sync' };
			}
			persistDraft();
		} finally {
			noteInFlight.delete(versionId);
		}
	}

	async function toggleMark() {
		if (!item) return;
		const versionId = item.question_version_id;
		const nextMarked = !Boolean(markState[versionId]);
		markState = { ...markState, [versionId]: nextMarked };
		pendingMarks = { ...pendingMarks, [versionId]: nextMarked };
		persistDraft();
		if (!navigator.onLine) {
			offlineMessage = 'Mark saved on this device. It will sync when you reconnect.';
			return;
		}
		await syncPending();
	}

	function chooseCalculator(kind) {
		selectedCalculator = kind;
		calculatorInputs = {};
		calculatorResult = '';
		calculatorError = '';
	}

	async function runCalculator() {
		calculatorError = '';
		calculatorResult = '';
		calculating = true;
		const inputs = Object.fromEntries(
			calculatorDefinition.fields.map(([key]) => [key, Number(calculatorInputs[key])])
		);
		if (calculatorDefinition.sex) inputs.female = Boolean(calculatorInputs.female);
		try {
			const response = await Api.calculate(selectedCalculator, inputs);
			const value = Number.isInteger(response.value) ? String(response.value) : response.value.toFixed(2);
			calculatorResult = `${value} ${response.unit}`;
		} catch (err) {
			calculatorError = err instanceof ApiError ? err.message : 'Calculator unavailable. Reconnect and try again.';
		} finally {
			calculating = false;
		}
	}

	async function runConversion() {
		converterError = '';
		converterResult = '';
		const [from, to] = converterPair.split('|');
		try {
			const response = await Api.convertUnits({
				value: Number(converterValue),
				analyte: converterAnalyte === 'general' ? '' : converterAnalyte,
				from,
				to
			});
			const value = Number.isInteger(response.value) ? String(response.value) : response.value.toFixed(3);
			converterResult = `${value} ${response.unit}`;
		} catch (err) {
			converterError = err instanceof ApiError ? err.message : 'Converter unavailable. Reconnect and try again.';
		}
	}

	async function requestHint() {
		if (!item?.hint_available || session.preset !== 'tutor' || hintBusy) return;
		const requestedItem = item;
		const requestedIndex = current;
		hintBusy = true;
		error = '';
		try {
			const response = await Api.getSessionHint(sid, requestedIndex);
			hintByVersion = { ...hintByVersion, [requestedItem.question_version_id]: response.hint };
			session.items[requestedIndex] = { ...session.items[requestedIndex], hint_used: true };
			if (current === requestedIndex) hintText = response.hint;
			persistDraft();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Hint unavailable while offline.';
		} finally {
			hintBusy = false;
		}
	}

	function addHighlight() {
		if (!item) return;
		const text = window.getSelection()?.toString().trim();
		if (!text) return;
		const versionId = item.question_version_id;
		highlights = {
			...highlights,
			[versionId]: [...(highlights[versionId] ?? []), text].slice(0, 30)
		};
		window.getSelection()?.removeAllRanges();
		persistDraft();
	}

	function onNetworkOnline() {
		offlineMessage = '';
		void syncPending();
	}

	async function submitReport() {
		if (reportBusy || !item) return;
		reportBusy = true;
		reportError = '';
		try {
			const res = await Api.reportQuestion(item.question_version_id, {
				category: reportCategory,
				note: reportNote.trim() ? reportNote.trim() : undefined
			});
			const acknowledgedAt = new Date(res.acknowledged_at).toLocaleString();
			const acknowledgementDueAt = new Date(res.acknowledgement_due_at).toLocaleString();
			const resolutionDueAt = new Date(res.resolution_due_at).toLocaleString();
			session.items[current] = {
				...session.items[current],
				report_status: res.status,
				my_report: {
					status: res.status,
					resolution_note: res.resolution_note,
					correction_note: res.correction_note,
					resolved_at: res.resolved_at,
					corrected_version_id: res.corrected_version_id,
					corrected_version_number: res.corrected_version_number,
					acknowledged_at: res.acknowledged_at,
					acknowledgement_due_at: res.acknowledgement_due_at,
					resolution_due_at: res.resolution_due_at,
					resolution_overdue: false
				}
			};
			reportDone = `${res.already_recorded ? 'Your report is already on file.' : 'Thanks — report received and acknowledged'} ${acknowledgedAt}. Acknowledgement target: ${acknowledgementDueAt}. Resolution target: ${resolutionDueAt}.`;
			reportOpen = false;
			reportNote = '';
		} catch (err) {
			reportError =
				err instanceof ApiError ? err.message : 'Could not send the report.';
		} finally {
			reportBusy = false;
		}
	}

	// Fresh report panel per question — navigating never leaks state.
	$effect(() => {
		current;
		reportOpen = false;
		reportDone = '';
		reportError = '';
		reportNote = '';
		hintText = item ? (hintByVersion[item.question_version_id] ?? '') : '';
	});

	function onKeydown(event) {
		if (result || !session || !item) return;
		// Browser/OS shortcuts (Ctrl+C, Ctrl+A, ⌘F…) must never pick an answer.
		if (event.metaKey || event.ctrlKey || event.altKey) return;
		if (event.target instanceof HTMLElement && event.target.closest('input, textarea, select, [contenteditable="true"]')) return;
		const key = event.key.toLowerCase();
		if (key === 'e' && !item.answered) {
			eliminating = !eliminating;
			return;
		}
		if (key === 'h' && session.preset === 'tutor' && item.hint_available && !item.answered) {
			void requestHint();
			return;
		}
		if (key === 'f') {
			void toggleMark();
			return;
		}
		if (key === 'p' && current > 0) {
			navigateTo(current - 1);
			return;
		}
		const letter = 'abcdefghij'.indexOf(event.key.toLowerCase());
		if (letter >= 0 && !item.answered && letter < item.options.length) {
			if (eliminating) toggleEliminated(letter);
			else {
				selected = letter;
				persistDraft();
			}
		} else if (event.key === 'Enter' || event.key === 'n' || event.key === 'N') {
			if (!item.answered && selected !== null) {
				void answer(selected);
			} else if (item.answered && current < session.items.length - 1) {
				navigateTo(current + 1);
			} else if (item.answered && allAnswered) {
				void submitSession();
			}
		}
	}

	$effect(() => {
		if (!session?.deadline) return;
		const timer = setInterval(tick, 500);
		return () => clearInterval(timer);
	});

	function letterLabel(index) {
		return String.fromCharCode(65 + index);
	}

	onMount(() => {
		integrityActive = true;
		void load();
		window.addEventListener('online', onNetworkOnline);
		const handleVisibilityChange = async () => {
			if (document.visibilityState === 'hidden') {
				recordIntegritySignal('background');
				clockOffsetBaselineMs = null;
				clockSampleAtMs = null;
				return;
			}
			if (document.visibilityState !== 'visible') return;
			recordIntegritySignal('foreground');
			if (!session?.deadline || !navigator.onLine) return;
			try {
				const remote = await Api.getSession(sid);
				anchorTimer(remote);
				tick();
			} catch {
				// The monotonic local timer continues through a temporary outage.
			}
		};
		let wasFullscreen = Boolean(document.fullscreenElement);
		const reportFullscreenExit = () => {
			const fullscreen = Boolean(document.fullscreenElement);
			if (wasFullscreen && !fullscreen) recordIntegritySignal('fullscreen_exit');
			wasFullscreen = fullscreen;
		};
		const reportWindowBlur = () => recordIntegritySignal('window_blur');
		const reportWindowFocus = () => recordIntegritySignal('foreground');
		document.addEventListener('visibilitychange', handleVisibilityChange);
		document.addEventListener('fullscreenchange', reportFullscreenExit);
		window.addEventListener('blur', reportWindowBlur);
		window.addEventListener('focus', reportWindowFocus);
		return () => {
			integrityActive = false;
			window.removeEventListener('online', onNetworkOnline);
			document.removeEventListener('visibilitychange', handleVisibilityChange);
			document.removeEventListener('fullscreenchange', reportFullscreenExit);
			window.removeEventListener('blur', reportWindowBlur);
			window.removeEventListener('focus', reportWindowFocus);
			for (const timer of noteTimers.values()) clearTimeout(timer);
		};
	});
</script>

<svelte:window onkeydown={onKeydown} ontouchstart={touchStart} ontouchend={touchEnd} />

<svelte:head>
	{#if focusMode}
		<style>
			header.top,
			nav {
				display: none !important;
			}
		</style>
	{/if}
</svelte:head>

<div class="session-workspace" data-testid="session-workspace" style={`--session-font-scale: ${[1, 1.125, 1.25, 1.5][textSize]};`}>
{#if !loadFailed && !result && session}
	<div class="card session-toolbar">
		<button
			class="btn {focusMode ? 'primary' : ''}"
			type="button"
			data-testid="focus-toggle"
			onclick={toggleFocus}
		>
			{focusMode ? 'Exit Focus Mode' : 'Focus Mode'}
		</button>
		<button class="btn" type="button" data-testid="fullscreen-toggle" onclick={toggleFullscreen}>
			{typeof document !== 'undefined' && document.fullscreenElement
				? 'Exit fullscreen'
				: 'Fullscreen'}
		</button>
		<button class="btn" type="button" data-testid="tools-toggle" aria-expanded={toolsOpen} onclick={() => (toolsOpen = !toolsOpen)}>
			{toolsOpen ? 'Hide tools' : 'Tools'}
		</button>
		<div class="text-size-control" role="group" aria-label="Text size">
			<span class="muted">Text size</span>
			{#each ['Default', 'Large', 'Larger', 'Largest'] as label, index}
				<button
					class="btn {textSize === index ? 'primary' : ''}"
					type="button"
					aria-pressed={textSize === index}
					data-testid={`text-size-${index}`}
					onclick={() => {
						textSize = index;
						persistDraft();
					}}
				>
					{label}
				</button>
			{/each}
		</div>
		<span class="muted small toolbar-hint">
			In focus mode, swipe left or right to move between answered items.
		</span>
	</div>
{/if}

{#if timerWarning && !result}
	<p class="feedback timer-warning" role="alert" data-testid="timer-warning">{timerWarning}</p>
{/if}
{#if integrityWarning && !result}
	<p class="feedback timer-warning" role="status" data-testid="integrity-warning">{integrityWarning}</p>
{/if}
{#if offlineMessage && !result}
	<p class="feedback" role="status" data-testid="offline-status">{offlineMessage}</p>
{/if}

{#if session && item && toolsOpen && !result}
	<section class="card session-tools" aria-label="Session tools" data-testid="tool-tray">
		<h2>Session tools</h2>
		<div class="tool-grid">
			<section class="tool-panel" aria-label="Medical calculator">
				<label class="field">
					<span>Medical calculator</span>
					<select value={selectedCalculator} onchange={(event) => chooseCalculator(event.currentTarget.value)}>
						{#each Object.entries(CALCULATORS) as [key, calculator]}
							<option value={key}>{calculator.label}</option>
						{/each}
					</select>
				</label>
				{#each calculatorDefinition.fields as [key, label]}
					<label class="field">
						<span>{label}</span>
						<input
							type="number"
							step="any"
							value={calculatorInputs[key] ?? ''}
							data-testid={`calc-input-${key}`}
							oninput={(event) => {
								calculatorInputs = { ...calculatorInputs, [key]: event.currentTarget.value };
							}}
						/>
					</label>
				{/each}
				{#if calculatorDefinition.sex}
					<label class="check-field">
						<input
							type="checkbox"
							checked={Boolean(calculatorInputs.female)}
							onchange={(event) => {
								calculatorInputs = { ...calculatorInputs, female: event.currentTarget.checked };
							}}
						/>
						Use the formula's female factor
					</label>
				{/if}
				<button class="btn primary" type="button" disabled={calculating} data-testid="calculate" onclick={runCalculator}>
					{calculating ? 'Calculating…' : 'Calculate'}
				</button>
				{#if calculatorResult}<p class="feedback" data-testid="calculator-result">{calculatorResult}</p>{/if}
				{#if calculatorError}<p class="error-text" role="alert">{calculatorError}</p>{/if}
				<p class="muted" data-testid="calculator-disclaimer">For exam practice only; not for clinical use.</p>
			</section>

			<section class="tool-panel" aria-label="Unit converter">
				<h3>Unit converter</h3>
				<label class="field">
					<span>Conversion type</span>
					<select
						value={converterAnalyte}
						onchange={(event) => {
							converterAnalyte = event.currentTarget.value;
							converterPair = UNIT_PAIRS[converterAnalyte][0][0];
							converterResult = '';
						}}
					>
						<option value="general">General units</option>
						<option value="glucose">Glucose lab units</option>
					</select>
				</label>
				<label class="field">
					<span>Units</span>
					<select bind:value={converterPair}>
						{#each UNIT_PAIRS[converterAnalyte] as [value, label]}
							<option {value}>{label}</option>
						{/each}
					</select>
				</label>
				<label class="field">
					<span>Value</span>
					<input type="number" step="any" bind:value={converterValue} data-testid="conversion-value" />
				</label>
				<button class="btn" type="button" data-testid="convert" onclick={runConversion}>Convert</button>
				{#if converterResult}<p class="feedback" data-testid="conversion-result">{converterResult}</p>{/if}
				{#if converterError}<p class="error-text" role="alert">{converterError}</p>{/if}
			</section>

			<section class="tool-panel" aria-label="Question workspace">
				<h3>Question workspace</h3>
				<button class="btn" type="button" data-testid="question-mark" aria-pressed={marked} onclick={toggleMark}>
					{marked ? 'Remove mark' : 'Mark question'}
				</button>
				{#if pendingMarks[item.question_version_id]}
					<p class="muted">Mark saved on this device; waiting to sync.</p>
				{/if}
				<label class="field">
					<span>Private note for this question</span>
					<textarea
						rows="4"
						value={questionNote}
						data-testid="question-note"
						oninput={(event) => editQuestionNote(event.currentTarget.value)}
					></textarea>
				</label>
				<p class="muted" role="status" data-testid="note-save-state">{noteSaveState[item.question_version_id] ?? 'Saved'}</p>
				<button class="btn" type="button" data-testid="highlight-selection" onclick={addHighlight}>Save selected text</button>
				{#each questionHighlights as highlight, index}
					<p class="highlight-note" data-testid="saved-highlight-{index}">“{highlight}”</p>
				{/each}
				{#if session.preset === 'tutor' && item.hint_available && !item.answered}
					<button class="btn" type="button" disabled={hintBusy} data-testid="show-hint" onclick={requestHint}>
						{hintBusy ? 'Opening hint…' : 'Show reasoning hint'}
					</button>
				{/if}
				{#if hintText}<p class="feedback" data-testid="tutor-hint">{hintText}</p>{/if}
			</section>
		</div>
	</section>
{/if}

{#if loadFailed}
	<p class="error-text" role="alert">{loadFailed}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if result}
	<div class="card results-card" data-testid="results">
		<h1>Session submitted</h1>
		{#if integrityAutoSubmitted}
			<p class="feedback" role="status" data-testid="integrity-auto-submitted">
				This session was submitted after reaching its configured time-away limit.
			</p>
		{/if}
		<div class="stat-row">
			<span>Score<strong data-testid="score">{result.score}%</strong></span>
			<span>Correct<strong>{result.correct}</strong></span>
			<span>Incorrect<strong>{result.incorrect}</strong></span>
			<span>Skipped<strong>{result.skipped}</strong></span>
		</div>
		{#if result.mock}
			<div class="feedback {result.mock.passed ? 'good' : 'bad'}" data-testid="mock-result">
				<p class="verdict">{result.mock.passed ? 'Passed' : 'Not passed'} — mark was {result.mock.pass_mark_percent}%</p>
				{#if result.mock.ranked === false}
					<p class="muted" data-testid="mock-unranked-note">
						This personal result includes late offline answers, so it is excluded from percentile ranking.
					</p>
				{:else if result.mock.percentile === null}
					<p class="muted">Percentile appears once at least {result.mock.takers < 20 ? 20 : result.mock.takers} learners have taken this mock — nothing is invented meanwhile.</p>
				{:else}
					<p>You are ahead of {result.mock.percentile}% of takers of this same form.</p>
				{/if}
				{#if result.mock.total_time_seconds !== undefined}
					<p class="muted tight" data-testid="mock-time-analysis">
						Total time: {Math.floor(result.mock.total_time_seconds / 60)}m {result.mock.total_time_seconds % 60}s
						{#if result.mock.avg_time_per_question_seconds !== undefined}
							· {result.mock.avg_time_per_question_seconds}s/question avg
						{/if}
					</p>
				{/if}
				{#each result.mock.breakdown as row (row.chapter)}
					<p class="breakdown-row">
						{row.chapter}: {row.correct}/{row.total}
						{#if row.time_seconds !== undefined}
							<span class="muted">({Math.floor(row.time_seconds / 60)}m {row.time_seconds % 60}s)</span>
						{/if}
					</p>
				{/each}
			</div>
		{:else if result.expected_score !== null}
			<p class="muted" data-testid="expected">
				You scored {result.score}% — learners with history on these questions
				score about {result.expected_score}% on average.
			</p>
		{/if}
		<p class="muted">This result is about this form — it is not a prediction of anything.</p>
		<a class="btn primary" href={`${base}/today`} data-testid="back-today">Back to Today</a>
	</div>
{:else if session}
	<p class="muted session-progress">
		Question {current + 1} of {session.items.length}
		· {session.preset === 'revision' ? 'Re-practice' : session.preset === 'timed' ? 'Timed' : 'Tutor mode'}
		{#if remainingMs !== null}
			· <span
				class="timer"
				style:color={remainingMs < 60_000 ? 'var(--color-warning)' : 'inherit'}
				style:font-weight="700"
				data-testid="timer"
			>
				{fmt(remainingMs)} left
			</span>
		{/if}
	</p>
	<div class="meter session-meter" style:--value={(current + 1) / session.items.length}><span></span></div>

	{#if item}
		<div class="card question-card">
			<div class="watermarked-copy" data-testid="question-watermark">
				<span class="watermark-tiles" aria-hidden="true" data-watermark={watermarkLabel}></span>
				<p data-testid="question-text">{item.vignette}</p>
				<p><strong>{item.lead_in}</strong></p>
				{#if item.corrected}
					<p class="cluster">
						<span class="chip info" data-testid="corrected-badge">Corrected</span>
						{#if item.correction_note}
							<span class="muted" data-testid="correction-changelog">Update: {item.correction_note}</span>
						{/if}
					</p>
				{/if}
			</div>

			{#if eliminating}
				<p class="muted" role="status">Elimination mode is on. Select options to strike out; press E again to exit.</p>
			{/if}
			<div class="options">
				{#each item.options as option, i (i)}
					<div class="option-row">
						<button
							type="button"
							class="option {item.answered && i === item.correct_index ? 'correct' : ''}
								{item.answered && item.chosen_index === i && item.correct === false ? 'incorrect' : ''}
								{itemEliminated[i] ? 'eliminated' : ''}"
							aria-pressed={!item.answered && selected === i}
							disabled={item.answered || busy || Boolean(pendingAnswers[current])}
							data-testid={`option-${i}`}
							onclick={() => selectOption(i)}
						>
							<span class="key">{letterLabel(i)}</span>
							<span>{option.text}</span>
						</button>
						{#if !item.answered && session.preset !== 'mock'}
							<button
								class="linklike eliminate-option"
								type="button"
								aria-pressed={Boolean(itemEliminated[i])}
								data-testid={`eliminate-${i}`}
								disabled={Boolean(pendingAnswers[current])}
								onclick={() => toggleEliminated(i)}
							>
								{itemEliminated[i] ? 'Restore' : 'Eliminate'}
							</button>
						{/if}
					</div>
				{/each}
			</div>

			{#if error}
				<p class="error-text" role="alert">{error}</p>
			{/if}

			{#if !item.answered && deferredFeedback}
				<p class="muted small">
					Answers and explanations are revealed after you submit this assessment.
				</p>
			{/if}

			{#if !item.answered}
				<button
					class="btn primary"
					type="button"
					disabled={selected === null || busy || Boolean(pendingAnswers[current]) || (remainingMs !== null && remainingMs <= 0)}
					data-loading={busy}
					data-testid="answer"
					onclick={() => void answer(selected)}
				>
					{busy ? 'Recording…' : deferredFeedback ? 'Record answer' : 'Answer'}
				</button>
				{#if session.preset !== 'mock'}
				<button
					class="linklike"
					type="button"
					disabled={busy || (remainingMs !== null && remainingMs <= 0)}
					data-testid="skip"
					onclick={() => void answer(null)}
				>
					Skip — I don't want to guess
				</button>
				{/if}
			{:else if pendingAnswers[current]}
				<p class="muted" role="status" data-testid="offline-answer">
					Answer saved on this device. Feedback will appear after the server confirms the sync.
				</p>
				{#if current < session.items.length - 1}
					<button class="btn primary" type="button" data-testid="next" onclick={() => navigateTo(current + 1)}>
						Next
					</button>
				{:else}
					<button
						class="btn primary"
						type="button"
						disabled={submitting}
						data-loading={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						{submitting ? 'Submitting…' : 'Submit session'}
					</button>
				{/if}
			{:else if deferredFeedback}
				<p class="muted" data-testid="assessment-recorded">
						{item.chosen_index === null ? 'Recorded as skipped.' : 'Answer recorded.'}
						Nothing is revealed until submission.
					</p>
				{#if current < session.items.length - 1}
					<button
						class="btn primary"
						type="button"
						data-testid="next"
						onclick={() => navigateTo(current + 1)}
					>
						Next
					</button>
				{:else}
					<button
						class="btn primary"
						type="button"
						disabled={submitting}
						data-loading={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						{submitting ? 'Submitting…' : session.preset === 'mock' ? 'Submit mock' : 'Submit timed session'}
					</button>
				{/if}
			{:else if session.preset !== 'mock'}
				<div
					class="feedback {item.correct === true ? 'good' : item.correct === false ? 'bad' : ''}"
					data-testid="feedback"
				>
					<div class="watermarked-copy" data-testid="explanation-watermark">
						<span class="watermark-tiles" aria-hidden="true" data-watermark={watermarkLabel}></span>
						<p class="verdict">
							{item.correct === true ? 'Correct.' : item.correct === false ? 'Not quite.' : 'Skipped.'}
						</p>
						{#each item.options as option, i (i)}
							{#if option.rationale && (i === item.correct_index || i === item.chosen_index)}
								<p class="tight">
									<strong>{letterLabel(i)}.</strong> {option.rationale}
								</p>
							{/if}
						{/each}
						<p class="key-point">
							<strong>Key learning point:</strong> {item.key_learning_point}
						</p>
						{#if item.exam_tip}
							<p class="muted tight-top">Exam tip: {item.exam_tip}</p>
						{/if}
					</div>
					{#if session.preset === 'tutor' && item.tutoring_cards?.length}
						<div class="cluster tutoring-cards" role="group" aria-label="One-tap tutoring cards">
							{#each TUTORING_PROMPTS as [promptType, label] (promptType)}
								{#if promptType !== 'why_wrong' || item.correct === false}
									<button
									class="btn"
									type="button"
									aria-pressed={activeTutoringType === promptType}
									data-testid={`pregen-card-${promptType}`}
									onclick={() => {
										activeTutoringType = promptType;
										testAnswerRevealed = false;
										persistDraft();
									}}
									>
										{label}
									</button>
								{/if}
							{/each}
						</div>
						{#if activeTutoringCard}
							<div class="feedback">
								<div
									class="watermarked-copy"
									data-testid="tutor-card-content"
									role="status"
									aria-live="polite"
									aria-atomic="true"
								>
									<span class="watermark-tiles" aria-hidden="true" data-watermark={watermarkLabel}></span>
									{#if activeTutoringCard.prompt_type === 'why_wrong'}
										<p>Compare your selected answer with the keyed answer.</p>
										{#each item.options as option, i (i)}
											{#if i === item.chosen_index || i === item.correct_index}
												<p class="tight">
													<strong>{i === item.chosen_index ? 'Your answer' : 'Keyed answer'}: {option.text}</strong>
													{#if option.rationale} — {option.rationale}{/if}
												</p>
											{/if}
										{/each}
									{:else if activeTest}
										<p>{activeTest.prompt}</p>
										{#if testAnswerRevealed}
											<p><strong>Answer:</strong> {activeTest.answer}</p>
										{/if}
									{:else}
										<p>{activeTutoringCard.content}</p>
									{/if}
								</div>
								{#if activeTest && !testAnswerRevealed}
									<button
										class="btn"
										type="button"
										data-testid="reveal-tutor-answer"
										onclick={() => (testAnswerRevealed = true)}
									>Reveal answer</button>
								{/if}
								<p class="muted small">Source: {activeTutoringCard.source_ref}</p>
							</div>
						{/if}
					{/if}
					{#if item.report_status === 'quarantined'}
						<p class="muted small tight-top">
							Flagged by learners — out of rotation pending editorial review.
						</p>
					{:else if item.report_status === 'open'}
						<p class="muted small tight-top">
							Flagged by a learner — under review.
						</p>
					{:else if item.report_status === 'resolved_fixed'}
						<p class="muted small tight-top">
							This version was corrected. A newer reviewed version is in use.
						</p>
					{:else if item.report_status === 'resolved_rejected'}
						<p class="muted small tight-top">
							A report was reviewed; this version remains in use.
						</p>
					{/if}
				</div>

				{#if item.my_report}
					<div class="feedback" data-testid="my-report-status">
						<p class="verdict">Your report: {item.my_report.status.replaceAll('_', ' ')}</p>
						{#if item.my_report.resolution_note}
							<p class="tight">Review note: {item.my_report.resolution_note}</p>
						{/if}
						{#if item.my_report.correction_note}
							<p class="tight">Public correction: {item.my_report.correction_note}</p>
						{/if}
						{#if item.my_report.corrected_version_number}
							<p class="tight">Corrected version v{item.my_report.corrected_version_number} was published.</p>
						{/if}
						{#if item.my_report.status === 'open' || item.my_report.status === 'quarantined'}
							<p class="muted small tight">
								Acknowledged {new Date(item.my_report.acknowledged_at).toLocaleString()} ·
								resolution due {new Date(item.my_report.resolution_due_at).toLocaleString()}
								{#if item.my_report.resolution_overdue}<strong class="danger-text">Overdue</strong>{/if}
							</p>
						{/if}
					</div>
				{/if}

				{#if item.my_report}
					<p class="muted" data-testid="report-done">
						{reportDone || 'Your report is already on file. See its current status above.'}
					</p>
				{:else if !reportOpen}
					<button
						class="linklike"
						type="button"
						disabled={reportBusy}
						data-testid="report-open"
						onclick={() => {
							reportOpen = true;
							reportError = '';
						}}
					>
						Report a problem with this question
					</button>
				{:else}
					<div data-testid="report-form">
						<p class="field">
							<span>What's wrong?</span>
							<span class="cluster">
								{#each REPORT_CATEGORIES as [value, label]}
									<button
										type="button"
										class="btn small {reportCategory === value ? 'primary' : ''}"
										disabled={reportBusy}
										data-testid={`report-cat-${value}`}
										onclick={() => {
											reportCategory = value;
										}}
									>
										{label}
									</button>
								{/each}
							</span>
						</p>
						<label class="field">
							<span>Details (optional)</span>
							<input
								type="text"
								bind:value={reportNote}
								disabled={reportBusy}
								maxlength="2000"
								placeholder="What looks wrong?"
								data-testid="report-note"
							/>
						</label>
						{#if reportError}
							<p class="error-text" role="alert">{reportError}</p>
						{/if}
						<p class="cluster">
							<button
								class="btn primary"
								type="button"
								disabled={reportBusy}
								data-loading={reportBusy}
								data-testid="report-submit"
								onclick={submitReport}
							>
								{reportBusy ? 'Sending…' : 'Send report'}
							</button>
							<button
								class="linklike"
								type="button"
								disabled={reportBusy}
								onclick={() => {
									reportOpen = false;
									reportError = '';
								}}
							>
								Cancel
							</button>
						</p>
					</div>
				{/if}

				{#if current < session.items.length - 1}
					<button
						class="btn primary"
						type="button"
						data-testid="next"
						onclick={() => navigateTo(current + 1)}
					>
						Next
					</button>
				{:else if allAnswered}
					<button
						class="btn primary"
						type="button"
						disabled={submitting}
						data-loading={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						{submitting ? 'Submitting…' : 'Submit session'}
					</button>
				{:else}
					<button
						class="btn"
						type="button"
						disabled={submitting}
						data-testid="submit-session"
						onclick={submitSession}
					>
						Submit with unanswered items (they count as skipped)
					</button>
				{/if}
			{/if}
		</div>
	{/if}
{:else}
	<p class="muted is-loading">Loading the session…</p>
{/if}
</div>

<style>
	.session-workspace {
		font-size: calc(1rem * var(--session-font-scale, 1));
	}

	/* Toolbar: compact, secondary to the question (§7.3 reading first). */
	.session-toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
		padding: var(--space-md);
	}

	.session-toolbar > .btn {
		padding-inline: var(--space-lg);
		font-size: var(--text-sm);
	}

	@media (pointer: fine) {
		.session-toolbar > .btn {
			min-height: 40px;
		}
	}

	.toolbar-hint {
		flex: 1 1 14rem;
	}

	.session-progress {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-xs);
		margin: var(--space-lg) 0 var(--space-sm);
		font-weight: 600;
	}

	.session-meter {
		margin-bottom: var(--space-lg);
		height: 4px;
	}

	/* Question text stays the strongest element on the screen (§7.5). */
	.question-card :global([data-testid='question-text']) {
		font-size: 1.0625em;
		line-height: 1.7;
	}

	.key-point {
		margin: var(--space-md) 0 0;
		padding: var(--space-md);
		border-radius: var(--radius-sm);
		background: color-mix(in oklab, var(--color-surface) 60%, transparent);
	}

	.tutoring-cards {
		margin-top: var(--space-md);
	}

	.question-card > .linklike {
		margin-right: var(--space-lg);
	}

	.results-card h1 {
		margin-bottom: var(--space-md);
	}

	.results-card .stat-row > :first-child {
		border-color: color-mix(in oklab, var(--color-accent) 50%, transparent);
		background: var(--color-action-wash);
	}

	.breakdown-row {
		margin: 2px 0;
	}

	.session-workspace :is(input, select, textarea) {
		box-sizing: border-box;
		width: 100%;
		min-height: 44px;
		padding: var(--space-sm) var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: inherit;
	}

	.session-workspace textarea {
		resize: vertical;
	}

	.tool-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 16rem), 1fr));
		gap: var(--space-lg);
	}

	.tool-panel {
		min-width: 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.tool-panel h3 {
		margin-top: 0;
	}

	.text-size-control {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-xs);
	}

	.text-size-control .btn {
		padding-inline: var(--space-md);
	}

	.check-field {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		margin-bottom: var(--space-md);
	}

	.check-field input {
		width: 20px;
		min-height: 20px;
	}

	/* The Eliminate control takes an implicit column only while it exists, so
	 * answered options keep the full width (no phantom gap). */
	.option-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-auto-flow: column;
		grid-auto-columns: auto;
		align-items: center;
		gap: var(--space-sm);
	}

	.option.eliminated {
		text-decoration: line-through;
		opacity: 0.62;
	}

	.eliminate-option {
		padding-inline: var(--space-xs);
		font-size: var(--text-sm);
	}

	.highlight-note {
		padding: var(--space-sm) var(--space-md);
		border-left: 3px solid var(--color-accent);
		background: var(--color-surface-elevated);
	}

	.watermarked-copy {
		position: relative;
		isolation: isolate;
		overflow: hidden;
	}

	.watermark-tiles {
		position: absolute;
		z-index: 2;
		inset: 0;
		pointer-events: none;
		user-select: none;
	}

	.watermark-tiles::before {
		position: absolute;
		inset: -15%;
		pointer-events: none;
		content: attr(data-watermark) "  ·  " attr(data-watermark) "  ·  " attr(data-watermark) "\A"
			attr(data-watermark) "  ·  " attr(data-watermark) "  ·  " attr(data-watermark) "\A"
			attr(data-watermark) "  ·  " attr(data-watermark) "  ·  " attr(data-watermark) "\A"
			attr(data-watermark) "  ·  " attr(data-watermark) "  ·  " attr(data-watermark);
		color: var(--color-text-secondary);
		font-size: var(--text-xs);
		font-weight: 700;
		letter-spacing: 0.08em;
		line-height: 3;
		text-align: center;
		white-space: pre;
		opacity: 0.12;
		transform: rotate(-18deg) scale(1.15);
	}

	.timer-warning {
		border: 1px solid var(--color-warning);
	}

	@media (max-width: 640px) {
		.session-workspace > .card {
			padding: var(--space-md);
		}

		.text-size-control {
			width: 100%;
		}
	}
</style>
