<script lang="ts">
	import type { LotView } from '#lib/bindings.js';
	import { displayName, lotMeta, statusLabel } from '#lib/format.js';
	import type { RemovalKind } from './pending.svelte.js';
	import FoodIcon from '#lib/FoodIcon.svelte';
	import Icon from './Icon.svelte';

	let { lots, onremove }: { lots: LotView[]; onremove: (lot: LotView, kind: RemovalKind) => void } =
		$props();
</script>

{#if lots.length}
	<section aria-labelledby="soon-h" class="mb-6">
		<h2
			id="soon-h"
			class="mb-2 text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
		>
			Use first
		</h2>
		<ul class="strip">
			{#each lots as lot (lot.id)}
				{@const expired = statusLabel(lot).tone === 'expired'}
				<li class="card" class:expired>
					<FoodIcon category={lot.category} size={20} />
					<div class="min-w-0 flex-1">
						<p class="truncate text-sm font-medium">
							{displayName(lot.item)}
							<span class="font-normal text-[var(--ink-3)] tabular-nums">{lot.left}</span>
						</p>
						<p
							class="truncate text-xs"
							style:color={expired ? 'var(--terracotta)' : 'var(--saffron)'}
						>
							{lotMeta({ ...lot, store: null, opened: false })}
						</p>
					</div>
					<button
						type="button"
						class="used"
						aria-label="Used up: {displayName(lot.item)}"
						onclick={() => onremove(lot, 'used')}
					>
						<Icon name="check" size={16} />
					</button>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.strip {
		display: flex;
		gap: 0.6rem;
		overflow-x: auto;
		padding: 0.1rem 0.1rem 0.5rem;
		scroll-snap-type: x proximity;
	}
	.card {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.6rem;
		width: 14.5rem;
		scroll-snap-align: start;
		border-radius: 1rem;
		padding: 0.55rem 0.55rem 0.55rem 0.8rem;
		background: var(--surface);
		border: 1px solid var(--line);
		border-left: 3px solid var(--saffron);
	}
	.card.expired {
		border-left-color: var(--terracotta);
	}
	.used {
		display: grid;
		flex: none;
		place-items: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: 9999px;
		color: var(--ink-3);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.used:hover,
	.used:active {
		color: var(--sage);
		background: var(--sage-bg);
	}
</style>
