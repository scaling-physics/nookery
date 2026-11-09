<script lang="ts">
	import type { RemovalKind } from './pending.svelte.js';

	let {
		count,
		onremove,
		oncancel
	}: { count: number; onremove: (kind: RemovalKind) => void; oncancel: () => void } = $props();
</script>

<div class="bar" role="region" aria-label="Selected items">
	<div class="top">
		<p><span class="tabular-nums">{count}</span> selected</p>
		<button type="button" class="cancel" onclick={oncancel}>Done</button>
	</div>
	<div class="actions">
		<button type="button" disabled={!count} onclick={() => onremove('used')}>Used up</button>
		<button type="button" disabled={!count} onclick={() => onremove('binned')}>Thrown away</button>
		<button type="button" disabled={!count} onclick={() => onremove('wrong')}>Wrong entry</button>
	</div>
</div>

<style>
	.bar {
		position: fixed;
		inset-inline: 0;
		bottom: 0;
		z-index: 45;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem max(1rem, calc((100vw - 72rem) / 2 + 2rem));
		background: var(--surface);
		border-top: 1px solid var(--line);
		box-shadow: 0 -8px 24px -12px color-mix(in srgb, var(--ink) 25%, transparent);
		font-size: 0.875rem;
	}
	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.top .cancel {
		padding: 0.2rem 0.6rem;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	button {
		border-radius: 9999px;
		border: 1px solid var(--line);
		padding: 0.4rem 0.9rem;
		font-weight: 500;
		transition: background-color 120ms;
	}
	button:hover:not(:disabled) {
		background: var(--sage-bg);
		border-color: var(--sage);
	}
	button:disabled {
		opacity: 0.45;
	}
	.cancel {
		border-color: transparent;
		color: var(--ink-2);
	}
</style>
