<script>
	import { onMount, tick as svelteTick } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
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
	let reviewing = $state(false);
	let actionBusy = $state(false);
	let actionError = $state('');
	let markBusy = $state(false);
	let markError = $state('');
	let toolsOpen = $state(false);
	let toolsOpenButton;
	let toolsCloseButton;
	let textSize = $state('default');
	let hintText = $state('');
	let hintBusy = $state(false);
	let hintError = $state('');
	let converterKind = $state('cm-in');
	let converterValue = $state('');
	let calcKind = $state('bmi');
	let calcValues = $state({});
	let calcBusy = $state(false);
	let calcResult = $state(null);
	let calcError = $state('');
	let navigatorOpen = $state(false);
	let navigatorFilter = $state('all');
	let eliminationMode = $state(false);
	let draftSelections = $state({});
	let visited = $state([]);
	let eliminatedOptions = $state({});
	let focusMode = $state(false);
	let focusError = $state('');
	let wakeLock = null;
	let wakeLockPending = false;
	let questionTouchStart = null;
	let optionTouch = null;
	let optionLongPressTimer = null;
	let suppressOptionClickUntil = 0;

	const TEXT_SIZES = [
		['small', 'Small'],
		['default', 'Default'],
		['large', 'Large'],
		['xlarge', 'Extra large']
	];

	const CALCULATORS = [
		{ value: 'bmi', label: 'BMI', fields: [['weight_kg', 'Weight (kg)'], ['height_m', 'Height (m)']] },
		{ value: 'bsa', label: 'BSA (Mosteller)', fields: [['weight_kg', 'Weight (kg)'], ['height_cm', 'Height (cm)']] },
		{ value: 'map', label: 'Mean arterial pressure', fields: [['systolic', 'Systolic (mmHg)'], ['diastolic', 'Diastolic (mmHg)']] },
		{ value: 'gcs', label: 'Glasgow Coma Scale', fields: [['eye', 'Eye (1–4)'], ['verbal', 'Verbal (1–5)'], ['motor', 'Motor (1–6)']] },
		{ value: 'cockcroft-gault', label: 'Cockcroft–Gault', fields: [['age_years', 'Age (years)'], ['weight_kg', 'Weight (kg)'], ['serum_creatinine_mg_dl', 'Creatinine (mg/dL)'], ['female', 'Sex factor', 'boolean']] },
		{ value: 'ckd-epi-2021', label: 'CKD-EPI 2021', fields: [['serum_creatinine_mg_dl', 'Creatinine (mg/dL)'], ['age_years', 'Age (years)'], ['female', 'Sex factor', 'boolean']] },
		{ value: 'anion-gap', label: 'Anion gap', fields: [['sodium_mmol_l', 'Sodium (mmol/L)'], ['chloride_mmol_l', 'Chloride (mmol/L)'], ['bicarbonate_mmol_l', 'Bicarbonate (mmol/L)']] },
		{ value: 'corrected-calcium', label: 'Corrected calcium', fields: [['calcium_mg_dl', 'Calcium (mg/dL)'], ['albumin_g_dl', 'Albumin (g/dL)']] }
	];

	// QB-08: report-a-problem control on answered items.
	let reportOpen = $state(false);
	let reportCategory = $state('wrong_answer');
	let reportNote = $state('');
	let reportBusy = $state(false);
	let reportDone = $state('');
	let reportError = $state('');

	const REPORT_CATEGORIES = [
		['wrong_answer', 'Wrong answer'],
		['bad_explanation', 'Bad explanation'],
		['typo', 'Typo'],
		['duplicate', 'Duplicate'],
		['outdated', 'Outdated'],
		['broken_image', 'Broken image'],
		['other', 'Other']
	];

	const item = $derived(session?.items?.[current] ?? null);
	const allAnswered = $derived(
		session?.items ? session.items.every((i) => i.answered) : false
	);
	const answeredCount = $derived(session?.items?.filter((i) => i.answered).length ?? 0);
	const unansweredCount = $derived((session?.items?.length ?? 0) - answeredCount);
	const markedCount = $derived(session?.items?.filter((i) => i.marked).length ?? 0);
	const missedCount = $derived(result ? result.incorrect + result.skipped : 0);

	// EX-08: countdown derives from the server-issued deadline and server_now,
	// so changing the device clock never extends the timer.
	let clockSkewMs = $state(0);
	let remainingMs = $state(null);
	let autoSubmitted = $state(false);

	function tick() {
		if (!session?.deadline) return;
		remainingMs = new Date(session.deadline).getTime() - (Date.now() + clockSkewMs);
		if (remainingMs <= 0 && !autoSubmitted && !result && session.status === 'open') {
			autoSubmitted = true;
			submitSession();
		}
	}

	function fmt(ms) {
		const total = Math.max(0, Math.floor(ms / 1000));
		const m = Math.floor(total / 60);
		const s = total % 60;
		return `${m}:${String(s).padStart(2, '0')}`;
	}

	function fmtDuration(seconds) {
		const total = Math.max(0, Math.floor(seconds ?? 0));
		const h = Math.floor(total / 3600);
		const m = Math.floor((total % 3600) / 60);
		const s = total % 60;
		if (h > 0) return `${h}h ${m}m ${s}s`;
		if (m > 0) return `${m}m ${s}s`;
		return `${s}s`;
	}

	function autoSubmitWarning() {
		if (remainingMs === null || remainingMs <= 0 || remainingMs > 300_000) return '';
		return remainingMs <= 60_000
			? 'Less than 1 minute remains. This session will submit automatically at zero.'
			: 'Less than 5 minutes remain. This session will submit automatically at zero.';
	}

	function setTextSize(size) {
		textSize = size;
		try {
			localStorage.setItem('mlos_session_text_size_v1', size);
		} catch {
			// Browser preference only; storage denial must not block the session.
		}
	}

	function restoreTextSize() {
		try {
			const stored = localStorage.getItem('mlos_session_text_size_v1');
			if (TEXT_SIZES.some(([value]) => value === stored)) textSize = stored;
		} catch {
			// Keep the default when storage is unavailable.
		}
	}

	function workspaceStorageKey() {
		return `mlos_session_workspace_v1_${sid}`;
	}

	function saveWorkspaceState() {
		if (!session || session.status !== 'open') return;
		try {
			localStorage.setItem(
				workspaceStorageKey(),
				JSON.stringify({
					current,
					drafts: draftSelections,
					visited,
					eliminated: eliminatedOptions
				})
			);
		} catch {
			// Draft UI state is best-effort; storage denial must not block learning.
		}
	}

	function clearWorkspaceState() {
		try {
			localStorage.removeItem(workspaceStorageKey());
		} catch {
			// Submission remains authoritative even when local storage is unavailable.
		}
	}

	function restoreWorkspaceState() {
		if (!session?.items?.length || session.status !== 'open') return false;
		try {
			const raw = localStorage.getItem(workspaceStorageKey());
			if (!raw) return false;
			const parsed = JSON.parse(raw);
			const length = session.items.length;
			const restoredCurrent =
				Number.isInteger(parsed.current) && parsed.current >= 0 && parsed.current < length
					? parsed.current
					: current;
			const restoredDrafts = {};
			for (const [indexKey, chosen] of Object.entries(parsed.drafts ?? {})) {
				const index = Number(indexKey);
				const optionCount = session.items[index]?.options?.length ?? 0;
				if (
					Number.isInteger(index) &&
					index >= 0 &&
					index < length &&
					!session.items[index].answered &&
					Number.isInteger(chosen) &&
					chosen >= 0 &&
					chosen < optionCount
				) {
					restoredDrafts[index] = chosen;
				}
			}
			const restoredVisited = Array.from(
				new Set(
					(Array.isArray(parsed.visited) ? parsed.visited : []).filter(
						(index) => Number.isInteger(index) && index >= 0 && index < length
					)
				)
			);
			if (!restoredVisited.includes(restoredCurrent)) restoredVisited.push(restoredCurrent);
			const restoredEliminated = {};
			for (const [indexKey, optionIndexes] of Object.entries(parsed.eliminated ?? {})) {
				const index = Number(indexKey);
				if (!Number.isInteger(index) || index < 0 || index >= length || session.items[index].answered) {
					continue;
				}
				const optionCount = session.items[index]?.options?.length ?? 0;
				const valid = Array.from(
					new Set(
						(Array.isArray(optionIndexes) ? optionIndexes : []).filter(
							(optionIndex) =>
								Number.isInteger(optionIndex) && optionIndex >= 0 && optionIndex < optionCount
						)
					)
				);
				if (valid.length) restoredEliminated[index] = valid;
			}

			current = restoredCurrent;
			draftSelections = restoredDrafts;
			visited = restoredVisited;
			eliminatedOptions = restoredEliminated;
			selected = restoredDrafts[restoredCurrent] ?? null;
			return true;
		} catch {
			return false;
		}
	}

	function setDraftSelection(value) {
		selected = value;
		const next = { ...draftSelections };
		if (value === null) delete next[current];
		else next[current] = value;
		draftSelections = next;
		saveWorkspaceState();
	}

	function markVisited(index) {
		if (visited.includes(index)) return;
		visited = [...visited, index];
	}

	function goToQuestion(index) {
		if (!session?.items?.length || index < 0 || index >= session.items.length || index === current) return;
		current = index;
		selected = session.items[index].answered ? null : (draftSelections[index] ?? null);
		markVisited(index);
		navigatorOpen = false;
		saveWorkspaceState();
	}

	function previousQuestion() {
		if (current > 0) goToQuestion(current - 1);
	}

	function nextQuestion() {
		if (session && current < session.items.length - 1) goToQuestion(current + 1);
	}

	function firstUnansweredQuestion() {
		const index = session?.items?.findIndex((question) => !question.answered) ?? -1;
		if (index >= 0) goToQuestion(index);
	}

	function questionState(index) {
		if (index === current) return 'current';
		if (session.items[index].answered) return 'answered';
		return visited.includes(index) ? 'unanswered' : 'not-visited';
	}

	function questionStateLabel(index) {
		return {
			current: 'current',
			answered: 'answered',
			unanswered: 'unanswered',
			'not-visited': 'not visited'
		}[questionState(index)];
	}

	function questionStateIcon(index) {
		return { current: '●', answered: '✓', unanswered: '◐', 'not-visited': '○' }[
			questionState(index)
		];
	}

	function navigatorIndices() {
		if (!session?.items) return [];
		return session.items
			.map((_, index) => index)
			.filter((index) => {
				if (navigatorFilter === 'marked') return !!session.items[index].marked;
				if (navigatorFilter === 'unanswered') return !session.items[index].answered;
				return true;
			});
	}

	function isEliminated(questionIndex, optionIndex) {
		return eliminatedOptions[questionIndex]?.includes(optionIndex) ?? false;
	}

	function toggleElimination(optionIndex) {
		if (!item || item.answered || reviewing || session?.status !== 'open') return;
		const currentOptions = eliminatedOptions[current] ?? [];
		const nextOptions = currentOptions.includes(optionIndex)
			? currentOptions.filter((index) => index !== optionIndex)
			: [...currentOptions, optionIndex];
		const next = { ...eliminatedOptions };
		if (nextOptions.length) next[current] = nextOptions;
		else delete next[current];
		eliminatedOptions = next;
		if (selected === optionIndex && nextOptions.includes(optionIndex)) {
			selected = null;
			const drafts = { ...draftSelections };
			delete drafts[current];
			draftSelections = drafts;
		}
		saveWorkspaceState();
	}

	function handleOptionClick(optionIndex) {
		if (Date.now() < suppressOptionClickUntil || item?.answered) return;
		if (eliminationMode) toggleElimination(optionIndex);
		else setDraftSelection(selected === optionIndex ? null : optionIndex);
	}

	function touchPoint(event) {
		return event.changedTouches?.[0] ?? event.touches?.[0] ?? null;
	}

	function startQuestionTouch(event) {
		const point = touchPoint(event);
		if (point) questionTouchStart = { x: point.clientX, y: point.clientY };
	}

	function endQuestionTouch(event) {
		const point = touchPoint(event);
		const start = questionTouchStart;
		questionTouchStart = null;
		if (!point || !start || window.getSelection()?.toString()) return;
		const dx = point.clientX - start.x;
		const dy = point.clientY - start.y;
		if (Math.abs(dx) < 72 || Math.abs(dx) <= Math.abs(dy) * 1.25) return;
		if (dx < 0) nextQuestion();
		else previousQuestion();
	}

	function cancelOptionLongPress() {
		if (optionLongPressTimer) clearTimeout(optionLongPressTimer);
		optionLongPressTimer = null;
	}

	function startOptionTouch(event, optionIndex) {
		event.stopPropagation();
		const point = touchPoint(event);
		if (!point) return;
		cancelOptionLongPress();
		optionTouch = { optionIndex, x: point.clientX, y: point.clientY, longPressed: false };
		optionLongPressTimer = setTimeout(() => {
			if (!optionTouch || optionTouch.optionIndex !== optionIndex) return;
			optionTouch.longPressed = true;
			suppressOptionClickUntil = Date.now() + 700;
			toggleElimination(optionIndex);
		}, 550);
	}

	function moveOptionTouch(event) {
		event.stopPropagation();
		const point = touchPoint(event);
		if (!point || !optionTouch) return;
		if (Math.abs(point.clientX - optionTouch.x) > 12 || Math.abs(point.clientY - optionTouch.y) > 12) {
			cancelOptionLongPress();
		}
	}

	function endOptionTouch(event, optionIndex) {
		event.stopPropagation();
		const point = touchPoint(event);
		const start = optionTouch;
		cancelOptionLongPress();
		optionTouch = null;
		if (!point || !start || start.optionIndex !== optionIndex || start.longPressed) return;
		const dx = point.clientX - start.x;
		const dy = point.clientY - start.y;
		if (Math.abs(dx) >= 56 && Math.abs(dx) > Math.abs(dy) * 1.25) {
			suppressOptionClickUntil = Date.now() + 700;
			toggleElimination(optionIndex);
		}
	}

	function cancelOptionTouch(event) {
		event.stopPropagation();
		cancelOptionLongPress();
		optionTouch = null;
	}

	async function requestWakeLock() {
		if (!navigator.wakeLock?.request || wakeLock || wakeLockPending) return;
		wakeLockPending = true;
		try {
			const lock = await navigator.wakeLock.request('screen');
			if (!focusMode) {
				await lock?.release?.();
				return;
			}
			wakeLock = lock;
			wakeLock?.addEventListener?.('release', () => {
				wakeLock = null;
			});
		} catch {
			if (focusMode) {
				focusError = 'Screen wake lock could not be enabled. Focus Mode remains usable.';
			}
		} finally {
			wakeLockPending = false;
		}
	}

	async function releaseWakeLock() {
		const lock = wakeLock;
		wakeLock = null;
		try {
			await lock?.release?.();
		} catch {
			// A released/invalidated wake lock needs no further recovery.
		}
	}

	async function onFullscreenChange() {
		focusMode = !!document.fullscreenElement;
		if (focusMode) await requestWakeLock();
		else await releaseWakeLock();
	}

	async function toggleFocusMode() {
		focusError = '';
		if (document.fullscreenElement || focusMode) {
			try {
				if (document.fullscreenElement && document.exitFullscreen) await document.exitFullscreen();
			} catch {
				focusMode = !!document.fullscreenElement;
				focusError = 'Focus Mode could not exit fullscreen. Use your browser fullscreen control to exit.';
			}
			return;
		}
		if (!document.documentElement.requestFullscreen) {
			focusError = 'Fullscreen is not available in this browser. The session remains fully usable.';
			return;
		}
		try {
			await document.documentElement.requestFullscreen();
		} catch {
			focusMode = !!document.fullscreenElement;
			focusError = 'Focus Mode could not enter fullscreen. The session remains fully usable.';
		}
	}

	function convertValue(kind, raw) {
		if (raw === '' || raw === null || raw === undefined) return 'Enter a number';
		const value = Number(raw);
		if (!Number.isFinite(value)) return 'Enter a number';
		const converted = {
			'cm-in': [value / 2.54, 'in'],
			'in-cm': [value * 2.54, 'cm'],
			'kg-lb': [value * 2.2046226218, 'lb'],
			'lb-kg': [value / 2.2046226218, 'kg'],
			'c-f': [(value * 9) / 5 + 32, '°F'],
			'f-c': [((value - 32) * 5) / 9, '°C'],
			'glucose-mgdl-mmol': [value / 18, 'mmol/L'],
			'glucose-mmol-mgdl': [value * 18, 'mg/dL']
		}[kind];
		return converted ? `${converted[0].toFixed(2)} ${converted[1]}` : 'Choose a conversion';
	}

	function selectedCalculator() {
		return CALCULATORS.find((calculator) => calculator.value === calcKind) ?? CALCULATORS[0];
	}

	function formattedCalculatorResult() {
		if (!calcResult) return '';
		return `${Number(calcResult.value).toFixed(2)} ${calcResult.unit}`;
	}

	async function openTools() {
		toolsOpen = true;
		await svelteTick();
		toolsCloseButton?.focus();
	}

	async function closeTools() {
		toolsOpen = false;
		await svelteTick();
		toolsOpenButton?.focus();
	}

	async function runCalculator() {
		calcBusy = true;
		calcError = '';
		calcResult = null;
		try {
			const inputs = {};
			for (const [key, label, type] of selectedCalculator().fields) {
				if (type === 'boolean') {
					inputs[key] = calcValues[key] === 'true';
					continue;
				}
				const value = Number(calcValues[key]);
				if (!Number.isFinite(value)) throw new Error(`${label} is required.`);
				inputs[key] = value;
			}
			calcResult = await Api.calculate({ calculator: calcKind, inputs });
		} catch (err) {
			calcError = err instanceof ApiError || err instanceof Error ? err.message : 'Could not calculate.';
		} finally {
			calcBusy = false;
		}
	}

	async function revealHint() {
		if (!item || item.answered || session?.preset !== 'tutor' || hintBusy || hintText) return;
		const itemIndex = current;
		hintBusy = true;
		hintError = '';
		try {
			const response = await Api.hint(sid, itemIndex);
			if (current === itemIndex) hintText = response.hint;
		} catch (err) {
			if (current === itemIndex) {
				hintError = err instanceof ApiError ? err.message : 'Could not load the hint.';
			}
		} finally {
			hintBusy = false;
		}
	}

	function storageKey(index) {
		return `mlos_key_${sid}_${index}`;
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
		try {
			session = await Api.getSession(sid);
			if (session.status === 'submitted' && session.result) {
				result = session.result;
				current = 0;
				clearWorkspaceState();
				return;
			}
			const firstUnanswered = session.items.findIndex((i) => !i.answered);
			current = firstUnanswered === -1 ? session.items.length - 1 : firstUnanswered;
			if (!restoreWorkspaceState()) {
				selected = null;
				draftSelections = {};
				eliminatedOptions = {};
				visited = [current];
				saveWorkspaceState();
			}
			if (session.deadline && session.server_now) {
				clockSkewMs = new Date(session.server_now).getTime() - Date.now();
				tick();
			}
		} catch (err) {
			loadFailed =
				err instanceof ApiError ? err.message : 'Could not load the session.';
		}
	}

	async function answer(chosen) {
		if (!item || item.answered || busy) return;
		const itemIndex = current;
		busy = true;
		error = '';
		try {
			const res = await Api.answer(sid, {
				item_index: itemIndex,
				chosen_index: chosen,
				idempotency_key: idempotencyKey(itemIndex)
			});
			session.items[itemIndex] = {
				...session.items[itemIndex],
				answered: true,
				chosen_index: chosen,
				correct: res.correct,
				correct_index: res.correct_index,
				options: res.options,
				key_learning_point: res.key_learning_point,
				exam_tip: res.exam_tip
			};
			if (current === itemIndex) selected = null;
			const drafts = { ...draftSelections };
			delete drafts[itemIndex];
			draftSelections = drafts;
			const eliminated = { ...eliminatedOptions };
			delete eliminated[itemIndex];
			eliminatedOptions = eliminated;
			saveWorkspaceState();
		} catch (err) {
			if (err instanceof ApiError && err.code === 'session_expired') {
				// Server deadline passed: submit what exists (auto-submit, §11.6).
				autoSubmitted = true;
				await submitSession();
				return;
			}
			error = err instanceof ApiError ? err.message : 'Could not record the answer.';
		} finally {
			busy = false;
		}
	}

	async function submitSession() {
		if (submitting) return;
		submitting = true;
		error = '';
		try {
			result = await Api.submit(sid);
			try {
				session = await Api.getSession(sid);
				if (session.result) result = session.result;
			} catch {
				if (session) session.status = 'submitted';
			}
			current = 0;
			clearWorkspaceState();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not submit the session.';
		} finally {
			submitting = false;
		}
	}

	function startReview() {
		current = 0;
		selected = null;
		reviewing = true;
		actionError = '';
	}

	async function startResultSession(kind) {
		if (!session || actionBusy) return;
		actionBusy = true;
		actionError = '';
		try {
			let request;
			if (kind === 'missed') {
				request = { preset: 'revision', source_session_id: sid };
			} else {
				if (!session.chapter_id) throw new Error('This session cannot be retried by chapter.');
				request = {
					preset: session.preset === 'timed' ? 'timed' : 'tutor',
					chapter_id: session.chapter_id,
					question_count: session.items.length
				};
				if (request.preset === 'timed' && session.time_limit_seconds) {
					request.time_limit_seconds = session.time_limit_seconds;
				}
			}
			const { session_id } = await Api.createSession(request);
			await goto(`${base}/session/${session_id}`);
		} catch (err) {
			actionError =
				err instanceof ApiError || err instanceof Error
					? err.message
					: 'Could not start the next session.';
		} finally {
			actionBusy = false;
		}
	}

	async function submitReport() {
		if (reportBusy || !item) return;
		const itemIndex = current;
		const questionVersionId = item.question_version_id;
		reportBusy = true;
		reportError = '';
		try {
			const res = await Api.reportQuestion(questionVersionId, {
				category: reportCategory,
				note: reportNote.trim() ? reportNote.trim() : undefined
			});
			session.items[itemIndex] = {
				...session.items[itemIndex],
				report_status: res.quarantined ? 'quarantined' : 'open'
			};
			if (current === itemIndex) {
				reportDone = res.quarantined
					? 'Thanks — enough learners flagged this, so it is out of rotation pending review.'
					: 'Thanks — your report is recorded for editorial review.';
				reportOpen = false;
				reportNote = '';
			}
		} catch (err) {
			if (current === itemIndex) {
				reportError = err instanceof ApiError ? err.message : 'Could not send the report.';
			}
		} finally {
			reportBusy = false;
		}
	}

	async function toggleMark() {
		if (!item || markBusy) return;
		const itemIndex = current;
		const questionVersionId = item.question_version_id;
		const marked = item.marked;
		markBusy = true;
		markError = '';
		try {
			const next = await Api.setQuestionMark(questionVersionId, !marked);
			session.items[itemIndex] = { ...session.items[itemIndex], marked: next.marked };
		} catch (err) {
			if (current === itemIndex) {
				markError = err instanceof ApiError ? err.message : 'Could not update this question mark.';
			}
		} finally {
			markBusy = false;
		}
	}

	// Fresh report panel per question — navigating never leaks state.
	$effect(() => {
		current;
		reportOpen = false;
		reportDone = '';
		reportError = '';
		reportNote = '';
		markError = '';
		hintText = '';
		hintError = '';
	});

	function onKeydown(event) {
		if ((result && !reviewing) || !session || !item) return;
		if (event.key === 'Escape' && toolsOpen) {
			closeTools();
			return;
		}
		if (event.key === 'Escape' && navigatorOpen) {
			navigatorOpen = false;
			return;
		}
		if (
			['INPUT', 'SELECT', 'TEXTAREA'].includes(event.target?.tagName) ||
			event.target?.isContentEditable
		) {
			return;
		}
		if (reviewing) {
			if (event.key === 'Enter' || event.key === 'n' || event.key === 'N' || event.key === 'ArrowRight') {
				event.preventDefault();
				nextQuestion();
			} else if (event.key === 'p' || event.key === 'P' || event.key === 'ArrowLeft') {
				event.preventDefault();
				previousQuestion();
			}
			return;
		}
		const lower = event.key.toLowerCase();
		if (lower === 'f') {
			event.preventDefault();
			toggleMark();
			return;
		}
		if (lower === 'e' && !item.answered && session.status === 'open') {
			event.preventDefault();
			eliminationMode = !eliminationMode;
			return;
		}
		const letter = 'abcdefghij'.indexOf(lower);
		if (lower === 'h' && session.preset === 'tutor' && !item.answered) {
			event.preventDefault();
			revealHint();
		} else if (event.key === 'ArrowRight' || lower === 'n') {
			event.preventDefault();
			nextQuestion();
		} else if (event.key === 'ArrowLeft' || lower === 'p') {
			event.preventDefault();
			previousQuestion();
		} else if (letter >= 0 && !item.answered && letter < item.options.length) {
			event.preventDefault();
			if (eliminationMode) toggleElimination(letter);
			else setDraftSelection(letter);
		} else if (event.key === 'Enter') {
			if (!item.answered && selected !== null) {
				answer(selected);
			} else if (item.answered && current < session.items.length - 1) {
				nextQuestion();
			} else if (item.answered && allAnswered) {
				submitSession();
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
		restoreTextSize();
		load();
		document.addEventListener('fullscreenchange', onFullscreenChange);
		return () => {
			document.removeEventListener('fullscreenchange', onFullscreenChange);
			cancelOptionLongPress();
			void releaseWakeLock();
		};
	});
</script>

<svelte:window onkeydown={onKeydown} />

{#if loadFailed}
	<p class="error-text" role="alert">{loadFailed}</p>
	<button class="btn" type="button" onclick={load}>Retry</button>
{:else if result && !reviewing}
	<div class="card" data-testid="results">
		<h1>Session submitted</h1>
		<div class="stat-row">
			<span>Score<strong data-testid="score">{result.score}%</strong></span>
			<span>Correct<strong>{result.correct}</strong></span>
			<span>Incorrect<strong>{result.incorrect}</strong></span>
			<span>Skipped<strong>{result.skipped}</strong></span>
			<span>Assisted<strong data-testid="assisted">{result.assisted}</strong></span>
			<span>Total<strong data-testid="total">{result.total}</strong></span>
			<span
				>Time taken<strong data-testid="time-taken">{fmtDuration(result.time_taken_seconds)}</strong></span
			>
		</div>
		<p class="muted">This result is about this form — it is not a prediction of anything.</p>
		<div class="result-actions" aria-label="Result actions">
			{#if missedCount > 0}
				<button
					class="btn primary"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="practice-missed"
					onclick={() => startResultSession('missed')}
				>
					Practice missed questions
				</button>
			{:else if session?.chapter_id}
				<button
					class="btn primary"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="retry-session"
					onclick={() => startResultSession('retry')}
				>
					Retry this session
				</button>
			{:else}
				<a class="btn primary" href={`${base}/today`} data-testid="back-today">Back to Today</a>
			{/if}

			<button class="btn" type="button" data-testid="review-answers" onclick={startReview}>
				Review answers
			</button>

			{#if missedCount > 0 && session?.chapter_id}
				<button
					class="btn"
					type="button"
					disabled={actionBusy}
					data-loading={actionBusy}
					data-testid="retry-session"
					onclick={() => startResultSession('retry')}
				>
					Retry this session
				</button>
			{/if}

			{#if missedCount > 0 || session?.chapter_id}
				<a class="btn" href={`${base}/today`} data-testid="back-today">Back to Today</a>
			{/if}
		</div>
		{#if actionError}
			<p class="error-text" role="alert">{actionError}</p>
		{/if}
	</div>
{:else if session}
	<p class="muted" style="margin-bottom: var(--space-sm);">
		{reviewing
			? `Reviewing question ${current + 1} of ${session.items.length}`
			: `Question ${current + 1} of ${session.items.length}`}
		· {session.preset === 'revision' ? 'Re-practice' : session.preset === 'timed' ? 'Timed' : 'Tutor mode'}
		{#if !reviewing && remainingMs !== null}
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
	<div class="session-top-actions">
		<button
			class="btn"
			class:primary={focusMode}
			type="button"
			aria-pressed={focusMode}
			data-testid="focus-mode"
			onclick={toggleFocusMode}
		>
			{focusMode ? 'Exit Focus Mode' : 'Focus Mode'}
		</button>
		<button
			class="btn"
			type="button"
			bind:this={toolsOpenButton}
			aria-expanded={toolsOpen}
			aria-controls="session-tools"
			data-testid="session-tools-open"
			onclick={openTools}
		>
			Tools
		</button>
	</div>
	{#if focusMode}
		<p class="focus-note muted" data-testid="focus-mode-note">
			Focus Mode is active. Consider enabling Do Not Disturb on your device if you want fewer interruptions.
		</p>
	{/if}
	{#if focusError}
		<p class="error-text" role="status">{focusError}</p>
	{/if}
	{#if !reviewing && autoSubmitWarning()}
		<p class="deadline-warning" role="status" data-testid="auto-submit-warning">
			{autoSubmitWarning()}
		</p>
	{/if}
	{#if reviewing}
		<button
			class="linklike"
			type="button"
			data-testid="back-results"
			onclick={() => (reviewing = false)}
		>
			Back to results
		</button>
	{/if}

	{#if item}
		<div
			class={`card session-card text-${textSize}`}
			data-testid="question-swipe-surface"
			ontouchstart={startQuestionTouch}
			ontouchend={endQuestionTouch}
		>
			<div class="question-actions">
				<button
					class="btn"
					class:primary={item.marked}
					type="button"
					disabled={markBusy}
					data-loading={markBusy}
					data-testid="question-mark"
					aria-pressed={item.marked}
					onclick={toggleMark}
				>
					{markBusy ? 'Saving…' : item.marked ? 'Marked' : 'Mark question'}
				</button>
				{#if !reviewing && session.status === 'open' && !item.answered}
					<button
						class="btn"
						class:primary={eliminationMode}
						type="button"
						aria-pressed={eliminationMode}
						data-testid="elimination-mode"
						onclick={() => (eliminationMode = !eliminationMode)}
					>
						{eliminationMode ? 'Elimination on' : 'Eliminate options'}
					</button>
				{/if}
				{#if !reviewing && session.preset === 'tutor' && session.status === 'open' && !item.answered}
					<button
						class="btn"
						type="button"
						disabled={hintBusy || !!hintText}
						data-loading={hintBusy}
						data-testid="hint-open"
						onclick={revealHint}
					>
						{hintBusy ? 'Loading hint…' : hintText ? 'Hint shown' : 'Hint'}
					</button>
				{/if}
			</div>
			{#if markError}
				<p class="error-text" role="alert">{markError}</p>
			{/if}
			{#if hintText}
				<div class="hint-panel" data-testid="hint">
					<strong>Hint</strong>
					<p>{hintText}</p>
				</div>
			{:else if hintError}
				<p class="error-text" role="alert">{hintError}</p>
			{/if}
			<p>{item.vignette}</p>
			<p><strong>{item.lead_in}</strong></p>

			<div class="options">
				{#each item.options as option, i (i)}
					<button
						type="button"
						class="option {(item.answered || reviewing || session.status === 'submitted') && i === item.correct_index ? 'correct' : ''}
							{item.answered && item.chosen_index === i && item.correct === false ? 'incorrect' : ''}
							{isEliminated(current, i) ? 'eliminated' : ''}"
						aria-pressed={!item.answered && selected === i}
						aria-label={`${letterLabel(i)}. ${option.text}${isEliminated(current, i) ? ', eliminated' : ''}`}
						disabled={item.answered || busy || reviewing || session.status !== 'open'}
						data-testid={`option-${i}`}
						data-eliminated={isEliminated(current, i) ? 'true' : 'false'}
						onclick={() => handleOptionClick(i)}
						ontouchstart={(event) => startOptionTouch(event, i)}
						ontouchmove={moveOptionTouch}
						ontouchend={(event) => endOptionTouch(event, i)}
						ontouchcancel={cancelOptionTouch}
					>
						<span class="key">{letterLabel(i)}</span>
						<span class="option-text">{option.text}</span>
					</button>
				{/each}
			</div>

			{#if error}
				<p class="error-text" role="alert">{error}</p>
			{/if}

			{#if !item.answered && !reviewing && session.status === 'open'}
				<button
					class="btn primary"
					type="button"
					disabled={selected === null || busy || (remainingMs !== null && remainingMs <= 0)}
					data-loading={busy}
					data-testid="answer"
					onclick={() => answer(selected)}
				>
					{busy ? 'Recording…' : 'Answer'}
				</button>
				<button
					class="linklike"
					type="button"
					disabled={busy || (remainingMs !== null && remainingMs <= 0)}
					data-testid="skip"
					onclick={() => answer(null)}
				>
					Skip — I don't want to guess
				</button>
			{:else}
				<div
					class="feedback {item.correct === true ? 'good' : item.correct === false ? 'bad' : ''}"
					data-testid="feedback"
				>
					<p class="verdict">
						{item.correct === true
							? 'Correct.'
							: item.correct === false
								? 'Not quite.'
								: item.answered
									? 'Skipped.'
									: 'Not answered.'}
					</p>
					{#each item.options as option, i (i)}
						{#if option.rationale && (i === item.correct_index || i === item.chosen_index)}
							<p style="margin: 4px 0;">
								<strong>{letterLabel(i)}.</strong> {option.rationale}
							</p>
						{/if}
					{/each}
					<p style="margin: var(--space-sm) 0 0;">
						<strong>Key learning point:</strong> {item.key_learning_point}
					</p>
					{#if item.exam_tip}
						<p class="muted" style="margin: 4px 0 0;">Exam tip: {item.exam_tip}</p>
					{/if}
					{#if item.report_status === 'quarantined'}
						<p class="muted" style="margin: 4px 0 0;">
							Flagged by learners — out of rotation pending editorial review.
						</p>
					{:else if item.report_status === 'open'}
						<p class="muted" style="margin: 4px 0 0;">
							Flagged by a learner — under review.
						</p>
					{/if}
				</div>

				{#if reportDone}
					<p class="muted" data-testid="report-done">{reportDone}</p>
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
							<span style="display:flex; gap:8px; flex-wrap:wrap;">
								{#each REPORT_CATEGORIES as [value, label]}
									<button
										type="button"
										class="btn {reportCategory === value ? 'primary' : ''}"
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
						<p style="display:flex; gap:12px;">
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

				{#if reviewing && current === session.items.length - 1}
					<button
						class="btn"
						type="button"
						data-testid="back-results-end"
						onclick={() => (reviewing = false)}
					>
						Back to results
					</button>
				{:else if !reviewing && current === session.items.length - 1 && allAnswered}
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
				{:else if !reviewing && current === session.items.length - 1}
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

		<div class="session-bottom-bar" aria-label="Question navigation">
			<button class="btn" type="button" disabled={current === 0} data-testid="previous" onclick={previousQuestion}>
				Previous
			</button>
			<button
				class="btn"
				class:primary={navigatorOpen}
				type="button"
				aria-expanded={navigatorOpen}
				aria-controls="question-navigator"
				data-testid="navigator-toggle"
				onclick={() => (navigatorOpen = !navigatorOpen)}
			>
				Navigator
			</button>
			<button
				class="btn primary"
				type="button"
				disabled={current === session.items.length - 1}
				data-testid="next"
				onclick={nextQuestion}
			>
				Next
			</button>
		</div>

		<div class="submission-status" data-testid="submission-status">
			<span><strong>{answeredCount}</strong> answered</span>
			<span><strong>{unansweredCount}</strong> unanswered</span>
			<span><strong>{markedCount}</strong> marked</span>
			{#if unansweredCount > 0}
				<button class="linklike" type="button" data-testid="first-unanswered" onclick={firstUnansweredQuestion}>
					Go to first unanswered
				</button>
			{/if}
		</div>

		{#if navigatorOpen}
			<section id="question-navigator" class="navigator-panel" aria-label="Question navigator">
				<div class="navigator-heading">
					<div>
						<h2>Question navigator</h2>
						<p class="muted">Marked questions keep their star independently of answer status.</p>
					</div>
					<div class="navigator-filters" aria-label="Navigator filters">
						<button
							class="btn"
							class:primary={navigatorFilter === 'all'}
							type="button"
							aria-pressed={navigatorFilter === 'all'}
							data-testid="navigator-filter-all"
							onclick={() => (navigatorFilter = 'all')}
						>
							All
						</button>
						<button
							class="btn"
							class:primary={navigatorFilter === 'marked'}
							type="button"
							aria-pressed={navigatorFilter === 'marked'}
							data-testid="navigator-filter-marked"
							onclick={() => (navigatorFilter = 'marked')}
						>
							Marked
						</button>
						<button
							class="btn"
							class:primary={navigatorFilter === 'unanswered'}
							type="button"
							aria-pressed={navigatorFilter === 'unanswered'}
							data-testid="navigator-filter-unanswered"
							onclick={() => (navigatorFilter = 'unanswered')}
						>
							Unanswered
						</button>
					</div>
				</div>
				<div class="navigator-grid">
					{#each navigatorIndices() as index (index)}
						<button
							class={`navigator-question state-${questionState(index)}`}
							type="button"
							data-testid={`navigator-question-${index}`}
							data-state={questionState(index)}
							aria-current={index === current ? 'step' : undefined}
							aria-label={`Question ${index + 1}, ${questionStateLabel(index)}${session.items[index].marked ? ', marked' : ''}`}
							onclick={() => goToQuestion(index)}
						>
							<span class="navigator-state-icon" aria-hidden="true">{questionStateIcon(index)}</span>
							<span>{index + 1}</span>
							{#if session.items[index].marked}
								<span class="navigator-mark" aria-hidden="true">★</span>
							{/if}
						</button>
					{/each}
				</div>
				{#if navigatorIndices().length === 0}
					<p class="muted navigator-empty">No questions match this filter.</p>
				{/if}
			</section>
		{/if}
	{/if}

	{#if toolsOpen}
		<aside
			id="session-tools"
			class="session-tools"
			aria-label="Session tools"
			data-testid="session-tools"
		>
			<div class="tools-heading">
				<div>
					<h2>Session tools</h2>
					<p class="muted">Practice utilities stay separate from your answer evidence.</p>
				</div>
				<button
					class="btn"
					type="button"
					bind:this={toolsCloseButton}
					data-testid="session-tools-close"
					onclick={closeTools}
				>
					Close
				</button>
			</div>

			<section class="tool-section" aria-labelledby="calculator-heading">
				<h3 id="calculator-heading">Calculator</h3>
				<label class="field">
					<span>Calculator</span>
					<select
						class="tool-select"
						bind:value={calcKind}
						data-testid="calculator-kind"
						onchange={() => {
							calcResult = null;
							calcError = '';
						}}
					>
						{#each CALCULATORS as calculator}
							<option value={calculator.value}>{calculator.label}</option>
						{/each}
					</select>
				</label>
				<div class="calculator-fields">
					{#each selectedCalculator().fields as [key, label, type]}
						<label class="field">
							<span>{label}</span>
							{#if type === 'boolean'}
								<select class="tool-select" bind:value={calcValues[key]} data-testid={`calc-${key}`}>
									<option value="false">Not female</option>
									<option value="true">Female</option>
								</select>
							{:else}
								<input type="number" step="any" bind:value={calcValues[key]} data-testid={`calc-${key}`} />
							{/if}
						</label>
					{/each}
				</div>
				<button
					class="btn primary"
					type="button"
					disabled={calcBusy}
					data-loading={calcBusy}
					data-testid="calculator-run"
					onclick={runCalculator}
				>
					{calcBusy ? 'Calculating…' : 'Calculate'}
				</button>
				{#if calcResult}
					<p class="tool-result" data-testid="calculator-result">{formattedCalculatorResult()}</p>
				{:else if calcError}
					<p class="error-text" role="alert">{calcError}</p>
				{/if}
				<p class="muted tool-note">For exam practice only. Not for clinical use.</p>
			</section>

			<section class="tool-section" aria-labelledby="converter-heading">
				<h3 id="converter-heading">Converter</h3>
				<label class="field">
					<span>Conversion</span>
					<select class="tool-select" bind:value={converterKind} data-testid="converter-kind">
						<option value="cm-in">cm → in</option>
						<option value="in-cm">in → cm</option>
						<option value="kg-lb">kg → lb</option>
						<option value="lb-kg">lb → kg</option>
						<option value="c-f">°C → °F</option>
						<option value="f-c">°F → °C</option>
						<option value="glucose-mgdl-mmol">Glucose mg/dL → mmol/L</option>
						<option value="glucose-mmol-mgdl">Glucose mmol/L → mg/dL</option>
					</select>
				</label>
				<label class="field">
					<span>Value</span>
					<input type="number" step="any" bind:value={converterValue} data-testid="converter-value" />
				</label>
				<p class="tool-result" data-testid="converter-result">
					{convertValue(converterKind, converterValue)}
				</p>
			</section>

			<section class="tool-section" aria-labelledby="text-size-heading">
				<h3 id="text-size-heading">Text size</h3>
				<div class="text-size-options" data-testid="text-size-options">
					{#each TEXT_SIZES as [value, label]}
						<button
							class="btn"
							class:primary={textSize === value}
							type="button"
							aria-pressed={textSize === value}
							onclick={() => setTextSize(value)}
						>
							{label}
						</button>
					{/each}
				</div>
			</section>
		</aside>
	{/if}
{:else}
	<p class="muted">Loading the session…</p>
{/if}

<style>
	/* Hallmark · pre-emit critique: P5 H4 E5 S5 R5 V4 */
	.session-top-actions,
	.question-actions,
	.tools-heading,
	.text-size-options {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		flex-wrap: wrap;
	}

	.session-top-actions {
		justify-content: flex-end;
		margin-bottom: var(--space-sm);
	}

	.focus-note {
		margin: 0 0 var(--space-md);
		padding: var(--space-sm) var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.question-actions {
		justify-content: space-between;
		margin-bottom: var(--space-md);
	}

	.deadline-warning,
	.hint-panel,
	.tool-result {
		border: 1px solid var(--color-warning);
		border-radius: var(--radius-control);
		padding: var(--space-md);
		background: var(--color-surface);
	}

	.deadline-warning {
		margin: 0 0 var(--space-md);
		color: var(--color-warning);
		font-weight: 600;
	}

	.hint-panel {
		margin-bottom: var(--space-lg);
		border-color: var(--color-accent);
	}

	.hint-panel p,
	.tool-result,
	.tool-note,
	.tools-heading p {
		margin-bottom: 0;
	}

	.session-card.text-small {
		font-size: var(--text-sm);
	}

	.session-card.text-default {
		font-size: var(--text-body);
	}

	.session-card.text-large {
		font-size: var(--text-body-lg);
	}

	.session-card.text-xlarge {
		font-size: var(--text-xl);
	}

	.session-card :global(.option) {
		font-size: inherit;
	}

	.session-card :global(.option.eliminated) {
		border-style: dashed;
		opacity: 0.68;
	}

	.session-card :global(.option.eliminated) .option-text {
		text-decoration: line-through;
		text-decoration-thickness: 2px;
	}

	.session-bottom-bar,
	.submission-status,
	.navigator-heading,
	.navigator-filters {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		flex-wrap: wrap;
	}

	.session-bottom-bar {
		position: sticky;
		z-index: 10;
		bottom: calc(var(--space-sm) + env(safe-area-inset-bottom));
		justify-content: space-between;
		padding: var(--space-sm);
		margin-bottom: var(--space-md);
		background: var(--color-surface);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.session-bottom-bar .btn {
		flex: 1 1 0;
		min-width: 0;
		padding-inline: var(--space-sm);
	}

	.submission-status {
		margin-bottom: var(--space-md);
		color: var(--color-text-secondary);
	}

	.submission-status span {
		white-space: nowrap;
	}

	.navigator-panel {
		margin-bottom: var(--space-lg);
		padding: var(--space-lg);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card);
		background: var(--color-surface);
	}

	.navigator-heading {
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--space-md);
	}

	.navigator-heading h2,
	.navigator-heading p,
	.navigator-empty {
		margin: 0;
	}

	.navigator-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(52px, 1fr));
		gap: var(--space-sm);
	}

	.navigator-question {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-xs);
		min-height: 48px;
		padding: var(--space-sm);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
		background: var(--color-canvas);
		color: var(--color-text-secondary);
		font: 700 var(--text-body) / 1 var(--font-body);
		cursor: pointer;
	}

	.navigator-question:hover,
	.navigator-question:focus-visible {
		border-color: var(--color-accent);
	}

	.navigator-question:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.navigator-question.state-current {
		border-color: var(--color-accent);
		color: var(--color-text-primary);
		background: var(--color-surface-elevated);
	}

	.navigator-question.state-answered {
		border-color: var(--color-success);
		color: var(--color-success);
	}

	.navigator-question.state-unanswered {
		border-color: var(--color-warning);
		color: var(--color-warning);
	}

	.navigator-mark {
		position: absolute;
		top: 2px;
		right: 4px;
		color: var(--color-warning);
		font-size: var(--text-sm);
	}

	.navigator-empty {
		padding-top: var(--space-sm);
	}

	.session-tools {
		position: fixed;
		z-index: 30;
		left: 0;
		right: 0;
		bottom: 0;
		max-height: 82vh;
		overflow-y: auto;
		box-sizing: border-box;
		padding: var(--space-lg);
		padding-bottom: calc(var(--space-lg) + env(safe-area-inset-bottom));
		background: var(--color-surface);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-card) var(--radius-card) 0 0;
	}

	.tools-heading {
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--space-lg);
	}

	.tools-heading h2,
	.tool-section h3 {
		margin: 0 0 var(--space-xs);
	}

	.tool-section + .tool-section {
		border-top: 1px solid var(--color-surface-elevated);
		padding-top: var(--space-lg);
		margin-top: var(--space-lg);
	}

	.tool-select {
		width: 100%;
		box-sizing: border-box;
		min-height: 44px;
		padding: 0 var(--space-md);
		border-radius: var(--radius-control);
		border: 1px solid var(--color-surface-elevated);
		background: var(--color-canvas);
		color: var(--color-text-primary);
		font: var(--text-body) var(--font-body);
	}

	.tool-select:hover {
		border-color: var(--color-accent);
	}

	.tool-select:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 1px;
		border-color: var(--color-focus);
	}

	.tool-select:disabled {
		opacity: 0.55;
		cursor: not-allowed;
		background: var(--color-surface-elevated);
	}

	.calculator-fields {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 0 var(--space-md);
	}

	.tool-result {
		margin-top: var(--space-md);
		border-color: var(--color-surface-elevated);
		font-weight: 700;
	}

	.text-size-options {
		align-items: stretch;
	}

	@media (min-width: 768px) {
		.session-bottom-bar {
			padding-inline: var(--space-md);
		}

		.session-tools {
			left: auto;
			top: 0;
			width: min(420px, 42vw);
			max-height: 100vh;
			border-radius: var(--radius-card) 0 0 var(--radius-card);
		}

		.calculator-fields {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
