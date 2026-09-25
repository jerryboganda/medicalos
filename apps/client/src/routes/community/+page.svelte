<script lang="ts">
	/* Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 */
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	// COMMUNITY-01/02/03 + COMP-03/GROW-01: everything here is opt-in.
	// Identity starts with a handle; groups are moderated; duels are private
	// challenges between two people.

	let profile = $state<{ opted_in: boolean; handle?: string } | null>(null);
	let handle = $state('');
	let busy = $state(false);
	let message = $state('');
	let error = $state('');

	let groups = $state<
		{ group_id: string; name: string; members: number }[]
	>([]);
	let examId = $state('');
	let activeGroup = $state('');
	let isGroupMember = $state(false);
	let isGroupModerator = $state(false);
	let posts = $state<Awaited<ReturnType<typeof Api.listGroupPosts>>['posts']>([]);
	let postBody = $state('');
	type ReportReason = Parameters<typeof Api.reportGroupPost>[2];
	let reportTarget = $state('');
	let reportReason = $state<ReportReason>('spam');
	let reportNote = $state('');
	let myReports = $state<
		Awaited<ReturnType<typeof Api.myCommunityPostReports>>['reports'] | null
	>(null);
	let reportQueue = $state<
		Awaited<ReturnType<typeof Api.groupPostReportQueue>>['reports'] | null
	>(null);
	let reportQueueState = $state<'loading' | 'loaded' | 'failed'>('loading');

	let duelHandle = $state('');
	let duelCount = $state(5);
	let duels = $state<
		{
			duel_id: string;
			status: string;
			question_count: number;
			winner: string | null;
			sent_by_me: boolean;
			challenger: string;
			opponent: string;
		}[]
	>([]);
	let lastShare = $state('');
	let cards = $state<
		{
			kind: string;
			headline: string;
			subline: string;
			detail: string;
			share_text: string;
		}[]
	>([]);
	let unavailableCards = $state<{ kind: string; reason: string }[]>([]);

	async function refreshMyReports() {
		myReports = null;
		try {
			myReports = (await Api.myCommunityPostReports()).reports;
		} catch {
			myReports = null;
		}
	}

	async function refresh() {
		try {
			profile = await Api.communityProfile();
		} catch {
			profile = { opted_in: false };
		}
		try {
			duels = (await Api.myDuels()).duels as typeof duels;
		} catch {
			duels = [];
		}
		try {
			groups = (await Api.listGroups()).groups;
		} catch {
			groups = [];
		}
		await refreshMyReports();
		try {
			const chapters = (await Api.myCurriculum()).chapters;
			examId = chapters[0]?.exam_id ?? '';
		} catch {
			examId = '';
		}
		try {
			const share = await Api.shareCards();
			cards = share.cards;
			unavailableCards = share.unavailable;
		} catch {
			cards = [];
			unavailableCards = [];
		}
	}

	function copyShare(text: string) {
		void navigator.clipboard?.writeText(text);
		lastShare = text;
	}

	function downloadCard(card: (typeof cards)[number]) {
		const styles = getComputedStyle(document.documentElement);
		const token = (name: string, fallback: string) =>
			styles.getPropertyValue(name).trim() || fallback;
		const canvas = token('--color-canvas', '#09090f');
		const surface = token('--color-surface', '#14141e');
		const accent = token('--color-accent', '#a78bfa');
		const ink = token('--color-text-primary', '#f5f3ff');
		const muted = token('--color-text-secondary', '#b8b4c6');
		const esc = (s: string) =>
			s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
		const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="315" viewBox="0 0 600 315">
	<rect width="600" height="315" rx="20" fill="${canvas}"/>
	<rect x="1" y="1" width="598" height="313" rx="19" fill="${surface}" stroke="${accent}"/>
	<text x="40" y="70" fill="${muted}" font-family="system-ui" font-size="15">Medical Learning OS</text>
	<text x="40" y="150" fill="${ink}" font-family="Georgia, serif" font-size="58" font-weight="600">${esc(card.headline)}</text>
	<text x="40" y="195" fill="${accent}" font-family="system-ui" font-size="20">${esc(card.subline)}</text>
	<text x="40" y="235" fill="${muted}" font-family="system-ui" font-size="16">${esc(card.detail)}</text>
	<text x="40" y="280" fill="${muted}" font-family="system-ui" font-size="13">Real numbers from real attempts — nothing invented.</text>
</svg>`;
		const blob = new Blob([svg], { type: 'image/svg+xml' });
		const url = URL.createObjectURL(blob);
		const anchor = document.createElement('a');
		anchor.href = url;
		anchor.download = `share-${card.kind}.svg`;
		anchor.click();
		URL.revokeObjectURL(url);
	}

	async function optIn(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			await Api.createCommunityProfile(handle);
			await refresh();
			message = `You're in as ${handle}.`;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not save the handle.';
		} finally {
			busy = false;
		}
	}

	async function createGroup(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			const nameEl = e.target as HTMLFormElement;
			const input = nameEl.querySelector('input') as HTMLInputElement;
			const res = await Api.createCommunityGroup(input.value);
			await refresh();
			input.value = '';
			await openGroup(res.group_id);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not create the group.';
		} finally {
			busy = false;
		}
	}

	async function openGroup(id: string) {
		activeGroup = id;
		isGroupMember = false;
		isGroupModerator = false;
		posts = [];
		reportQueue = null;
		reportQueueState = 'loading';
		reportTarget = '';
		error = '';
		let feed: Awaited<ReturnType<typeof Api.listGroupPosts>>;
		try {
			feed = await Api.listGroupPosts(id);
		} catch (err) {
			if (activeGroup !== id) return;
			if (err instanceof ApiError && err.message.includes('join')) {
				isGroupMember = false;
			} else {
				error = err instanceof ApiError ? err.message : 'Could not load this group.';
			}
			return;
		}
		if (activeGroup !== id) return;
		posts = feed.posts;
		isGroupMember = true;
		isGroupModerator = feed.is_moderator;
		if (feed.is_moderator) {
			try {
				const queue = await Api.groupPostReportQueue(id);
				if (activeGroup !== id) return;
				reportQueue = queue.reports;
				reportQueueState = 'loaded';
			} catch (err) {
				if (activeGroup === id) {
					reportQueueState = 'failed';
					error = err instanceof ApiError ? err.message : 'Could not load moderator reports.';
				}
			}
		}
	}

	async function joinGroup() {
		if (!activeGroup || busy) return;
		busy = true;
		try {
			await Api.joinGroup(activeGroup);
			await openGroup(activeGroup);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not join this group.';
		} finally {
			busy = false;
		}
	}

	async function post(e: Event) {
		e.preventDefault();
		if (!activeGroup || busy) return;
		busy = true;
		error = '';
		try {
			await Api.createGroupPost(activeGroup, postBody);
			postBody = '';
			await openGroup(activeGroup);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not post.';
		} finally {
			busy = false;
		}
	}

	async function moderate(postId: string) {
		if (!activeGroup || busy) return;
		busy = true;
		try {
			await Api.removeGroupPost(activeGroup, postId);
			await openGroup(activeGroup);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not remove the post.';
		} finally {
			busy = false;
		}
	}

	async function reportPost(e: Event) {
		e.preventDefault();
		if (!activeGroup || !reportTarget || busy) return;
		busy = true;
		error = '';
		try {
			await Api.reportGroupPost(activeGroup, reportTarget, reportReason, reportNote);
			reportTarget = '';
			reportNote = '';
			message = 'Report sent to this group’s moderators.';
			await refreshMyReports();
			await openGroup(activeGroup);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not send the report.';
		} finally {
			busy = false;
		}
	}

	async function resolveReport(reportId: string, action: 'dismiss' | 'remove') {
		if (!activeGroup || busy) return;
		busy = true;
		error = '';
		try {
			await Api.resolveGroupPostReport(activeGroup, reportId, action);
			message = action === 'remove' ? 'Post removed and reports resolved.' : 'Report dismissed.';
			await refreshMyReports();
			await openGroup(activeGroup);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not resolve the report.';
		} finally {
			busy = false;
		}
	}

	async function challenge(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			const opponent = await Api.profileByHandle(duelHandle.trim());
			if (!examId) {
				throw new Error('No exam curriculum is available to duel on yet.');
			}
			const res = await Api.createDuel(
				opponent.user_id,
				examId,
				duelCount
			);
			lastShare = res.share_token;
			await refresh();
		} catch (err) {
			error =
				err instanceof ApiError
					? err.message
					: 'Could not create the duel.';
		} finally {
			busy = false;
		}
	}

	async function act(duelId: string, action: 'accept' | 'decline') {
		busy = true;
		error = '';
		try {
			if (action === 'accept') {
				const res = await Api.acceptDuel(duelId);
				goto(`${base}/session/${res.your_session_id}`);
				return;
			}
			await Api.declineDuel(duelId);
			await refresh();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Duel action failed.';
		} finally {
			busy = false;
		}
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await refresh();
	});
</script>

<h1>Community</h1>

{#if profile && !profile.opted_in}
	<div class="card">
		<h2>Opt in with a handle</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Nothing about you is shared until you pick a handle. Handles are
			public within the community; your account, notes, and record never
			are.
		</p>
		<form onsubmit={optIn}>
			<label class="field" for="handle">
				<span>Handle (a-z, 0-9, dash)</span>
				<input id="handle" bind:value={handle} data-testid="community-handle" />
			</label>
			<button class="btn primary" type="submit" disabled={busy || !handle}>
				Join the community
			</button>
		</form>
		{#if error}<p class="danger-text">{error}</p>{/if}
	</div>
{:else if profile}
	{#if message}<p class="muted" role="status" aria-live="polite">{message}</p>{/if}

	<div class="card">
		<h2>Duels</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Challenge someone by handle. They accept or decline — a link never
			enters anyone automatically.
		</p>
		<form onsubmit={challenge} style="display:flex; gap:12px; flex-wrap:wrap; align-items:end;">
			<label class="field" for="duel-handle">
				<span>Opponent handle</span>
				<input id="duel-handle" bind:value={duelHandle} data-testid="duel-handle" />
			</label>
			<label class="field" for="duel-count">
				<span>Questions</span>
				<input id="duel-count" type="number" min="3" max="20" bind:value={duelCount} />
			</label>
			<button class="btn primary" type="submit" disabled={busy || !duelHandle}>
				Challenge
			</button>
		</form>
		{#if lastShare}
			<p class="muted" style="font-size: var(--text-sm);">
				Share token: <code>{lastShare}</code>
			</p>
		{/if}
		{#if error}<p class="danger-text">{error}</p>{/if}
		<h3>Your duels</h3>
		{#if duels.length === 0}
			<p class="muted">No duels yet.</p>
		{:else}
			<ul>
				{#each duels as d (d.duel_id)}
					<li data-testid="duel-row">
						{d.challenger} vs {d.opponent} — {d.status}
						{#if d.status === 'pending' && !d.sent_by_me}
							<button
								class="btn primary"
								type="button"
								disabled={busy}
								onclick={() => act(d.duel_id, 'accept')}
							>
								Accept
							</button>
							<button
								class="btn"
								type="button"
								disabled={busy}
								onclick={() => act(d.duel_id, 'decline')}
							>
								Decline
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	<div class="card">
		<h2>Groups</h2>
		<form onsubmit={createGroup} style="display:flex; gap:12px; align-items:end;">
			<label class="field" for="group-name">
				<span>New group</span>
				<input id="group-name" placeholder="Anatomy cram" />
			</label>
			<button class="btn" type="submit" disabled={busy}>Create</button>
		</form>
		<div style="margin-top: var(--space-md);">
			<h3>All groups</h3>
			{#if groups.length === 0}
				<p class="muted">No groups exist yet — create the first one.</p>
			{:else}
				<ul>
					{#each groups as g (g.group_id)}
						<li>
							<button
								class="btn"
								type="button"
								disabled={busy}
								onclick={() => openGroup(g.group_id)}
							>
								{g.name}
							</button>
							<span class="muted" style="font-size: var(--text-sm);">
								{g.members} member(s)
							</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
		{#if activeGroup}
			<h3>Group feed</h3>
			{#if error}<p class="danger-text" role="alert">{error}</p>{/if}
			{#if !isGroupMember}
				<p class="muted">Join this group to read, post, and report content.</p>
				<button class="btn primary" type="button" disabled={busy} onclick={joinGroup}>
					Join this group
				</button>
			{:else}
				<form onsubmit={post}>
					<label class="field" for="post-body">
						<span>Post to the group</span>
						<textarea id="post-body" bind:value={postBody} rows="2"></textarea>
					</label>
					<button class="btn" type="submit" disabled={busy || !postBody}>
						Post
					</button>
				</form>
				{#if posts.length === 0}
					<p class="muted">No posts visible here yet.</p>
				{:else}
					<ul>
						{#each posts as p (p.post_id)}
							<li data-testid="community-post">
								<strong>{p.handle}</strong>:
								{p.status === 'removed'
									? '(removed by a moderator)'
									: p.body}
								{#if p.status === 'visible'}
									{#if isGroupModerator}
										<button
										class="btn"
										type="button"
										disabled={busy}
										onclick={() => moderate(p.post_id)}
										data-testid="moderator-remove-post"
									>
										Remove post
										</button>
									{/if}
									<button
									class="btn"
									type="button"
									disabled={busy}
									aria-expanded={reportTarget === p.post_id}
									aria-controls={`report-form-${p.post_id}`}
									onclick={() => {
										reportTarget = p.post_id;
										reportReason = 'spam';
										reportNote = '';
									}}
								>
									Report
									</button>
								{/if}
								{#if reportTarget === p.post_id}
									<form id={`report-form-${p.post_id}`} onsubmit={reportPost}>
										<fieldset>
										<legend>Report this post to group moderators</legend>
										<label class="field" for="report-reason">
											<span>Reason</span>
											<select id="report-reason" bind:value={reportReason}>
												<option value="spam">Spam</option>
												<option value="harassment">Harassment</option>
												<option value="medical_misinformation">Medical misinformation</option>
												<option value="other">Other</option>
											</select>
										</label>
										<label class="field" for="report-note">
											<span>Note (optional, up to 500 characters)</span>
											<textarea
												id="report-note"
												bind:value={reportNote}
												maxlength="500"
												rows="3"
											></textarea>
										</label>
										<button class="btn primary" type="submit" disabled={busy}>
											Send report
										</button>
										<button class="btn" type="button" disabled={busy} onclick={() => (reportTarget = '')}>
											Cancel
										</button>
									</fieldset>
									</form>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			{/if}
			{#if isGroupModerator}
				<section aria-labelledby="report-queue-heading" data-testid="moderator-report-queue">
					<h3 id="report-queue-heading">Open reports for this group</h3>
					{#if reportQueueState === 'failed'}
						<p class="muted">The report queue could not be loaded.</p>
					{:else if reportQueueState === 'loading'}
						<p class="muted">Loading reports…</p>
					{:else if reportQueue?.length === 0}
						<p class="muted">No reports need review.</p>
					{:else}
						<ul>
							{#each reportQueue ?? [] as report (report.report_id)}
								<li data-testid="moderation-report">
									<strong>{report.reason.replaceAll('_', ' ')}</strong>
									from {report.author_handle}: {report.post_body}
									{#if report.note}<p>{report.note}</p>{/if}
									<button
									class="btn"
									type="button"
									disabled={busy}
									onclick={() => resolveReport(report.report_id, 'dismiss')}
								>
									Dismiss report
									</button>
									<button
									class="btn"
									type="button"
									disabled={busy}
									onclick={() => resolveReport(report.report_id, 'remove')}
								>
									Remove post
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</section>
			{/if}
		{/if}
		{#if error && !activeGroup}<p class="danger-text" role="alert">{error}</p>{/if}
		<section aria-labelledby="my-reports-heading" data-testid="my-community-reports">
			<h3 id="my-reports-heading">Your post reports</h3>
			{#if myReports === null}
				<p class="muted">Report status is temporarily unavailable.</p>
			{:else if myReports.length === 0}
				<p class="muted">You have not reported any posts.</p>
			{:else}
				<ul>
					{#each myReports as report (report.report_id)}
						<li>
							{report.group_name} · {report.reason.replaceAll('_', ' ')} · {report.status.replaceAll('_', ' ')}
							<time datetime={report.created_at}>{new Date(report.created_at).toLocaleString()}</time>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	</div>
{/if}

{#if cards.length > 0 || unavailableCards.length > 0}
	<div class="card">
		<h2>Share cards</h2>
		<p class="muted" style="font-size: var(--text-sm);">
			Real numbers from your own record only. Cards never contain question
			content, and nothing is shared until you share it.
		</p>
		{#each cards as card (card.kind)}
			<div class="share-card" data-testid={'share-' + card.kind}>
				<strong class="share-headline">{card.headline}</strong>
				<span class="muted">{card.subline}</span>
				<span class="muted small">{card.detail}</span>
				<div style="display:flex; gap:8px; flex-wrap:wrap; margin-top:8px;">
					<button
						class="btn"
						type="button"
						onclick={() => copyShare(card.share_text)}
					>
						Copy share text
					</button>
					<button
						class="btn"
						type="button"
						onclick={() => downloadCard(card)}
					>
						Download card
					</button>
				</div>
			</div>
		{/each}
		{#each unavailableCards as item (item.kind)}
			<p class="muted small" data-testid={'share-unavailable-' + item.kind}>
				{item.kind}: {item.reason}
			</p>
		{/each}
		{#if lastShare}
			<p class="feedback" role="status" data-testid="share-copied">Copied to the clipboard.</p>
		{/if}
	</div>
{/if}
