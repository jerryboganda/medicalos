<script>
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth.svelte';

	onMount(() => {
		goto(auth.token ? '/today' : '/login');
	});
</script>

<p class="muted">Loading…</p>
