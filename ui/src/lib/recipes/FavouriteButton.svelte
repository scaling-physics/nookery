<script lang="ts">
	// A heart that saves the recipe, or forgets it. The recipe it belongs to is
	// remembered by id (`favouriteId`), falling back to the title, so after the
	// recipe chat changes a saved recipe the heart offers "Save changes".
	import { onMount } from 'svelte';
	import type { RecipeView } from '../bindings.js';
	import {
		favs,
		loadFavourites,
		sameContent,
		savedByTitle,
		toggleFavourite,
		updateFavourite
	} from '../plan/favourites.svelte.js';

	let {
		recipe,
		why,
		favouriteId = $bindable()
	}: { recipe: RecipeView; why?: string; favouriteId?: string | null } = $props();

	const linked = $derived(
		favs.loaded
			? ((favouriteId ? favs.list.find((f) => f.id === favouriteId) : undefined) ??
					savedByTitle(recipe.title))
			: undefined
	);
	const saved = $derived(!!linked);
	const changed = $derived(!!linked && !sameContent(linked.recipe, recipe));
	let busy = $state(false);
	let failed = $state(false);
	let justUpdated = $state(false);

	onMount(() => void loadFavourites());

	// Remember which favourite this is before the chat can rename it.
	$effect(() => {
		if (linked && favouriteId !== linked.id) favouriteId = linked.id;
	});

	async function toggle() {
		if (busy) return;
		busy = true;
		failed = false;
		const r = await toggleFavourite(recipe, why, linked);
		failed = r === null;
		if (r === true) favouriteId = savedByTitle(recipe.title)?.id ?? null;
		if (r === false) favouriteId = null;
		busy = false;
	}

	async function saveChanges() {
		if (busy || !linked) return;
		busy = true;
		failed = false;
		const ok = await updateFavourite(linked.id, recipe);
		failed = !ok;
		busy = false;
		if (ok) {
			justUpdated = true;
			setTimeout(() => (justUpdated = false), 2500);
		}
	}
</script>

<span class="wrap">
	{#if changed}
		<button type="button" class="save" disabled={busy} onclick={saveChanges}> Save changes </button>
	{:else if justUpdated}
		<span class="done" role="status">Saved</span>
	{/if}
	<button
		type="button"
		class="heart"
		class:on={saved}
		aria-pressed={saved}
		aria-label={saved ? 'Saved to favourites. Remove it' : 'Save to favourites'}
		title={saved
			? changed
				? 'Saved before your changes. Tap “Save changes” to keep them.'
				: 'Saved. Tap to remove.'
			: 'Save to favourites'}
		disabled={busy || !favs.loaded}
		onclick={toggle}
	>
		<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
			<path
				d="M12 20.4C5.5 15.6 3 12.6 3 9.2 3 6.6 5 4.8 7.3 4.8c1.7 0 3.4.9 4.7 2.9 1.3-2 3-2.9 4.7-2.9C19 4.8 21 6.6 21 9.2c0 3.4-2.5 6.4-9 11.2Z"
				fill={saved ? 'currentColor' : 'none'}
				stroke="currentColor"
				stroke-width="1.7"
				stroke-linejoin="round"
			/>
		</svg>
	</button>
</span>
{#if failed}
	<span role="alert" class="text-xs text-[var(--terracotta)]">Could not save.</span>
{/if}

<style>
	.wrap {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}
	.save {
		min-height: 2.25rem;
		padding: 0 0.85rem;
		border-radius: 9999px;
		font-size: 13px;
		font-weight: 600;
		color: var(--saffron);
		background: var(--saffron-bg);
		border: 1px solid color-mix(in srgb, var(--saffron) 35%, transparent);
		white-space: nowrap;
	}
	.save:disabled {
		opacity: 0.6;
	}
	.done {
		font-size: 13px;
		color: var(--sage);
		padding: 0 0.4rem;
	}
	.heart {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.75rem;
		height: 2.75rem;
		border-radius: 9999px;
		color: var(--ink-3);
		transition:
			color 120ms,
			background-color 120ms,
			transform 120ms;
	}
	.heart:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--terracotta);
	}
	.heart.on {
		color: var(--terracotta);
	}
	.heart:active:not(:disabled) {
		transform: scale(0.92);
	}
	.heart:disabled {
		opacity: 0.5;
	}
	@media (prefers-reduced-motion: reduce) {
		.heart {
			transition: none;
		}
		.heart:active:not(:disabled) {
			transform: none;
		}
	}
</style>
