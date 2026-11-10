<script lang="ts">
	import { MAC_UNREACHABLE_DEFAULT, PHONE_MODEL_MISSING } from '#lib/phone/messages.js';
	import { onPhone } from '#lib/phone/env.js';
	import { loadRanOn } from '#lib/phone/where.js';
	import { onDestroy, onMount } from 'svelte';
	import { api, textChannel } from '#lib/api.js';
	import type {
		AiModeView,
		CommandError,
		FavouriteView,
		RecipeView,
		SuggestionView,
		TemplateView
	} from '#lib/bindings.js';
	import { CLOUD_CREDITS, CLOUD_SIGN_IN } from '#lib/format.js';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import CloudTag from '#lib/CloudTag.svelte';
	import CookingLoader from '#lib/art/CookingLoader.svelte';
	import KitchenScene from '#lib/art/KitchenScene.svelte';
	import IdeaCard from '#lib/recipes/IdeaCard.svelte';
	import RecipeSheet from '#lib/recipes/RecipeSheet.svelte';
	import RecipeChat from '#lib/recipes/RecipeChat.svelte';
	import FavouriteButton from '#lib/recipes/FavouriteButton.svelte';
	import CookedSheet from '#lib/recipes/CookedSheet.svelte';
	import Favourites from '#lib/recipes/Favourites.svelte';
	import { newChat, rs } from '#lib/recipes-state.svelte.js';
	import TemplatesDialog from '#lib/TemplatesDialog.svelte';

	let templates = $state<TemplateView[]>([]);
	let loaded = $state(false);
	let showTemplates = $state(false);
	let thinking = $state(false);
	let elapsed = $state(0);
	let timer: ReturnType<typeof setInterval> | undefined;
	let error = $state('');
	let recipesMode = $state<AiModeView>('local');
	let phoneRanOn = $state('on this phone');
	const ranOn = $derived(
		onPhone ? phoneRanOn : recipesMode === 'cloud' ? 'in the cloud' : 'on this Mac'
	);
	let templatesButton: HTMLButtonElement | undefined = $state();

	// "I cooked it" (the sheet asks how much of each was used)
	let cookedOpen = $state(false);
	let cookedDone = $state(false);

	const sug = $derived(rs.suggestions);
	const cooking = $derived(rs.cooking);

	const LINES = [
		'Looking through your kitchen…',
		'Checking what needs eating first…',
		'Matching things that go well together…',
		'Writing up the best ideas…'
	];
	const line = $derived(LINES[Math.min(LINES.length - 1, Math.floor(elapsed / 4))]);

	onDestroy(() => clearInterval(timer));

	function errorText(e: CommandError): string {
		switch (e.kind) {
			case 'cloud_sign_in':
				return CLOUD_SIGN_IN;
			case 'cloud_credits':
				return CLOUD_CREDITS;
			case 'phone_model_missing':
				return PHONE_MODEL_MISSING;
			case 'mac_unreachable':
				return e.detail || MAC_UNREACHABLE_DEFAULT;
			case 'recipe_model_unavailable':
				if (onPhone) return MAC_UNREACHABLE_DEFAULT;
				return `Nookery could not reach the local model at ${e.detail}. Start Ollama (open the Ollama app, or run “ollama serve”) and try again.`;
			case 'recipe_model_missing':
				return `The recipe model is not installed yet. Run “ollama pull ${e.detail}” in a terminal, then try again.`;
			case 'bad_input':
				return e.detail === 'stock'
					? 'There is nothing in stock to cook with yet. Add a few things in the Kitchen tab first.'
					: 'Could not read that request.';
			case 'not_found':
				return 'Those ideas are no longer available. Ask for new ones.';
			default:
				return 'Something went wrong. Please try again.';
		}
	}

	async function loadTemplates() {
		const r = await api.listTemplates();
		if (r.status === 'ok') {
			templates = r.data;
			if (!templates.some((t) => t.id === rs.templateId)) rs.templateId = templates[0]?.id ?? null;
		} else error = 'Could not read your templates.';
		loaded = true;
	}

	async function suggest(e?: SubmitEvent) {
		e?.preventDefault();
		if (thinking) return;
		error = '';
		elapsed = 0;
		thinking = true;
		const started = Date.now();
		timer = setInterval(() => (elapsed = Math.round((Date.now() - started) / 1000)), 500);
		const r = await api.suggestRecipes({
			templateId: rs.templateId,
			note: rs.note,
			count: rs.count
		});
		clearInterval(timer);
		thinking = false;
		if (r.status === 'ok') {
			rs.suggestions = r.data;
			rs.cooking = null;
		} else error = errorText(r.error);
	}

	async function cook(item: SuggestionView) {
		if (!sug) return;
		error = '';
		cookedDone = false;
		window.scrollTo({ top: 0 });
		const c = { item, text: '', recipe: null, error: '', chat: newChat(), favouriteId: null };
		rs.cooking = c;
		const mine = rs.cooking; // the reactive proxy
		const r = await api.writeRecipe(
			{ suggestionsId: sug.id, index: item.index },
			textChannel((t) => {
				if (rs.cooking === mine && mine) mine.text += t;
			})
		);
		if (rs.cooking !== mine || !mine) return; // the user went back
		if (r.status === 'ok') mine.recipe = r.data;
		else mine.error = errorText(r.error);
	}

	function back() {
		rs.cooking = null;
		cookedDone = false;
	}

	/** A saved recipe opens without the model; it has no stock lots attached. */
	function openFavourite(f: FavouriteView) {
		error = '';
		cookedDone = false;
		window.scrollTo({ top: 0 });
		const item: SuggestionView = {
			index: -1,
			name: f.recipe.title,
			why: f.why ?? '',
			minutes: f.recipe.minutes,
			have: [],
			need: [],
			needCategories: [],
			expired: [],
			usesSoon: 0,
			score: 0
		};
		const recipe: RecipeView = { ...f.recipe, raw: '', model: '', usedCloud: false, ms: null };
		rs.cooking = { item, text: '', recipe, error: '', chat: newChat(), favouriteId: f.id };
	}

	onMount(() => {
		loadTemplates();
		if (onPhone) loadRanOn().then((v) => (phoneRanOn = v));
		api.getAiSettings().then((s) => {
			if (s.status === 'ok') recipesMode = s.data.recipes;
		});
	});
</script>

<main class="mx-auto max-w-6xl px-4 pt-6 pb-16 sm:px-8">
	{#if cooking}
		{@const c = cooking}
		<div class="mx-auto max-w-3xl">
			<div class="mb-4 flex items-center justify-between gap-3">
				<button class="chip" onclick={back}>← Back to ideas</button>
			</div>

			<RecipeSheet {c} {ranOn}>
				{#snippet actions()}
					{#if c.recipe}<FavouriteButton
							recipe={c.recipe}
							why={c.item.why}
							bind:favouriteId={c.favouriteId}
						/>{/if}
				{/snippet}
			</RecipeSheet>

			{#if c.recipe}
				<RecipeChat {c} {errorText} {ranOn} />
			{/if}

			{#if c.recipe && c.item.have.length > 0}
				<section class="panel mt-6" aria-labelledby="cooked-h">
					<h2 id="cooked-h" class="text-[17px] font-semibold">I cooked it</h2>
					{#if cookedDone}
						<div role="status" class="mt-2">
							<p class="text-[var(--sage)]">Your kitchen is up to date.</p>
							<div class="mt-3 flex gap-2">
								<a href="/" class="btn-primary">See the kitchen</a>
								<button class="chip" onclick={back}>Back to ideas</button>
							</div>
						</div>
					{:else}
						<p class="mt-1 text-sm text-[var(--ink-2)]">
							Tell Nookery what you used, all of it or just some, and your kitchen list stays right.
						</p>
						<button class="btn-primary mt-3" onclick={() => (cookedOpen = true)}>I cooked it</button
						>
					{/if}
				</section>
				<CookedSheet
					bind:open={cookedOpen}
					suggestion={c.item}
					onDone={() => (cookedDone = true)}
				/>
			{/if}
		</div>
	{:else}
		<header class="mb-5 flex items-center gap-4 sm:gap-6">
			<div class="shrink-0 max-sm:hidden"><KitchenScene size={116} /></div>
			<div class="shrink-0 sm:hidden"><KitchenScene size={72} /></div>
			<div class="min-w-0 flex-1">
				<h1 class="text-[24px] leading-tight font-semibold tracking-tight sm:text-[28px]">
					What shall we cook?
				</h1>
				<p class="mt-1 text-[15px] text-[var(--ink-2)]">
					Ideas from what you have, use-soon things first. Written {ranOn}.
				</p>
			</div>
			<button
				class="chip shrink-0 self-start"
				bind:this={templatesButton}
				onclick={() => (showTemplates = true)}
			>
				Templates
			</button>
		</header>

		<Favourites onOpen={openFavourite} />

		<form onsubmit={suggest} class="panel mb-6 grid gap-4">
			<fieldset disabled={thinking} class="grid gap-2">
				<legend class="mb-1 text-[13px] font-semibold text-[var(--ink-2)]"
					>What kind of meal?</legend
				>
				{#if loaded && templates.length === 0}
					<p class="text-sm text-[var(--ink-3)]">
						No templates yet. Open Templates to add one, or just ask below.
					</p>
				{/if}
				<div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Template">
					{#each templates as t (t.id)}
						<label class="seg" class:on={rs.templateId === t.id} title={t.body}>
							<input
								type="radio"
								name="template"
								value={t.id}
								bind:group={rs.templateId}
								class="sr-only"
							/>
							{t.name}
						</label>
					{/each}
				</div>
			</fieldset>

			<div class="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-end">
				<label class="grid gap-1 text-[13px] font-semibold text-[var(--ink-2)]">
					Anything else?
					<input
						bind:value={rs.note}
						disabled={thinking}
						placeholder="e.g. something Indian, no chilli"
						class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 font-normal text-[var(--ink)] placeholder:text-[var(--ink-3)]"
					/>
				</label>
				<div class="flex items-center gap-2">
					<span id="count-l" class="text-[13px] font-semibold text-[var(--ink-2)]">Ideas</span>
					<div class="stepper" role="group" aria-labelledby="count-l">
						<button
							type="button"
							aria-label="Fewer ideas"
							disabled={thinking || rs.count <= 1}
							onclick={() => (rs.count = Math.max(1, rs.count - 1))}>−</button
						>
						<output aria-live="polite" class="w-6 text-center tabular-nums">{rs.count}</output>
						<button
							type="button"
							aria-label="More ideas"
							disabled={thinking || rs.count >= 8}
							onclick={() => (rs.count = Math.min(8, rs.count + 1))}>+</button
						>
					</div>
				</div>
			</div>

			<div>
				<button class="btn-primary w-full sm:w-auto" disabled={thinking || !loaded}>
					{thinking ? 'Thinking…' : sug ? 'Suggest again' : 'Suggest'}
				</button>
			</div>
		</form>

		{#if error}
			<ErrorBanner message={error} class="mb-4" />
		{/if}

		{#if thinking}
			<div
				role="status"
				aria-live="polite"
				class="panel flex flex-col items-center py-8 text-center"
			>
				<CookingLoader size={170} />
				<p class="mt-3 text-[17px] font-medium">{line}</p>
				<p class="mt-1 text-sm text-[var(--ink-2)]">
					Thinking {ranOn}, usually about {onPhone && ranOn === 'on this phone' ? '30' : '15'} s
					<span class="text-[var(--ink-3)] tabular-nums">· {elapsed} s</span>
				</p>
			</div>
		{:else if sug}
			<p class="mb-3 text-sm text-[var(--ink-2)]">
				{sug.items.length}
				{sug.items.length === 1 ? 'idea' : 'ideas'}, best first ·
				{Math.round((sug.ms ?? 0) / 1000)} s · {sug.model}<CloudTag show={sug.usedCloud} />
			</p>
			{#if sug.items.length === 0}
				<div class="panel flex flex-col items-center py-8 text-center">
					<KitchenScene size={150} steam={false} />
					<p class="mt-3 text-[15px] text-[var(--ink-2)]">
						The model came back with nothing usable. Try again, or change the note.
					</p>
				</div>
			{/if}
			<ul class="grid gap-5 md:grid-cols-2" aria-label="Recipe ideas">
				{#each sug.items as it, i (it.index)}
					<IdeaCard {it} position={i} onCook={() => cook(it)} />
				{/each}
			</ul>
		{:else if !error}
			<div class="panel flex flex-col items-center py-8 text-center">
				<KitchenScene size={170} />
				<p class="mt-3 text-[17px] font-medium">Nothing cooking yet</p>
				<p class="mt-1 max-w-sm text-sm text-[var(--ink-2)]">
					Pick a kind of meal and press Suggest. {onPhone || recipesMode === 'local'
						? onPhone
							? 'Nothing leaves your home.'
							: 'Nothing leaves this Mac.'
						: 'Cloud may be used (see Settings).'}
				</p>
			</div>
		{/if}
	{/if}
</main>

{#if showTemplates}
	<TemplatesDialog
		{templates}
		onchange={loadTemplates}
		onclose={() => {
			showTemplates = false;
			templatesButton?.focus();
		}}
	/>
{/if}

<style>
	.panel {
		border-radius: 1.25rem;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 1.1rem 1.2rem;
	}
	.chip {
		display: inline-block;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.25rem 0.8rem;
		font-size: 0.8125rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.chip:hover {
		background: var(--surface-2);
		color: var(--ink);
	}
	.seg {
		cursor: pointer;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.4rem 1rem;
		font-size: 0.9375rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.seg:hover {
		color: var(--ink);
	}
	.seg.on {
		border-color: var(--sage);
		background: var(--sage-bg);
		color: var(--sage);
		font-weight: 600;
	}
	.seg:focus-within {
		outline: 2px solid var(--focus);
		outline-offset: 2px;
	}
	fieldset:disabled .seg {
		cursor: default;
		opacity: 0.6;
	}
	.btn-primary {
		display: inline-block;
		border-radius: 0.75rem;
		background: var(--sage);
		padding: 0.55rem 1.2rem;
		text-align: center;
		font-weight: 500;
		color: var(--surface);
		transition: opacity 120ms;
	}
	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn-primary:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.stepper {
		display: inline-flex;
		align-items: center;
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.15rem;
	}
	.stepper button {
		width: 2rem;
		height: 2rem;
		border-radius: 0.6rem;
		color: var(--ink-2);
	}
	.stepper button:hover:not(:disabled) {
		background: var(--surface);
		color: var(--ink);
	}
	.stepper button:disabled {
		opacity: 0.35;
	}
</style>
