<script lang="ts">
	import { fade } from 'svelte/transition';
	import { motionMs } from './motion.js';

	let {
		message,
		onundo,
		raised = false
	}: { message: string; onundo: () => void; raised?: boolean } = $props();
</script>

<div class="wrap" class:raised aria-live="polite" aria-atomic="true">
	{#if message}
		<div class="toast" transition:fade={{ duration: motionMs(120) }}>
			<span>{message}</span>
			<button type="button" onclick={onundo}>Undo</button>
		</div>
	{/if}
</div>

<style>
	.wrap {
		position: fixed;
		inset-inline: 0;
		bottom: 1.25rem;
		z-index: 50;
		display: flex;
		justify-content: center;
		pointer-events: none;
		padding: 0 1rem;
	}
	.wrap.raised {
		bottom: 5rem;
	}
	.toast {
		pointer-events: auto;
		display: flex;
		align-items: center;
		gap: 1rem;
		border-radius: 9999px;
		background: var(--ink);
		color: var(--surface);
		padding: 0.55rem 0.6rem 0.55rem 1.1rem;
		font-size: 0.875rem;
		box-shadow: 0 10px 30px -8px color-mix(in srgb, var(--ink) 50%, transparent);
	}
	button {
		border-radius: 9999px;
		padding: 0.25rem 0.8rem;
		font-weight: 600;
		color: var(--lemon);
	}
	button:hover {
		background: color-mix(in srgb, var(--surface) 15%, transparent);
	}
</style>
