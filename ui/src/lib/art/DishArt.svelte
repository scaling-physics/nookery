<script lang="ts">
	// A small hand-drawn-feeling picture of a dish. The dish kind (matched
	// from the title and ingredients, or picked by the recipe model) gives
	// vessel, base and tint; the ingredients give the pieces; the layout is
	// seeded by the title, so a recipe always looks the same. The drawing is
	// parts/DishFigure.svelte, the layout compose.ts.
	import type { FoodCategoryView } from '../bindings.js';
	import { byId, matchDish } from './dishes.js';
	import { piecesFor } from './ingredients.js';
	import { art, pickFor } from './pick.svelte.js';
	import DishFigure from './parts/DishFigure.svelte';

	let {
		title,
		categories,
		ingredients = [],
		size,
		hero = false,
		decorative = false
	}: {
		title: string;
		categories: FoodCategoryView[];
		/** Ingredient names: pick the dish kind and the pieces drawn. */
		ingredients?: string[];
		size?: number;
		hero?: boolean;
		/** Next to the title already: hide from assistive tech. */
		decorative?: boolean;
	} = $props();

	const picked = $derived(art.mode === 'model' ? pickFor(title, ingredients) : undefined);
	const kind = $derived((picked && byId(picked.kind)) ?? matchDish(title, ingredients));
	const pieces = $derived(piecesFor(picked?.show ?? ingredients, categories));
</script>

<DishFigure {kind} {pieces} {title} {size} {hero} {decorative} />
