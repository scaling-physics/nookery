// The UI's only door to the backend. Inside Tauri it calls the generated
// commands; in a plain browser (`pnpm -C ui dev`) it serves SYNTHETIC sample
// data so screens can be previewed and screenshotted without the app.

import { commands } from './bindings.js';
import type { AddInput, CommandError, InventoryView, LotAction, LotView } from './bindings.js';

export type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: CommandError };

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const api = inTauri ? commands : fakeApi();

function fakeApi() {
	const day = 86_400_000;
	const iso = (offset: number) => new Date(Date.now() + offset * day).toISOString().slice(0, 10);
	let n = 0;
	const lot = (
		item: string,
		left: string,
		location: string,
		days: number | null,
		extra: Partial<LotView> = {}
	): LotView => {
		const status: LotView['status'] =
			days === null ? 'undated' : days < 0 ? 'expired' : days <= 2 ? 'use_soon' : 'fresh';
		const id = (++n).toString(16).padStart(8, '0');
		return {
			id,
			short: id,
			item,
			left,
			location,
			status,
			days: days === null ? null : Math.abs(days),
			bestBefore: days === null ? null : iso(days),
			opened: false,
			frozenOn: null,
			store: null,
			...extra
		};
	};
	const lots: LotView[] = [
		lot('cream', '200ml', 'fridge', -1, { opened: true }),
		lot('spinach', '250g', 'fridge', 0, { store: 'REWE' }),
		lot('greek yoghurt', '500g', 'fridge', 1, { store: 'Lidl' }),
		lot('eggs', '4', 'fridge', 9, { store: 'Lidl' }),
		lot('halloumi', '225g', 'fridge', 21),
		lot('minced beef', '500g', 'freezer', null, { frozenOn: iso(-12) }),
		lot('peas', '750g', 'freezer', null, { frozenOn: iso(-40) }),
		lot('basmati rice', '1.5kg', 'pantry', 300),
		lot('chickpeas', '2', 'pantry', 500),
		lot('olive oil', '750ml', 'pantry', null, { opened: true })
	];
	const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: 'ok', data });
	return {
		getInventory: (): Promise<Result<InventoryView>> =>
			ok({ lots: [...lots], events: lots.length, openMs: 0.4, queryMs: 0.05 }),
		addItem: (input: AddInput): Promise<Result<LotView>> => {
			const l = lot(input.item, input.qty || '1', input.location, null, { store: input.store });
			lots.push(l);
			return ok(l);
		},
		lotAction: (id: string, action: LotAction): Promise<Result<LotView>> => {
			const i = lots.findIndex((l) => l.short === id);
			const l = lots[i];
			if (action.type === 'use' || action.type === 'bin') lots.splice(i, 1);
			if (action.type === 'open') l.opened = true;
			if (action.type === 'move') {
				l.location = action.to;
				l.frozenOn = action.to === 'freezer' ? iso(0) : null;
			}
			return ok(l);
		}
	};
}
