// Saved recipes, shared by every heart button and the Favourites list so one
// read serves them all and they stay in step.

import { api } from '../api.js';
import type { FavouriteView, RecipeView } from '../bindings.js';

export const favs = $state<{ list: FavouriteView[]; loaded: boolean }>({
	list: [],
	loaded: false
});

let loading: Promise<void> | null = null;

export function loadFavourites(force = false): Promise<void> {
	if (loading && !force) return loading;
	loading = api.listFavourites().then((r) => {
		if (r.status === 'ok') {
			favs.list = r.data;
			favs.loaded = true;
		}
	});
	return loading;
}

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/** The saved recipe with this title, if any (case-insensitive). */
export function savedByTitle(title: string): FavouriteView | undefined {
	return favs.list.find((f) => same(f.recipe.title, title));
}

/** Saves the recipe, or forgets it when it is already saved. Returns whether
 *  it is saved afterwards, or null if the write failed. */
export async function toggleFavourite(
	recipe: Pick<RecipeView, 'title' | 'serves' | 'minutes' | 'ingredients' | 'steps'>,
	why?: string,
	linked?: FavouriteView
): Promise<boolean | null> {
	const have = linked ?? savedByTitle(recipe.title);
	if (have) {
		const r = await api.deleteFavourite(have.id);
		if (r.status !== 'ok' && r.error.kind !== 'not_found') return null;
		favs.list = favs.list.filter((f) => f.id !== have.id);
		return false;
	}
	const r = await api.saveFavourite({
		title: recipe.title,
		serves: recipe.serves,
		minutes: recipe.minutes,
		ingredients: recipe.ingredients,
		steps: recipe.steps,
		why: why ?? null
	});
	if (r.status !== 'ok') return null;
	favs.list = [r.data, ...favs.list.filter((f) => f.id !== r.data.id)];
	return true;
}

type RecipeContent = Pick<RecipeView, 'title' | 'serves' | 'minutes' | 'ingredients' | 'steps'>;

/** Whether a saved recipe still matches what is on screen (the recipe chat
 *  may have changed it since). */
export function sameContent(a: RecipeContent, b: RecipeContent): boolean {
	return (
		a.title.trim() === b.title.trim() &&
		a.serves === b.serves &&
		a.minutes === b.minutes &&
		a.ingredients.join('\n') === b.ingredients.join('\n') &&
		a.steps.join('\n') === b.steps.join('\n')
	);
}

/** Writes the recipe on screen into the saved favourite `id` (same id,
 *  reason and cooked count). Returns false if the write failed. */
export async function updateFavourite(id: string, recipe: RecipeContent): Promise<boolean> {
	const r = await api.updateFavourite(id, {
		title: recipe.title,
		serves: recipe.serves,
		minutes: recipe.minutes,
		ingredients: recipe.ingredients,
		steps: recipe.steps,
		why: null
	});
	if (r.status !== 'ok') return false;
	favs.list = favs.list.map((f) => (f.id === id ? r.data : f));
	return true;
}

export async function removeFavourite(id: string): Promise<boolean> {
	const r = await api.deleteFavourite(id);
	if (r.status !== 'ok' && r.error.kind !== 'not_found') return false;
	favs.list = favs.list.filter((f) => f.id !== id);
	return true;
}
