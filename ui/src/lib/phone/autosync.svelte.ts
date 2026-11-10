// The phone keeps itself in step with the Mac: once at start, and whenever the
// app comes back to the foreground (at most every 30 s). Never blocks the UI;
// a failure only changes a small status.
import { api } from '#lib/api.js';

export const THROTTLE_MS = 30_000;

export const autosync = $state({
	/** A Mac is paired (else there is nothing to do). */
	paired: false,
	state: 'idle' as 'idle' | 'syncing' | 'ok' | 'failed',
	/** ms epoch of the last successful sync. */
	at: null as number | null,
	error: null as string | null,
	/** Bumped when a sync changed what the pages show: the layout reloads them. */
	version: 0
});

let lastAttempt = 0;
let lastShape = '';
let running = false;

/** Whether a sync should start now (pure, for tests). */
export function shouldSync(now: number, last: number, force = false): boolean {
	return force || now - last >= THROTTLE_MS;
}

export async function syncSoon(force = false): Promise<void> {
	if (running || !shouldSync(Date.now(), lastAttempt, force)) return;
	running = true;
	lastAttempt = Date.now();
	try {
		const s = await api.getSyncStatus();
		autosync.paired = s.status === 'ok' && !!s.data.peer;
		if (!autosync.paired) {
			autosync.state = 'idle';
			return;
		}
		autosync.state = 'syncing';
		const r = await api.syncNow();
		const last = r.status === 'ok' ? r.data.last : null;
		if (r.status === 'ok' && last?.ok) {
			autosync.state = 'ok';
			autosync.at = Date.parse(last.at) || Date.now();
			autosync.error = null;
			const shape = `${last.pulled}/${last.docsChanged}/${last.pushed}`;
			if (shape !== lastShape) {
				lastShape = shape;
				autosync.version++;
			}
		} else {
			autosync.state = 'failed';
			autosync.error =
				r.status === 'error'
					? r.error.kind === 'sync_unreachable'
						? r.error.detail
						: 'Sync failed.'
					: (last?.error ?? 'Sync failed.');
		}
	} finally {
		running = false;
	}
}

/** Call once on the phone; returns the cleanup. */
export function startAutoSync(): () => void {
	void syncSoon(true);
	const onVisible = () => {
		if (document.visibilityState === 'visible') void syncSoon();
	};
	document.addEventListener('visibilitychange', onVisible);
	return () => document.removeEventListener('visibilitychange', onVisible);
}

/** After the phone changed something that the Mac should see (e.g. a photo scan). */
export const syncNowForced = () => syncSoon(true);
