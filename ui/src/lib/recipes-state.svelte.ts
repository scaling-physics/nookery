// Recipes tab state that outlives the page, so switching to the Kitchen tab
// and back doesn't throw away ideas that took a while to write.

import type { RecipeView, SuggestionsView, SuggestionView } from './bindings.js';

export type Cooking = {
	item: SuggestionView;
	/** Text streamed so far. */
	text: string;
	/** Set when the model has finished. */
	recipe: RecipeView | null;
	/** An error message, if writing failed. */
	error: string;
};

export const rs = $state<{
	templateId: number | null;
	note: string;
	count: number;
	suggestions: SuggestionsView | null;
	cooking: Cooking | null;
}>({ templateId: null, note: '', count: 4, suggestions: null, cooking: null });
