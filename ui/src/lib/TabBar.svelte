<script lang="ts">
	// Phone bottom bar: the four main sections. Shown by CSS under 640 px.
	let { tabs, path }: { tabs: { href: string; label: string }[]; path: string } = $props();

	const ICONS: Record<string, string> = {
		'/': 'M4 11l8-6 8 6v8a1 1 0 0 1-1 1h-4v-6H9v6H5a1 1 0 0 1-1-1z',
		'/recipes':
			'M7 3v8a2 2 0 0 0 2 2v8M11 3v8a2 2 0 0 1-2 2M9 3v6M17 3c-2 1.5-3 4-3 7 0 1.5 1 2.5 3 3v8',
		'/plan':
			'M5 5h14a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zM4 10h16M8 3v4M16 3v4',
		'/shopping': 'M5 7h14l-1.5 11a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1zM9 10V6a3 3 0 0 1 6 0v4'
	};
</script>

<nav class="bar" aria-label="Sections">
	{#each tabs as t (t.href)}
		<a href={t.href} class:on={path === t.href} aria-current={path === t.href ? 'page' : undefined}>
			<span class="pill">
				<svg
					viewBox="0 0 24 24"
					width="22"
					height="22"
					fill="none"
					stroke="currentColor"
					stroke-width="1.7"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path d={ICONS[t.href]} />
				</svg>
			</span>
			<span class="label">{t.label}</span>
		</a>
	{/each}
</nav>

<style>
	.bar {
		position: fixed;
		inset: auto 0 0 0;
		z-index: 30;
		display: none;
		justify-content: space-around;
		padding: 0.4rem calc(0.25rem + env(safe-area-inset-right))
			calc(0.4rem + env(safe-area-inset-bottom)) calc(0.25rem + env(safe-area-inset-left));
		background: color-mix(in srgb, var(--bg) 94%, transparent);
		backdrop-filter: blur(10px);
		border-top: 1px solid var(--line);
	}
	:global(html.android) .bar {
		padding-bottom: max(env(safe-area-inset-bottom), 1.2rem);
	}
	@media (max-width: 639px) {
		.bar {
			display: flex;
		}
	}
	a {
		display: flex;
		flex: 1;
		flex-direction: column;
		align-items: center;
		gap: 0.15rem;
		min-height: 3.25rem;
		color: var(--ink-2);
		font-size: 0.6875rem;
		font-weight: 500;
	}
	.pill {
		display: grid;
		place-items: center;
		width: 3.5rem;
		height: 1.9rem;
		border-radius: 9999px;
		transition: background-color 120ms;
	}
	a.on {
		color: var(--ink);
	}
	a.on .pill {
		background: var(--sage-bg);
		color: var(--sage);
	}
	a:active .pill {
		background: var(--surface-2);
	}
</style>
