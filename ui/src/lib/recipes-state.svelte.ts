// Recipes tab state that outlives the page, so switching to the Kitchen tab
// and back doesn't throw away ideas that took a while to write.

import type { RecipeView, SuggestionsView, SuggestionView } from './bindings.js';

/** One message in the chat about a recipe. */
export type ChatMsg = {
	role: 'user' | 'cook';
	text: string;
	/** The cook changed the recipe with this reply. */
	revised?: boolean;
	/** Who answered, for cook replies. */
	model?: string;
	usedCloud?: boolean;
};

/** The conversation about the recipe being cooked; lives and dies with it. */
export type Chat = {
	msgs: ChatMsg[];
	/** Raw text streamed so far for the reply in progress. */
	live: string;
	busy: boolean;
	error: string;
	/** Earlier versions of the recipe, newest last, for Undo. */
	undo: RecipeView[];
	/** Lines changed by the last revision (indices), highlighted briefly. */
	flash: { ingredients: number[]; steps: number[]; n: number };
	draft: string;
};

export function newChat(): Chat {
	return {
		msgs: [],
		live: '',
		busy: false,
		error: '',
		undo: [],
		flash: { ingredients: [], steps: [], n: 0 },
		draft: ''
	};
}

export type Cooking = {
	item: SuggestionView;
	/** Text streamed so far. */
	text: string;
	/** Set when the model has finished. */
	recipe: RecipeView | null;
	/** An error message, if writing failed. */
	error: string;
	/** Questions and changes for this recipe. */
	chat: Chat;
	/** The saved favourite this recipe is (or was saved as), so the heart
	 *  can offer to save chat changes into it even after the title changed. */
	favouriteId?: string | null;
};

export const rs = $state<{
	templateId: number | null;
	note: string;
	count: number;
	suggestions: SuggestionsView | null;
	cooking: Cooking | null;
}>({ templateId: null, note: '', count: 4, suggestions: null, cooking: null });
