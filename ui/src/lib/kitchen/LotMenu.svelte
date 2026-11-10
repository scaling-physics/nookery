<script lang="ts">
	import { tick } from 'svelte';
	import type { LotAction, LotView } from '#lib/bindings.js';
	import type { RemovalKind } from './pending.svelte.js';
	import { displayName } from '#lib/format.js';
	import { pushBack } from '#lib/back.svelte.js';
	import Icon from './Icon.svelte';

	let {
		lot,
		onact,
		onremove
	}: {
		lot: LotView;
		onact: (action: LotAction) => void;
		onremove: (kind: RemovalKind) => void;
	} = $props();

	const PLACES = ['fridge', 'freezer', 'pantry'];

	let open = $state(false);
	let mode = $state<'menu' | 'some'>('menu');
	let amount = $state('');
	let flip = $state(false);
	let root: HTMLDivElement | undefined = $state();
	let trigger: HTMLButtonElement | undefined = $state();
	let menu: HTMLDivElement | undefined = $state();
	let amountInput: HTMLInputElement | undefined = $state();

	const moves = $derived(
		PLACES.filter((p) => p !== lot.location).map((p) => ({
			to: p,
			label: p === 'freezer' ? 'Freeze' : `Move to ${p}`
		}))
	);

	async function show() {
		open = true;
		mode = 'menu';
		amount = '';
		await tick();
		// Open upwards when there is no room below.
		if (trigger && menu) {
			const below = window.innerHeight - trigger.getBoundingClientRect().bottom;
			flip = below < menu.offsetHeight + 16;
		}
		items()[0]?.focus();
	}

	let offBack: (() => void) | undefined;
	$effect(() => {
		if (open) offBack = pushBack(() => hide(false));
		return () => offBack?.();
	});

	function hide(refocus = true) {
		open = false;
		if (refocus) trigger?.focus();
	}

	const items = () => (menu ? [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')] : []);

	function onkeydown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			hide();
		} else if (mode === 'menu' && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
			e.preventDefault();
			const list = items();
			const i = list.indexOf(document.activeElement as HTMLElement);
			const next =
				e.key === 'ArrowDown' ? (i + 1) % list.length : (i - 1 + list.length) % list.length;
			list[next]?.focus();
		} else if (mode === 'menu' && (e.key === 'Home' || e.key === 'End')) {
			e.preventDefault();
			const list = items();
			list[e.key === 'Home' ? 0 : list.length - 1]?.focus();
		} else if (e.key === 'Tab') {
			hide(false);
		}
	}

	function pick(fn: () => void) {
		hide(false);
		fn();
	}

	async function some() {
		mode = 'some';
		await tick();
		amountInput?.focus();
	}

	function submitSome(e: SubmitEvent) {
		e.preventDefault();
		const q = amount.trim();
		if (!q) return;
		pick(() => onact({ type: 'use', qty: q }));
	}
</script>

<svelte:window
	onpointerdown={(e) => {
		if (open && root && !root.contains(e.target as Node)) hide(false);
	}}
/>

<div class="relative" bind:this={root} {onkeydown} role="presentation">
	<button
		bind:this={trigger}
		type="button"
		class="more"
		aria-label="More actions for {displayName(lot.item)}"
		aria-haspopup="menu"
		aria-expanded={open}
		onclick={() => (open ? hide() : show())}
	>
		<Icon name="dots" size={18} />
	</button>
	{#if open}
		<div
			bind:this={menu}
			class="menu"
			class:flip
			role={mode === 'menu' ? 'menu' : 'group'}
			aria-label="Actions for {displayName(lot.item)}"
		>
			{#if mode === 'menu'}
				<button role="menuitem" type="button" onclick={some}>Used some…</button>
				{#if !lot.opened}
					<button role="menuitem" type="button" onclick={() => pick(() => onact({ type: 'open' }))}>
						Opened
					</button>
				{/if}
				{#each moves as m (m.to)}
					<button
						role="menuitem"
						type="button"
						class="capitalize-first"
						onclick={() => pick(() => onact({ type: 'move', to: m.to }))}
					>
						{m.label}
					</button>
				{/each}
				<button role="menuitem" type="button" onclick={() => pick(() => onact({ type: 'seen' }))}>
					Still there
				</button>
				<hr />
				<button role="menuitem" type="button" onclick={() => pick(() => onremove('binned'))}>
					Thrown away
				</button>
				<button
					role="menuitem"
					type="button"
					class="two-line"
					onclick={() => pick(() => onremove('wrong'))}
				>
					Wrong entry
					<small>Entered by mistake. Not counted as eaten or wasted.</small>
				</button>
			{:else}
				<form onsubmit={submitSome} class="some">
					<label for="some-{lot.id}">How much was used? (left: {lot.left})</label>
					<div>
						<input
							id="some-{lot.id}"
							bind:this={amountInput}
							bind:value={amount}
							placeholder="1, 200g, 0.5l"
							autocomplete="off"
						/>
						<button type="submit" class="go">Use</button>
					</div>
				</form>
			{/if}
		</div>
	{/if}
</div>

<style>
	.more {
		display: grid;
		place-items: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: 9999px;
		color: var(--ink-3);
		transition:
			background-color 120ms,
			color 120ms;
	}
	@media (max-width: 639px) {
		.more {
			width: 2.75rem;
			height: 2.75rem;
		}
	}
	.more:hover,
	.more[aria-expanded='true'] {
		background: var(--surface-2);
		color: var(--ink);
	}
	.menu {
		position: absolute;
		right: 0;
		top: calc(100% + 4px);
		z-index: 40;
		min-width: 12.5rem;
		max-width: 17rem;
		padding: 0.3rem;
		border-radius: 0.9rem;
		border: 1px solid var(--line);
		background: var(--surface);
		box-shadow: 0 10px 30px -8px color-mix(in srgb, var(--ink) 30%, transparent);
	}
	.menu.flip {
		top: auto;
		bottom: calc(100% + 4px);
	}
	.menu button[role='menuitem'] {
		display: block;
		width: 100%;
		text-align: left;
		padding: 0.5rem 0.7rem;
		border-radius: 0.6rem;
		font-size: 0.875rem;
		color: var(--ink);
	}
	.menu button[role='menuitem']:hover,
	.menu button[role='menuitem']:focus-visible {
		background: var(--surface-2);
		outline: none;
	}
	.capitalize-first::first-letter {
		text-transform: uppercase;
	}
	.two-line small {
		display: block;
		margin-top: 1px;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--ink-3);
	}
	hr {
		margin: 0.25rem 0.4rem;
		border-color: var(--line);
	}
	.some {
		padding: 0.5rem 0.6rem 0.6rem;
	}
	.some label {
		display: block;
		margin-bottom: 0.4rem;
		font-size: 0.8125rem;
		color: var(--ink-2);
	}
	.some div {
		display: flex;
		gap: 0.4rem;
	}
	.some input {
		min-width: 0;
		flex: 1;
		border-radius: 0.6rem;
		background: var(--surface-2);
		padding: 0.4rem 0.6rem;
		font-size: 0.875rem;
	}
	.go {
		border-radius: 0.6rem;
		background: var(--sage);
		color: var(--surface);
		padding: 0.4rem 0.8rem;
		font-size: 0.875rem;
		font-weight: 500;
	}
</style>
