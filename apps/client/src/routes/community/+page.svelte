<script lang="ts">
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
	let posts = $state<
		{ post_id: string; body: string; status: string; handle: string }[]
	>([]);
	let postBody = $state('');

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
		try {
			const chapters = (await Api.myCurriculum()).chapters;
			examId = chapters[0]?.exam_id ?? '';
		} catch {
			examId = '';
		}
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
		error = '';
		try {
			posts = (await Api.listGroupPosts(id)).posts as typeof posts;
		} catch (err) {
			posts = [];
			if (!(err instanceof ApiError && err.message.includes('join'))) {
				// join-required is expected; anything else stays silent-empty.
			}
		}
	}

	async function joinGroup() {
		if (!activeGroup || busy) return;
		busy = true;
		try {
			await Api.joinGroup(activeGroup);
			await openGroup(activeGroup);
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
	{#if message}<p class="muted">{message}</p>{/if}

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
			<form onsubmit={post}>
				<label class="field" for="post-body">
					<span>Post to the group</span>
					<textarea id="post-body" bind:value={postBody} rows="2"></textarea>
				</label>
				<button class="btn" type="submit" disabled={busy || !postBody}>
					Post
				</button>
				<button class="btn" type="button" disabled={busy} onclick={joinGroup}>
					Join this group
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
								<button
									class="btn"
									type="button"
									disabled={busy}
									onclick={() => moderate(p.post_id)}
								>
									Remove
								</button>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</div>
{/if}
