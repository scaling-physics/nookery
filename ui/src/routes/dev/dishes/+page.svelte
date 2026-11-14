<script lang="ts">
	// Dev gallery of every dish kind (browser only, no backend). Open
	// /dev/dishes under `pnpm -C ui dev`; ?mode=vessels|bases shows the raw
	// vocabulary, ?size=72 the small size. The theme follows the system.
	import DishFigure from '#lib/art/parts/DishFigure.svelte';
	import { DISHES } from '#lib/art/dishes.js';
	import { ALL_VESSELS, ALL_BASES } from './combos.js';
	import type { DishKind } from '#lib/art/types.js';

	const params =
		typeof location === 'undefined' ? new URLSearchParams() : new URLSearchParams(location.search);
	const mode = params.get('mode') ?? 'dishes';
	const size = Number(params.get('size') ?? 120);

	// Without a catalogue (or on ?mode=vessels / bases) show the raw vocabulary.
	const vessels: DishKind[] = ALL_VESSELS.map((v) => ({
		id: `v-${v}`,
		label: v,
		words: [],
		vessel: v,
		base: v === 'skewer' || v === 'taco' ? 'none' : 'rice',
		tint: 'wheat',
		signature: [],
		hot: false
	}));
	const bases: DishKind[] = ALL_BASES.map((b) => ({
		id: `b-${b}`,
		label: b,
		words: [],
		vessel: 'plate',
		base: b,
		tint: 'wheat',
		signature: [],
		hot: false
	}));
	const list = $derived(
		mode === 'vessels' ? vessels : mode === 'bases' ? bases : DISHES.length ? DISHES : vessels
	);
	const heroes = $derived(list.slice(0, 4));
</script>

<svelte:head><title>Dish gallery</title></svelte:head>

{#snippet grid(items: DishKind[], px: number)}
	<ul class="grid">
		{#each items as d (d.id)}
			<li>
				<DishFigure kind={d} title={d.label} size={px} decorative />
				<span class="l">{d.label}</span>
				<span class="id">{d.id}</span>
			</li>
		{/each}
	</ul>
{/snippet}

<div class="col">
	<h2>{list.length} kinds, {size} px</h2>
	{@render grid(list, size)}
	<div class="heroes">
		{#each heroes as d (d.id)}<DishFigure
				kind={d}
				title={d.label}
				size={300}
				hero
				decorative
			/>{/each}
	</div>
</div>

<style>
	.col {
		padding: 12px;
		background: var(--bg);
		color: var(--ink);
	}
	h2 {
		font-size: 14px;
		margin-bottom: 8px;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
		gap: 6px;
		list-style: none;
		padding: 0;
	}
	li {
		display: flex;
		flex-direction: column;
		align-items: center;
		font-size: 11px;
		text-align: center;
	}
	.id {
		color: var(--ink-2);
		font-size: 10px;
	}
	.heroes {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 16px;
	}
</style>
