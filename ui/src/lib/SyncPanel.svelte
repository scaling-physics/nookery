<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { api } from '#lib/api.js';
	import type { CommandError, SyncStatusView } from '#lib/bindings.js';
	import { canScanQr, cancelScan, scanPairingQr } from '#lib/phone/scanner.js';

	let status = $state<SyncStatusView | null>(null);
	let error = $state('');
	let busy = $state(false);
	let code = $state('');
	let address = $state('');
	let now = $state(Date.now());
	let tick: ReturnType<typeof setInterval> | undefined;

	onMount(async () => {
		tick = setInterval(() => (now = Date.now()), 30_000);
		const r = await api.getSyncStatus();
		if (r.status === 'ok') status = r.data;
		else error = 'Could not read the sync settings.';
	});
	onDestroy(() => clearInterval(tick));

	function words(e: CommandError): string {
		if (e.kind === 'sync_bad_code')
			return 'That code isn’t right. Check the code shown on the Mac (it changes if you make a new one).';
		if (e.kind === 'sync_unreachable') return e.detail;
		if (e.kind === 'bad_input') return 'Type the Mac’s address, for example 192.168.1.20.';
		return 'Something went wrong. Try again.';
	}

	async function run(call: () => ReturnType<typeof api.getSyncStatus>) {
		if (busy) return;
		busy = true;
		error = '';
		const r = await call();
		busy = false;
		if (r.status === 'ok') {
			status = r.data;
			// Other panels (the phone's AI card) depend on the pairing.
			window.dispatchEvent(new Event('nookery:pairing'));
		} else {
			error = words(r.error);
			// A failed sync still records itself; show it.
			const s = await api.getSyncStatus();
			if (s.status === 'ok') status = s.data;
		}
	}

	const toggle = (on: boolean) => run(() => api.setSharing(on));
	const newCode = () => run(() => api.newPairingCode());
	const syncNow = () => run(() => api.syncNow());
	const forget = () => run(() => api.forgetMac());
	let scanning = $state(false);
	let scanNote = $state('');
	async function scanQr() {
		if (busy || scanning) return;
		scanNote = '';
		error = '';
		scanning = true;
		const r = await scanPairingQr();
		scanning = false;
		if (r.kind === 'ok') {
			await run(() => api.pairWithMac(r.link.code, r.link.addr));
		} else if (r.kind === 'denied') {
			scanNote = 'Nookery needs the camera to scan the code. You can still type it below.';
		} else if (r.kind === 'wrong') {
			scanNote = 'That QR code isn’t from Nookery. Use the one shown on your Mac in Settings.';
		}
	}
	async function pair() {
		await run(() => api.pairWithMac(code, address));
		if (!error) code = '';
	}

	function ago(iso: string): string {
		const mins = Math.max(0, Math.round((now - new Date(iso).getTime()) / 60_000));
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins} min ago`;
		const h = Math.round(mins / 60);
		if (h < 24) return `${h} h ago`;
		return `${Math.round(h / 24)} d ago`;
	}

	const dropped = $derived(status?.last?.dropped ?? []);
</script>

<section class="card" aria-labelledby="sync-h">
	<h2 id="sync-h" class="text-[20px] font-semibold tracking-tight">
		{status?.role === 'phone' ? 'Sync with my Mac' : 'Share with my phone'}
	</h2>

	{#if error}
		<p
			role="alert"
			class="mt-3 rounded-xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)]"
		>
			{error}
		</p>
	{/if}

	{#if !status}
		<p class="mt-2 text-sm text-[var(--ink-2)]">Loading…</p>
	{:else if status.role === 'mac'}
		<p class="mt-1 text-sm text-[var(--ink-2)]">
			Your phone gets your kitchen list, favourites, plans and shopping list. It only works over
			your home Wi-Fi, everything is encrypted, and nothing goes to the internet.
		</p>
		<label class="option mt-3">
			<input
				type="checkbox"
				role="switch"
				checked={status.sharing}
				disabled={busy}
				onchange={(e) => toggle(e.currentTarget.checked)}
				class="mt-1 h-4 w-4 shrink-0 accent-[var(--sage)]"
			/>
			<span>
				<span class="block text-[15px]">Share with my phone</span>
				<span class="block text-sm text-[var(--ink-2)]">
					{status.sharing
						? 'On: your phone can sync while Nookery is open here.'
						: 'Off: nothing is shared.'}
				</span>
			</span>
		</label>

		{#if status.sharing}
			<div class="mt-4 grid gap-3 rounded-xl bg-[var(--surface-2)] p-4">
				{#if status.qr}
					<div class="qr-wrap">
						<!-- Made on this Mac from the pairing link; light tile so it scans in dark mode. -->
						<div
							class="qr"
							data-testid="sync-qr"
							role="img"
							aria-label="QR code for pairing the phone"
						>
							{@html status.qr}
						</div>
						<p class="text-sm text-[var(--ink-2)]">
							On the phone, open Settings in Nookery and choose “Scan the QR code”.
							<span class="block text-[var(--ink-3)]"
								>It holds the pairing secret: keep it private, don’t photograph or share it.</span
							>
						</p>
					</div>
				{/if}
				<p class="text-sm text-[var(--ink-2)]">
					{status.qr
						? 'Or type this code and address on the phone.'
						: 'On the phone, enter this code and address.'}
				</p>
				<div>
					<div class="label">Code</div>
					<div class="big" data-testid="sync-code">{status.code}</div>
				</div>
				<div>
					<div class="label">Address</div>
					{#if status.addresses.length}
						{#each status.addresses as a (a)}
							<div class="big">{a}:{status.port}</div>
						{/each}
					{:else}
						<div class="text-sm text-[var(--ink-2)]">
							Couldn’t find this Mac on a network. Connect to Wi-Fi.
						</div>
					{/if}
				</div>
				<div>
					<button class="chip" onclick={newCode} disabled={busy}>Make a new code</button>
					<p class="mt-1.5 text-sm text-[var(--ink-2)]">
						A new code locks out phones paired before; they need the new one.
					</p>
				</div>
			</div>
		{/if}
	{:else}
		<p class="mt-1 text-sm text-[var(--ink-2)]">
			Keeps this phone and your Mac in step over your home Wi-Fi. Open Nookery on the Mac, switch on
			“Share with my phone” in Settings, then scan the QR code it shows (or type the code and
			address).
		</p>

		{#if !status.peer}
			<div class="mt-4 grid gap-3">
				{#if canScanQr}
					<div>
						<button class="chip primary" onclick={scanQr} disabled={busy || scanning}>
							Scan the QR code
						</button>
						{#if scanNote}
							<p class="mt-2 text-sm text-[var(--terracotta)]" role="alert">{scanNote}</p>
						{/if}
					</div>
					<p class="text-xs text-[var(--ink-3)]">Or type it in:</p>
				{/if}
				<label class="block text-sm">
					<span class="label">Code from the Mac</span>
					<input
						class="field mt-1 w-full"
						bind:value={code}
						placeholder="ABCD-EFGH-IJKL-MNOP-QRST-UV"
						autocapitalize="characters"
						autocomplete="off"
						spellcheck="false"
					/>
				</label>
				<label class="block text-sm">
					<span class="label">Mac’s address</span>
					<input
						class="field mt-1 w-full"
						bind:value={address}
						placeholder="192.168.1.20:47480"
						inputmode="url"
						autocapitalize="off"
						autocomplete="off"
						spellcheck="false"
					/>
				</label>
				<div>
					<button
						class="chip primary"
						onclick={pair}
						disabled={busy || !code.trim() || !address.trim()}
					>
						{busy ? 'Pairing…' : 'Pair'}
					</button>
				</div>
			</div>
		{:else}
			<p class="mt-3 text-[15px]">
				Paired with <span class="font-medium">{status.peerName ?? 'your Mac'}</span> at
				<span class="mono">{status.peer}</span>.
			</p>
			<div class="mt-3 flex flex-wrap items-center gap-3">
				<button class="chip primary" onclick={syncNow} disabled={busy}>
					{busy ? 'Syncing…' : 'Sync now'}
				</button>
				<button class="chip" onclick={forget} disabled={busy}>Forget this Mac</button>
			</div>
			<p role="status" aria-live="polite" class="mt-3 text-sm">
				{#if status.last?.ok}
					<span class="text-[var(--sage)]">Synced {ago(status.last.at)}</span>
				{:else if status.last}
					<span class="text-[var(--terracotta)]">
						Last try {ago(status.last.at)} failed: {status.last.error}
					</span>
				{:else}
					<span class="text-[var(--ink-2)]">Not synced yet.</span>
				{/if}
			</p>
			{#if dropped.length}
				<p class="mt-2 rounded-xl bg-[var(--saffron-bg)] p-3 text-sm text-[var(--ink)]">
					{dropped.length === 1 ? '1 change' : `${dropped.length} changes`} from the phone
					{dropped.length === 1 ? 'was' : 'were'} dropped because the Mac had changed the same item:
					{dropped.join(', ')}. The phone now shows what the Mac has.
				</p>
			{/if}
		{/if}
	{/if}
</section>

{#if scanning}
	<div class="scan-overlay" role="dialog" aria-label="Scan the QR code">
		<p class="scan-text">Point the camera at the QR code on your Mac.</p>
		<button class="chip scan-cancel" onclick={cancelScan}>Cancel</button>
	</div>
{/if}

<style>
	/* While the camera is open the web page must be see-through. */
	:global(html.scanning),
	:global(html.scanning body) {
		background: transparent !important;
	}
	:global(html.scanning body) {
		visibility: hidden;
	}
	.scan-overlay {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: space-between;
		padding: calc(2rem + env(safe-area-inset-top)) 1.5rem calc(2rem + env(safe-area-inset-bottom));
		visibility: visible;
	}
	.scan-text {
		border-radius: 1rem;
		background: color-mix(in srgb, var(--surface) 92%, transparent);
		padding: 0.6rem 1rem;
		text-align: center;
		font-size: 0.9375rem;
		color: var(--ink);
	}
	.scan-cancel {
		padding: 0.6rem 1.4rem;
		font-size: 1rem;
	}
	.qr-wrap {
		display: grid;
		gap: 0.75rem;
		justify-items: start;
	}
	.qr {
		width: min(100%, 220px);
		border-radius: 0.75rem;
		overflow: hidden;
		background: #fffdf8;
		box-shadow: 0 0 0 1px var(--line);
	}
	.qr :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}
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
	.label {
		font-size: 0.8125rem;
		color: var(--ink-2);
	}
	.big {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 1.375rem;
		letter-spacing: 0.04em;
		color: var(--ink);
		overflow-wrap: anywhere;
	}
	.mono {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.9em;
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
</style>
