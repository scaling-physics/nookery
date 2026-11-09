<script lang="ts">
	import { api, progressChannel } from '#lib/api.js';
	import type { AiModeView, CommandError, LotView, PhotoReviewView } from '#lib/bindings.js';
	import {
		CLOUD_CREDITS,
		CLOUD_SIGN_IN,
		itemName,
		parseFieldError,
		statusLabel
	} from '#lib/format.js';
	import ErrorBanner from '#lib/ErrorBanner.svelte';
	import CloudTag from '#lib/CloudTag.svelte';
	import type { Tone } from '#lib/format.js';
	import { onDestroy, onMount } from 'svelte';

	const PLACES = [
		{ id: 'fridge', label: 'Fridge' },
		{ id: 'freezer', label: 'Freezer' },
		{ id: 'pantry', label: 'Pantry' }
	];
	const TONE: Record<Tone, string> = {
		expired: 'var(--terracotta)',
		soon: 'var(--saffron)',
		fresh: 'var(--sage)',
		muted: 'var(--ink-3)'
	};

	type Choice = 'seen' | 'add' | 'ignore';
	type Row = { choice: Choice; item: string; qty: string; bb: string };

	let place = $state(''); // starts unselected: the user must say where
	let preview = $state(''); // data URL for the thumbnail
	let base64 = $state('');
	let fileName = $state('');
	let phase = $state<'pick' | 'scanning' | 'review' | 'done'>('pick');
	let progress = $state({ done: 0, total: 0 });
	let elapsed = $state(0);
	let timer: ReturnType<typeof setInterval> | undefined;
	let review = $state<PhotoReviewView | null>(null);
	let rows = $state<Record<number, Row>>({});
	let error = $state('');
	let photosMode = $state<AiModeView>('local');
	let badRow = $state<{ index: number; field: string } | null>(null);
	let busy = $state(false);
	let done = $state<{ seen: number; added: number; learned: number } | null>(null);
	let lotNote = $state<Record<string, string>>({});

	const matched = $derived(review?.items.filter((i) => i.matches.length > 0) ?? []);
	const fresh = $derived(review?.items.filter((i) => i.matches.length === 0) ?? []);
	const nSeen = $derived(matched.filter((i) => rows[i.index]?.choice === 'seen').length);
	const nAdd = $derived(fresh.filter((i) => rows[i.index]?.choice === 'add').length);

	onDestroy(() => clearInterval(timer));
	onMount(async () => {
		const r = await api.getAiSettings();
		if (r.status === 'ok') photosMode = r.data.photos;
	});

	function errorText(e: CommandError): string {
		switch (e.kind) {
			case 'vision_unavailable':
				return `Nookery could not reach the local vision model at ${e.detail}. Start Ollama (open the Ollama app, or run “ollama serve”) and try again.`;
			case 'vision_model_missing':
				return `The vision model is not installed yet. Run “ollama pull ${e.detail}” in a terminal, then try again.`;
			case 'cloud_sign_in':
				return CLOUD_SIGN_IN;
			case 'cloud_credits':
				return CLOUD_CREDITS;
			case 'not_found':
				return 'That scan is no longer available. Scan the photo again.';
			case 'bad_input': {
				const { field, line } = parseFieldError(e.detail);
				const name = line !== null ? (review?.items[line]?.item ?? `item ${line + 1}`) : '';
				switch (field) {
					case 'image':
						return 'That file could not be read as a photo. Try a JPEG or PNG.';
					case 'qty':
						return `Check the amount for “${name}”.`;
					case 'item':
						return `Give “${name}” a name.`;
					case 'bestBefore':
						return `Check the best-before date for “${name}”.`;
					default:
						return 'Could not read that.';
				}
			}
			default:
				return 'Something went wrong. Please try again.';
		}
	}

	function onFile(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		fileName = file.name;
		const reader = new FileReader();
		reader.onload = () => {
			const url = String(reader.result);
			preview = url;
			base64 = url.slice(url.indexOf(',') + 1);
		};
		reader.readAsDataURL(file);
	}

	function initRows(r: PhotoReviewView) {
		const next: Record<number, Row> = {};
		for (const i of r.items) {
			next[i.index] = {
				choice: i.matches.length ? 'seen' : 'ignore',
				item: i.item,
				qty: String(i.count),
				bb: ''
			};
		}
		rows = next;
	}

	async function scan() {
		if (!base64 || !place || phase === 'scanning') return;
		error = '';
		badRow = null;
		progress = { done: 0, total: 0 };
		elapsed = 0;
		phase = 'scanning';
		const started = Date.now();
		timer = setInterval(() => (elapsed = Math.round((Date.now() - started) / 1000)), 500);
		const r = await api.scanPhoto(
			base64,
			place,
			progressChannel((p) => (progress = p))
		);
		clearInterval(timer);
		if (r.status === 'error') {
			error = errorText(r.error);
			phase = 'pick';
			return;
		}
		review = r.data;
		initRows(r.data);
		phase = 'review';
	}

	async function apply() {
		if (!review || busy) return;
		error = '';
		badRow = null;
		busy = true;
		const r = await api.applyPhoto({
			scanId: review.scanId,
			decisions: review.items.map((i) => {
				const row = rows[i.index];
				const add = row.choice === 'add';
				return {
					index: i.index,
					action: row.choice,
					item: add ? row.item : null,
					qty: add ? row.qty : null,
					bestBefore: add ? row.bb.trim() || null : null
				};
			})
		});
		busy = false;
		if (r.status === 'ok') {
			done = r.data;
			phase = 'done';
			return;
		}
		if (r.error.kind === 'bad_input') {
			const { field, line } = parseFieldError(r.error.detail);
			if (line !== null) badRow = { index: line, field };
		}
		error = errorText(r.error);
	}

	async function quick(lot: LotView, action: 'use' | 'seen') {
		const r = await api.lotAction(
			lot.short,
			action === 'use' ? { type: 'use', qty: null } : { type: 'seen' }
		);
		lotNote[lot.id] =
			r.status === 'ok'
				? action === 'use'
					? 'Marked used up'
					: 'Marked still there'
				: 'That did not work';
	}

	function reset() {
		phase = 'pick';
		review = null;
		done = null;
		preview = '';
		base64 = '';
		place = '';
		fileName = '';
		error = '';
	}

	const bad = (index: number, field: string) => badRow?.index === index && badRow.field === field;
</script>

<main class="mx-auto max-w-4xl px-5 pt-8 pb-16 sm:px-8">
	<header class="mb-6 flex items-baseline justify-between gap-4">
		<div>
			<h1 class="text-[26px] font-semibold tracking-tight">Scan a photo</h1>
			<p class="text-sm text-[var(--ink-2)]">
				A photo of your fridge, freezer or pantry.
				{#if photosMode === 'local'}Nothing leaves this Mac.{:else}Cloud may be used: the photo is
					sent to Ollama's servers. <a href="/settings" class="underline underline-offset-2"
						>Settings</a
					>{/if}
			</p>
		</div>
		<a href="/" class="chip shrink-0 whitespace-nowrap">Back to the kitchen</a>
	</header>

	{#if error}
		<ErrorBanner message={error} class="mb-4" />
	{/if}

	{#if phase === 'done' && done}
		<section
			role="status"
			class="rounded-2xl border border-[var(--line)] bg-[var(--sage-bg)] p-5"
			aria-labelledby="done-h"
		>
			<h2 id="done-h" class="font-medium text-[var(--sage)]">
				Marked {done.seen} seen, added {done.added}, learned {done.learned}
			</h2>
			<p class="mt-1 text-sm text-[var(--ink-2)]">
				Nothing was removed. Learned names are used the next time you scan.
			</p>
			<div class="mt-4 flex gap-2">
				<a href="/" class="btn-primary">See the kitchen</a>
				<button class="chip" onclick={reset}>Scan another</button>
			</div>
		</section>
	{:else if phase === 'pick' || phase === 'scanning'}
		<section
			class="rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-4"
			aria-labelledby="pick-h"
		>
			<h2 id="pick-h" class="sr-only">Choose a photo</h2>
			<fieldset class="mb-4" disabled={phase === 'scanning'}>
				<legend class="mb-1.5 text-[13px] font-semibold text-[var(--ink-2)]">Where is it?</legend>
				{#if !place}
					<p class="mb-2 text-xs text-[var(--ink-3)]" id="place-hint">
						Choose where the photo was taken, so what you see lands in the right place.
					</p>
				{/if}
				<div class="flex gap-2" aria-describedby={place ? undefined : 'place-hint'}>
					{#each PLACES as p (p.id)}
						<label class="seg" class:on={place === p.id}>
							<input type="radio" name="place" value={p.id} bind:group={place} class="sr-only" />
							{p.label}
						</label>
					{/each}
				</div>
			</fieldset>

			<div class="flex flex-wrap items-center gap-4">
				<label
					class="chip cursor-pointer focus-within:outline-2 focus-within:outline-[var(--focus)]"
					class:disabled={phase === 'scanning'}
				>
					{preview ? 'Choose another photo' : 'Choose a photo'}
					<input
						type="file"
						accept="image/*"
						class="sr-only"
						onchange={onFile}
						disabled={phase === 'scanning'}
					/>
				</label>
				{#if preview}
					<img
						src={preview}
						alt="Your photo: {fileName}"
						class="h-24 w-24 rounded-xl border border-[var(--line)] object-cover"
					/>
				{/if}
			</div>

			<div class="mt-4">
				<button
					class="btn-primary"
					onclick={scan}
					disabled={!base64 || !place || phase === 'scanning'}
				>
					{phase === 'scanning' ? 'Scanning…' : 'Scan'}
				</button>
			</div>

			{#if phase === 'scanning'}
				<div role="status" class="mt-4" aria-live="polite">
					<p class="text-sm">
						{#if progress.total > 0}
							Looking at part {Math.min(progress.done + 1, progress.total)} of {progress.total}…
						{:else}
							Getting the photo ready…
						{/if}
						<span class="text-[var(--ink-3)] tabular-nums"> {elapsed} s</span>
					</p>
					<progress
						class="bar mt-2"
						max={progress.total || 1}
						value={progress.done}
						aria-label="Scan progress"
					></progress>
					<p class="mt-2 text-xs text-[var(--ink-3)]">
						This {photosMode === 'cloud' ? 'runs in the cloud and' : 'runs on this Mac and'} can take
						up to a minute. Please keep this page open.
					</p>
				</div>
			{/if}
		</section>
	{:else if review}
		<h2 class="text-[20px] font-semibold tracking-tight">Scanned your {review.location}</h2>
		<p class="mt-0.5 mb-4 text-sm text-[var(--ink-2)]">
			{Math.round((review.totalMs ?? 0) / 1000)} s · {review.model}<CloudTag
				show={review.usedCloud}
			/>. New items will be added to the {review.location}.
		</p>

		<section class="mb-8" aria-labelledby="here-h">
			<h2 id="here-h" class="mb-2 text-[15px] font-semibold">Already here ({matched.length})</h2>
			{#if matched.length === 0}
				<p class="text-sm text-[var(--ink-3)]">Nothing in the photo matched what you have.</p>
			{:else}
				<ul
					class="divide-y divide-[var(--line)] rounded-2xl border border-[var(--line)] bg-[var(--surface)]"
				>
					{#each matched as it (it.index)}
						{@const row = rows[it.index]}
						<li class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-3">
							<div class="min-w-0">
								<p class="text-sm">Seen: {it.seenAs}{it.count > 1 ? ` × ${it.count}` : ''}</p>
								{#each it.matches as m (m.id)}
									{@const s = statusLabel(m)}
									<p class="text-xs text-[var(--ink-2)]">
										{itemName(m.item)} · {m.left}
										{#if s.text}· <span style:color={TONE[s.tone]}>{s.text}</span>{/if}
									</p>
								{/each}
							</div>
							<div class="flex gap-1.5" role="group" aria-label="What to do with {it.seenAs}">
								<button
									class="chip"
									class:on={row.choice === 'seen'}
									aria-pressed={row.choice === 'seen'}
									onclick={() => (row.choice = 'seen')}>Seen</button
								>
								<button
									class="chip"
									class:on={row.choice === 'ignore'}
									aria-pressed={row.choice === 'ignore'}
									onclick={() => (row.choice = 'ignore')}>Ignore</button
								>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<section class="mb-8" aria-labelledby="new-h">
			<h2 id="new-h" class="mb-1 text-[15px] font-semibold">New? ({fresh.length})</h2>
			<p class="mb-2 text-sm text-[var(--ink-2)]">
				Photo guesses are noisy, so nothing is added unless you say so.
			</p>
			{#if fresh.length === 0}
				<p class="text-sm text-[var(--ink-3)]">Nothing new spotted.</p>
			{:else}
				<ul
					class="divide-y divide-[var(--line)] rounded-2xl border border-[var(--line)] bg-[var(--surface)]"
				>
					{#each fresh as it (it.index)}
						{@const row = rows[it.index]}
						<li class="px-4 py-3">
							<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
								<div class="min-w-0">
									<p class="text-sm">
										{it.seenAs}{it.count > 1 ? ` × ${it.count}` : ''}
										{#if it.learned}
											<span
												class="ml-1 rounded-full bg-[var(--sage-bg)] px-2 py-0.5 text-[11px] text-[var(--sage)]"
												>learned: {it.item}</span
											>
										{/if}
									</p>
									{#if it.text}<p class="text-xs text-[var(--ink-3)]">Label: {it.text}</p>{/if}
								</div>
								<div class="flex gap-1.5" role="group" aria-label="What to do with {it.seenAs}">
									<button
										class="chip"
										class:on={row.choice === 'add'}
										aria-pressed={row.choice === 'add'}
										onclick={() => (row.choice = 'add')}>Add</button
									>
									<button
										class="chip"
										class:on={row.choice === 'ignore'}
										aria-pressed={row.choice === 'ignore'}
										onclick={() => (row.choice = 'ignore')}>Ignore</button
									>
								</div>
							</div>
							{#if row.choice === 'add'}
								<div class="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-[2fr_1fr_1fr]">
									<input
										bind:value={row.item}
										aria-label="Item name for {it.seenAs}"
										aria-invalid={bad(it.index, 'item')}
										class="col-span-2 min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5 sm:col-span-1"
									/>
									<input
										bind:value={row.qty}
										aria-label="Amount for {it.seenAs}"
										aria-invalid={bad(it.index, 'qty')}
										class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5"
									/>
									<input
										bind:value={row.bb}
										placeholder="Best before"
										aria-label="Best before for {it.seenAs}"
										aria-invalid={bad(it.index, 'bestBefore')}
										class="min-w-0 rounded-xl bg-[var(--surface-2)] px-3 py-1.5 placeholder:text-[var(--ink-3)]"
									/>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<section class="mb-8" aria-labelledby="gone-h">
			<h2 id="gone-h" class="mb-1 text-[15px] font-semibold">
				Not in the photo ({review.missing.length})
			</h2>
			<p class="mb-2 text-sm text-[var(--ink-2)]">
				Nothing is removed. They may be hidden behind something, so you decide.
			</p>
			{#if review.missing.length === 0}
				<p class="text-sm text-[var(--ink-3)]">Everything you have here was spotted.</p>
			{:else}
				<ul
					class="divide-y divide-[var(--line)] rounded-2xl border border-[var(--line)] bg-[var(--surface)]"
				>
					{#each review.missing as m (m.id)}
						{@const s = statusLabel(m)}
						<li class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-3">
							<p class="text-sm">
								{itemName(m.item)} · {m.left}
								{#if s.text}<span class="text-xs" style:color={TONE[s.tone]}> · {s.text}</span>{/if}
							</p>
							{#if lotNote[m.id]}
								<p class="text-xs text-[var(--ink-2)]" role="status">{lotNote[m.id]}</p>
							{:else}
								<div class="flex gap-1.5">
									<button class="chip" onclick={() => quick(m, 'use')}>Used up</button>
									<button class="chip" onclick={() => quick(m, 'seen')}>Still there</button>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<div class="flex flex-wrap items-center gap-3">
			<button class="btn-primary" onclick={apply} disabled={busy}>Apply</button>
			<p class="text-sm text-[var(--ink-2)]">
				{nSeen} to mark seen, {nAdd} to add.
			</p>
			<button class="chip" onclick={reset}>Start over</button>
		</div>
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
	.chip.on {
		background: var(--sage-bg);
		border-color: var(--sage);
		color: var(--sage);
	}
	.chip.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.seg {
		cursor: pointer;
		border-radius: 0.75rem;
		border: 1px solid var(--line);
		background: var(--surface-2);
		padding: 0.4rem 1rem;
		font-size: 0.875rem;
		color: var(--ink-2);
	}
	.seg.on {
		border-color: var(--ice);
		background: var(--ice-bg);
		color: var(--ice);
		font-weight: 500;
	}
	.seg:focus-within {
		outline: 2px solid var(--focus);
	}
	.btn-primary {
		display: inline-block;
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
	.bar {
		width: 100%;
		height: 0.5rem;
		appearance: none;
		border: 0;
		border-radius: 9999px;
		overflow: hidden;
		background: var(--surface-2);
	}
	.bar::-webkit-progress-bar {
		background: var(--surface-2);
	}
	.bar::-webkit-progress-value {
		background: var(--sage);
		transition: width 300ms;
	}
	.bar::-moz-progress-bar {
		background: var(--sage);
	}
	input[aria-invalid='true'] {
		outline: 2px solid var(--terracotta);
	}
</style>
