<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';
	import { auth, loadAuth } from '$lib/auth.svelte';
	import { goto } from '$app/navigation';

	interface Note {
		note_id: string;
		title: string;
		body: string;
		backlinks: { note_id: string; title: string }[];
	}

	let notes = $state<Note[]>([]);
	let title = $state('');
	let body = $state('');
	let busy = $state(false);
	let error = $state('');

	async function load() {
		notes = (await Api.listNotes()).notes;
	}

	async function create(e: Event) {
		e.preventDefault();
		busy = true;
		error = '';
		try {
			await Api.createNote(title, body);
			title = '';
			body = '';
			await load();
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Could not create the note.';
		} finally {
			busy = false;
		}
	}

	async function remove(id: string) {
		await Api.deleteNote(id);
		await load();
	}

	onMount(async () => {
		loadAuth();
		if (!auth.token) {
			goto(`${base}/login`);
			return;
		}
		await load();
	});
</script>

<h1>Notes</h1>

<div class="card">
	<h2>New note</h2>
	<form onsubmit={create}>
		<label class="field" for="note-title">
			<span>Title</span>
			<input id="note-title" bind:value={title} data-testid="note-title" />
		</label>
		<label class="field" for="note-body">
			<span>Note</span>
			<textarea id="note-body" bind:value={body} rows="3" data-testid="note-body"></textarea>
		</label>
		<button class="btn primary" type="submit" disabled={busy}>Save note</button>
	</form>
</div>

{#if notes.length === 0}
	<p class="muted">No notes yet — write your first one above.</p>
{:else}
	{#each notes as note (note.note_id)}
		<div class="card" data-testid="note">
			<strong>{note.title || 'Untitled'}</strong>
			<p class="tight-top pre-wrap">{note.body}</p>
			{#if note.backlinks.length > 0}
				<p class="muted small">
					Linked from: {note.backlinks.map((b) => b.title).join(', ')}
				</p>
			{/if}
			<button class="btn danger-text small" type="button" onclick={() => remove(note.note_id)}>
				Delete
			</button>
		</div>
	{/each}
{/if}

<svelte:head>
	<title>Notes | Medical Learning OS</title>
</svelte:head>
