<script lang="ts">
	// PROTOTYPE (dev only): today's procedural dishes next to watercolour
	// paintings made locally with Z-Image Turbo (mflux). Open /dev/watercolour
	// under `pnpm -C ui dev`. Images in static/dev/painted/. Not used by the app.
	import DishArt from '#lib/art/DishArt.svelte';
	import type { FoodCategoryView } from '#lib/bindings.js';

	const EXAMPLES: {
		id: string;
		title: string;
		ingredients: string[];
		categories: FoodCategoryView[];
	}[] = [
		{
			id: 'carbonara',
			title: 'Spaghetti carbonara',
			ingredients: ['spaghetti', 'egg', 'bacon', 'parmesan'],
			categories: ['grains', 'eggs', 'meat', 'cheese']
		},
		{
			id: 'ramen',
			title: 'Shoyu ramen',
			ingredients: ['ramen noodles', 'egg', 'spring onion', 'pork'],
			categories: ['grains', 'eggs', 'vegetables', 'meat']
		},
		{
			id: 'shakshuka',
			title: 'Shakshuka',
			ingredients: ['egg', 'tomato', 'bell pepper', 'onion', 'parsley'],
			categories: ['eggs', 'vegetables', 'herbs']
		},
		{
			id: 'chicken-curry',
			title: 'Chicken curry',
			ingredients: ['chicken', 'rice', 'onion', 'coriander'],
			categories: ['meat', 'grains', 'vegetables', 'herbs']
		},
		{
			id: 'tomato-soup',
			title: 'Tomato soup',
			ingredients: ['tomato', 'cream', 'basil', 'bread'],
			categories: ['vegetables', 'dairy', 'herbs', 'bread']
		},
		{
			id: 'pancakes',
			title: 'Pancakes',
			ingredients: ['flour', 'egg', 'milk', 'blueberries', 'banana'],
			categories: ['grains', 'eggs', 'dairy', 'fruit']
		}
	];
</script>

<main>
	<h1>Recipe pictures: today vs. painted (prototype)</h1>
	<div class="grid">
		{#each EXAMPLES as e (e.id)}
			<figure>
				<div class="pair">
					<div class="cell">
						<DishArt
							title={e.title}
							ingredients={e.ingredients}
							categories={e.categories}
							size={200}
							hero
						/>
						<span>today</span>
					</div>
					<div class="cell painted">
						<picture>
							<source srcset="/dev/painted/{e.id}-dark.webp" media="(prefers-color-scheme: dark)" />
							<img src="/dev/painted/{e.id}.webp" alt="" width="220" height="220" />
						</picture>
						<span>painted</span>
					</div>
				</div>
				<figcaption>{e.title}</figcaption>
			</figure>
		{/each}
	</div>
</main>

<style>
	main {
		padding: 24px;
		max-width: 1300px;
		margin: 0 auto;
	}
	h1 {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: 16px;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(520px, 1fr));
		gap: 24px;
	}
	figure {
		margin: 0;
	}
	.pair {
		display: flex;
		gap: 12px;
	}
	.cell {
		display: grid;
		place-items: center;
		padding: 12px;
		border-radius: 16px;
		background: var(--surface);
		border: 1px solid var(--line);
		flex: 1;
	}
	.cell span {
		font-size: 11px;
		color: var(--ink-3);
	}
	/* Light: painted on white paper, levelled so the paper is exactly white;
	   multiply drops it into the card and keeps the painted shadow.
	   Dark: a second painting on a dark lamp-lit ground, its edges faded into
	   the card. No cut-outs (they turned the painted shadow into a second plate). */
	.painted img {
		display: block;
		mix-blend-mode: multiply;
	}
	@media (prefers-color-scheme: dark) {
		.painted img {
			mix-blend-mode: normal;
			mask-image: radial-gradient(closest-side, #000 72%, transparent 100%);
		}
	}
	figcaption {
		margin-top: 6px;
		font-size: 14px;
		font-weight: 500;
	}
</style>
