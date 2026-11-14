<script lang="ts">
	// One recipe idea: illustration on top, then name, why, time, what you
	// have and what you would need.
	import type { LotView, SuggestionView } from '../bindings.js';
	import { itemName, statusLabel } from '../format.js';
	import DishArt from '../art/DishArt.svelte';
	import FoodIcon from '../FoodIcon.svelte';

	let {
		it,
		position = 0,
		onCook
	}: { it: SuggestionView; position?: number; onCook: () => void } = $props();

	/** The have-chip whose location is shown (lot id). */
	let shown = $state<string | null>(null);

	const ingredients = $derived([...it.have.map((h) => h.ingredient), ...it.need]);
	const categories = $derived([...it.have.map((h) => h.lot.category), ...it.needCategories]);

	function tone(l: LotView) {
		return l.status === 'use_soon' ? 'soon' : l.status === 'expired' ? 'expired' : 'plain';
	}
	function where(l: LotView): string {
		const s = statusLabel(l).text;
		return `${itemName(l.item)}: ${l.left} in the ${l.location}${s ? `, ${s.toLowerCase()}` : ''}`;
	}
	function expiredWarning(l: LotView): string {
		const s = statusLabel(l).text;
		return `${itemName(l.item)} ${s ? s.replace(/^Expired /, 'expired ') : 'is past its date'} — check it before using.`;
	}
</script>

<li class="card" style="--i:{position}">
	<div class="art">
		<DishArt title={it.name} {categories} {ingredients} size={184} decorative />
		{#if it.usesSoon > 0}
			<span class="soon-badge"
				><svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"
					><circle
						cx="8"
						cy="8"
						r="6.2"
						fill="none"
						stroke="currentColor"
						stroke-width="1.6"
					/><path
						d="M8 4.6V8l2.2 1.4"
						fill="none"
						stroke="currentColor"
						stroke-width="1.6"
						stroke-linecap="round"
					/></svg
				>
				Uses {it.usesSoon} soon</span
			>
		{/if}
	</div>
	<div class="body">
		<div class="flex items-start justify-between gap-3">
			<h2 class="text-[18px] leading-snug font-semibold tracking-tight">{it.name}</h2>
			{#if it.minutes}
				<span class="time"
					><svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"
						><circle
							cx="8"
							cy="8"
							r="6.2"
							fill="none"
							stroke="currentColor"
							stroke-width="1.5"
						/><path
							d="M8 4.6V8l2.4 1.4"
							fill="none"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
						/></svg
					>{it.minutes} min</span
				>
			{/if}
		</div>
		{#if it.why}<p class="mt-1 text-[15px] leading-relaxed text-[var(--ink-2)]">{it.why}</p>{/if}

		{#if it.have.length}
			<p class="label">You have</p>
			<ul class="flex flex-wrap gap-1.5">
				{#each it.have as h (h.lot.id)}
					<li>
						<button
							class="tag {tone(h.lot)}"
							aria-pressed={shown === h.lot.id}
							aria-label="{h.ingredient}{h.lot.status === 'use_soon'
								? ' (use soon)'
								: ''}: show where it is"
							onclick={() => (shown = shown === h.lot.id ? null : h.lot.id)}
						>
							<FoodIcon category={h.lot.category} size={18} />{h.ingredient}
						</button>
					</li>
				{/each}
			</ul>
			{#each it.have as h (h.lot.id)}
				{#if shown === h.lot.id}
					<p role="status" class="mt-2 text-sm text-[var(--ink-2)]">{where(h.lot)}</p>
				{/if}
			{/each}
		{/if}

		{#if it.need.length}
			<p class="label">You would need</p>
			<ul class="flex flex-wrap gap-1.5">
				{#each it.need as n, i (i)}
					<li class="tag muted">
						<FoodIcon category={it.needCategories[i] ?? 'other'} size={18} />{n}
					</li>
				{/each}
			</ul>
		{/if}

		{#each it.expired as l (l.id)}
			<p class="mt-3 text-sm text-[var(--terracotta)]">{expiredWarning(l)}</p>
		{/each}

		<div class="mt-auto pt-4">
			<button class="btn-primary" onclick={onCook}>Cook this</button>
		</div>
	</div>
</li>

<style>
	.card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: 1.25rem;
		border: 1px solid var(--line);
		background: var(--surface);
		animation: enter 420ms cubic-bezier(0.2, 0.7, 0.2, 1) both;
		animation-delay: calc(var(--i) * 70ms);
	}
	@keyframes enter {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
	}
	.art {
		position: relative;
		display: flex;
		justify-content: center;
		padding: 0.9rem 0 0.4rem;
		background: linear-gradient(
			to bottom,
			color-mix(in srgb, var(--lemon) 10%, var(--surface-2)),
			var(--surface)
		);
	}
	.soon-badge {
		position: absolute;
		top: 0.7rem;
		left: 0.7rem;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		border-radius: 9999px;
		background: var(--saffron-bg);
		padding: 0.2rem 0.65rem 0.2rem 0.5rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--saffron);
	}
	.body {
		display: flex;
		flex: 1;
		flex-direction: column;
		padding: 0.5rem 1.15rem 1.15rem;
	}
	.time {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: 0.3rem;
		border-radius: 9999px;
		background: var(--surface-2);
		padding: 0.15rem 0.6rem;
		font-size: 0.8125rem;
		color: var(--ink-2);
		font-variant-numeric: tabular-nums;
	}
	.label {
		margin: 0.9rem 0 0.4rem;
		font-size: 0.75rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--ink-3);
	}
	.tag {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.12rem 0.7rem 0.12rem 0.2rem;
		font-size: 0.875rem;
		color: var(--ink);
	}
	button.tag {
		cursor: pointer;
	}
	.tag.soon {
		border-color: var(--saffron);
		background: var(--saffron-bg);
		color: var(--saffron);
		font-weight: 500;
	}
	.tag.expired {
		border-color: var(--terracotta);
		color: var(--terracotta);
	}
	.tag.muted {
		border-style: dashed;
		background: transparent;
		color: var(--ink-2);
	}
	button.tag[aria-pressed='true'] {
		outline: 2px solid var(--focus);
		outline-offset: 1px;
	}
	.btn-primary {
		display: inline-block;
		border-radius: 0.75rem;
		background: var(--sage);
		padding: 0.5rem 1.1rem;
		font-weight: 500;
		color: var(--surface);
		transition: opacity 120ms;
	}
	.btn-primary:hover {
		opacity: 0.9;
	}
	@media (prefers-reduced-motion: reduce) {
		.card {
			animation: none;
		}
	}
</style>
