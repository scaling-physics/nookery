<script lang="ts">
	import { api } from '#lib/api.js';
	import type { ReceiptLineView, ReceiptView } from '#lib/bindings.js';
	import { formatCents, formatDiff, parseFieldError } from '#lib/format.js';

	const STORES = ['REWE', 'EDEKA', 'Netto', "denn's"];
	const PLACES = [
		{ id: 'fridge', label: 'Fridge' },
		{ id: 'freezer', label: 'Freezer' },
		{ id: 'pantry', label: 'Pantry' }
	];

	type Row = { track: boolean; item: string; qty: string; location: string; bb: string };

	let text = $state('');
	let receipt = $state<ReceiptView | null>(null);
	let store = $state(''); // the chosen chain ('' = as detected)
	let rows = $state<Record<number, Row>>({});
	let error = $state('');
	let badRow = $state<{ line: number; field: string } | null>(null);
	let busy = $state(false);
	let done = $state<{ added: number; learned: number } | null>(null);

	const tracked = $derived(
		receipt ? receipt.lines.filter((l) => l.kind === 'item' && rows[l.index]?.track) : []
	);
	const diff = $derived(
		receipt?.totalCents != null ? receipt.linesCents - receipt.totalCents : null
	);
	const canImport = $derived(
		!!receipt &&
			receipt.totalOk &&
			!receipt.alreadyImported &&
			!!receipt.store &&
			tracked.length > 0 &&
			!busy
	);

	function describe(detail: string): string {
		const { field, line } = parseFieldError(detail);
		const where = line === null ? '' : ` on “${receipt?.lines[line]?.text ?? `line ${line}`}”`;
		switch (field) {
			case 'text':
				return 'No receipt lines found in that text.';
			case 'store':
				return 'Pick the store.';
			case 'total':
				return 'The lines do not add up to the total.';
			case 'qty':
				return `Check the amount${where}.`;
			case 'item':
				return `Give the item a name${where}.`;
			case 'location':
				return `Pick a place${where}.`;
			case 'bestBefore':
				return `Check the best-before date${where}.`;
			default:
				return 'Could not read that.';
		}
	}

	function initRows(r: ReceiptView) {
		const next: Record<number, Row> = {};
		for (const l of r.lines) {
			if (l.kind !== 'item') continue;
			const p = l.proposal;
			next[l.index] = {
				track: true,
				item: p?.item ?? l.text.toLowerCase(),
				qty: p?.qty ?? '1',
				location: p?.location ?? 'fridge',
				bb: ''
			};
		}
		rows = next;
	}

	async function parse() {
		error = '';
		badRow = null;
		busy = true;
		const r = await api.parseReceipt(text, store || null);
		busy = false;
		if (r.status === 'error') {
			error = r.error.kind === 'bad_input' ? describe(r.error.detail) : 'Could not read that.';
			return;
		}
		receipt = r.data;
		store = r.data.store ?? '';
		initRows(r.data);
	}

	async function loadFile(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		text = await file.text();
		await parse();
	}

	async function doImport() {
		if (!receipt || !canImport) return;
		error = '';
		badRow = null;
		busy = true;
		const r = await api.importReceipt({
			text,
			store: store || null,
			lines: tracked.map((l) => {
				const row = rows[l.index];
				return {
					line: l.index,
					item: row.item,
					qty: row.qty,
					location: row.location,
					bestBefore: row.bb.trim() || null
				};
			})
		});
		busy = false;
		if (r.status === 'ok') {
			done = r.data;
			return;
		}
		if (r.error.kind === 'already_imported') {
			receipt.alreadyImported = true;
		} else if (r.error.kind === 'bad_input') {
			const { field, line } = parseFieldError(r.error.detail);
			if (line !== null) badRow = { line, field };
			error = describe(r.error.detail);
		} else {
			error = 'Could not import it.';
		}
	}

	function reset() {
		receipt = null;
		done = null;
		text = '';
		store = '';
		error = '';
	}

	const bad = (l: ReceiptLineView, field: string) =>
		badRow?.line === l.index && badRow.field === field;
</script>

<main class="mx-auto max-w-6xl px-5 pt-8 pb-16 sm:px-8">
	<header class="mb-6 flex items-baseline justify-between gap-4">
		<div>
			<h1 class="text-[26px] font-semibold tracking-tight">Import a receipt</h1>
			<p class="text-sm text-[var(--ink-2)]">
				Check what you bought, say where it goes, and Nookery remembers for next time.
			</p>
		</div>
		<a href="/" class="chip shrink-0 whitespace-nowrap">Back to the kitchen</a>
	</header>

	{#if done}
		<section
			role="status"
			class="rounded-2xl border border-[var(--line)] bg-[var(--sage-bg)] p-5"
			aria-labelledby="done-h"
		>
			<h2 id="done-h" class="font-medium text-[var(--sage)]">
				Added {done.added}, learned {done.learned}
			</h2>
			<p class="mt-1 text-sm text-[var(--ink-2)]">
				{done.learned === 1 ? 'That line' : 'Those lines'} will need no questions next time.
			</p>
			<div class="mt-4 flex gap-2">
				<a href="/" class="btn-primary">See the kitchen</a>
				<button class="chip" onclick={reset}>Import another</button>
			</div>
		</section>
	{:else}
		<section class="mb-6 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-3">
			<label for="paste" class="mb-1.5 block text-[13px] font-semibold text-[var(--ink-2)]">
				Receipt text
			</label>
			<textarea
				id="paste"
				bind:value={text}
				rows={receipt ? 4 : 9}
				placeholder="Paste the text of a receipt here…"
				class="w-full resize-y rounded-xl bg-[var(--surface-2)] px-3 py-2 font-mono text-[13px] placeholder:text-[var(--ink-3)]"
			></textarea>
			<div class="mt-2 flex flex-wrap items-center gap-3">
				<button class="btn-primary" onclick={parse} disabled={busy || !text.trim()}>
					{receipt ? 'Read again' : 'Read receipt'}
				</button>
				<label
					class="chip cursor-pointer focus-within:outline-2 focus-within:outline-[var(--focus)]"
				>
					Open a .txt file
					<input type="file" accept=".txt,text/plain" class="sr-only" onchange={loadFile} />
				</label>
				<p class="text-xs text-[var(--ink-3)]">PDF and photos of receipts come later.</p>
			</div>
		</section>

		{#if error}
			<p role="alert" class="mb-4 text-sm text-[var(--terracotta)]">{error}</p>
		{/if}

		{#if receipt}
			<section aria-labelledby="review-h">
				<h2 id="review-h" class="sr-only">Review</h2>

				<div class="mb-4 flex flex-wrap items-end gap-4">
					<div>
						<label for="store" class="mb-1 block text-[13px] font-semibold text-[var(--ink-2)]">
							Store
						</label>
						<select
							id="store"
							bind:value={store}
							onchange={parse}
							class="rounded-xl bg-[var(--surface-2)] px-3 py-2"
						>
							{#if !receipt.store}<option value="">Which store?</option>{/if}
							{#each STORES as s (s)}
								<option value={s}>{s}</option>
							{/each}
						</select>
					</div>
					<p class="pb-2 text-sm text-[var(--ink-2)] tabular-nums">
						{receipt.date ?? 'No date found'}
					</p>
				</div>

				{#if receipt.alreadyImported}
					<p
						role="status"
						class="mb-4 rounded-2xl border border-[var(--saffron)] bg-[var(--saffron-bg)] p-3 text-sm text-[var(--saffron)]"
					>
						This receipt was already imported. Nothing more to do.
					</p>
				{/if}

				<p
					role="status"
					class="mb-4 rounded-2xl p-3 text-sm tabular-nums"
					style:background={receipt.totalOk ? 'var(--sage-bg)' : 'var(--terracotta-bg)'}
					style:color={receipt.totalOk ? 'var(--sage)' : 'var(--terracotta)'}
				>
					{#if receipt.totalOk}
						Lines add up to the total: {formatCents(receipt.linesCents)}
					{:else if receipt.totalCents == null}
						No total found on the receipt. Lines add up to {formatCents(receipt.linesCents)}.
					{:else}
						Lines add up to {formatCents(receipt.linesCents)}, the receipt says {formatCents(
							receipt.totalCents
						)} ({formatDiff(diff ?? 0)}). Some line is missing or misread, so importing is off.
					{/if}
				</p>

				<ul
					class="divide-y divide-[var(--line)] rounded-2xl border border-[var(--line)] bg-[var(--surface)]"
				>
					{#each receipt.lines as l (l.index)}
						{@const row = rows[l.index]}
						{#if l.kind === 'item' && row}
							<li
								class="grid grid-cols-2 items-center gap-2 px-4 py-3 lg:grid-cols-[auto_minmax(0,1.3fr)_minmax(0,1.2fr)_6rem_7rem_8rem]"
								style:opacity={row.track ? 1 : 0.55}
							>
								<div class="col-span-2 flex min-w-0 items-center gap-3 lg:contents">
									<input
										type="checkbox"
										bind:checked={row.track}
										aria-label="Track {l.text}"
										class="size-4 accent-[var(--sage)]"
									/>
									<div class="min-w-0 lg:order-none">
										<p class="truncate text-sm">
											{l.text}
											{#if l.proposal?.known}
												<span
													class="ml-1 rounded-full bg-[var(--sage-bg)] px-2 py-0.5 text-[11px] text-[var(--sage)]"
													>learned</span
												>
											{/if}
										</p>
										<p class="text-xs text-[var(--ink-3)] tabular-nums">
											{formatCents(l.proposal?.priceCents ?? l.amountCents)}{l.count > 1
												? ` · ${l.count} ×`
												: ''}{l.weight ? ` · ${l.weight}` : ''}
										</p>
									</div>
								</div>
								<input
									bind:value={row.item}
									aria-label="Item for {l.text}"
									aria-invalid={bad(l, 'item')}
									disabled={!row.track}
									class="col-span-2 min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5 lg:col-span-1"
								/>
								<input
									bind:value={row.qty}
									aria-label="Amount for {l.text}"
									aria-invalid={bad(l, 'qty')}
									disabled={!row.track}
									class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5"
								/>
								<select
									bind:value={row.location}
									aria-label="Where {l.text} goes"
									disabled={!row.track}
									class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5"
								>
									{#each PLACES as p (p.id)}
										<option value={p.id}>{p.label}</option>
									{/each}
								</select>
								<input
									bind:value={row.bb}
									placeholder="Best before"
									aria-label="Best before for {l.text}"
									aria-invalid={bad(l, 'bestBefore')}
									disabled={!row.track}
									class="col-span-2 min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5 placeholder:text-[var(--ink-3)] lg:col-span-1"
								/>
							</li>
						{:else}
							<li
								class="flex items-baseline justify-between gap-3 px-4 py-2 text-sm text-[var(--ink-3)]"
							>
								<span class="truncate">
									{l.text}
									<span class="ml-1 text-xs">
										{l.kind === 'deposit'
											? 'deposit'
											: l.kind === 'deposit_return'
												? 'deposit returned'
												: 'discount'}
									</span>
								</span>
								<span class="tabular-nums">{formatCents(l.amountCents)}</span>
							</li>
						{/if}
					{/each}
				</ul>

				<div class="mt-5 flex flex-wrap items-center gap-3">
					<button class="btn-primary" onclick={doImport} disabled={!canImport}>
						Import {tracked.length}
						{tracked.length === 1 ? 'item' : 'items'}
					</button>
					{#if !receipt.store}
						<p class="text-sm text-[var(--ink-2)]">Pick the store first.</p>
					{/if}
				</div>
			</section>
		{/if}
	{/if}
</main>

<style>
	.chip {
		display: inline-block;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.2rem 0.75rem;
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
	.btn-primary {
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
	input[aria-invalid='true'] {
		outline: 2px solid var(--terracotta);
	}
	input:disabled,
	select:disabled {
		cursor: not-allowed;
	}
</style>
