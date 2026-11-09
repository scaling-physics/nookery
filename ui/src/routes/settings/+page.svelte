<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { AiModeView, AiSettingsView, CloudCheckView } from '#lib/bindings.js';

	const DEFAULT_MODEL = 'gemma4:31b-cloud';
	const SIGN_IN = 'ollama signin';

	const MODES: { id: AiModeView; label: string; hint: string }[] = [
		{ id: 'local', label: 'On this Mac only', hint: 'Nothing leaves this Mac.' },
		{
			id: 'fallback',
			label: 'This Mac, cloud if it fails',
			hint: 'Uses the cloud only when the model on this Mac is missing or fails.'
		},
		{ id: 'cloud', label: 'Cloud', hint: 'Faster and a bigger model. Free with limits.' }
	];
	const GROUPS = [
		{
			key: 'photos' as const,
			title: 'Photos',
			sent: 'The photo (cut into parts) is sent to Ollama’s servers.'
		},
		{
			key: 'recipes' as const,
			title: 'Recipes',
			sent: 'Your item names, amounts and template text are sent to Ollama’s servers. No photos.'
		}
	];

	let settings = $state<AiSettingsView>({
		photos: 'local',
		recipes: 'local',
		cloudModel: DEFAULT_MODEL
	});
	let loaded = $state(false);
	let error = $state('');
	let saved = $state(false);
	let savedTimer: ReturnType<typeof setTimeout> | undefined;

	let checking = $state(false);
	let check = $state<CloudCheckView | null>(null);
	let copied = $state(false);
	let copiedTimer: ReturnType<typeof setTimeout> | undefined;

	onDestroy(() => {
		clearTimeout(savedTimer);
		clearTimeout(copiedTimer);
	});

	onMount(async () => {
		const r = await api.getAiSettings();
		if (r.status === 'ok') settings = r.data;
		else error = 'Could not read the settings.';
		loaded = true;
	});

	async function save() {
		error = '';
		const r = await api.setAiSettings({ ...settings });
		if (r.status === 'ok') {
			settings = r.data;
			saved = true;
			clearTimeout(savedTimer);
			savedTimer = setTimeout(() => (saved = false), 2000);
		} else {
			saved = false;
			error =
				r.error.kind === 'bad_input'
					? 'The cloud model name can’t contain spaces.'
					: 'Could not save the settings.';
		}
	}

	async function runCheck() {
		if (checking) return;
		checking = true;
		check = null;
		const r = await api.checkCloud();
		checking = false;
		if (r.status === 'ok') check = r.data;
		else error = 'Could not run the check.';
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(SIGN_IN);
			copied = true;
			clearTimeout(copiedTimer);
			copiedTimer = setTimeout(() => (copied = false), 1500);
		} catch {
			copied = false;
		}
	}
</script>

<main class="mx-auto max-w-3xl px-5 pt-8 pb-16 sm:px-8">
	<header class="mb-6 flex items-baseline justify-between gap-4">
		<div>
			<h1 class="text-[26px] font-semibold tracking-tight">Settings</h1>
			<p class="text-sm text-[var(--ink-2)]">Changes are saved as you make them.</p>
		</div>
		<a href="/" class="chip shrink-0 whitespace-nowrap">Back to the kitchen</a>
	</header>

	{#if error}
		<p
			role="alert"
			class="mb-4 rounded-2xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
		>
			{error}
		</p>
	{/if}

	<div class="flex items-baseline justify-between gap-4">
		<h2 class="text-[20px] font-semibold tracking-tight">Where the AI runs</h2>
		<span role="status" class="text-sm text-[var(--sage)]">{saved ? 'Saved' : ''}</span>
	</div>
	<p class="mt-0.5 mb-4 text-sm text-[var(--ink-2)]">
		Everything runs on this Mac unless you choose otherwise, per feature.
	</p>

	<div class="grid gap-4" class:opacity-60={!loaded}>
		{#each GROUPS as g (g.key)}
			<fieldset class="card">
				<legend class="px-1 text-[15px] font-semibold">{g.title}</legend>
				<div class="mt-1 grid gap-1">
					{#each MODES as m (m.id)}
						<label class="option">
							<input
								type="radio"
								name={g.key}
								value={m.id}
								bind:group={settings[g.key]}
								onchange={save}
								disabled={!loaded}
								class="mt-1 h-4 w-4 shrink-0 accent-[var(--sage)]"
							/>
							<span>
								<span class="block text-[15px]">{m.label}</span>
								<span class="block text-sm text-[var(--ink-2)]">{m.hint}</span>
							</span>
						</label>
					{/each}
				</div>
				<p class="mt-3 border-t border-[var(--line)] pt-3 text-sm text-[var(--ink-2)]">
					<span class="font-medium text-[var(--ink)]">If the cloud is used:</span>
					{g.sent}
				</p>
			</fieldset>
		{/each}

		<section class="card" aria-labelledby="cloud-h">
			<h2 id="cloud-h" class="text-[15px] font-semibold">Cloud model</h2>
			<input
				type="text"
				aria-labelledby="cloud-h"
				aria-describedby="cloud-note"
				bind:value={settings.cloudModel}
				onchange={save}
				placeholder={DEFAULT_MODEL}
				spellcheck="false"
				autocapitalize="off"
				autocomplete="off"
				disabled={!loaded}
				class="field mt-2 w-full"
			/>
			<p id="cloud-note" class="mt-1.5 text-sm text-[var(--ink-2)]">
				Ollama Cloud. Needs a free ollama.com account.
			</p>

			<div class="mt-4 flex flex-wrap items-center gap-3">
				<button class="chip" onclick={runCheck} disabled={checking || !loaded}>
					{checking ? 'Checking…' : 'Check cloud'}
				</button>
				<div role="status" aria-live="polite" class="text-sm">
					{#if check?.signedIn}
						<span class="text-[var(--sage)]"
							>Signed in, answered in {((check.ms ?? 0) / 1000).toFixed(1)} s</span
						>
					{:else if check && check.detail}
						<span class="text-[var(--terracotta)]">{check.detail}</span>
					{/if}
				</div>
			</div>

			{#if check && !check.signedIn && !check.detail}
				<div class="mt-3 rounded-xl bg-[var(--saffron-bg)] p-3 text-sm">
					<p class="text-[var(--ink)]">
						Not signed in. In Terminal, run <code class="code">{SIGN_IN}</code> (opens the browser to
						sign in), then check again.
					</p>
					<button class="chip mt-2" onclick={copy}>{copied ? 'Copied' : 'Copy command'}</button>
				</div>
			{/if}
		</section>
	</div>
</main>

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
	.field {
		border: 1px solid var(--line);
		background: var(--bg);
		border-radius: 0.75rem;
		padding: 0.55rem 0.8rem;
		font-size: 0.9375rem;
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
	.code {
		border-radius: 0.375rem;
		background: var(--surface);
		padding: 0.1rem 0.4rem;
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.85em;
	}
</style>
