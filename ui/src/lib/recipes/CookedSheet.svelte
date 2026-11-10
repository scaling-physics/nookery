<script lang="ts">
	// "I cooked it": what did you use? For each of the idea's lots in stock:
	// all of it, some of it (an amount like 200g or 1), or none. One write.
	// A bottom sheet on phones, a centred dialog on wide screens.
	import { api } from '../api.js';
	import type { CommandError, SuggestionView } from '../bindings.js';
	import { displayName } from '../format.js';
	import FoodIcon from '../FoodIcon.svelte';

	let {
		open = $bindable(false),
		suggestion,
		onDone
	}: { open: boolean; suggestion: SuggestionView; onDone?: () => void } = $props();

	type Choice = 'all' | 'some' | 'none';
	let dialog: HTMLDialogElement | undefined = $state();
	let choice = $state<Record<string, Choice>>({});
	let amount = $state<Record<string, string>>({});
	let fieldError = $state<Record<string, string>>({});
	let error = $state('');
	let busy = $state(false);
	let result = $state<{ lots: number; usedUp: number } | null>(null);

	const lots = $derived(suggestion.have);

	// Open and close the native dialog from `open`; start fresh each time.
	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			choice = Object.fromEntries(lots.map((h) => [h.lot.id, 'all' as Choice]));
			amount = {};
			fieldError = {};
			error = '';
			result = null;
			dialog.showModal();
		} else if (!open && dialog.open) dialog.close();
	});

	const used = $derived(lots.filter((h) => (choice[h.lot.id] ?? 'all') !== 'none'));

	function describe(e: CommandError): string {
		if (e.kind === 'bad_input') return 'Check the amounts marked below.';
		if (e.kind === 'not_found')
			return 'One of these is no longer in the kitchen. Close this and look again.';
		return 'Something went wrong. Please try again.';
	}

	async function submit() {
		if (busy) return;
		error = '';
		fieldError = {};
		const missing: Record<string, string> = {};
		for (const h of lots)
			if (choice[h.lot.id] === 'some' && !(amount[h.lot.id] ?? '').trim())
				missing[h.lot.id] = `How much ${itemOf(h.lot.item)}? For example ${example(h.lot.left)}.`;
		if (Object.keys(missing).length) {
			fieldError = missing;
			return;
		}
		busy = true;
		const r = await api.cooked({
			items: used.map((h) => ({
				lot: h.lot.short,
				amount: choice[h.lot.id] === 'some' ? amount[h.lot.id].trim() : null
			})),
			title: suggestion.name
		});
		busy = false;
		if (r.status === 'ok') {
			result = r.data;
			onDone?.();
			return;
		}
		error = describe(r.error);
		if (r.error.kind === 'bad_input') {
			const m = /^(amount|lot):(\d+)$/.exec(r.error.detail);
			const h = m ? used[Number(m[2])] : undefined;
			if (h)
				fieldError = {
					[h.lot.id]:
						m![1] === 'amount'
							? `That does not fit: ${h.lot.left} left. Use the same unit, for example ${example(h.lot.left)}.`
							: 'This one is not in the kitchen any more.'
				};
		}
	}

	const itemOf = (s: string) => s.trim().toLowerCase();
	/** A sensible half of "500g" -> "250g", "4" -> "2", else the amount. */
	function example(left: string): string {
		const m = /^(\d+(?:\.\d+)?)\s*([a-z]*)$/i.exec(left.trim());
		if (!m) return left;
		const half = Number(m[1]) / 2;
		return `${Number.isInteger(half) ? half : Math.max(1, Math.round(half))}${m[2]}`;
	}

	function close() {
		open = false;
	}
	function backdrop(e: MouseEvent) {
		if (e.target === dialog) close();
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	aria-labelledby="cooked-h"
	onclose={() => (open = false)}
	onclick={backdrop}
	class="sheet text-[var(--ink)]"
>
	<div class="flex max-h-[85vh] flex-col">
		<header class="px-5 pt-5 pb-2">
			<h2 id="cooked-h" class="text-[20px] font-semibold tracking-tight">I cooked it</h2>
			<p class="text-sm text-[var(--ink-2)]">
				{#if result}
					{suggestion.name}
				{:else if lots.length}
					What did you use for {suggestion.name}?
				{:else}
					Nothing from your kitchen was matched to this dish.
				{/if}
			</p>
		</header>

		{#if result}
			<div role="status" class="px-5 py-3">
				<p class="text-[var(--sage)]">
					{result.lots === 0
						? 'Nothing taken out of the kitchen.'
						: `Updated ${result.lots} ${result.lots === 1 ? 'item' : 'items'}${result.usedUp ? `, ${result.usedUp} used up` : ''}.`}
				</p>
			</div>
			<footer class="flex justify-end gap-2 px-5 pt-2 pb-5">
				<a href="/" class="btn-quiet">See the kitchen</a>
				<button type="button" class="btn-primary" onclick={close}>Done</button>
			</footer>
		{:else}
			<ul class="grid gap-3 overflow-y-auto px-5 py-2">
				{#each lots as h (h.lot.id)}
					{@const c = choice[h.lot.id] ?? 'all'}
					<li class="rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-3">
						<div class="flex items-center gap-2">
							<FoodIcon category={h.lot.category} size={22} />
							<span class="min-w-0 flex-1 truncate font-medium">{displayName(h.lot.item)}</span>
							<span class="text-sm text-[var(--ink-3)] tabular-nums">{h.lot.left} left</span>
						</div>
						<div class="seg-group mt-2" role="radiogroup" aria-label="How much {h.lot.item}">
							{#each [['all', 'All of it'], ['some', 'Some'], ['none', 'None']] as [v, label] (v)}
								<label class="seg" class:on={c === v}>
									<input
										type="radio"
										class="sr-only"
										name="use-{h.lot.id}"
										value={v}
										checked={c === v}
										onchange={() => (choice[h.lot.id] = v as Choice)}
									/>
									{label}
								</label>
							{/each}
						</div>
						{#if c === 'some'}
							<label class="mt-2 grid gap-1 text-[13px] font-semibold text-[var(--ink-2)]">
								How much did you use?
								<input
									bind:value={amount[h.lot.id]}
									inputmode="decimal"
									autocomplete="off"
									placeholder={example(h.lot.left)}
									aria-invalid={fieldError[h.lot.id] ? 'true' : undefined}
									aria-describedby={fieldError[h.lot.id] ? `err-${h.lot.id}` : undefined}
									class="field"
								/>
							</label>
						{/if}
						{#if fieldError[h.lot.id]}
							<p id="err-{h.lot.id}" role="alert" class="mt-1 text-sm text-[var(--terracotta)]">
								{fieldError[h.lot.id]}
							</p>
						{/if}
					</li>
				{/each}
			</ul>
			{#if error}
				<p
					role="alert"
					class="mx-5 mt-2 rounded-2xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
				>
					{error}
				</p>
			{/if}
			<footer class="flex items-center justify-end gap-2 px-5 pt-3 pb-5">
				<button type="button" class="btn-quiet" onclick={close}>Cancel</button>
				<button type="button" class="btn-primary" disabled={busy} onclick={submit}>
					{busy
						? 'Saving…'
						: used.length === 0
							? 'Used nothing'
							: `Update ${used.length} ${used.length === 1 ? 'item' : 'items'}`}
				</button>
			</footer>
		{/if}
	</div>
</dialog>

<style>
	.sheet {
		margin: auto;
		width: min(30rem, calc(100vw - 1.5rem));
		max-width: none;
		padding: 0;
		border: 1px solid var(--line);
		border-radius: 1.5rem;
		background: var(--bg);
		box-shadow: 0 24px 60px -20px rgb(0 0 0 / 0.4);
	}
	.sheet::backdrop {
		background: var(--backdrop);
	}
	@media (max-width: 639px) {
		.sheet {
			margin: auto 0 0 0;
			width: 100%;
			border-radius: 1.5rem 1.5rem 0 0;
			border-bottom: 0;
			padding-bottom: env(safe-area-inset-bottom);
		}
		.sheet[open] {
			animation: rise 200ms ease-out;
		}
	}
	@keyframes rise {
		from {
			transform: translateY(2rem);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.sheet[open] {
			animation: none;
		}
	}
	.seg-group {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.35rem;
	}
	.seg {
		cursor: pointer;
		border-radius: 0.75rem;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.6rem 0.4rem;
		text-align: center;
		font-size: 0.875rem;
		color: var(--ink-2);
		min-height: 2.75rem;
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
	.field {
		min-width: 0;
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.6rem 0.75rem;
		font-weight: 400;
		color: var(--ink);
		font-size: 1rem;
	}
	.field[aria-invalid='true'] {
		outline: 2px solid var(--terracotta);
	}
	.btn-primary {
		border-radius: 0.75rem;
		background: var(--sage);
		padding: 0.6rem 1.1rem;
		font-weight: 500;
		color: var(--surface);
		min-height: 2.75rem;
	}
	.btn-primary:disabled {
		opacity: 0.4;
	}
	.btn-quiet {
		display: inline-flex;
		align-items: center;
		border-radius: 0.75rem;
		padding: 0.6rem 0.9rem;
		color: var(--ink-2);
		min-height: 2.75rem;
	}
	.btn-quiet:hover {
		background: var(--surface-2);
		color: var(--ink);
	}
</style>
