import type { LotView } from './bindings.js';

export type Tone = 'expired' | 'soon' | 'fresh' | 'muted';

/** The short label beside a lot, and its colour. */
export function statusLabel(lot: Pick<LotView, 'status' | 'days'>): { text: string; tone: Tone } {
	const d = lot.days ?? 0;
	switch (lot.status) {
		case 'expired':
			return { text: d === 1 ? 'Expired yesterday' : `Expired ${d} days ago`, tone: 'expired' };
		case 'use_soon':
			return {
				text: d === 0 ? 'Use today' : d === 1 ? 'Use by tomorrow' : `Use within ${d} days`,
				tone: 'soon'
			};
		case 'fresh':
			return {
				text:
					d < 14
						? `${d} days left`
						: d < 60
							? `${Math.round(d / 7)} weeks left`
							: `${Math.round(d / 30)} months left`,
				tone: 'fresh'
			};
		case 'unconfirmed':
			return { text: `Not seen for ${d} days`, tone: 'muted' };
		case 'undated':
			return { text: '', tone: 'muted' };
		case 'used':
			return { text: 'Used', tone: 'muted' };
		case 'binned':
			return { text: 'Binned', tone: 'muted' };
	}
}

/** "Use first": expired and soon-to-expire lots, most urgent first (the
 *  backend already sorts them). */
export function urgent(lots: LotView[]): LotView[] {
	return lots.filter((l) => l.status === 'expired' || l.status === 'use_soon');
}

/** Title-case the first letter for display ("greek yoghurt" -> "Greek yoghurt"). */
export function itemName(s: string): string {
	return s.charAt(0).toUpperCase() + s.slice(1);
}

/** "Frozen today", "Frozen 12 days ago", "Frozen 6 weeks ago" from a
 *  YYYY-MM-DD date, relative to `today` (local). */
export function frozenLabel(frozenOn: string, today: Date = new Date()): string {
	const [y, m, d] = frozenOn.split('-').map(Number);
	const start = Date.UTC(y, m - 1, d);
	const now = Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
	const days = Math.max(0, Math.round((now - start) / 86_400_000));
	if (days === 0) return 'Frozen today';
	if (days === 1) return 'Frozen yesterday';
	if (days < 28) return `Frozen ${days} days ago`;
	return `Frozen ${Math.round(days / 7)} weeks ago`;
}

/** Euro cents as printed on German receipts: 977 -> "9,77 €", -25 -> "-0,25 €". */
export function formatCents(cents: number): string {
	const abs = Math.abs(cents);
	const body = `${Math.floor(abs / 100)},${String(abs % 100).padStart(2, '0')}`;
	return `${cents < 0 ? '-' : ''}${body} €`;
}

/** A difference with an explicit sign: 5 -> "+0,05 €", -5 -> "-0,05 €". */
export function formatDiff(cents: number): string {
	return cents > 0 ? `+${formatCents(cents)}` : formatCents(cents);
}

/** Which input an import error names: "qty:3" -> { field: 'qty', line: 3 }. */
export function parseFieldError(detail: string): { field: string; line: number | null } {
	const [field, n] = detail.split(':');
	return { field, line: n === undefined || n === '' ? null : Number(n) };
}

/** The message for `cloud_sign_in`; `ErrorBanner` adds the Settings link. */
export const CLOUD_SIGN_IN =
	'The cloud model needs an Ollama account. In Terminal run “ollama signin”, or switch to “On this Mac only” in Settings.';
