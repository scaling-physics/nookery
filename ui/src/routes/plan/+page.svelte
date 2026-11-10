<script lang="ts">
	import { MAC_UNREACHABLE_DEFAULT, PHONE_MODEL_MISSING } from '#lib/phone/messages.js';
	import { onPhone } from '#lib/phone/env.js';
	import { loadRanOn } from '#lib/phone/where.js';
	import { onDestroy, onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { AiModeView, CommandError, DayView, TemplateView } from '#lib/bindings.js';
	import { CLOUD_CREDITS, CLOUD_SIGN_IN, displayName } from '#lib/format.js';
	import CloudTag from '#lib/CloudTag.svelte';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import DishArt from '#lib/art/DishArt.svelte';
	import FoodIcon from '#lib/FoodIcon.svelte';
	import CookedSheet from '#lib/recipes/CookedSheet.svelte';
	import { ps } from '#lib/plan/state.svelte.js';
	import {
		cookedCount,
		dayHeading,
		daySuggestion,
		isoDate,
		planNeeds,
		shortDate,
		weekLabel
	} from '#lib/plan/helpers.js';

	let templates = $state<TemplateView[]>([]);
	let loaded = $state(false);
	let thinking = $state(false);
	let elapsed = $state(0);
	let timer: ReturnType<typeof setInterval> | undefined;
	let error = $state('');
	let planMode = $state<AiModeView>('local');
	let phoneRanOn = $state('on this phone');
	const ranOn = $derived(
		onPhone ? phoneRanOn : planMode === 'cloud' ? 'in the cloud' : 'on this Mac'
	);
	let showForm = $state(false);
	let notice = $state('');
	let added = $state<{ added: number; skipped: number } | null>(null);
	let adding = $state(false);
	let sheetOpen = $state(false);
	let sheetDay = $state<DayView | null>(null);
	let confirmDelete = $state(false);

	const today = isoDate(new Date());
	const plan = $derived(ps.plan);
	const formOpen = $derived(showForm || !plan);
	const needs = $derived(plan ? planNeeds(plan) : []);
	const soonTotal = $derived(plan ? plan.days.reduce((n, d) => n + d.usesSoon, 0) : 0);

	onDestroy(() => clearInterval(timer));

	function errorText(e: CommandError): string {
		switch (e.kind) {
			case 'cloud_sign_in':
				return CLOUD_SIGN_IN;
			case 'cloud_credits':
				return CLOUD_CREDITS;
			case 'phone_model_missing':
				return PHONE_MODEL_MISSING;
			case 'mac_unreachable':
				return e.detail || MAC_UNREACHABLE_DEFAULT;
			case 'recipe_model_unavailable':
				if (onPhone) return MAC_UNREACHABLE_DEFAULT;
				return `Nookery could not reach the local model at ${e.detail}. Start Ollama (open the Ollama app, or run “ollama serve”) and try again.`;
			case 'recipe_model_missing':
				return `The recipe model is not installed yet. Run “ollama pull ${e.detail}” in a terminal, then try again.`;
			case 'bad_input':
				return e.detail === 'stock'
					? 'There is nothing in stock to plan with yet. Add a few things in the Kitchen tab first.'
					: 'Could not read that request.';
			case 'not_found':
				return 'That is no longer there. Please reload the plan.';
			default:
				return 'Something went wrong. Please try again.';
		}
	}

	async function load() {
		const t = await api.listTemplates();
		if (t.status === 'ok') {
			templates = t.data;
			if (!templates.some((x) => x.id === ps.templateId)) ps.templateId = templates[0]?.id ?? null;
		}
		loaded = true;
		// Show the newest stored plan unless one is already on screen.
		if (!ps.plan) {
			const l = await api.listPlans();
			if (l.status === 'ok' && l.data.length) {
				const p = await api.getPlan(l.data[0].week);
				if (p.status === 'ok') ps.plan = p.data;
			}
		}
	}

	async function make(e?: SubmitEvent) {
		e?.preventDefault();
		if (thinking) return;
		error = '';
		notice = '';
		added = null;
		confirmDelete = false;
		elapsed = 0;
		thinking = true;
		const started = Date.now();
		timer = setInterval(() => (elapsed = Math.round((Date.now() - started) / 1000)), 500);
		const r = await api.makePlan({
			templateId: ps.templateId,
			note: ps.note,
			days: ps.days,
			start: null
		});
		clearInterval(timer);
		thinking = false;
		if (r.status === 'ok') {
			ps.plan = r.data.plan;
			ps.model = r.data.model;
			ps.usedCloud = r.data.usedCloud;
			ps.ms = r.data.ms ?? 0;
			showForm = false;
			if (r.data.plan.days.length === 0)
				error = 'The model came back with nothing usable. Try again, or change the note.';
		} else error = errorText(r.error);
	}

	async function swap(d: DayView) {
		if (!plan) return;
		error = '';
		const r = await api.swapDay({ week: plan.week, date: d.date });
		if (r.status === 'ok') {
			ps.plan = r.data;
			notice = `Swapped ${dayHeading(d.date, today)}.`;
		} else
			error =
				r.error.kind === 'not_found'
					? 'There is no other idea left to swap in. Plan again for new ones.'
					: errorText(r.error);
	}

	async function cooked(d: DayView) {
		if (!plan) return;
		error = '';
		const r = await api.setDayCooked({ week: plan.week, date: d.date, cooked: true });
		if (r.status !== 'ok') {
			error = errorText(r.error);
			return;
		}
		ps.plan = r.data;
		if (d.have.length) {
			sheetDay = d;
			sheetOpen = true;
		}
	}

	async function uncook(d: DayView) {
		if (!plan) return;
		const r = await api.setDayCooked({ week: plan.week, date: d.date, cooked: false });
		if (r.status === 'ok') ps.plan = r.data;
		else error = errorText(r.error);
	}

	async function afterSheet() {
		// The kitchen changed: reload the plan so used-up lots drop out.
		if (!plan) return;
		const r = await api.getPlan(plan.week);
		if (r.status === 'ok') ps.plan = r.data;
	}

	async function toShopping() {
		if (!plan || adding) return;
		adding = true;
		error = '';
		const r = await api.addPlanToShopping(plan.week);
		adding = false;
		if (r.status === 'ok') added = { added: r.data.added.length, skipped: r.data.skipped };
		else error = errorText(r.error);
	}

	async function remove() {
		if (!plan) return;
		const r = await api.deletePlan(plan.week);
		if (r.status === 'ok') {
			ps.plan = null;
			confirmDelete = false;
			added = null;
		} else error = errorText(r.error);
	}

	onMount(() => {
		load();
		if (onPhone) loadRanOn().then((v) => (phoneRanOn = v));
		api.getAiSettings().then((s) => {
			if (s.status === 'ok') planMode = s.data.recipes;
		});
	});
</script>

<main class="mx-auto max-w-3xl px-5 pt-6 pb-28 sm:px-8">
	<header class="mb-5 flex items-start justify-between gap-4">
		<div>
			<h1 class="text-[20px] font-semibold tracking-tight">Plan</h1>
			<p class="text-sm text-[var(--ink-2)]">
				Dinners for the week, using what needs eating first. Written {ranOn}.
			</p>
		</div>
		{#if plan && !thinking}
			<button class="chip shrink-0" onclick={() => (showForm = !showForm)} aria-expanded={formOpen}>
				{formOpen ? 'Hide' : 'Plan again'}
			</button>
		{/if}
	</header>

	{#if formOpen}
		<form
			onsubmit={make}
			class="mb-6 grid gap-4 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-4"
		>
			<fieldset disabled={thinking} class="grid gap-2">
				<legend class="mb-1 text-[13px] font-semibold text-[var(--ink-2)]"
					>What kind of week?</legend
				>
				<div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Template">
					{#each templates as t (t.id)}
						<label class="seg" class:on={ps.templateId === t.id} title={t.body}>
							<input
								type="radio"
								name="template"
								value={t.id}
								bind:group={ps.templateId}
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
						bind:value={ps.note}
						disabled={thinking}
						placeholder="e.g. no chilli, one fish dinner"
						class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-2 text-base font-normal text-[var(--ink)] placeholder:text-[var(--ink-3)]"
					/>
				</label>
				<div class="flex items-center gap-2">
					<span id="days-l" class="text-[13px] font-semibold text-[var(--ink-2)]">Dinners</span>
					<div class="stepper" role="group" aria-labelledby="days-l">
						<button
							type="button"
							aria-label="Fewer dinners"
							disabled={thinking || ps.days <= 3}
							onclick={() => (ps.days = Math.max(3, ps.days - 1))}>−</button
						>
						<output aria-live="polite" class="w-6 text-center tabular-nums">{ps.days}</output>
						<button
							type="button"
							aria-label="More dinners"
							disabled={thinking || ps.days >= 7}
							onclick={() => (ps.days = Math.min(7, ps.days + 1))}>+</button
						>
					</div>
				</div>
			</div>

			<div class="flex flex-wrap items-center gap-3">
				<button class="btn-primary" disabled={thinking || !loaded}>
					{thinking ? 'Planning…' : plan ? 'Plan my week again' : 'Plan my week'}
				</button>
				{#if thinking}
					<p role="status" aria-live="polite" class="text-sm text-[var(--ink-2)]">
						Planning {ranOn}… usually {onPhone && ranOn === 'on this phone'
							? '30 to 60'
							: '15 to 30'} s
						<span class="text-[var(--ink-3)] tabular-nums">{elapsed} s</span>
					</p>
				{:else if plan}
					<p class="text-sm text-[var(--ink-3)]">This replaces the plan for that week.</p>
				{/if}
			</div>
			{#if thinking}
				<progress class="bar" aria-label="Planning"></progress>
			{/if}
		</form>
	{/if}

	{#if error}
		<ErrorBanner message={error} class="mb-4" />
	{/if}

	{#if plan && !thinking}
		<section aria-labelledby="week-h">
			<div class="mb-3 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
				<h2 id="week-h" class="text-[17px] font-semibold tracking-tight">{weekLabel(plan.week)}</h2>
				<p class="text-sm text-[var(--ink-2)]">
					{plan.days.length}
					{plan.days.length === 1 ? 'dinner' : 'dinners'}
					{#if soonTotal}· uses {soonTotal} use-soon {soonTotal === 1 ? 'item' : 'items'}{/if}
					{#if cookedCount(plan)}· {cookedCount(plan)} cooked{/if}
				</p>
			</div>
			{#if ps.model}
				<p class="mb-3 text-xs text-[var(--ink-3)]">
					{Math.round(ps.ms / 1000)} s · {ps.model}<CloudTag show={ps.usedCloud} />
					{#if plan.template}· {plan.template}{/if}
				</p>
			{:else if plan.template}
				<p class="mb-3 text-xs text-[var(--ink-3)]">{plan.template}</p>
			{/if}

			<p role="status" aria-live="polite" class="sr-only">{notice}</p>
			<ul class="grid gap-3" aria-label="Dinners this week">
				{#each plan.days as d, i (d.date)}
					{@const categories = [...d.have.map((h) => h.lot.category), ...d.needCategories]}
					<li class="day" class:done={d.cooked} style="--i:{i}">
						<div class="flex gap-3">
							<div class="art" aria-hidden="true">
								<DishArt title={d.name} {categories} size={72} decorative />
							</div>
							<div class="min-w-0 flex-1">
								<p class="flex flex-wrap items-center gap-x-2 text-[13px] text-[var(--ink-2)]">
									<span class="font-semibold">{dayHeading(d.date, today)}</span>
									<span class="text-[var(--ink-3)]">{shortDate(d.date)}</span>
									{#if d.minutes}<span class="text-[var(--ink-3)] tabular-nums"
											>· {d.minutes} min</span
										>{/if}
								</p>
								<h3 class="text-[17px] leading-snug font-semibold tracking-tight">{d.name}</h3>
								{#if d.usesSoon > 0 && !d.cooked}
									<span class="soon-badge">Uses {d.usesSoon} soon</span>
								{/if}
								{#if d.cooked}<span class="cooked-badge">Cooked</span>{/if}
							</div>
						</div>
						{#if d.why}<p class="mt-2 text-[15px] leading-relaxed text-[var(--ink-2)]">
								{d.why}
							</p>{/if}

						{#if d.have.length}
							<p class="label">You have</p>
							<ul class="flex flex-wrap gap-1.5">
								{#each d.have as h (h.lot.id)}
									<li class="tag" class:soon={h.lot.status === 'use_soon'}>
										<FoodIcon category={h.lot.category} size={18} />{displayName(h.ingredient)}
									</li>
								{/each}
							</ul>
						{/if}
						{#if d.need.length}
							<p class="label">You would need</p>
							<ul class="flex flex-wrap gap-1.5">
								{#each d.need as n, k (k)}
									<li class="tag muted">
										<FoodIcon category={d.needCategories[k] ?? 'other'} size={18} />{displayName(n)}
									</li>
								{/each}
							</ul>
						{/if}

						<div class="mt-3 flex flex-wrap gap-2">
							{#if d.cooked}
								<button class="chip" onclick={() => uncook(d)}>Not cooked after all</button>
							{:else}
								<button class="btn-primary small" onclick={() => cooked(d)}>Cooked</button>
								<button class="chip" disabled={plan.spares.length === 0} onclick={() => swap(d)}>
									Swap
								</button>
							{/if}
						</div>
					</li>
				{/each}
			</ul>

			{#if plan.days.length}
				<div class="mt-6 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-4">
					<h2 class="font-medium">Shopping</h2>
					{#if added}
						<p role="status" class="mt-1 text-[var(--sage)]">
							{added.added === 0
								? 'Nothing to add: it is all on the list or in the kitchen.'
								: `Added ${added.added} ${added.added === 1 ? 'thing' : 'things'} to the shopping list.`}
						</p>
						<a href="/shopping" class="btn-primary mt-3 inline-block">Open the shopping list</a>
					{:else if needs.length === 0}
						<p class="mt-1 text-sm text-[var(--ink-2)]">
							Nothing to buy: everything is in your kitchen.
						</p>
					{:else}
						<p class="mt-1 text-sm text-[var(--ink-2)]">
							{needs.length}
							{needs.length === 1 ? 'thing is' : 'things are'} missing: {needs
								.slice(0, 6)
								.join(', ')}{needs.length > 6 ? '…' : ''}.
						</p>
						<button class="btn-primary mt-3" disabled={adding} onclick={toShopping}>
							{adding ? 'Adding…' : 'Add what’s missing to the shopping list'}
						</button>
					{/if}
				</div>
			{/if}

			<p class="mt-5 text-xs leading-relaxed text-[var(--ink-3)]">
				“Balanced” here means by food groups: a vegetable in at least half of the dinners, and no
				more than two with the same main protein. Nookery does not count calories or nutrients.
			</p>

			<div class="mt-4">
				{#if confirmDelete}
					<p class="text-sm text-[var(--ink-2)]">Delete this week’s plan?</p>
					<div class="mt-2 flex gap-2">
						<button class="chip danger" onclick={remove}>Delete plan</button>
						<button class="chip" onclick={() => (confirmDelete = false)}>Keep it</button>
					</div>
				{:else}
					<button class="chip" onclick={() => (confirmDelete = true)}>Delete this plan</button>
				{/if}
			</div>
		</section>
	{:else if !plan && !thinking && !error}
		<p class="text-sm text-[var(--ink-3)]">
			Pick a kind of week and press Plan my week. {onPhone || planMode === 'local'
				? onPhone
					? 'Nothing leaves your home.'
					: 'Nothing leaves this Mac.'
				: 'Cloud may be used (see Settings).'}
		</p>
	{/if}
</main>

{#if sheetDay}
	<CookedSheet bind:open={sheetOpen} suggestion={daySuggestion(sheetDay)} onDone={afterSheet} />
{/if}

<style>
	.chip {
		display: inline-flex;
		align-items: center;
		min-height: 2.5rem;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 0.25rem 0.9rem;
		font-size: 0.875rem;
		color: var(--ink-2);
		transition:
			background-color 120ms,
			color 120ms;
	}
	.chip:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}
	.chip:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.chip.danger {
		border-color: var(--terracotta);
		color: var(--terracotta);
	}
	.seg {
		cursor: pointer;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.45rem 0.95rem;
		font-size: 0.875rem;
		color: var(--ink-2);
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
		padding: 0.6rem 1.1rem;
		font-weight: 500;
		color: var(--surface);
		min-height: 2.75rem;
		transition: opacity 120ms;
	}
	.btn-primary.small {
		min-height: 2.5rem;
		padding: 0.4rem 1rem;
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
		width: 2.5rem;
		height: 2.5rem;
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
	.day {
		border-radius: 1rem;
		border: 1px solid var(--line);
		background: var(--surface);
		padding: 1rem 1.1rem;
		animation: settle 260ms ease-out both;
		animation-delay: calc(var(--i, 0) * 40ms);
	}
	.day.done {
		opacity: 0.7;
	}
	@keyframes settle {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.day {
			animation: none;
		}
	}
	.art {
		flex: none;
		width: 72px;
		height: 72px;
	}
	.soon-badge,
	.cooked-badge {
		display: inline-block;
		margin-top: 0.2rem;
		border-radius: 9999px;
		padding: 0.05rem 0.6rem;
		font-size: 0.75rem;
		font-weight: 600;
	}
	.soon-badge {
		background: var(--saffron-bg);
		color: var(--saffron);
	}
	.cooked-badge {
		background: var(--sage-bg);
		color: var(--sage);
	}
	.label {
		margin: 0.8rem 0 0.35rem;
		font-size: 0.75rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--ink-3);
	}
	.tag {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		border-radius: 9999px;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.15rem 0.7rem 0.15rem 0.3rem;
		font-size: 0.875rem;
		color: var(--ink);
	}
	.tag.soon {
		border-color: var(--saffron);
		background: var(--saffron-bg);
		color: var(--saffron);
		font-weight: 500;
	}
	.tag.muted {
		border-style: dashed;
		background: transparent;
		color: var(--ink-3);
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
