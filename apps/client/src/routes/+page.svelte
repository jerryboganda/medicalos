<script>
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/auth.svelte';

	onMount(() => {
		goto(auth.token ? `${base}/today` : `${base}/login`);
	});
</script>

<p class="muted">Loading…</p>
