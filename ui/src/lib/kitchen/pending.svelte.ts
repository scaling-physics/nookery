// Removals that can still be taken back. Removing a lot (used up, thrown
// away, entered by mistake) hides it at once and queues the ledger write;
// the write happens when the toast times out, or earlier when the page is
// left. Undo cancels the queue, so the ledger never sees a removal the user
// took back.

import type { LotAction, LotView } from '../bindings.js';
import { displayName } from '../format.js';

export type RemovalKind = 'used' | 'binned' | 'wrong';

export const REMOVAL_ACTION: Record<RemovalKind, LotAction> = {
	used: { type: 'use', qty: null },
	binned: { type: 'bin' },
	wrong: { type: 'wrong' }
};

const PHRASE: Record<RemovalKind, string> = {
	used: 'used up',
	binned: 'thrown away',
	wrong: 'removed as a mistake'
};

export type PendingEntry = { lot: LotView; kind: RemovalKind };

export type PendingOptions = {
	/** Writes one event; may reject. Called at most once per queued removal. */
	write: (short: string, action: LotAction) => Promise<unknown>;
	/** Called after a flush, e.g. to reload the inventory. */
	onSettled?: () => unknown;
	/** Called with the number of writes that failed. */
	onError?: (failed: number) => void;
	/** How long Undo stays available. */
	ms?: number;
};

export class PendingRemovals {
	/** Waiting for the timeout (Undo possible). */
	queued = $state<PendingEntry[]>([]);
	/** Being written; still hidden until the inventory has been reloaded. */
	#writing = $state<PendingEntry[]>([]);
	#timer: ReturnType<typeof setTimeout> | undefined;
	#opts: PendingOptions;

	constructor(opts: PendingOptions) {
		this.#opts = opts;
	}

	/** Lot ids to keep out of the lists. */
	hiddenIds = $derived(new Set([...this.queued, ...this.#writing].map((e) => e.lot.id)));

	/** "Pesto used up", or "3 removed" when several are pending; '' if none. */
	message = $derived.by(() => {
		const n = this.queued.length;
		if (n === 0) return '';
		if (n === 1) {
			const e = this.queued[0];
			return `${displayName(e.lot.item)} ${PHRASE[e.kind]}`;
		}
		return `${n} removed`;
	});

	/** Hide these lots and write the removal soon. Restarts the Undo timer. */
	add(lots: LotView[], kind: RemovalKind): void {
		const fresh = lots.filter((l) => !this.hiddenIds.has(l.id));
		if (fresh.length === 0) return;
		this.queued = [...this.queued, ...fresh.map((lot) => ({ lot, kind }))];
		this.#arm();
	}

	/** Take everything queued back: nothing is written. */
	undo(): void {
		clearTimeout(this.#timer);
		this.#timer = undefined;
		this.queued = [];
	}

	/** Write everything queued now. Safe to call any number of times: the
	 *  queue is emptied before the first write, so nothing is written twice. */
	async flush(): Promise<void> {
		clearTimeout(this.#timer);
		this.#timer = undefined;
		const batch = this.queued;
		if (batch.length === 0) return;
		this.queued = [];
		this.#writing = [...this.#writing, ...batch];
		let failed = 0;
		for (const e of batch) {
			try {
				await this.#opts.write(e.lot.short, REMOVAL_ACTION[e.kind]);
			} catch {
				failed++;
			}
		}
		if (failed) this.#opts.onError?.(failed);
		try {
			await this.#opts.onSettled?.();
		} finally {
			this.#writing = this.#writing.filter((e) => !batch.includes(e));
		}
	}

	/** Flush when the page is hidden or closed. Returns a function that
	 *  removes the listeners (call it, then `flush()`, on destroy). */
	attach(): () => void {
		const hide = () => {
			if (document.visibilityState === 'hidden') void this.flush();
		};
		const gone = () => void this.flush();
		document.addEventListener('visibilitychange', hide);
		window.addEventListener('pagehide', gone);
		return () => {
			document.removeEventListener('visibilitychange', hide);
			window.removeEventListener('pagehide', gone);
		};
	}

	#arm() {
		clearTimeout(this.#timer);
		this.#timer = setTimeout(() => void this.flush(), this.#opts.ms ?? 6000);
	}
}
