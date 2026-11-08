<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { InventoryView, LotView } from '#lib/bindings.js';
	import { frozenLabel, itemName, statusLabel, urgent } from '#lib/format.js';

	const PLACES = [
		{ id: 'fridge', label: 'Fridge' },
		{ id: 'freezer', label: 'Freezer' },
		{ id: 'pantry', label: 'Pantry' }
	] as const;

	let inv = $state<InventoryView | null>(null);
	let error = $state('');

	// Quick add
	let item = $state('');
	let qty = $state('');
	let place = $state<string>('fridge');
	let bb = $state('');
	let shop = $state('');
	let itemInput: HTMLInputElement | undefined = $state();

	const soon = $derived(inv ? urgent(inv.lots) : []);
	const byPlace = $derived(
		Object.fromEntries(
			PLACES.map((p) => [p.id, inv?.lots.filter((l) => l.location === p.id) ?? []])
		)
	);

	async function load() {
		const r = await api.getInventory();
		if (r.status === 'ok') inv = r.data;
		else error = 'Could not read the inventory.';
	}

	async function add(e: SubmitEvent) {
		e.preventDefault();
		if (!item.trim()) return;
		const r = await api.addItem({
			item,
			qty,
			location: place,
			bestBefore: bb || null,
			store: shop || null
		});
		if (r.status === 'error') {
			error =
				r.error.kind === 'bad_input'
					? `Check the ${r.error.detail === 'bestBefore' ? 'date' : r.error.detail === 'qty' ? 'amount' : r.error.detail}.`
					: 'Could not add it.';
			return;
		}
		error = '';
		item = qty = bb = '';
		await load();
		itemInput?.focus();
	}

	async function act(lot: LotView, action: Parameters<typeof api.lotAction>[1]) {
		const r = await api.lotAction(lot.short, action);
		if (r.status === 'error') error = 'That did not work.';
		await load();
	}

	onMount(load);
</script>

<main class="mx-auto max-w-6xl px-5 pt-8 pb-16 sm:px-8">
	<header class="mb-6 flex items-baseline justify-between gap-4">
		<div>
			<h1 class="text-[26px] font-semibold tracking-tight">Nookery</h1>
			<p class="text-sm text-[var(--ink-2)]">
				{#if inv}
					{inv.lots.length} things in stock{soon.length ? ` · ${soon.length} to use first` : ''}
				{:else}
					&nbsp;
				{/if}
			</p>
		</div>
	</header>

	<form
		onsubmit={add}
		class="mb-8 grid grid-cols-2 gap-2 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-2 sm:grid-cols-[2fr_1fr_1fr_1fr_1fr_auto]"
	>
		<input
			bind:this={itemInput}
			bind:value={item}
			placeholder="Add something… (milk)"
			aria-label="Item"
			class="col-span-2 min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 placeholder:text-[var(--ink-3)] sm:col-span-1"
		/>
		<input
			bind:value={qty}
			placeholder="1, 500g, 1l"
			aria-label="Amount"
			class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 placeholder:text-[var(--ink-3)]"
		/>
		<select
			bind:value={place}
			aria-label="Where"
			class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2"
		>
			{#each PLACES as p (p.id)}
				<option value={p.id}>{p.label}</option>
			{/each}
		</select>
		<input
			bind:value={bb}
			placeholder="Best before"
			aria-label="Best before"
			class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 placeholder:text-[var(--ink-3)]"
		/>
		<input
			bind:value={shop}
			placeholder="Store"
			aria-label="Store"
			class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 placeholder:text-[var(--ink-3)]"
		/>
		<button
			class="rounded-xl bg-[var(--sage)] px-4 py-2 font-medium text-[var(--surface)] transition-opacity hover:opacity-90 active:opacity-80"
		>
			Add
		</button>
	</form>

	{#if error}
		<p role="alert" class="mb-6 text-sm text-[var(--terracotta)]">{error}</p>
	{/if}

	{#if soon.length}
		<section class="mb-10" aria-labelledby="soon-h">
			<h2
				id="soon-h"
				class="mb-3 text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
			>
				Use first
			</h2>
			<ul class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
				{#each soon as lot (lot.id)}
					{@const s = statusLabel(lot)}
					<li
						class="flex items-center justify-between gap-3 rounded-2xl border p-4"
						style:border-color={s.tone === 'expired' ? 'var(--terracotta)' : 'var(--line)'}
						style:background={s.tone === 'expired' ? 'var(--terracotta-bg)' : 'var(--saffron-bg)'}
					>
						<div class="min-w-0">
							<p class="truncate font-medium">{itemName(lot.item)}</p>
							<p
								class="text-sm"
								style:color={s.tone === 'expired' ? 'var(--terracotta)' : 'var(--saffron)'}
							>
								{s.text} · {lot.left} · {lot.location}
							</p>
						</div>
						<div class="flex shrink-0 gap-1">
							<button class="chip" onclick={() => act(lot, { type: 'use', qty: null })}>Used</button
							>
							{#if lot.location !== 'freezer'}
								<button class="chip" onclick={() => act(lot, { type: 'move', to: 'freezer' })}>
									Freeze
								</button>
							{/if}
							<button class="chip" onclick={() => act(lot, { type: 'bin' })}>Bin</button>
						</div>
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
		{#each PLACES as p (p.id)}
			{@const lots = byPlace[p.id]}
			<section aria-labelledby="{p.id}-h">
				<h2
					id="{p.id}-h"
					class="mb-3 flex items-baseline justify-between text-[13px] font-semibold tracking-wide text-[var(--ink-2)] uppercase"
				>
					{p.label}
					<span class="font-normal normal-case tabular-nums">{lots.length}</span>
				</h2>
				{#if lots.length === 0}
					<p
						class="rounded-2xl border border-dashed border-[var(--line)] p-4 text-sm text-[var(--ink-3)]"
					>
						Nothing in the {p.id} yet.
					</p>
				{:else}
					<ul
						class="divide-y divide-[var(--line)] rounded-2xl border border-[var(--line)] bg-[var(--surface)]"
					>
						{#each lots as lot (lot.id)}
							{@const s = statusLabel(lot)}
							<li class="group relative flex items-center gap-3 px-4 py-2.5">
								<div class="min-w-0 flex-1">
									<p class="truncate">
										{itemName(lot.item)}
										{#if lot.opened}<span class="ml-1 text-xs text-[var(--ink-3)]">opened</span
											>{/if}
									</p>
									<p
										class="text-xs"
										style:color={s.tone === 'expired'
											? 'var(--terracotta)'
											: s.tone === 'soon'
												? 'var(--saffron)'
												: 'var(--ink-3)'}
									>
										{#if lot.frozenOn}{frozenLabel(lot.frozenOn)}{:else}{s.text}{/if}{lot.store
											? ` · ${lot.store}`
											: ''}
									</p>
								</div>
								<span class="text-sm text-[var(--ink-2)] tabular-nums">{lot.left}</span>
								<!-- Overlaid on hover/focus so hidden buttons take no room from the name. -->
								<div
									class="pointer-events-none absolute inset-y-0 right-2 flex items-center gap-1 bg-[var(--surface)] pl-2 opacity-0 transition-opacity group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:opacity-100"
								>
									<button class="chip" onclick={() => act(lot, { type: 'use', qty: null })}>
										Used
									</button>
									{#if !lot.opened}
										<button class="chip" onclick={() => act(lot, { type: 'open' })}>Open</button>
									{/if}
									{#if lot.location === 'freezer'}
										<button class="chip" onclick={() => act(lot, { type: 'move', to: 'fridge' })}>
											Thaw
										</button>
									{:else}
										<button class="chip" onclick={() => act(lot, { type: 'move', to: 'freezer' })}>
											Freeze
										</button>
									{/if}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		{/each}
	</div>

	{#if inv}
		<footer class="mt-12 text-xs text-[var(--ink-3)] tabular-nums">
			{inv.events} events · opened and replayed in {inv.openMs?.toFixed(2)} ms · this view {(
				(inv.queryMs ?? 0) * 1000
			).toFixed(0)} µs
		</footer>
	{/if}
</main>

<style>
	.chip {
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.2rem 0.65rem;
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
</style>
