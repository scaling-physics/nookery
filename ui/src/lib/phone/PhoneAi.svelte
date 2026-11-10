<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api, downloadChannel } from '#lib/api.js';
	import type { CommandError, PhoneAiPref, PhoneAiView } from '#lib/bindings.js';
	import { formatBytes, formatSpeed, formatEta } from './format.js';

	let view = $state<PhoneAiView | null>(null);
	let error = $state('');
	let progress = $state({ done: 0, total: 0 });
	let speed = $state(0);
	let started = 0;
	let startDone = 0;
	let timer: ReturnType<typeof setInterval> | undefined;
	let downloading = $state(false);
	let confirmDelete = $state(false);

	async function refresh() {
		const r = await api.getPhoneAi();
		if (r.status === 'ok') {
			view = r.data;
			downloading = r.data.model.downloading;
		}
	}
	onMount(() => {
		void refresh();
		window.addEventListener('nookery:pairing', refresh);
		return () => window.removeEventListener('nookery:pairing', refresh);
	});
	onDestroy(() => clearInterval(timer));

	function words(e: CommandError): string {
		switch (e.kind) {
			case 'mac_unreachable':
				return e.detail;
			case 'sync_bad_code':
				return 'The Mac no longer accepts this phone. Pair again in “Sync with my Mac”.';
			case 'bad_input':
				return e.detail === 'downloading'
					? 'A download is already running.'
					: 'That isn’t possible on this device.';
			default:
				return 'Something went wrong. Try again.';
		}
	}

	async function choose(pref: PhoneAiPref) {
		const r = await api.setPhoneAi(pref);
		if (r.status === 'ok') view = r.data;
	}

	async function download() {
		if (downloading) return;
		error = '';
		downloading = true;
		const have = n(view?.model.partialBytes);
		progress = { done: have, total: n(view?.model.expectedBytes) };
		started = Date.now();
		startDone = have;
		speed = 0;
		timer = setInterval(() => {
			const s = (Date.now() - started) / 1000;
			if (s > 1) speed = (progress.done - startDone) / s;
		}, 1000);
		const r = await api.downloadPhoneModel(
			downloadChannel((p) => (progress = { done: p.done ?? 0, total: p.total ?? 0 }))
		);
		clearInterval(timer);
		downloading = false;
		if (r.status === 'ok') {
			view = r.data;
			return;
		}
		// Pressing Stop is not an error.
		if (!(r.error.kind === 'bad_input' && r.error.detail === 'cancelled')) error = words(r.error);
		const v = await api.getPhoneAi();
		if (v.status === 'ok') view = v.data;
	}

	const stop = () => api.cancelPhoneDownload();

	async function remove() {
		confirmDelete = false;
		const r = await api.deletePhoneModel();
		if (r.status === 'ok') view = r.data;
	}

	const n = (v: number | null | undefined) => v ?? 0;
	const m = $derived(view?.model);
	const pct = $derived(progress.total ? Math.min(100, (progress.done / progress.total) * 100) : 0);
	const lowSpace = $derived(
		!!m && m.freeBytes !== null && n(m.freeBytes) < n(m.expectedBytes) - n(m.partialBytes)
	);
</script>

{#if view?.available}
	<section class="card" aria-labelledby="phone-ai-h">
		<h2 id="phone-ai-h" class="text-[20px] font-semibold tracking-tight">
			Recipes and plans on this phone
		</h2>
		<p class="mt-1 text-sm text-[var(--ink-2)]">
			Where the ideas, recipes and weekly plans are written. Nothing goes to the internet: it is
			this phone, or your own Mac over your home Wi-Fi.
		</p>

		{#if error}
			<p
				role="alert"
				class="mt-3 rounded-xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
			>
				{error}
			</p>
		{/if}

		<fieldset class="mt-3 grid gap-1">
			<legend class="sr-only">Where recipes are written on this phone</legend>
			{#each [{ id: 'auto', label: 'Automatic', hint: 'On this phone if the model is downloaded, otherwise on your Mac.' }, { id: 'phone', label: 'On this phone', hint: 'Works anywhere, without Wi-Fi.' }, { id: 'mac', label: 'On my Mac', hint: 'Faster, but only at home with Nookery open on the Mac.' }] as o (o.id)}
				<label class="option">
					<input
						type="radio"
						name="phone-ai"
						value={o.id}
						checked={view.pref === o.id}
						onchange={() => choose(o.id as PhoneAiPref)}
						class="mt-1 h-4 w-4 shrink-0 accent-[var(--sage)]"
					/>
					<span>
						<span class="block text-[15px]">{o.label}</span>
						<span class="block text-sm text-[var(--ink-2)]">{o.hint}</span>
					</span>
				</label>
			{/each}
		</fieldset>

		<div class="mt-4 rounded-xl bg-[var(--surface-2)] p-4">
			<p class="text-[15px] font-medium">The recipe model</p>
			{#if m?.downloaded}
				<p class="mt-1 text-sm text-[var(--ink-2)]">
					On this phone ({formatBytes(n(m.bytes))}). Nothing more to do.
				</p>
				{#if confirmDelete}
					<div class="mt-3 flex flex-wrap items-center gap-2">
						<span class="text-sm">Delete it? You can download it again.</span>
						<button class="chip" onclick={remove}>Delete</button>
						<button class="chip" onclick={() => (confirmDelete = false)}>Keep</button>
					</div>
				{:else}
					<button class="chip mt-3" onclick={() => (confirmDelete = true)}
						>Delete from this phone</button
					>
				{/if}
			{:else}
				<p class="mt-1 text-sm text-[var(--ink-2)]">
					Download it from your Mac, {formatBytes(n(m?.expectedBytes))}, over your home Wi-Fi.
					{#if !view.paired}Pair with your Mac first (above).{/if}
				</p>
				{#if downloading}
					<progress
						class="bar mt-3"
						max={progress.total || 1}
						value={progress.done}
						aria-label="Download progress"
					></progress>
					<p class="mt-1.5 text-sm tabular-nums" role="status" aria-live="polite">
						{Math.round(pct)}% · {formatBytes(progress.done)} of {formatBytes(progress.total)}
						{#if speed > 0}· {formatSpeed(speed)} · {formatEta(
								(progress.total - progress.done) / speed
							)}{/if}
					</p>
					<button class="chip mt-3" onclick={stop}>Stop</button>
				{:else}
					<div class="mt-3 flex flex-wrap items-center gap-3">
						<button class="chip primary" onclick={download} disabled={!view.paired || lowSpace}>
							{m && n(m.partialBytes) > 0
								? `Resume (${formatBytes(n(m.partialBytes))} done)`
								: 'Download the recipe model'}
						</button>
						{#if m && n(m.partialBytes) > 0}
							<button class="chip" onclick={remove}>Start over</button>
						{/if}
					</div>
					{#if m && m.freeBytes !== null}
						<p class="mt-2 text-xs" class:warn={lowSpace}>
							{formatBytes(n(m.freeBytes))} free on this phone{lowSpace
								? ': not enough for the model. Free some space first.'
								: '.'}
						</p>
					{/if}
				{/if}
			{/if}
		</div>

		<p class="mt-3 text-sm text-[var(--ink-2)]">
			Speed: on the phone an answer takes roughly 25 to 35 seconds; your Mac is faster when you are
			home. The first answer after opening the app also loads the model, a few seconds more.
		</p>
	</section>
{/if}

<style>
	.card {
		border: 1px solid var(--line);
		background: var(--surface);
		border-radius: 1rem;
		padding: 1.1rem 1.25rem;
	}
	.option {
		display: flex;
		gap: 0.75rem;
		border-radius: 0.75rem;
		padding: 0.5rem 0.4rem;
		cursor: pointer;
	}
	.option:hover {
		background: var(--surface-2);
	}
	.chip {
		border: 1px solid var(--line);
		background: var(--surface);
		border-radius: 9999px;
		padding: 0.35rem 0.9rem;
		font-size: 0.875rem;
		color: var(--ink);
	}
	.chip:hover:not(:disabled) {
		background: var(--surface-2);
	}
	.chip:disabled {
		opacity: 0.6;
	}
	.primary {
		background: var(--sage);
		border-color: var(--sage);
		color: var(--paper, #fffdf8);
	}
	.primary:hover:not(:disabled) {
		background: var(--sage);
		filter: brightness(1.08);
	}
	.warn {
		color: var(--terracotta);
	}
	.bar {
		width: 100%;
		height: 0.5rem;
		accent-color: var(--sage);
	}
</style>
