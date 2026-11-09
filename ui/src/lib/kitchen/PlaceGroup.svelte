<script lang="ts">
	import { slide } from 'svelte/transition';
	import type { LotAction, LotView } from '#lib/bindings.js';
	import type { RemovalKind } from './pending.svelte.js';
	import LotRow from './LotRow.svelte';
	import { motionMs } from './motion.js';

	let {
		id,
		label,
		lots,
		emptyText,
		selecting = false,
		selected,
		ontoggle,
		onselectall,
		onact,
		onremove
	}: {
		id: string;
		label: string;
		lots: LotView[];
		emptyText: string;
		selecting?: boolean;
		selected: string[];
		ontoggle: (lot: LotView) => void;
		onselectall: (lots: LotView[]) => void;
		onact: (lot: LotView, action: LotAction) => void;
		onremove: (lot: LotView, kind: RemovalKind) => void;
	} = $props();

	const allSelected = $derived(lots.length > 0 && lots.every((l) => selected.includes(l.id)));
</script>

<section class="card" aria-labelledby="{id}-h">
	<header>
		<h2 id="{id}-h">{label}</h2>
		<span class="count">{lots.length}</span>
		{#if selecting && lots.length}
			<button type="button" class="all" onclick={() => onselectall(lots)}>
				{allSelected ? 'Clear' : 'Select all'}
			</button>
		{/if}
	</header>
	{#if lots.length === 0}
		<p class="empty">{emptyText}</p>
	{:else}
		<ul>
			{#each lots as lot (lot.id)}
				<li out:slide={{ duration: motionMs() }}>
					<LotRow
						{lot}
						{selecting}
						selected={selected.includes(lot.id)}
						ontoggle={() => ontoggle(lot)}
						{onact}
						{onremove}
					/>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.card {
		border-radius: 1.25rem;
		background: var(--surface);
		border: 1px solid var(--line);
		padding-bottom: 0.35rem;
	}
	header {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		padding: 0.95rem 1rem 0.5rem;
	}
	h2 {
		font-size: 0.9375rem;
		font-weight: 600;
		letter-spacing: -0.005em;
	}
	.count {
		font-size: 0.8125rem;
		color: var(--ink-3);
		font-variant-numeric: tabular-nums;
	}
	.all {
		margin-left: auto;
		font-size: 0.8125rem;
		color: var(--sage);
	}
	.all:hover {
		text-decoration: underline;
	}
	li + li {
		border-top: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
	}
	.empty {
		padding: 0.25rem 1rem 0.9rem;
		font-size: 0.875rem;
		color: var(--ink-3);
	}
</style>
