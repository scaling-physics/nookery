<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { InventoryView, LotAction, LotView } from '#lib/bindings.js';
	import { matchesQuery, urgent } from '#lib/format.js';
	import Icon from '#lib/kitchen/Icon.svelte';
	import PlaceGroup from '#lib/kitchen/PlaceGroup.svelte';
	import QuickAdd from '#lib/kitchen/QuickAdd.svelte';
	import SelectBar from '#lib/kitchen/SelectBar.svelte';
	import UndoToast from '#lib/kitchen/UndoToast.svelte';
	import UseFirst from '#lib/kitchen/UseFirst.svelte';
	import { PendingRemovals, type RemovalKind } from '#lib/kitchen/pending.svelte.js';

	const PLACES = [
		{ id: 'fridge', label: 'Fridge', empty: 'Nothing in the fridge.' },
		{ id: 'freezer', label: 'Freezer', empty: 'Nothing in the freezer.' },
		{ id: 'pantry', label: 'Pantry', empty: 'Nothing in the pantry.' }
	];

	let inv = $state<InventoryView | null>(null);
	let error = $state('');
	let query = $state('');
	let filter = $state('all');
	let selecting = $state(false);
	let selected = $state<string[]>([]);
	let search: HTMLInputElement | undefined = $state();

	async function load() {
		const r = await api.getInventory();
		if (r.status === 'ok') inv = r.data;
		else error = 'Could not read the inventory.';
	}

	const pending = new PendingRemovals({
		write: async (short, action) => {
			const r = await api.lotAction(short, action);
			if (r.status === 'error') throw new Error(r.error.kind);
		},
		onSettled: load,
		onError: () => (error = 'Some changes could not be saved. They are back in the list.')
	});

	// What is shown: not pending removal, and matching the search.
	const visible = $derived(inv ? inv.lots.filter((l) => !pending.hiddenIds.has(l.id)) : []);
	const found = $derived(visible.filter((l) => matchesQuery(l, query)));
	const soon = $derived(query.trim() ? [] : urgent(visible));
	const byPlace = $derived(
		Object.fromEntries(PLACES.map((p) => [p.id, found.filter((l) => l.location === p.id)]))
	);
	const shown = $derived(PLACES.filter((p) => filter === 'all' || filter === p.id));
	const none = $derived(query.trim() !== '' && found.length === 0);

	function removeLots(lots: LotView[], kind: RemovalKind) {
		pending.add(lots, kind);
		selected = selected.filter((id) => !lots.some((l) => l.id === id));
	}

	async function act(lot: LotView, action: LotAction) {
		const r = await api.lotAction(lot.short, action);
		error = r.status === 'error' ? 'That did not work.' : '';
		await load();
	}

	function toggle(lot: LotView) {
		selected = selected.includes(lot.id)
			? selected.filter((id) => id !== lot.id)
			: [...selected, lot.id];
	}

	function selectAll(lots: LotView[]) {
		const all = lots.every((l) => selected.includes(l.id));
		const ids = new Set(lots.map((l) => l.id));
		selected = all ? selected.filter((id) => !ids.has(id)) : [...new Set([...selected, ...ids])];
	}

	function removeSelected(kind: RemovalKind) {
		removeLots(
			visible.filter((l) => selected.includes(l.id)),
			kind
		);
		selecting = false;
	}

	function endSelect() {
		selecting = false;
		selected = [];
	}

	function onkeydown(e: KeyboardEvent) {
		const t = e.target as HTMLElement | null;
		if (e.key === '/' && !e.metaKey && !e.ctrlKey && !e.altKey) {
			if (t && (t.matches('input, textarea, select') || t.isContentEditable)) return;
			e.preventDefault();
			search?.focus();
		}
	}

	let detach = () => {};
	onMount(() => {
		void load();
		detach = pending.attach();
	});
	onDestroy(() => {
		detach();
		void pending.flush();
	});
</script>

<svelte:window {onkeydown} />

<main class="mx-auto max-w-6xl px-5 pt-6 pb-24 sm:px-8">
	<header class="mb-5 flex flex-wrap items-end justify-between gap-x-4 gap-y-3">
		<div>
			<h1 class="text-[20px] font-semibold tracking-tight">Kitchen</h1>
			<p class="text-sm text-[var(--ink-2)]">
				{#if inv}
					{visible.length} things in stock{urgent(visible).length
						? ` · ${urgent(visible).length} to use first`
						: ''}
				{:else}
					&nbsp;
				{/if}
			</p>
		</div>
		<nav class="flex gap-2" aria-label="Add stock">
			<a href="/photo" class="pill"><Icon name="camera" size={15} /> Scan a photo</a>
			<a href="/receipt" class="pill"><Icon name="receipt" size={15} /> Import receipt</a>
		</nav>
	</header>

	<QuickAdd onadded={load} onerror={(m) => (error = m)} />

	{#if error}
		<p role="alert" class="mb-4 text-sm text-[var(--terracotta)]">{error}</p>
	{/if}

	<UseFirst lots={soon} onremove={(l, k) => removeLots([l], k)} />

	<div class="mb-4 flex flex-wrap items-center gap-x-3 gap-y-2.5">
		<label class="search">
			<span class="sr-only">Search the kitchen</span>
			<Icon name="search" size={16} />
			<input
				bind:this={search}
				bind:value={query}
				type="search"
				placeholder="Search   /"
				autocomplete="off"
				onkeydown={(e) => e.key === 'Escape' && ((query = ''), search?.blur())}
			/>
		</label>
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<div class="chips" role="group" aria-label="Show">
				<button
					type="button"
					class="fchip"
					class:on={filter === 'all'}
					aria-pressed={filter === 'all'}
					onclick={() => (filter = 'all')}
				>
					All <span>{found.length}</span>
				</button>
				{#each PLACES as p (p.id)}
					<button
						type="button"
						class="fchip"
						class:on={filter === p.id}
						aria-pressed={filter === p.id}
						onclick={() => (filter = p.id)}
					>
						{p.label} <span>{byPlace[p.id].length}</span>
					</button>
				{/each}
			</div>
			<button
				type="button"
				class="pill ml-auto flex-none"
				class:active={selecting}
				aria-pressed={selecting}
				onclick={() => (selecting ? endSelect() : (selecting = true))}
			>
				{selecting ? 'Cancel' : 'Select'}
			</button>
		</div>
	</div>

	{#if !inv}
		<p class="text-sm text-[var(--ink-3)]">Loading…</p>
	{:else if none}
		<p
			class="rounded-2xl border border-dashed border-[var(--line)] p-6 text-center text-sm text-[var(--ink-3)]"
		>
			Nothing matches “{query.trim()}”.
		</p>
	{:else}
		{#snippet group(p: (typeof PLACES)[number])}
			<PlaceGroup
				id={p.id}
				label={p.label}
				lots={byPlace[p.id]}
				emptyText={query.trim() ? `No match in the ${p.id}.` : p.empty}
				{selecting}
				{selected}
				ontoggle={toggle}
				onselectall={selectAll}
				onact={act}
				onremove={(l, k) => removeLots([l], k)}
			/>
		{/snippet}
		{#if filter === 'all'}
			<!-- Fridge on the left; freezer and pantry stacked on the right. -->
			<div class="grid items-start gap-5 lg:grid-cols-2">
				{@render group(PLACES[0])}
				<div class="flex flex-col gap-5">
					{@render group(PLACES[1])}
					{@render group(PLACES[2])}
				</div>
			</div>
		{:else}
			<div class="mx-auto max-w-2xl">
				{#each shown as p (p.id)}
					{@render group(p)}
				{/each}
			</div>
		{/if}
	{/if}

	{#if inv}
		<footer class="mt-12 text-xs text-[var(--ink-3)] tabular-nums">
			{inv.events} events · opened and replayed in {inv.openMs?.toFixed(2)} ms · this view {(
				(inv.queryMs ?? 0) * 1000
			).toFixed(0)} µs
		</footer>
	{/if}
</main>

{#if selecting}
	<SelectBar count={selected.length} onremove={removeSelected} oncancel={endSelect} />
{/if}
<UndoToast message={pending.message} onundo={() => pending.undo()} raised={selecting} />

<style>
	.pill {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.4rem 0.9rem;
		font-size: 0.875rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.pill:hover {
		background: var(--surface-2);
		color: var(--ink);
	}
	.pill.active {
		background: var(--sage-bg);
		border-color: var(--sage);
		color: var(--sage);
	}
	.search {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		max-width: 100%;
		border-radius: 9999px;
		background: var(--surface);
		border: 1px solid var(--line);
		padding: 0.4rem 0.9rem;
		color: var(--ink-3);
	}
	@media (min-width: 640px) {
		.search {
			width: 16rem;
		}
	}
	.search:focus-within {
		border-color: var(--sage);
	}
	.search input {
		min-width: 0;
		flex: 1;
		background: none;
		font-size: 0.875rem;
		color: var(--ink);
		outline: none;
	}
	.search input::placeholder {
		color: var(--ink-3);
	}
	.chips {
		display: flex;
		gap: 0.35rem;
		overflow-x: auto;
		max-width: 100%;
		padding: 0.1rem;
	}
	.fchip {
		flex: none;
		border-radius: 9999px;
		padding: 0.35rem 0.8rem;
		font-size: 0.875rem;
		color: var(--ink-2);
		transition: background-color 120ms;
	}
	.fchip span {
		margin-left: 0.2rem;
		color: var(--ink-3);
		font-variant-numeric: tabular-nums;
	}
	.fchip:hover {
		background: var(--surface-2);
	}
	.fchip.on {
		background: var(--sage);
		color: var(--surface);
	}
	.fchip.on span {
		color: inherit;
		opacity: 0.8;
	}
</style>
