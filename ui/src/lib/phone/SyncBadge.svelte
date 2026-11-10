<script lang="ts">
	import { autosync } from './autosync.svelte.js';

	let now = $state(Date.now());
	$effect(() => {
		const t = setInterval(() => (now = Date.now()), 20_000);
		return () => clearInterval(t);
	});

	const label = $derived.by(() => {
		if (autosync.state === 'syncing') return 'Syncing…';
		if (autosync.state === 'failed') return 'Can’t reach your Mac';
		if (autosync.state === 'ok' && autosync.at) {
			const mins = Math.floor((now - autosync.at) / 60_000);
			return mins < 1 ? 'Synced just now' : `Synced ${mins} min ago`;
		}
		return '';
	});
</script>

{#if autosync.paired && label}
	<span class="badge" class:bad={autosync.state === 'failed'} role="status" aria-live="polite">
		<span class="dot" aria-hidden="true"></span>{label}
	</span>
{/if}

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.6875rem;
		color: var(--ink-3);
		white-space: nowrap;
	}
	.dot {
		width: 6px;
		height: 6px;
		border-radius: 9999px;
		background: var(--sage);
	}
	.bad {
		color: var(--ink-2);
	}
	.bad .dot {
		background: var(--saffron);
	}
</style>
