<script lang="ts">
	// "Ask about this recipe or change it": a small chat under the recipe.
	// Questions get a plain answer; a change request also rewrites the recipe
	// above in place (with Undo). State lives in `c.chat` (recipes-state), so
	// it survives tab switches and is dropped when another recipe opens.
	import { tick } from 'svelte';
	import { api, textChannel } from '#lib/api.js';
	import type { CommandError, RecipeView } from '#lib/bindings.js';
	import CloudTag from '#lib/CloudTag.svelte';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import type { Cooking } from '#lib/recipes-state.svelte.js';

	let {
		c,
		errorText,
		ranOn
	}: {
		c: Cooking;
		errorText: (e: CommandError) => string;
		ranOn: string;
	} = $props();

	const chat = $derived(c.chat);
	const CHIPS = [
		'Make it vegetarian',
		'Fewer ingredients',
		'No oven',
		'Spicier',
		'What can I swap?',
		'For 4 people'
	];

	let log: HTMLElement | undefined = $state();
	let input: HTMLInputElement | undefined = $state();

	/** The reply so far, without the recipe part (it goes into the recipe above). */
	const spoken = $derived.by(() => {
		const t = chat.live;
		const m = /^\s*(?:-{3,}|\*{0,2}#(?!#)[^\n]*)\s*$/m.exec(t);
		return { text: (m ? t.slice(0, m.index) : t).trim(), writing: !!m };
	});

	function changedLines(a: RecipeView, b: RecipeView) {
		const changed = (x: string[], y: string[]) =>
			x.map((v, i) => (y[i]?.trim() === v.trim() ? -1 : i)).filter((i) => i >= 0);
		return {
			ingredients: changed(b.ingredients, a.ingredients).filter(
				(i) => !a.ingredients.some((o) => o.trim() === b.ingredients[i].trim())
			),
			steps: changed(b.steps, a.steps).filter(
				(i) => !a.steps.some((o) => o.trim() === b.steps[i].trim())
			)
		};
	}

	async function scroll() {
		await tick();
		const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
		log?.lastElementChild?.scrollIntoView({
			block: 'nearest',
			behavior: reduce ? 'auto' : 'smooth'
		});
	}

	async function send(text: string) {
		const message = text.trim();
		const before = c.recipe;
		if (!message || chat.busy || !before) return;
		chat.error = '';
		chat.draft = '';
		chat.busy = true;
		chat.live = '';
		const history = chat.msgs.slice(-8).map((m) => ({ role: m.role, text: m.text }));
		chat.msgs.push({ role: 'user', text: message });
		scroll();
		const r = await api.recipeChat(
			{
				recipe: {
					title: before.title,
					serves: before.serves,
					minutes: before.minutes,
					ingredients: before.ingredients,
					steps: before.steps
				},
				history,
				message
			},
			textChannel((t) => (chat.live += t))
		);
		chat.busy = false;
		chat.live = '';
		if (r.status === 'error') {
			chat.error = errorText(r.error);
			return;
		}
		const revised = r.data.recipe;
		chat.msgs.push({
			role: 'cook',
			text: r.data.reply,
			revised: !!revised,
			model: r.data.model,
			usedCloud: r.data.usedCloud
		});
		if (revised) {
			chat.undo.push(before);
			chat.flash = { ...changedLines(before, revised), n: chat.flash.n + 1 };
			c.recipe = revised;
			const n = chat.flash.n;
			setTimeout(() => {
				if (chat.flash.n === n) chat.flash = { ingredients: [], steps: [], n };
			}, 3500);
		}
		scroll();
	}

	function undo() {
		const prev = chat.undo.pop();
		if (!prev || chat.busy) return;
		c.recipe = prev;
		chat.flash = { ingredients: [], steps: [], n: chat.flash.n + 1 };
		chat.msgs.push({ role: 'cook', text: 'Back to the previous version of the recipe.' });
		scroll();
	}

	/** Index of the last message that changed the recipe, for the Undo button. */
	const lastRevised = $derived(chat.msgs.findLastIndex((m) => m.revised));

	function submit(e: SubmitEvent) {
		e.preventDefault();
		send(chat.draft);
		input?.focus();
	}
</script>

{#snippet pot(px: number)}
	<svg viewBox="0 0 24 24" width={px} height={px}
		><path
			d="M5 11h14v4.5a3.5 3.5 0 0 1-3.5 3.5h-7A3.5 3.5 0 0 1 5 15.5Z"
			fill="var(--art-iron)"
		/><path
			d="M3 11.5h2M19 11.5h2"
			stroke="var(--art-iron)"
			stroke-width="1.8"
			stroke-linecap="round"
		/><path d="M4.5 10.5a7.5 5 0 0 1 15 0Z" fill="var(--art-iron-in)" /><circle
			cx="12"
			cy="5.4"
			r="1.2"
			fill="var(--c-lemon)"
		/></svg
	>
{/snippet}

<section class="chat" aria-labelledby="chat-h">
	<div class="head">
		<span class="avatar" aria-hidden="true">{@render pot(22)}</span>
		<div>
			<h2 id="chat-h" class="text-[17px] font-semibold">Ask about this recipe or change it</h2>
			<p class="text-sm text-[var(--ink-2)]">
				Questions, swaps, more people, no oven… the cook knows what is in your kitchen.
			</p>
		</div>
	</div>

	{#if chat.msgs.length || chat.busy}
		<div
			class="log"
			bind:this={log}
			role="log"
			aria-live="polite"
			aria-relevant="additions"
			aria-label="Conversation about this recipe"
		>
			{#each chat.msgs as m, i (i)}
				<div class="row {m.role}">
					{#if m.role === 'cook'}<span class="avatar sm" aria-hidden="true">{@render pot(16)}</span
						>{/if}
					<div class="bubble">
						<p class="whitespace-pre-line">{m.text}</p>
						{#if m.revised}
							<p class="note">
								Recipe updated above
								{#if i === lastRevised && chat.undo.length > 0}
									·
									<button class="undo" onclick={undo} disabled={chat.busy}>Undo change</button>
								{/if}
							</p>
						{/if}
						{#if m.role === 'cook' && m.model && i === chat.msgs.length - 1}
							<p class="meta">{m.model}<CloudTag show={!!m.usedCloud} /></p>
						{/if}
					</div>
				</div>
			{/each}
			{#if chat.busy}
				<div class="row cook">
					<span class="avatar sm" aria-hidden="true">{@render pot(16)}</span>
					<div class="bubble">
						{#if spoken.text}<p class="whitespace-pre-line">{spoken.text}</p>{/if}
						{#if spoken.writing}
							<p class="note">Rewriting the recipe…</p>
						{:else if !spoken.text}
							<span class="typing" role="status" aria-label="The cook is typing">
								<i></i><i></i><i></i>
							</span>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}

	{#if chat.error}
		<ErrorBanner message={chat.error} class="mt-3" />
	{/if}

	<div class="chips" role="group" aria-label="Quick requests">
		{#each CHIPS as q (q)}
			<button class="qchip" disabled={chat.busy} onclick={() => send(q)}>{q}</button>
		{/each}
	</div>

	<form onsubmit={submit} class="composer">
		<label class="sr-only" for="chat-in">Your message</label>
		<input
			id="chat-in"
			bind:this={input}
			bind:value={chat.draft}
			disabled={chat.busy}
			enterkeyhint="send"
			autocomplete="off"
			placeholder="e.g. Can I freeze the leftovers?"
			class="min-w-0 flex-1 rounded-xl bg-[var(--surface-2)] px-3 py-2 text-[var(--ink)] placeholder:text-[var(--ink-3)]"
		/>
		<button class="send" disabled={chat.busy || !chat.draft.trim()}>Send</button>
	</form>
	<p class="mt-2 text-xs text-[var(--ink-3)]">
		Answered {ranOn}. Your recipe only changes when you ask.
	</p>
</section>

<style>
	.chat {
		margin-top: 1.5rem;
		border-radius: 1.25rem;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 1.1rem 1.1rem 1rem;
	}
	.head {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
	}
	.avatar {
		display: inline-flex;
		flex: none;
		width: 2.4rem;
		height: 2.4rem;
		align-items: center;
		justify-content: center;
		border-radius: 9999px;
		background: var(--sage-bg);
	}
	.avatar.sm {
		width: 1.75rem;
		height: 1.75rem;
		background: var(--sage-bg);
	}
	.log {
		display: grid;
		gap: 0.7rem;
		max-height: 28rem;
		overflow-y: auto;
		margin-top: 1rem;
		padding-right: 0.15rem;
	}
	.row {
		display: flex;
		gap: 0.5rem;
		align-items: flex-end;
		animation: pop 260ms ease-out both;
	}
	.row.user {
		justify-content: flex-end;
	}
	.bubble {
		max-width: min(34rem, 85%);
		border-radius: 1.1rem;
		padding: 0.55rem 0.85rem;
		font-size: 0.9375rem;
		line-height: 1.55;
	}
	.user .bubble {
		border-bottom-right-radius: 0.3rem;
		background: var(--sage);
		color: var(--surface);
	}
	.cook .bubble {
		border-bottom-left-radius: 0.3rem;
		background: var(--surface-2);
		color: var(--ink);
	}
	.note {
		margin-top: 0.3rem;
		font-size: 0.8125rem;
		color: var(--ink-2);
	}
	.meta {
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--ink-3);
	}
	.undo {
		min-height: 0;
		border-radius: 0.4rem;
		padding: 0 0.15rem;
		font-weight: 600;
		color: var(--sage);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.undo:disabled {
		opacity: 0.5;
	}
	.typing {
		display: inline-flex;
		gap: 0.3rem;
		padding: 0.35rem 0;
	}
	.typing i {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 9999px;
		background: var(--ink-3);
		animation: bob 1.1s ease-in-out infinite;
	}
	.typing i:nth-child(2) {
		animation-delay: 0.15s;
	}
	.typing i:nth-child(3) {
		animation-delay: 0.3s;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.45rem;
		margin-top: 1rem;
	}
	.qchip {
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.3rem 0.85rem;
		font-size: 0.875rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.qchip:hover:not(:disabled) {
		border-color: var(--sage);
		background: var(--sage-bg);
		color: var(--sage);
	}
	.qchip:disabled {
		opacity: 0.5;
	}
	.composer {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.75rem;
	}
	.send {
		flex: none;
		border-radius: 0.75rem;
		background: var(--sage);
		padding: 0.5rem 1.1rem;
		font-weight: 500;
		color: var(--surface);
	}
	.send:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	@keyframes pop {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}
	@keyframes bob {
		0%,
		60%,
		100% {
			transform: translateY(0);
			opacity: 0.5;
		}
		30% {
			transform: translateY(-4px);
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.row,
		.typing i {
			animation: none;
		}
	}
</style>
