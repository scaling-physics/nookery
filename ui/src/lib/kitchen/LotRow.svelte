<script lang="ts">
	import type { LotAction, LotView } from '#lib/bindings.js';
	import { displayName, dotTone, lotMeta } from '#lib/format.js';
	import type { RemovalKind } from './pending.svelte.js';
	import FoodIcon from '#lib/FoodIcon.svelte';
	import Icon from './Icon.svelte';
	import LotMenu from './LotMenu.svelte';

	let {
		lot,
		selecting = false,
		selected = false,
		ontoggle,
		onact,
		onremove
	}: {
		lot: LotView;
		selecting?: boolean;
		selected?: boolean;
		ontoggle: () => void;
		onact: (lot: LotView, action: LotAction) => void;
		onremove: (lot: LotView, kind: RemovalKind) => void;
	} = $props();

	const tone = $derived(dotTone(lot));
	const meta = $derived(lotMeta(lot));
	const name = $derived(displayName(lot.item));
</script>

<div class="row" class:selected>
	{#if selecting}
		<input
			type="checkbox"
			class="check"
			checked={selected}
			onchange={ontoggle}
			aria-label="Select {name}"
		/>
	{/if}
	<span class="kind">
		<FoodIcon category={lot.category} size={20} />
		<span class="dot dot-{tone}" aria-hidden="true"></span>
	</span>
	<button
		type="button"
		class="text"
		tabindex={selecting ? 0 : -1}
		onclick={() => selecting && ontoggle()}
		aria-hidden={selecting ? undefined : true}
		disabled={!selecting}
	>
		<span class="name">{name}</span>
		{#if meta}<span class="meta" class:warn={tone === 'expired'}>{meta}</span>{/if}
	</button>
	<span class="qty">{lot.left}</span>
	{#if !selecting}
		<div class="slot">
			<button
				type="button"
				class="used"
				aria-label="Used up: {name}"
				onclick={() => onremove(lot, 'used')}
			>
				<Icon name="check" size={16} />
				<span class="label">Used up</span>
			</button>
		</div>
		<LotMenu {lot} onact={(a) => onact(lot, a)} onremove={(k) => onremove(lot, k)} />
	{/if}
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		min-height: 3rem;
		padding: 0.25rem 0.6rem 0.25rem 1rem;
		transition: background-color 120ms;
	}
	.row:hover {
		background: color-mix(in srgb, var(--surface-2) 55%, transparent);
	}
	.row.selected {
		background: var(--sage-bg);
	}
	.check {
		width: 1.1rem;
		height: 1.1rem;
		accent-color: var(--sage);
		flex: none;
	}
	.kind {
		position: relative;
		flex: none;
		display: grid;
		place-items: center;
		width: 1.75rem;
		height: 1.75rem;
		color: var(--ink-2);
	}
	.dot {
		position: absolute;
		right: -1px;
		bottom: -1px;
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 9999px;
		border: 2px solid var(--surface);
		box-sizing: content-box;
		transform: translate(15%, 15%);
	}
	.dot-expired {
		background: var(--terracotta);
	}
	.dot-soon {
		background: var(--saffron);
	}
	.dot-fresh {
		background: var(--sage);
	}
	.dot-muted {
		background: color-mix(in srgb, var(--ink-3) 55%, var(--surface));
	}
	.dot-ice {
		background: var(--ice);
	}
	.text {
		display: flex;
		min-width: 0;
		flex: 1;
		flex-direction: column;
		text-align: left;
		cursor: default;
	}
	.text:not(:disabled) {
		cursor: pointer;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.9375rem;
		line-height: 1.3;
	}
	.meta {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.75rem;
		line-height: 1.35;
		color: var(--ink-3);
	}
	.meta.warn {
		color: var(--terracotta);
	}
	.qty {
		flex: none;
		font-size: 0.875rem;
		color: var(--ink-2);
		font-variant-numeric: tabular-nums;
		min-width: 3.2rem;
		text-align: right;
	}
	/* Quiet by default: a ghost check. On a hovered or focused row (wide
	   screens) it becomes the sage pill with its label. */
	.slot {
		display: flex;
		flex: none;
		justify-content: flex-end;
	}
	.used {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.35rem;
		height: 2.5rem;
		min-width: 2.5rem;
		border-radius: 9999px;
		border: 1px solid transparent;
		color: var(--ink-3);
		font-size: 0.8125rem;
		font-weight: 500;
		transition:
			background-color 120ms,
			color 120ms,
			border-color 120ms;
	}
	.used:active {
		color: var(--sage);
		background: var(--sage-bg);
	}
	.label {
		display: none;
	}
	@media (min-width: 640px) {
		.slot {
			width: 6.25rem;
		}
		.used:hover,
		.used:focus-visible,
		.row:hover .used,
		.row:focus-within .used {
			padding: 0 0.8rem 0 0.65rem;
			color: var(--sage);
			background: var(--sage-bg);
			border-color: color-mix(in srgb, var(--sage) 45%, transparent);
		}
		.row:hover .label,
		.row:focus-within .label {
			display: inline;
		}
	}
	@media (max-width: 639px) {
		.row {
			gap: 0.55rem;
			padding: 0.15rem 0.35rem 0.15rem 0.85rem;
		}
	}
</style>
