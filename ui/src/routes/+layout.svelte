<script lang="ts">
	import './layout.css';
	import { page } from '$app/state';
	import Logo from '#lib/Logo.svelte';

	let { children } = $props();

	const TABS = [
		{ href: '/', label: 'Kitchen' },
		{ href: '/recipes', label: 'Recipes' }
	];
	const path = $derived(page.url.pathname.replace(/\/+$/, '') || '/');
	// The tab bar belongs to the two tabs; photo and receipt are sub-screens
	// with their own header and a way back.
	const showTabs = $derived(TABS.some((t) => t.href === path));
</script>

<svelte:head>
	<title>Nookery</title>
</svelte:head>

{#if showTabs}
	<header
		class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-x-6 gap-y-3 px-5 pt-7 sm:px-8"
	>
		<a href="/" class="brand" aria-label="Nookery, home">
			<Logo size={34} />
			<span class="wordmark" aria-hidden="true">Nookery</span>
		</a>
		<div class="flex items-center gap-1">
			<nav aria-label="Sections" class="tabs">
				{#each TABS as t (t.href)}
					<a
						href={t.href}
						class="tab"
						class:on={path === t.href}
						aria-current={path === t.href ? 'page' : undefined}
					>
						{t.label}
					</a>
				{/each}
			</nav>
			<a href="/settings" class="gear" aria-label="Settings">
				<svg
					viewBox="0 0 24 24"
					width="18"
					height="18"
					fill="none"
					stroke="currentColor"
					stroke-width="1.7"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<circle cx="12" cy="12" r="3" />
					<path
						d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3h0a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5h0a1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9v0a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
					/>
				</svg>
				<span class="hidden sm:inline">Settings</span>
			</a>
		</div>
	</header>
{/if}

{@render children()}

<style>
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		color: var(--ink);
	}
	.wordmark {
		font-size: 24px;
		font-weight: 650;
		letter-spacing: -0.03em;
		line-height: 1;
	}
	.gear {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		border-radius: 9999px;
		padding: 0.4rem 0.7rem;
		font-size: 0.9375rem;
		color: var(--ink-2);
	}
	.gear:hover {
		color: var(--ink);
		background: var(--surface-2);
	}
	.tabs {
		display: inline-flex;
		gap: 0.25rem;
		border-radius: 9999px;
		background: var(--surface-2);
		padding: 0.25rem;
	}
	.tab {
		border-radius: 9999px;
		padding: 0.35rem 1.1rem;
		font-size: 0.9375rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.tab:hover {
		color: var(--ink);
	}
	.tab.on {
		background: var(--surface);
		color: var(--ink);
		font-weight: 500;
		box-shadow: 0 0 0 1px var(--line);
	}
</style>
