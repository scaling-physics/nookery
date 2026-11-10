<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { PutAwayView, ShoppingItemView, ShoppingView } from '#lib/bindings.js';
	import { displayName } from '#lib/format.js';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import FoodIcon from '#lib/FoodIcon.svelte';
	import UndoToast from '#lib/kitchen/UndoToast.svelte';
	import { aisleName } from '#lib/plan/helpers.js';

	let list = $state<ShoppingView | null>(null);
	let item = $state('');
	let qty = $state('');
	let error = $state('');
	let busy = $state(false);
	let toast = $state('');
	let undoable = $state<PutAwayView | null>(null);
	let toastTimer: ReturnType<typeof setTimeout> | undefined;
	let itemInput: HTMLInputElement | undefined = $state();

	const open = $derived(list?.open ?? 0);
	const done = $derived(list?.done ?? 0);
	const empty = $derived(list !== null && open + done === 0);

	onDestroy(() => clearTimeout(toastTimer));

	async function reload() {
		const r = await api.listShopping();
		if (r.status === 'ok') list = r.data;
		else error = 'Could not read the shopping list.';
	}

	function fail(kind: string) {
		error =
			kind === 'bad_input'
				? 'Type what you want to buy first.'
				: 'Something went wrong. Please try again.';
	}

	async function add(e: SubmitEvent) {
		e.preventDefault();
		if (!item.trim() || busy) return;
		busy = true;
		error = '';
		const r = await api.addShopping({ item, qty: qty.trim() || null });
		busy = false;
		if (r.status === 'ok') {
			item = '';
			qty = '';
			await reload();
			itemInput?.focus();
		} else fail(r.error.kind);
	}

	async function tick(it: ShoppingItemView) {
		error = '';
		// Show the tick at once; the reload puts it in its place.
		it.done = !it.done;
		if (list) list.done += it.done ? 1 : -1;
		if (list) list.open += it.done ? -1 : 1;
		const r = await api.setShoppingDone({ id: it.id, done: it.done });
		if (r.status !== 'ok') fail(r.error.kind);
		await reload();
	}

	async function remove(it: ShoppingItemView) {
		const r = await api.deleteShopping(it.id);
		if (r.status !== 'ok' && r.error.kind !== 'not_found') fail(r.error.kind);
		await reload();
	}

	async function clearTicked() {
		busy = true;
		const r = await api.clearDoneShopping();
		busy = false;
		if (r.status !== 'ok') fail(r.error.kind);
		await reload();
	}

	function say(message: string, ms = 8000) {
		toast = message;
		clearTimeout(toastTimer);
		toastTimer = setTimeout(() => {
			toast = '';
			undoable = null;
		}, ms);
	}

	async function putAway() {
		if (busy || done === 0) return;
		busy = true;
		error = '';
		const r = await api.putAwayShopping();
		busy = false;
		if (r.status !== 'ok') {
			fail(r.error.kind);
			return;
		}
		undoable = r.data;
		const n = r.data.lots.length;
		say(`Put ${n} ${n === 1 ? 'thing' : 'things'} in the kitchen`);
		await reload();
	}

	async function undo() {
		const u = undoable;
		clearTimeout(toastTimer);
		toast = '';
		undoable = null;
		if (!u) return;
		// Take the new lots back out (as entered by mistake) and put the
		// items back on the list.
		for (const l of u.lots) await api.lotAction(l.short, { type: 'wrong' });
		for (const i of u.items) {
			const r = await api.addShopping({ item: i.item, qty: i.qty });
			if (r.status === 'ok') await api.setShoppingDone({ id: r.data.id, done: true });
		}
		await reload();
	}

	onMount(reload);
</script>

<main class="mx-auto max-w-2xl px-5 pt-6 pb-36 sm:px-8">
	<header class="mb-4">
		<h1 class="text-[20px] font-semibold tracking-tight">Shopping</h1>
		<p class="text-sm text-[var(--ink-2)]" aria-live="polite">
			{#if list === null}
				Loading…
			{:else if empty}
				Nothing to buy.
			{:else}
				{open} to buy{done ? ` · ${done} in the basket` : ''}
			{/if}
		</p>
	</header>

	<form onsubmit={add} class="mb-5 flex gap-2" aria-label="Add to the list">
		<label class="sr-only" for="shop-item">Item</label>
		<input
			id="shop-item"
			bind:this={itemInput}
			bind:value={item}
			placeholder="Add something…"
			autocomplete="off"
			enterkeyhint="done"
			class="field min-w-0 flex-1"
		/>
		<label class="sr-only" for="shop-qty">Amount (optional)</label>
		<input
			id="shop-qty"
			bind:value={qty}
			placeholder="Amount"
			autocomplete="off"
			enterkeyhint="done"
			class="field w-24 shrink-0"
		/>
		<button class="btn-primary shrink-0" disabled={busy || !item.trim()}>Add</button>
	</form>

	{#if error}
		<ErrorBanner message={error} class="mb-4" />
	{/if}

	{#if empty}
		<div
			class="rounded-2xl border border-dashed border-[var(--line)] p-5 text-sm text-[var(--ink-3)]"
		>
			<p>The list is empty. Type something above, or plan the week and add what is missing.</p>
			<a
				href="/plan"
				class="mt-2 inline-block font-medium text-[var(--sage)] underline underline-offset-2"
				>Go to the plan</a
			>
		</div>
	{/if}

	{#if list}
		<div class="grid gap-5">
			{#each list.groups as g (g.category)}
				<section aria-labelledby="aisle-{g.category}">
					<h2
						id="aisle-{g.category}"
						class="mb-1.5 flex items-center gap-2 text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
					>
						<FoodIcon category={g.category} size={20} />{aisleName(g.category)}
					</h2>
					<ul class="grid gap-1.5">
						{#each g.items as it (it.id)}
							<li class="row" class:ticked={it.done}>
								<label class="tap">
									<input
										type="checkbox"
										checked={it.done}
										onchange={() => tick(it)}
										class="sr-only"
									/>
									<span class="box" aria-hidden="true">
										{#if it.done}
											<svg viewBox="0 0 16 16" width="16" height="16"
												><path
													d="M3.5 8.5l3 3 6-7"
													fill="none"
													stroke="currentColor"
													stroke-width="2.2"
													stroke-linecap="round"
													stroke-linejoin="round"
												/></svg
											>
										{/if}
									</span>
									<span class="min-w-0 flex-1">
										<span class="name">{displayName(it.item)}</span>
										{#if it.from.startsWith('plan:')}
											<span class="from">for the plan</span>
										{/if}
									</span>
									{#if it.qty}<span class="text-sm text-[var(--ink-3)] tabular-nums">{it.qty}</span
										>{/if}
								</label>
								<button
									type="button"
									class="del"
									aria-label="Remove {it.item} from the list"
									onclick={() => remove(it)}
								>
									<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"
										><path
											d="M4 4l8 8M12 4l-8 8"
											stroke="currentColor"
											stroke-width="1.8"
											stroke-linecap="round"
										/></svg
									>
								</button>
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		</div>
	{/if}
</main>

{#if done > 0}
	<div class="bar" role="region" aria-label="Ticked items">
		<p class="text-sm text-[var(--ink-2)]">{done} ticked</p>
		<div class="flex gap-2">
			<button class="chip" disabled={busy} onclick={clearTicked}>Clear ticked</button>
			<button class="btn-primary" disabled={busy} onclick={putAway}>Put away</button>
		</div>
	</div>
{/if}

<UndoToast message={toast} onundo={undo} raised />

<style>
	.field {
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.7rem 0.9rem;
		font-size: 1rem;
		color: var(--ink);
		min-height: 2.9rem;
	}
	.field::placeholder {
		color: var(--ink-3);
	}
	.btn-primary {
		border-radius: 0.75rem;
		background: var(--sage);
		padding: 0.6rem 1.1rem;
		font-weight: 500;
		color: var(--surface);
		min-height: 2.9rem;
		transition: opacity 120ms;
	}
	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn-primary:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.chip {
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.5rem 1rem;
		min-height: 2.9rem;
		font-size: 0.9rem;
		color: var(--ink-2);
	}
	.chip:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}
	.chip:disabled {
		opacity: 0.4;
	}
	.row {
		display: flex;
		align-items: stretch;
		border-radius: 1rem;
		border: 1px solid var(--line);
		background: var(--surface);
	}
	.tap {
		display: flex;
		min-width: 0;
		flex: 1;
		cursor: pointer;
		align-items: center;
		gap: 0.8rem;
		padding: 0.5rem 0.5rem 0.5rem 0.9rem;
		min-height: 3.5rem;
		border-radius: 1rem;
	}
	.tap:has(input:focus-visible) {
		outline: 2px solid var(--focus);
		outline-offset: 2px;
	}
	.box {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		border-radius: 9999px;
		border: 2px solid var(--ink-3);
		color: var(--surface);
	}
	.ticked .box {
		border-color: var(--sage);
		background: var(--sage);
	}
	.name {
		font-size: 1.0625rem;
	}
	.from {
		margin-left: 0.4rem;
		font-size: 0.75rem;
		color: var(--ink-3);
	}
	.ticked .name {
		color: var(--ink-3);
		text-decoration: line-through;
	}
	.del {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 3rem;
		flex: none;
		border-radius: 1rem;
		color: var(--ink-3);
	}
	.del:hover {
		color: var(--terracotta);
		background: var(--surface-2);
	}
	.bar {
		position: fixed;
		inset-inline: 0;
		bottom: 0;
		z-index: 40;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-top: 1px solid var(--line);
		background: var(--surface);
		padding: 0.6rem 1.25rem;
		padding-bottom: calc(0.6rem + env(safe-area-inset-bottom));
	}
	@media (max-width: 639px) {
		.bar {
			bottom: calc(4.1rem + env(safe-area-inset-bottom));
			padding-bottom: 0.6rem;
		}
	}
</style>
