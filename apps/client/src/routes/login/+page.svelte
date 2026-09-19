<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { setAuthSession } from '$lib/auth.svelte';

	let mode = $state('signin');
	let email = $state('');
	let password = $state('');
	let verificationToken = $state('');
	let resetToken = $state('');
	let guestToken = $state('');
	let guestTrial = $state(null);
	let guestIndex = $state(0);
	let guestFeedback = $state(null);
	let guestChosenIndex = $state(-1);
	let guestIdempotencyKey = $state('');
	let guestBusy = $state(false);
	let busy = $state(false);
	let error = $state('');
	let status = $state('');

	function setMode(next) {
		mode = next;
		error = '';
		status = '';
	}

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const verify = params.get('verify_token');
		const reset = params.get('reset_token');
		if (verify) {
			verificationToken = verify;
			mode = 'verify';
		} else if (reset) {
			resetToken = reset;
			mode = 'reset';
		}
	});

	async function submit(event) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		status = '';
		try {
			if (mode === 'register') {
				const registered = await Api.register(email.trim(), password, guestToken || undefined);
				verificationToken = registered.verification_token ?? '';
				const migrated = registered.guest_answers_migrated;
				guestToken = '';
				mode = 'verify';
				status = migrated
					? `Account created and ${migrated} sample answer${migrated === 1 ? '' : 's'} saved. Verify your email before sign-in.`
					: 'Account created. Verification is required before sign-in. Enter the verification token from your configured verification channel.';
				return;
			}
			if (mode === 'verify') {
				await Api.verifyEmail(verificationToken.trim());
				mode = 'signin';
				status = 'Email verified. Sign in to continue.';
				return;
			}
			if (mode === 'forgot') {
				const recovery = await Api.forgotPassword(email.trim());
				if (recovery.reset_token) {
					resetToken = recovery.reset_token;
					mode = 'reset';
					status = 'Recovery token is available in this test environment.';
				} else {
					status =
						'If this account is eligible, a reset token will be issued through the configured recovery channel.';
				}
				return;
			}
			if (mode === 'reset') {
				await Api.resetPassword(resetToken.trim(), password);
				mode = 'signin';
				status = 'Password reset. Sign in with your new password.';
				return;
			}
			const session = await Api.login(email.trim(), password);
			setAuthSession(session);
			goto(`${base}/today`);
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Something went wrong. Check your connection and try again.';
		} finally {
			busy = false;
		}
	}

	async function startGuestTrial() {
		if (guestBusy) return;
		guestBusy = true;
		error = '';
		status = '';
		try {
			guestTrial = await Api.guestTrialStart();
			guestToken = guestTrial.trial_token;
			guestIndex = 0;
			guestFeedback = null;
			guestChosenIndex = -1;
			guestIdempotencyKey = '';
			mode = 'guest';
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Sample questions are unavailable right now.';
		} finally {
			guestBusy = false;
		}
	}

	async function answerGuest(chosenIndex) {
		const item = guestTrial?.items?.[guestIndex];
		if (!item || !guestToken || guestBusy || guestFeedback) return;
		guestBusy = true;
		error = '';
		guestChosenIndex = chosenIndex;
		guestIdempotencyKey ||= crypto.randomUUID();
		try {
			guestFeedback = await Api.guestTrialAnswer({
				trial_token: guestToken,
				item_index: item.item_index,
				chosen_index: chosenIndex,
				idempotency_key: guestIdempotencyKey
			});
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save that answer. Try again.';
		} finally {
			guestBusy = false;
		}
	}

	function nextGuest() {
		if (!guestTrial || !guestFeedback) return;
		if (guestIndex + 1 < guestTrial.items.length) {
			guestIndex += 1;
			guestFeedback = null;
			guestChosenIndex = -1;
			guestIdempotencyKey = '';
			return;
		}
		mode = 'register';
		error = '';
		status = 'Create your account to keep the sample answers you just completed.';
	}
</script>

<section class="card">
	<h1>
		{mode === 'signin'
			? 'Welcome back'
			: mode === 'guest'
				? 'Try sample questions'
			: mode === 'register'
				? 'Create your account'
				: mode === 'verify'
					? 'Verify your email'
					: mode === 'forgot'
						? 'Reset your password'
						: 'Choose a new password'}
	</h1>
	<p class="muted">One connected learning record for your whole medical education.</p>

	{#if error}
		<p class="error-text" role="alert">{error}</p>
	{/if}
	{#if status}
		<p class="muted" role="status" aria-live="polite">{status}</p>
	{/if}

	{#if mode !== 'guest'}
	<form onsubmit={submit}>
		{#if mode === 'signin' || mode === 'register' || mode === 'forgot'}
			<label class="field" for="email">
				<span>Email</span>
				<input
					id="email"
					type="email"
					bind:value={email}
					autocomplete="email"
					required
					data-testid="email"
				/>
			</label>
		{/if}
		{#if mode === 'verify'}
			<label class="field" for="verification-token">
				<span>Verification token</span>
				<input
					id="verification-token"
					type="text"
					bind:value={verificationToken}
					autocomplete="one-time-code"
					required
					data-testid="verification-token"
				/>
			</label>
		{/if}
		{#if mode === 'reset'}
			<label class="field" for="reset-token">
				<span>Reset token</span>
				<input
					id="reset-token"
					type="text"
					bind:value={resetToken}
					autocomplete="one-time-code"
					required
					data-testid="reset-token"
				/>
			</label>
		{/if}
		{#if mode === 'signin' || mode === 'register' || mode === 'reset'}
			<label class="field" for="password">
				<span>{mode === 'reset' ? 'New password' : 'Password'}</span>
				<input
					id="password"
					type="password"
					bind:value={password}
					autocomplete={mode === 'signin' ? 'current-password' : 'new-password'}
					minlength={8}
					required
					data-testid="password"
				/>
			</label>
		{/if}
		<button
			class="btn primary"
			type="submit"
			disabled={busy}
			data-loading={busy}
			data-testid="submit"
		>
			{busy
				? 'Working…'
				: mode === 'signin'
					? 'Sign in'
					: mode === 'register'
						? 'Create account'
						: mode === 'verify'
							? 'Verify email'
							: mode === 'forgot'
								? 'Request reset'
								: 'Reset password'}
		</button>
	</form>
	{/if}

	{#if mode === 'guest' && guestTrial}
		{@const item = guestTrial.items[guestIndex]}
		{#if item}
			<p class="chip">Sample {guestIndex + 1} of {guestTrial.items.length} · {item.difficulty}</p>
			<p>{item.vignette}</p>
			<p><strong>{item.lead_in}</strong></p>
			<div class="options" role="group" aria-label="Answer choices">
				{#each item.options as option, index}
					<button
						class="option"
						class:correct={guestFeedback && index === guestFeedback.correct_index}
						class:incorrect={guestFeedback && index === guestChosenIndex && index !== guestFeedback.correct_index}
						type="button"
						disabled={guestBusy || Boolean(guestFeedback)}
						onclick={() => answerGuest(index)}
					>
						<span class="key">{String.fromCharCode(65 + index)}</span>
						<span>{option.text}</span>
					</button>
				{/each}
			</div>

			{#if guestFeedback}
				<div class:good={guestFeedback.correct} class:bad={!guestFeedback.correct} class="feedback" role="status">
					<p class="verdict">{guestFeedback.correct ? 'Correct' : 'Not quite'}</p>
					<p>{guestFeedback.key_learning_point}</p>
					{#each guestFeedback.options as option, index}
						<p><strong>{String.fromCharCode(65 + index)}.</strong> {option.rationale}</p>
					{/each}
					{#if guestFeedback.exam_tip}<p><strong>Exam tip:</strong> {guestFeedback.exam_tip}</p>{/if}
				</div>
				<button class="btn primary" type="button" onclick={nextGuest}>
					{guestIndex + 1 < guestTrial.items.length ? 'Next sample' : 'Create account & keep progress'}
				</button>
			{/if}
		{/if}
	{/if}

	{#if mode === 'signin'}
		<button class="linklike" type="button" onclick={() => setMode('forgot')} data-testid="forgot-password">
			Forgot password?
		</button>
		<button class="linklike" type="button" onclick={startGuestTrial} disabled={guestBusy} data-testid="guest-trial">
			Try sample questions
		</button>
	{/if}

	{#if mode === 'signin' || mode === 'register'}
		<button
			class="linklike"
			type="button"
			onclick={() => setMode(mode === 'signin' ? 'register' : 'signin')}
			data-testid="toggle-mode"
		>
			{mode === 'signin' ? 'New here? Create an account' : 'Already have an account? Sign in'}
		</button>
	{:else if mode === 'guest'}
		<button class="linklike" type="button" onclick={() => setMode('signin')}>Back to sign in</button>
	{:else}
		<button class="linklike" type="button" onclick={() => setMode('signin')}>Back to sign in</button>
	{/if}
</section>
