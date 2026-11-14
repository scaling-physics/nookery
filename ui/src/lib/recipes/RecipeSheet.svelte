<script lang="ts">
	// The recipe being written (streamed) or finished: a big illustration,
	// title, serves/time, ingredients with icons and tick boxes, numbered steps.
	import type { Snippet } from 'svelte';
	import type { Cooking } from '../recipes-state.svelte.js';
	import CloudTag from '../CloudTag.svelte';
	import ErrorBanner from '../ErrorBanner.svelte';
	import FoodIcon from '../FoodIcon.svelte';
	import CookingLoader from '../art/CookingLoader.svelte';
	import DishArt from '../art/DishArt.svelte';

	let {
		c,
		ranOn,
		actions
	}: {
		c: Cooking;
		ranOn: string;
		/** Extra buttons beside the title (the favourite heart goes here). */
		actions?: Snippet;
	} = $props();

	let got = $state<Record<number, boolean>>({});
	// A revised recipe starts with a clean tick list.
	$effect(() => {
		void c.recipe;
		got = {};
	});
	const flash = $derived(c.chat.flash);
	const title = $derived(c.recipe?.title || c.item.name);
	const categories = $derived(
		c.recipe?.ingredientCategories?.length
			? c.recipe.ingredientCategories
			: [...c.item.have.map((h) => h.lot.category), ...c.item.needCategories]
	);
	// The written recipe's ingredient lines, else the idea's have + need names.
	const ingredients = $derived(
		c.recipe?.ingredients?.length
			? c.recipe.ingredients
			: [...c.item.have.map((h) => h.ingredient), ...c.item.need]
	);
	const meta = $derived(
		c.recipe
			? [
					c.recipe.serves ? `Serves ${c.recipe.serves}` : '',
					c.recipe.minutes ? `${c.recipe.minutes} min` : ''
				].filter(Boolean)
			: c.item.minutes
				? [`About ${c.item.minutes} min`]
				: []
	);
	/** The tail of the streamed text, so the box stays readable. */
	const tail = $derived(c.text.length > 420 ? '…' + c.text.slice(-420) : c.text);
</script>

<article aria-labelledby="recipe-h" class="sheet">
	<div class="hero">
		<DishArt {title} {categories} {ingredients} hero decorative />
	</div>
	<div class="px-5 pt-1 pb-6 sm:px-8">
		<div class="flex items-start justify-between gap-3">
			<h1
				id="recipe-h"
				class="text-[26px] leading-tight font-semibold tracking-tight sm:text-[30px]"
			>
				{title}
			</h1>
			{#if actions}<div class="shrink-0 pt-1">{@render actions()}</div>{/if}
		</div>
		{#if meta.length}
			<ul class="mt-2 flex flex-wrap gap-2" aria-label="Recipe details">
				{#each meta as m (m)}<li class="pill">{m}</li>{/each}
			</ul>
		{/if}

		{#if c.error}
			<ErrorBanner message={c.error} class="mt-5" />
		{:else if !c.recipe}
			<div role="status" aria-live="polite" class="mt-6">
				<div class="flex items-center gap-4">
					<CookingLoader size={92} />
					<p class="text-[15px] text-[var(--ink-2)]">
						{c.text ? `Writing the recipe ${ranOn}…` : 'Getting the pans out…'}
					</p>
				</div>
				{#if c.text}
					<pre class="stream mt-3">{tail}<span class="caret" aria-hidden="true"></span></pre>
				{/if}
			</div>
		{:else if c.recipe.ingredients.length || c.recipe.steps.length}
			{#key c.recipe}
				<div class="reveal">
					{#if c.recipe.ingredients.length}
						<h2 class="h">Ingredients</h2>
						<ul class="grid gap-x-6 gap-y-0.5 sm:grid-cols-2">
							{#each c.recipe.ingredients as ing, i (i)}
								<li>
									<label class="ing" class:got={got[i]} class:flash={flash.ingredients.includes(i)}>
										<input
											type="checkbox"
											bind:checked={got[i]}
											class="h-4 w-4 accent-[var(--sage)]"
										/>
										<FoodIcon category={c.recipe.ingredientCategories[i] ?? 'other'} size={24} />
										<span class="flex-1">{ing}</span>
									</label>
								</li>
							{/each}
						</ul>
					{/if}
					{#if c.recipe.steps.length}
						<h2 class="h mt-9">Steps</h2>
						<ol class="grid gap-6">
							{#each c.recipe.steps as step, i (i)}
								<li class="step-row flex gap-4" class:flash={flash.steps.includes(i)}>
									<span class="num" aria-hidden="true">{i + 1}</span>
									<p class="step"><span class="sr-only">Step {i + 1}: </span>{step}</p>
								</li>
							{/each}
						</ol>
					{/if}
				</div>
			{/key}
		{:else}
			<pre class="stream raw mt-4">{c.recipe.raw}</pre>
		{/if}

		{#if c.recipe && c.recipe.model}
			<p class="mt-8 text-xs text-[var(--ink-3)]">
				{Math.round((c.recipe.ms ?? 0) / 1000)} s · {c.recipe.model}<CloudTag
					show={c.recipe.usedCloud}
				/>
			</p>
		{/if}
	</div>
</article>

<style>
	.sheet {
		overflow: hidden;
		border-radius: 1.5rem;
		border: 1px solid var(--line);
		background: var(--surface);
	}
	.hero {
		display: flex;
		justify-content: center;
		padding: 1.25rem 1rem 0.25rem;
		background: linear-gradient(
			to bottom,
			color-mix(in srgb, var(--lemon) 12%, var(--surface-2)),
			var(--surface)
		);
	}
	.pill {
		border-radius: 9999px;
		background: var(--surface-2);
		padding: 0.2rem 0.75rem;
		font-size: 0.875rem;
		color: var(--ink-2);
	}
	.h {
		margin: 1.75rem 0 0.75rem;
		font-size: 0.8125rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-2);
	}
	.ing {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		border-radius: 0.75rem;
		padding: 0.4rem 0.4rem;
		font-size: 1rem;
		cursor: pointer;
	}
	.ing:hover {
		background: var(--surface-2);
	}
	.ing.got span {
		text-decoration: line-through;
		color: var(--ink-3);
	}
	.flash {
		animation: flash 2.8s ease-out both;
	}
	@keyframes flash {
		0%,
		35% {
			background: var(--saffron-bg);
			box-shadow: 0 0 0 6px var(--saffron-bg);
		}
		100% {
			background: transparent;
			box-shadow: 0 0 0 6px transparent;
		}
	}
	.step-row {
		border-radius: 0.5rem;
	}
	.num {
		display: flex;
		width: 2.2rem;
		height: 2.2rem;
		flex: none;
		align-items: center;
		justify-content: center;
		border-radius: 9999px 9999px 9999px 0.6rem;
		background: var(--sage-bg);
		font-size: 1.05rem;
		font-weight: 600;
		color: var(--sage);
		font-variant-numeric: tabular-nums;
	}
	.step {
		padding-top: 0.15rem;
		font-size: 1.125rem;
		line-height: 1.65;
	}
	.stream {
		white-space: pre-wrap;
		border-radius: 1rem;
		background: var(--surface-2);
		padding: 0.9rem 1.1rem;
		font: inherit;
		font-size: 0.9375rem;
		line-height: 1.65;
		color: var(--ink-2);
		mask-image: linear-gradient(to bottom, transparent 0, #000 1.4rem);
	}
	.raw {
		max-height: 24rem;
		overflow: auto;
		mask-image: none;
	}
	.caret {
		display: inline-block;
		width: 0.5ch;
		height: 1em;
		margin-left: 2px;
		vertical-align: text-bottom;
		background: var(--sage);
		animation: blink 1s steps(2) infinite;
	}
	.reveal {
		animation: fade 450ms ease-out both;
	}
	@keyframes blink {
		50% {
			opacity: 0;
		}
	}
	@keyframes fade {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.reveal,
		.flash {
			animation: none;
		}
	}
</style>
