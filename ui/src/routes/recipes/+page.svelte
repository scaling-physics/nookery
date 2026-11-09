<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api, textChannel } from '#lib/api.js';
	import type {
		AiModeView,
		CommandError,
		LotView,
		SuggestionView,
		TemplateView
	} from '#lib/bindings.js';
	import { CLOUD_CREDITS, CLOUD_SIGN_IN, itemName, statusLabel } from '#lib/format.js';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import CloudTag from '#lib/CloudTag.svelte';
	import { rs } from '#lib/recipes-state.svelte.js';
	import TemplatesDialog from '#lib/TemplatesDialog.svelte';

	let templates = $state<TemplateView[]>([]);
	let loaded = $state(false);
	let showTemplates = $state(false);
	let thinking = $state(false);
	let elapsed = $state(0);
	let timer: ReturnType<typeof setInterval> | undefined;
	let error = $state('');
	let recipesMode = $state<AiModeView>('local');
	const ranOn = $derived(recipesMode === 'cloud' ? 'in the cloud' : 'on this Mac');
	/** The have-chip whose location is shown: "<index>:<lot id>". */
	let shown = $state<string | null>(null);
	let templatesButton: HTMLButtonElement | undefined = $state();

	// "I cooked it"
	let ticked = $state<Record<string, boolean>>({});
	let marking = $state(false);
	let marked = $state<{ n: number; failed: number } | null>(null);

	const sug = $derived(rs.suggestions);
	const cooking = $derived(rs.cooking);

	onDestroy(() => clearInterval(timer));

	function errorText(e: CommandError): string {
		switch (e.kind) {
			case 'cloud_sign_in':
				return CLOUD_SIGN_IN;
			case 'cloud_credits':
				return CLOUD_CREDITS;
			case 'recipe_model_unavailable':
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
		shown = null;
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
		marked = null;
		const c = { item, text: '', recipe: null, error: '' };
		rs.cooking = c;
		const mine = rs.cooking; // the reactive proxy
		const r = await api.writeRecipe(
			{ suggestionsId: sug.id, index: item.index },
			textChannel((t) => {
				if (rs.cooking === mine && mine) mine.text += t;
			})
		);
		if (rs.cooking !== mine || !mine) return; // the user went back
		if (r.status === 'ok') {
			mine.recipe = r.data;
			ticked = Object.fromEntries(item.have.map((h) => [h.lot.short, true]));
		} else mine.error = errorText(r.error);
	}

	function back() {
		rs.cooking = null;
		marked = null;
	}

	async function markUsed() {
		if (!cooking || marking) return;
		marking = true;
		let n = 0;
		let failed = 0;
		for (const h of cooking.item.have) {
			if (!ticked[h.lot.short]) continue;
			const r = await api.lotAction(h.lot.short, { type: 'use', qty: null });
			if (r.status === 'ok') n++;
			else failed++;
		}
		marking = false;
		marked = { n, failed };
	}

	const nTicked = $derived(Object.values(ticked).filter(Boolean).length);

	function tone(l: LotView) {
		return l.status === 'use_soon' ? 'soon' : l.status === 'expired' ? 'expired' : 'plain';
	}

	function where(l: LotView): string {
		const s = statusLabel(l).text;
		return `${itemName(l.item)}: ${l.left} in the ${l.location}${s ? `, ${s.toLowerCase()}` : ''}`;
	}

	function expiredWarning(l: LotView): string {
		const s = statusLabel(l).text;
		return `${itemName(l.item)} ${s ? s.replace(/^Expired /, 'expired ') : 'is past its date'} — check it before using.`;
	}

	onMount(() => {
		loadTemplates();
		api.getAiSettings().then((s) => {
			if (s.status === 'ok') recipesMode = s.data.recipes;
		});
	});
</script>

<main class="mx-auto max-w-6xl px-5 pt-6 pb-16 sm:px-8">
	<div class="max-w-3xl">
		{#if cooking}
			{@const c = cooking}
			<div class="mb-4 flex items-center justify-between gap-3">
				<button class="chip" onclick={back}>← Back to ideas</button>
			</div>
			<article
				aria-labelledby="recipe-h"
				class="rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-5"
			>
				<h1 id="recipe-h" class="text-[22px] font-semibold tracking-tight">
					{c.recipe?.title || c.item.name}
				</h1>
				<p class="mt-0.5 text-sm text-[var(--ink-2)]">
					{#if c.recipe}
						{[
							c.recipe.serves ? `Serves ${c.recipe.serves}` : '',
							c.recipe.minutes ? `${c.recipe.minutes} min` : ''
						]
							.filter(Boolean)
							.join(' · ')}
					{:else if c.item.minutes}
						About {c.item.minutes} min
					{/if}
				</p>

				{#if c.error}
					<ErrorBanner message={c.error} class="mt-4" />
				{:else if !c.recipe}
					<div role="status" aria-live="polite" class="mt-4">
						<p class="text-sm text-[var(--ink-2)]">
							{c.text ? `Writing the recipe ${ranOn}…` : 'Getting started…'}
						</p>
						{#if c.text}
							<pre class="stream mt-3">{c.text}</pre>
						{/if}
					</div>
				{:else if c.recipe.ingredients.length || c.recipe.steps.length}
					{#if c.recipe.ingredients.length}
						<h2
							class="mt-5 mb-2 text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
						>
							Ingredients
						</h2>
						<ul class="grid gap-1 sm:grid-cols-2">
							{#each c.recipe.ingredients as ing, i (i)}
								<li class="flex gap-2 text-[15px]">
									<span aria-hidden="true" class="text-[var(--sage)]">·</span>{ing}
								</li>
							{/each}
						</ul>
					{/if}
					{#if c.recipe.steps.length}
						<h2
							class="mt-6 mb-2 text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
						>
							Steps
						</h2>
						<ol class="grid gap-3">
							{#each c.recipe.steps as step, i (i)}
								<li class="flex gap-3 text-[15px] leading-relaxed">
									<span
										class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[var(--sage-bg)] text-xs font-medium text-[var(--sage)] tabular-nums"
										>{i + 1}</span
									>
									<span>{step}</span>
								</li>
							{/each}
						</ol>
					{/if}
				{:else}
					<pre class="stream mt-4">{c.recipe.raw}</pre>
				{/if}
				{#if c.recipe}
					<p class="mt-5 text-xs text-[var(--ink-3)]">
						{Math.round((c.recipe.ms ?? 0) / 1000)} s · {c.recipe.model}<CloudTag
							show={c.recipe.usedCloud}
						/>
					</p>
				{/if}
			</article>

			{#if c.recipe}
				<section
					class="mt-6 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-5"
					aria-labelledby="cooked-h"
				>
					<h2 id="cooked-h" class="font-medium">I cooked it</h2>
					{#if marked}
						<div role="status" class="mt-2">
							<p class="text-[var(--sage)]">Marked {marked.n} used</p>
							{#if marked.failed}
								<p class="text-sm text-[var(--terracotta)]">
									{marked.failed} could not be marked. Check them in the Kitchen tab.
								</p>
							{/if}
							<div class="mt-3 flex gap-2">
								<a href="/" class="btn-primary">See the kitchen</a>
								<button class="chip" onclick={back}>Back to ideas</button>
							</div>
						</div>
					{:else if c.item.have.length === 0}
						<p class="mt-1 text-sm text-[var(--ink-2)]">
							Nothing from your stock was matched to this recipe, so there is nothing to mark.
						</p>
					{:else}
						<p class="mt-1 text-sm text-[var(--ink-2)]">
							Tick what you used up. Each one is marked as used completely.
						</p>
						<ul class="mt-3 grid gap-1.5">
							{#each c.item.have as h (h.lot.id)}
								<li>
									<label class="flex items-center gap-3 text-[15px]">
										<input
											type="checkbox"
											bind:checked={ticked[h.lot.short]}
											class="h-4 w-4 accent-[var(--sage)]"
										/>
										<span class="flex-1">{itemName(h.lot.item)}</span>
										<span class="text-sm text-[var(--ink-3)] tabular-nums"
											>{h.lot.left} · used all</span
										>
									</label>
								</li>
							{/each}
						</ul>
						<p class="mt-3 text-xs text-[var(--ink-3)]">
							Used only some? Adjust it in the Kitchen tab.
						</p>
						<button class="btn-primary mt-3" onclick={markUsed} disabled={marking || nTicked === 0}>
							{marking ? 'Marking…' : `Mark ${nTicked} as used`}
						</button>
					{/if}
				</section>
			{/if}
		{:else}
			<header class="mb-5 flex items-center justify-between gap-4">
				<div>
					<h1 class="text-[20px] font-semibold tracking-tight">Recipes</h1>
					<p class="text-sm text-[var(--ink-2)]">
						Ideas from what you have, use-soon things first. Written {ranOn}.
					</p>
				</div>
				<button
					class="chip shrink-0"
					bind:this={templatesButton}
					onclick={() => (showTemplates = true)}
				>
					Templates
				</button>
			</header>

			<form
				onsubmit={suggest}
				class="mb-6 grid gap-4 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-4"
			>
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

				<div class="flex flex-wrap items-center gap-3">
					<button class="btn-primary" disabled={thinking || !loaded}>
						{thinking ? 'Thinking…' : sug ? 'Suggest again' : 'Suggest'}
					</button>
					{#if thinking}
						<p role="status" aria-live="polite" class="text-sm text-[var(--ink-2)]">
							Thinking {ranOn}… usually ~15 s
							<span class="text-[var(--ink-3)] tabular-nums">{elapsed} s</span>
						</p>
					{/if}
				</div>
				{#if thinking}
					<progress class="bar" aria-label="Thinking"></progress>
				{/if}
			</form>

			{#if error}
				<ErrorBanner message={error} class="mb-4" />
			{/if}

			{#if sug && !thinking}
				<p class="mb-3 text-sm text-[var(--ink-2)]">
					{sug.items.length}
					{sug.items.length === 1 ? 'idea' : 'ideas'}, best first ·
					{Math.round((sug.ms ?? 0) / 1000)} s · {sug.model}<CloudTag show={sug.usedCloud} />
				</p>
				{#if sug.items.length === 0}
					<p
						class="rounded-2xl border border-dashed border-[var(--line)] p-4 text-sm text-[var(--ink-3)]"
					>
						The model came back with nothing usable. Try again, or change the note.
					</p>
				{/if}
				<ul class="grid gap-4" aria-label="Recipe ideas">
					{#each sug.items as it (it.index)}
						<li class="card">
							<div class="flex flex-wrap items-baseline justify-between gap-x-4">
								<h2 class="text-[17px] font-semibold tracking-tight">{it.name}</h2>
								{#if it.minutes}
									<span class="text-sm text-[var(--ink-2)] tabular-nums">{it.minutes} min</span>
								{/if}
							</div>
							{#if it.why}<p class="mt-1 text-[15px] text-[var(--ink-2)]">{it.why}</p>{/if}

							{#if it.have.length}
								<p class="label">You have</p>
								<ul class="flex flex-wrap gap-1.5">
									{#each it.have as h (h.lot.id)}
										{@const key = `${it.index}:${h.lot.id}`}
										<li>
											<button
												class="tag {tone(h.lot)}"
												aria-pressed={shown === key}
												aria-label="{h.ingredient}{h.lot.status === 'use_soon'
													? ' (use soon)'
													: ''}: show where it is"
												onclick={() => (shown = shown === key ? null : key)}
											>
												{h.ingredient}
											</button>
										</li>
									{/each}
								</ul>
								{#each it.have as h (h.lot.id)}
									{#if shown === `${it.index}:${h.lot.id}`}
										<p role="status" class="mt-2 text-sm text-[var(--ink-2)]">{where(h.lot)}</p>
									{/if}
								{/each}
							{/if}

							{#if it.need.length}
								<p class="label">You would need</p>
								<ul class="flex flex-wrap gap-1.5">
									{#each it.need as n, i (i)}
										<li class="tag muted">{n}</li>
									{/each}
								</ul>
							{/if}

							{#each it.expired as l (l.id)}
								<p class="mt-3 text-sm text-[var(--terracotta)]">{expiredWarning(l)}</p>
							{/each}

							<div class="mt-4">
								<button class="btn-primary" onclick={() => cook(it)}>Cook this</button>
							</div>
						</li>
					{/each}
				</ul>
			{:else if !thinking && !error}
				<p class="text-sm text-[var(--ink-3)]">
					Pick a kind of meal and press Suggest. {recipesMode === 'local'
						? 'Nothing leaves this Mac.'
						: 'Cloud may be used (see Settings).'}
				</p>
			{/if}
		{/if}
	</div>
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
		padding: 0.35rem 0.95rem;
		font-size: 0.875rem;
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
		font-weight: 500;
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
		padding: 0.5rem 1rem;
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
	.card {
		border-radius: 1rem;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 1.1rem 1.2rem;
	}
	.label {
		margin: 0.9rem 0 0.35rem;
		font-size: 0.75rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--ink-3);
	}
	.tag {
		display: inline-block;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.15rem 0.7rem;
		font-size: 0.875rem;
		color: var(--ink);
	}
	button.tag {
		cursor: pointer;
	}
	.tag.soon {
		border-color: var(--saffron);
		background: var(--saffron-bg);
		color: var(--saffron);
		font-weight: 500;
	}
	.tag.expired {
		border-color: var(--terracotta);
		color: var(--terracotta);
	}
	.tag.muted {
		border-style: dashed;
		background: transparent;
		color: var(--ink-3);
	}
	button.tag[aria-pressed='true'] {
		outline: 2px solid var(--focus);
		outline-offset: 1px;
	}
	.stream {
		max-height: 24rem;
		overflow: auto;
		white-space: pre-wrap;
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.75rem 1rem;
		font: inherit;
		font-size: 0.875rem;
		line-height: 1.6;
		color: var(--ink-2);
	}
	.bar {
		width: 100%;
		height: 0.35rem;
		appearance: none;
		border: 0;
		border-radius: 9999px;
		overflow: hidden;
		background: var(--surface-2);
	}
	.bar::-webkit-progress-bar {
		background: var(--surface-2);
	}
	.bar::-moz-progress-bar {
		background: var(--sage);
	}
</style>
