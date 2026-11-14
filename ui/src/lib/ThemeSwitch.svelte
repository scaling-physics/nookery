<script lang="ts">
	import { theme, type ThemeMode } from '#lib/theme.svelte.js';

	const OPTIONS: { id: ThemeMode; label: string }[] = [
		{ id: 'auto', label: 'Auto' },
		{ id: 'light', label: 'Light' },
		{ id: 'dark', label: 'Dark' }
	];
	const index = $derived(OPTIONS.findIndex((o) => o.id === theme.mode));
</script>

<div class="track" role="radiogroup" aria-label="Colour theme" style:--i={index}>
	<span class="thumb" aria-hidden="true"></span>
	{#each OPTIONS as o (o.id)}
		<label class:on={theme.mode === o.id}>
			<input
				type="radio"
				name="theme"
				value={o.id}
				checked={theme.mode === o.id}
				onchange={() => theme.set(o.id)}
			/>
			<svg viewBox="0 0 16 16" aria-hidden="true">
				{#if o.id === 'auto'}
					<circle cx="8" cy="8" r="5.25" />
					<path d="M8 2.75a5.25 5.25 0 0 1 0 10.5z" class="fill" />
				{:else if o.id === 'light'}
					<circle cx="8" cy="8" r="2.75" />
					<path
						d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1.06 1.06M11.54 11.54l1.06 1.06M3.4 12.6l1.06-1.06M11.54 4.46l1.06-1.06"
					/>
				{:else}
					<path d="M13 9.6A5.5 5.5 0 0 1 6.4 3a5.5 5.5 0 1 0 6.6 6.6z" />
				{/if}
			</svg>
			{o.label}
		</label>
	{/each}
</div>

<style>
	.track {
		position: relative;
		display: inline-grid;
		grid-template-columns: repeat(3, minmax(5.25rem, 1fr));
		padding: 3px;
		border: 1px solid var(--line);
		border-radius: 9999px;
		background: var(--surface-2);
	}
	.thumb {
		position: absolute;
		top: 3px;
		bottom: 3px;
		left: 3px;
		width: calc((100% - 6px) / 3);
		border-radius: 9999px;
		background: var(--surface);
		box-shadow:
			0 1px 2px rgb(0 0 0 / 0.12),
			0 0 0 1px var(--line);
		transform: translateX(calc(var(--i) * 100%));
		transition: transform 220ms cubic-bezier(0.3, 0.7, 0.2, 1);
	}
	@media (prefers-reduced-motion: reduce) {
		.thumb {
			transition: none;
		}
	}
	label {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0 0.9rem;
		border-radius: 9999px;
		font-size: 0.875rem;
		color: var(--ink-2);
		cursor: pointer;
		transition: color 150ms;
	}
	label:hover,
	label.on {
		color: var(--ink);
	}
	label.on {
		font-weight: 500;
	}
	label:has(input:focus-visible) {
		outline: 2px solid var(--focus);
		outline-offset: 1px;
	}
	input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}
	svg {
		width: 1rem;
		height: 1rem;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.4;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	label.on svg {
		stroke: var(--sage);
	}
	.fill {
		fill: currentColor;
		stroke: none;
	}
	label.on .fill {
		fill: var(--sage);
	}
	@media (max-width: 639px), (pointer: coarse) {
		label {
			min-height: 2.75rem;
		}
	}
</style>
