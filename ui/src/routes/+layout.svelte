<script lang="ts">
	import './layout.css';
	import { page } from '$app/state';
	import Logo from '#lib/Logo.svelte';
	import TabBar from '#lib/TabBar.svelte';
	import { onMount } from 'svelte';
	import { watchDialogs } from '#lib/back.svelte.js';
	import { isAndroid } from '#lib/platform.js';
	import { autosync, startAutoSync } from '#lib/phone/autosync.svelte.js';
	import SyncBadge from '#lib/phone/SyncBadge.svelte';

	let { children } = $props();

	// Phones: the banner slims down once you scroll (hysteresis, so it
	// doesn't flicker when its own height changes).
	let slim = $state(false);
	function onscroll() {
		const y = window.scrollY;
		if (!slim && y > 90) slim = true;
		else if (slim && y < 12) slim = false;
	}

	const TABS = [
		{ href: '/', label: 'Kitchen' },
		{ href: '/recipes', label: 'Recipes' },
		{ href: '/plan', label: 'Plan' },
		{ href: '/shopping', label: 'Shopping' }
	];
	const path = $derived(page.url.pathname.replace(/\/+$/, '') || '/');
	// The tab bar belongs to the main sections; photo and receipt are sub-screens
	// with their own header and a way back.
	onMount(() => {
		const stopWatching = watchDialogs();
		// The phone syncs with the Mac at start and whenever it comes back.
		const stopSync = isAndroid ? startAutoSync() : undefined;
		return () => {
			stopWatching?.();
			stopSync?.();
		};
	});
	const showTabs = $derived(TABS.some((t) => t.href === path));
</script>

<svelte:head>
	<title>Nookery</title>
</svelte:head>

<svelte:window {onscroll} />

{#if showTabs}
	<header class="banner" class:slim>
		<svg class="sprig" viewBox="0 0 220 120" aria-hidden="true" focusable="false">
			<g fill="none" stroke="var(--parsley)" stroke-width="1.6" stroke-linecap="round">
				<path d="M215 118C190 90 160 70 120 58" />
				<path
					d="M190 94C180 80 178 66 184 52M160 76C150 64 148 50 154 38M140 66C128 58 120 46 122 34"
				/>
			</g>
			<g fill="var(--parsley)">
				<circle cx="184" cy="48" r="9" /><circle cx="154" cy="34" r="8" /><circle
					cx="122"
					cy="30"
					r="7"
				/>
				<circle cx="205" cy="76" r="7" /><circle cx="172" cy="94" r="7" /><circle
					cx="130"
					cy="70"
					r="6"
				/>
			</g>
			<circle cx="96" cy="50" r="11" fill="var(--lemon)" />
		</svg>
		<div class="inner mx-auto flex max-w-6xl items-center justify-between gap-4">
			<a href="/" class="brand" aria-label="Nookery, home">
				<span class="mark"><Logo size={64} /></span>
				<span class="words">
					<span class="wordmark">Nookery</span>
					<span class="tagline">Your kitchen, kept kindly</span>
				</span>
			</a>
			<div class="flex items-center gap-1">
				{#if isAndroid}<SyncBadge />{/if}
				<nav aria-label="Sections" class="tabs desk-only">
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
						class="gear-icon"
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
		</div>
		<svg class="scallops" width="100%" height="10" aria-hidden="true" focusable="false">
			<defs>
				<pattern id="arches" width="28" height="10" patternUnits="userSpaceOnUse">
					<path d="M0 10V10Q0 10 0 10M0 10C0 3 6 0 14 0S28 3 28 10Z" fill="var(--bg)" />
				</pattern>
			</defs>
			<rect width="100%" height="10" fill="url(#arches)" />
		</svg>
	</header>
{/if}

<!-- A sync that changed the kitchen reloads the main pages (only on the phone; it
     never runs elsewhere, so `version` stays 0). Photo, receipt and settings are
     left alone: they may be in the middle of something (the camera returns here). -->
{#key showTabs ? autosync.version : 0}
	{@render children()}
{/key}

{#if showTabs}
	<div class="tabbar-spacer" aria-hidden="true"></div>
	<TabBar tabs={TABS} {path} />
{/if}

<style>
	.banner {
		position: relative;
		overflow: hidden;
		margin-bottom: 0.5rem;
		padding: 1.4rem 2rem 2.2rem;
		background:
			radial-gradient(
				ellipse 60% 120% at 12% 0%,
				color-mix(in srgb, var(--lemon) 16%, transparent),
				transparent 70%
			),
			linear-gradient(180deg, var(--surface) 0%, var(--surface-2) 100%);
		border-bottom: 1px solid var(--line);
	}
	.inner {
		position: relative;
		z-index: 1;
	}
	.sprig {
		position: absolute;
		right: 0;
		bottom: 0;
		width: 300px;
		height: 164px;
		opacity: 0.1;
		pointer-events: none;
	}
	.scallops {
		position: absolute;
		left: 0;
		bottom: -1px;
		width: 100%;
	}
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 1rem;
		color: var(--ink);
		min-width: 0;
	}
	.words {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
	}
	.wordmark {
		font-size: 34px;
		font-weight: 650;
		letter-spacing: -0.045em;
		line-height: 1;
	}
	.tagline {
		font-size: 0.875rem;
		color: var(--ink-3);
		letter-spacing: 0.01em;
		line-height: 1.1;
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
		background: color-mix(in srgb, var(--bg) 70%, transparent);
	}
	.tabs {
		display: inline-flex;
		gap: 0.25rem;
		border-radius: 9999px;
		background: color-mix(in srgb, var(--bg) 70%, transparent);
		padding: 0.25rem;
	}
	.tab {
		border-radius: 9999px;
		padding: 0.4rem 1.1rem;
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
	@media (max-width: 639px) {
		.banner {
			position: sticky;
			top: 0;
			z-index: 20;
			padding: calc(0.6rem + env(safe-area-inset-top)) 0.5rem 1.1rem
				calc(1.1rem + env(safe-area-inset-left));
			transition: padding 160ms;
		}
		.banner.slim {
			padding-top: calc(0.25rem + env(safe-area-inset-top));
			padding-bottom: 0.9rem;
		}
		.brand {
			gap: 0.7rem;
		}
		.mark :global(svg) {
			width: 41px;
			height: 48px;
			transition:
				width 160ms,
				height 160ms;
		}
		.slim .mark :global(svg) {
			width: 27px;
			height: 32px;
		}
		.wordmark {
			font-size: 26px;
		}
		.tagline {
			font-size: 0.75rem;
			max-height: 1rem;
			overflow: hidden;
			transition:
				max-height 160ms,
				opacity 160ms;
		}
		.slim .tagline {
			max-height: 0;
			opacity: 0;
		}
		.sprig {
			width: 180px;
			height: 98px;
			right: 3rem;
		}
	}
	@media (max-width: 359px) {
		.tagline {
			display: none;
		}
	}
</style>
