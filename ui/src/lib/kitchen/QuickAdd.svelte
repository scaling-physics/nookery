<script lang="ts">
	import { api } from '#lib/api.js';

	let { onadded, onerror }: { onadded: () => void; onerror: (msg: string) => void } = $props();

	const PLACES = ['fridge', 'freezer', 'pantry'];
	let item = $state('');
	let qty = $state('');
	let place = $state('fridge');
	let bb = $state('');
	let shop = $state('');
	let focused = $state(false);
	let form: HTMLFormElement | undefined = $state();
	let itemInput: HTMLInputElement | undefined = $state();
	const expanded = $derived(focused || !!(item || qty || bb || shop));

	// Collapsing while a click elsewhere is still in progress moves the list
	// under the pointer and the click is lost (e.g. ticking a row right after
	// typing). So when focus leaves during a press, collapse after it ends.
	let pressing = false;
	$effect(() => {
		const down = () => (pressing = true);
		const up = () => setTimeout(() => (pressing = false), 0);
		window.addEventListener('pointerdown', down, true);
		window.addEventListener('pointerup', up, true);
		window.addEventListener('pointercancel', up, true);
		return () => {
			window.removeEventListener('pointerdown', down, true);
			window.removeEventListener('pointerup', up, true);
			window.removeEventListener('pointercancel', up, true);
		};
	});

	function collapse() {
		if (!pressing) {
			focused = false;
			return;
		}
		const done = () =>
			setTimeout(() => (focused = form?.contains(document.activeElement) ?? false), 0);
		window.addEventListener('click', done, { once: true, capture: false });
		window.addEventListener('pointercancel', done, { once: true });
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
			onerror(
				r.error.kind === 'bad_input'
					? `Check the ${r.error.detail === 'bestBefore' ? 'date' : r.error.detail === 'qty' ? 'amount' : r.error.detail}.`
					: 'Could not add it.'
			);
			return;
		}
		onerror('');
		item = qty = bb = '';
		onadded();
		itemInput?.focus();
	}
</script>

<form
	bind:this={form}
	onsubmit={add}
	onfocusin={() => (focused = true)}
	onfocusout={(e) => {
		if (!form?.contains(e.relatedTarget as Node | null)) collapse();
	}}
	class="mb-6 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-1.5"
	aria-label="Add something"
>
	<div class="flex items-center gap-2">
		<input
			bind:this={itemInput}
			bind:value={item}
			placeholder="Add something…"
			aria-label="Item"
			autocomplete="off"
			class="min-w-0 flex-1 rounded-xl bg-transparent px-3 py-2 placeholder:text-[var(--ink-3)]"
		/>
		{#if expanded}
			<button
				disabled={!item.trim()}
				class="rounded-xl bg-[var(--sage)] px-4 py-2 text-sm font-medium text-[var(--surface)] hover:opacity-90 disabled:opacity-40"
			>
				Add
			</button>
		{/if}
	</div>
	{#if expanded}
		<div class="grid grid-cols-2 gap-2 p-1.5 pt-1 sm:grid-cols-4">
			<input
				bind:value={qty}
				placeholder="Amount (500g)"
				aria-label="Amount"
				class="field placeholder:text-[var(--ink-3)]"
			/>
			<select bind:value={place} aria-label="Where" class="field capitalize">
				{#each PLACES as p (p)}
					<option value={p}>{p}</option>
				{/each}
			</select>
			<input
				bind:value={bb}
				placeholder="Best before"
				aria-label="Best before"
				class="field placeholder:text-[var(--ink-3)]"
			/>
			<input
				bind:value={shop}
				placeholder="Store"
				aria-label="Store"
				class="field placeholder:text-[var(--ink-3)]"
			/>
		</div>
	{/if}
</form>

<style>
	.field {
		min-width: 0;
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
	}
</style>
