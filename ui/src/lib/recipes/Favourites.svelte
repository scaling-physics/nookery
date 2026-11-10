<script lang="ts">
	// The saved recipes: a simple list. `onOpen` shows one (the page decides
	// how); removing asks nothing, it is one tap to save it again.
	import { onMount } from 'svelte';
	import type { FavouriteView } from '../bindings.js';
	import DishArt from '../art/DishArt.svelte';
	import { favs, loadFavourites, removeFavourite } from '../plan/favourites.svelte.js';

	let { onOpen }: { onOpen?: (f: FavouriteView) => void } = $props();

	let failed = $state(false);

	onMount(() => void loadFavourites());

	async function remove(f: FavouriteView) {
		failed = !(await removeFavourite(f.id));
	}

	function meta(f: FavouriteView): string {
		return [
			f.recipe.minutes ? `${f.recipe.minutes} min` : '',
			f.recipe.serves ? `serves ${f.recipe.serves}` : '',
			f.cooked === 0
				? 'not cooked yet'
				: f.cooked === 1
					? 'cooked once'
					: `cooked ${f.cooked} times`
		]
			.filter(Boolean)
			.join(' · ');
	}
</script>

{#if failed}
	<p
		role="alert"
		class="mb-3 rounded-2xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
	>
		Could not remove that one. Please try again.
	</p>
{/if}
{#if favs.loaded && favs.list.length === 0}
	<p class="rounded-2xl border border-dashed border-[var(--line)] p-4 text-sm text-[var(--ink-3)]">
		No favourites yet. Tap the heart on a recipe you want to keep.
	</p>
{:else}
	<ul class="grid gap-2" aria-label="Favourite recipes">
		{#each favs.list as f (f.id)}
			<li class="row">
				<button type="button" class="open" onclick={() => onOpen?.(f)}>
					<DishArt
						title={f.recipe.title}
						categories={f.recipe.ingredientCategories}
						size={48}
						decorative
					/>
					<span class="min-w-0">
						<span class="block truncate font-medium">{f.recipe.title}</span>
						<span class="block text-sm text-[var(--ink-3)]">{meta(f)}</span>
					</span>
				</button>
				<button
					type="button"
					class="remove"
					aria-label="Remove {f.recipe.title} from favourites"
					onclick={() => remove(f)}
				>
					Remove
				</button>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: 1rem;
		border: 1px solid var(--line);
		background: var(--surface);
		padding-right: 0.5rem;
	}
	.open {
		display: flex;
		min-width: 0;
		flex: 1;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.75rem;
		text-align: left;
		min-height: 3.5rem;
	}
	.remove {
		border-radius: 9999px;
		padding: 0.5rem 0.8rem;
		font-size: 0.8125rem;
		color: var(--ink-3);
	}
	.remove:hover {
		background: var(--surface-2);
		color: var(--terracotta);
	}
</style>
