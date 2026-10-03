<!-- Hallmark · pre-emit critique: P4 H4 E4 S4 R5 V4 -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { base } from '$app/paths';
	import { auth, loadAuth } from '$lib/auth.svelte';

	type LaunchContext = {
		message_type?: string;
		resource_link?: {
			title?: string;
			custom?: Record<string, string>;
		};
	};

	let context = $state<LaunchContext | null>(null);
	let error = $state('');

	onMount(() => {
		loadAuth();
		try {
			const saved = sessionStorage.getItem('mlos_lti_launch');
			sessionStorage.removeItem('mlos_lti_launch');
			context = saved ? (JSON.parse(saved) as LaunchContext) : null;
		} catch {
			context = null;
		}

		if (!auth.token) {
			error = 'The LMS session could not be restored. Allow site storage, then launch again from your LMS.';
		} else if (context?.message_type !== 'LtiResourceLinkRequest') {
			error = 'This launch has expired or is incomplete. Return to your LMS and launch the resource again.';
		}
	});

	function articleHref(): string | null {
		const custom = context?.resource_link?.custom;
		const slug = custom?.medicalos_article_slug;
		if (
			custom?.medicalos_content_type !== 'article' ||
			!slug ||
			!/^[a-z0-9](?:[a-z0-9-]{0,118}[a-z0-9])?$/.test(slug)
		) {
			return null;
		}
		const params = new URLSearchParams();
		const jurisdiction = custom.medicalos_jurisdiction;
		if (jurisdiction && /^[A-Z]{2,3}$/.test(jurisdiction)) {
			params.set('jurisdiction', jurisdiction);
		}
		const query = params.toString();
		return `${base}/library/articles/${encodeURIComponent(slug)}${query ? `?${query}` : ''}`;
	}

	const destination = $derived(articleHref());
	const title = $derived(context?.resource_link?.title?.trim() || 'Your LMS launch is ready');
</script>

<svelte:head>
	<title>Continue from your LMS · Medical OS</title>
</svelte:head>

<main class="handoff">
	<section class="card" aria-labelledby="launch-heading">
		<p class="eyebrow">Institutional launch</p>
		{#if error}
			<h1 id="launch-heading">Launch could not continue</h1>
			<p role="alert">{error}</p>
			<a class="btn primary" href={`${base}/login`}>Sign in to Medical OS</a>
		{:else if context}
			<h1 id="launch-heading">{title}</h1>
			<p class="lede">Your institutional session is ready.</p>
			{#if destination}
				<a class="btn primary" href={destination}>Continue to article</a>
			{:else}
				<p class="muted">This resource does not point to a Medical OS article. You can continue to your study plan.</p>
				<a class="btn primary" href={`${base}/today`}>Open today's study plan</a>
			{/if}
		{:else}
			<h1 id="launch-heading">Preparing your launch</h1>
			<p class="muted" role="status">Restoring your Medical OS session…</p>
		{/if}
	</section>
</main>

<style>
	.handoff {
		box-sizing: border-box;
		width: min(100%, 44rem);
		margin: clamp(var(--space-xl), 8vh, 5rem) auto;
		padding-inline: var(--space-lg);
	}

	.eyebrow {
		color: var(--color-accent);
		font-size: var(--text-sm);
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	h1 {
		min-width: 0;
		overflow-wrap: anywhere;
		font-family: var(--font-display);
		font-size: clamp(var(--text-xl), 5vw, var(--text-2xl));
		font-style: normal;
	}

	.card {
		max-width: 44rem;
	}

	.btn {
		min-height: 44px;
	}
</style>
