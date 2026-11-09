<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { CommandError, TemplateView } from '#lib/bindings.js';

	let {
		templates,
		onchange,
		onclose
	}: {
		templates: TemplateView[];
		/** Called after every change with the fresh list. */
		onchange: () => void | Promise<void>;
		onclose: () => void;
	} = $props();

	let dialog: HTMLDialogElement | undefined = $state();
	let opener: Element | null = null;

	/** Which row is being edited: a template id, 'new', or none. */
	let editing = $state<number | 'new' | null>(null);
	let name = $state('');
	let body = $state('');
	let fieldError = $state<{ field: 'name' | 'body'; text: string } | null>(null);
	let confirming = $state<number | null>(null);
	let error = $state('');
	let busy = $state(false);
	let nameInput: HTMLInputElement | undefined = $state();

	onMount(() => {
		opener = document.activeElement;
		dialog?.showModal();
		return () => {
			if (opener instanceof HTMLElement) opener.focus();
		};
	});

	function startEdit(t: TemplateView | null) {
		editing = t ? t.id : 'new';
		name = t?.name ?? '';
		body = t?.body ?? '';
		fieldError = null;
		error = '';
		confirming = null;
		queueMicrotask(() => nameInput?.focus());
	}

	function cancelEdit() {
		editing = null;
		fieldError = null;
	}

	function describe(e: CommandError): string {
		return e.kind === 'not_found'
			? 'That template no longer exists. Close this window and open it again.'
			: 'Something went wrong. Please try again.';
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		if (!name.trim()) {
			fieldError = { field: 'name', text: 'Give the template a name, like “Quick weeknight”.' };
			return;
		}
		if (!body.trim()) {
			fieldError = { field: 'body', text: 'Say what you want, in your own words.' };
			return;
		}
		busy = true;
		const r = await api.saveTemplate({
			id: editing === 'new' ? null : editing,
			name,
			body
		});
		busy = false;
		if (r.status === 'ok') {
			editing = null;
			fieldError = null;
			error = '';
			await onchange();
			return;
		}
		if (r.error.kind === 'bad_input') {
			fieldError =
				r.error.detail === 'name'
					? { field: 'name', text: 'You already have a template with that name.' }
					: { field: 'body', text: 'Say what you want, in your own words.' };
		} else error = describe(r.error);
	}

	async function remove(t: TemplateView) {
		const r = await api.deleteTemplate(t.id);
		confirming = null;
		if (r.status === 'error') error = describe(r.error);
		else error = '';
		await onchange();
	}

	async function move(i: number, by: -1 | 1) {
		const ids = templates.map((t) => t.id);
		[ids[i], ids[i + by]] = [ids[i + by], ids[i]];
		const r = await api.orderTemplates(ids);
		if (r.status === 'error') error = describe(r.error);
		else error = '';
		await onchange();
	}

	function backdrop(e: MouseEvent) {
		if (e.target === dialog) dialog?.close();
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	aria-labelledby="tpl-h"
	{onclose}
	onclick={backdrop}
	class="m-auto w-[min(40rem,calc(100vw-1.5rem))] max-w-none rounded-2xl border border-[var(--line)] bg-[var(--bg)] p-0 text-[var(--ink)] shadow-2xl"
>
	<div class="flex max-h-[85vh] flex-col">
		<header class="flex items-start justify-between gap-4 px-5 pt-5 pb-3">
			<div>
				<h2 id="tpl-h" class="text-[18px] font-semibold tracking-tight">Templates</h2>
				<p class="text-sm text-[var(--ink-2)]">
					What you ask for, in your own words. The first one is picked by default.
				</p>
			</div>
			<button class="chip shrink-0" onclick={() => dialog?.close()}>Done</button>
		</header>

		<div class="overflow-y-auto px-5 pb-5">
			{#if error}
				<p
					role="alert"
					class="mb-3 rounded-xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
				>
					{error}
				</p>
			{/if}

			{#snippet form()}
				<form
					onsubmit={save}
					class="grid gap-2 rounded-2xl border border-[var(--sage)] bg-[var(--surface)] p-3"
				>
					<label class="grid gap-1 text-[13px] font-medium text-[var(--ink-2)]">
						Name
						<input
							bind:this={nameInput}
							bind:value={name}
							aria-invalid={fieldError?.field === 'name'}
							aria-describedby={fieldError?.field === 'name' ? 'tpl-err' : undefined}
							class="field"
							placeholder="Quick weeknight"
						/>
					</label>
					<label class="grid gap-1 text-[13px] font-medium text-[var(--ink-2)]">
						What to ask for
						<textarea
							bind:value={body}
							rows="4"
							aria-invalid={fieldError?.field === 'body'}
							aria-describedby={fieldError?.field === 'body' ? 'tpl-err' : undefined}
							class="field resize-y"
							placeholder="Dinner for two, ready in 30 minutes, one pan if possible."></textarea>
					</label>
					{#if fieldError}
						<p id="tpl-err" role="alert" class="text-sm text-[var(--terracotta)]">
							{fieldError.text}
						</p>
					{/if}
					<div class="flex gap-2">
						<button class="btn-primary" disabled={busy}>Save</button>
						<button type="button" class="chip" onclick={cancelEdit}>Cancel</button>
					</div>
				</form>
			{/snippet}

			<ul class="grid gap-2">
				{#each templates as t, i (t.id)}
					<li>
						{#if editing === t.id}
							{@render form()}
						{:else}
							<div
								class="flex flex-col gap-2 rounded-2xl border border-[var(--line)] bg-[var(--surface)] p-3 sm:flex-row sm:items-start sm:gap-3"
							>
								<div class="min-w-0 flex-1">
									<p class="font-medium">{t.name}</p>
									<p class="line-clamp-2 text-sm whitespace-pre-line text-[var(--ink-2)]">
										{t.body}
									</p>
									{#if confirming === t.id}
										<p class="mt-2 flex flex-wrap items-center gap-2 text-sm" role="group">
											<span>Delete “{t.name}”?</span>
											<button class="chip danger" onclick={() => remove(t)}>Delete</button>
											<button class="chip" onclick={() => (confirming = null)}>Keep</button>
										</p>
									{/if}
								</div>
								<div class="flex shrink-0 flex-wrap gap-1 sm:justify-end">
									<button
										class="chip"
										aria-label="Move {t.name} up"
										disabled={i === 0}
										onclick={() => move(i, -1)}>↑</button
									>
									<button
										class="chip"
										aria-label="Move {t.name} down"
										disabled={i === templates.length - 1}
										onclick={() => move(i, 1)}>↓</button
									>
									<button class="chip" onclick={() => startEdit(t)} aria-label="Edit {t.name}"
										>Edit</button
									>
									<button
										class="chip"
										onclick={() => (confirming = t.id)}
										aria-label="Delete {t.name}">Delete</button
									>
								</div>
							</div>
						{/if}
					</li>
				{/each}
				{#if editing === 'new'}
					<li>{@render form()}</li>
				{/if}
			</ul>

			{#if templates.length === 0 && editing !== 'new'}
				<p class="text-sm text-[var(--ink-3)]">No templates yet. Add one to get started.</p>
			{/if}

			{#if editing !== 'new'}
				<button class="btn-primary mt-3" onclick={() => startEdit(null)}>New template</button>
			{/if}
		</div>
	</div>
</dialog>

<style>
	dialog::backdrop {
		background: var(--backdrop);
	}
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
	.chip:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}
	.chip:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
	.chip.danger {
		border-color: var(--terracotta);
		color: var(--terracotta);
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
	}
	.field {
		min-width: 0;
		border-radius: 0.75rem;
		background: var(--surface-2);
		padding: 0.5rem 0.75rem;
		font-weight: 400;
		color: var(--ink);
	}
	.field[aria-invalid='true'] {
		outline: 2px solid var(--terracotta);
	}
</style>
