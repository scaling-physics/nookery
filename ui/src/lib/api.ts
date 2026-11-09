// The UI's only door to the backend. Inside Tauri it calls the generated
// commands; in a plain browser (`pnpm -C ui dev`) it serves SYNTHETIC sample
// data so screens can be previewed and screenshotted without the app.

import { Channel } from '@tauri-apps/api/core';
import { commands } from './bindings.js';
import type {
	AiModeView,
	AddInput,
	AiSettingsView,
	ApplyPhotoInput,
	ApplyPhotoResult,
	CloudCheckView,
	CommandError,
	ImportResultView,
	InventoryView,
	LotAction,
	LotView,
	PhotoItemView,
	PhotoReviewView,
	RecipeView,
	ReceiptImportInput,
	ReceiptLineView,
	ReceiptView,
	SaveTemplateInput,
	ScanProgress,
	SuggestInput,
	SuggestionsView,
	TemplateView,
	WriteRecipeInput
} from './bindings.js';

export type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: CommandError };

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** A progress channel for `scanPhoto`: a real Tauri Channel in the app, a
 *  plain object in a browser (where the fake API calls `onmessage`). */
export function progressChannel(handler: (p: ScanProgress) => void): Channel<ScanProgress> {
	if (inTauri) {
		const c = new Channel<ScanProgress>();
		c.onmessage = handler;
		return c;
	}
	return { onmessage: handler } as unknown as Channel<ScanProgress>;
}

/** A text channel for `writeRecipe`: a real Tauri Channel in the app, a plain
 *  object in a browser (where the fake API calls `onmessage`). */
export function textChannel(handler: (text: string) => void): Channel<string> {
	if (inTauri) {
		const c = new Channel<string>();
		c.onmessage = handler;
		return c;
	}
	return { onmessage: handler } as unknown as Channel<string>;
}

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
	// SYNTHETIC receipt (the REWE test fixture), with one "learned" line.
	const item = (
		index: number,
		text: string,
		amountCents: number,
		proposal: Partial<NonNullable<ReceiptLineView['proposal']>> = {},
		extra: Partial<ReceiptLineView> = {}
	): ReceiptLineView => ({
		index,
		kind: 'item',
		text,
		count: 1,
		weight: null,
		amountCents,
		proposal: {
			item: null,
			qty: '1',
			location: null,
			priceCents: amountCents,
			known: false,
			...proposal
		},
		...extra
	});
	const other = (
		index: number,
		kind: ReceiptLineView['kind'],
		text: string,
		amountCents: number
	): ReceiptLineView => ({
		index,
		kind,
		text,
		count: 1,
		weight: null,
		amountCents,
		proposal: null
	});
	const sampleLines = (): ReceiptLineView[] => [
		item(0, 'BIO H-MILCH 1,5%', 119, { item: 'milk', qty: '1l', location: 'fridge', known: true }),
		item(1, 'JOGHURT NATUR', 198, { qty: '2' }, { count: 2 }),
		item(2, 'BANANEN', 153, { qty: '512g' }, { weight: '512g' }),
		item(3, 'MINERALWASSER 1,5L', 79),
		other(4, 'deposit', 'PFAND EINWEG', 25),
		item(5, 'KAESE GOUDA', 199, { priceCents: 199 }, { amountCents: 249 }),
		other(6, 'discount', 'Sofort-Rabatt', -50),
		item(7, 'BROT DINKEL', 229),
		other(8, 'deposit_return', 'LEERGUTBON', -25)
	];
	const imported = new Set<string>();
	const sampleReceipt = (text: string, store: string | null): ReceiptView => {
		const lines = sampleLines();
		const broken = text.includes('BROKEN');
		return {
			id: broken ? 'f00dfeedf00dfeed' : 'a1b2c3d4e5f60718',
			store: store ?? (text.includes('REWE') ? 'REWE' : null),
			date: '2026-11-08',
			totalCents: 977 + (broken ? 5 : 0),
			linesCents: lines.reduce((s, l) => s + l.amountCents, 0),
			totalOk: !broken,
			alreadyImported: imported.has('a1b2c3d4e5f60718') && !broken,
			lines
		};
	};
	// SYNTHETIC photo review: invented, no image involved.
	// SYNTHETIC AI settings; "signed in" flips on the first cloud check, so
	// both states can be previewed.
	let ai: AiSettingsView = { photos: 'local', recipes: 'local', cloudModel: 'gemma4:31b-cloud' };
	let signedIn = false;
	const answeredBy = (mode: AiModeView) =>
		mode === 'cloud'
			? { model: `ollama ${ai.cloudModel} (cloud)`, usedCloud: true }
			: { model: 'ollama qwen3-vl:8b-instruct (this Mac)', usedCloud: false };
	const signInError = (): Result<never> => ({ status: 'error', error: { kind: 'cloud_sign_in' } });
	const photoPlans = new Map<string, number>();
	const sampleReview = (location: string): PhotoReviewView => {
		const here = lots.filter((l) => l.location === location);
		const seen = (
			seenAs: string,
			count: number,
			matches: LotView[],
			extra = {}
		): PhotoItemView => ({
			index: 0,
			seenAs,
			item: seenAs,
			learned: false,
			text: null,
			count,
			matches,
			...extra
		});
		const items: PhotoItemView[] = [
			...here.slice(0, 2).map((l) => seen(l.item, 1, [l])),
			seen('clear jar with green contents', 1, [], { item: 'pesto', learned: true }),
			seen('glass bottle with red label', 1, [], { text: 'Ketchup' }),
			seen('white tub', 2, []),
			seen('small yellow packet', 1, [], { text: 'Butter 250 g' })
		].map((it, index) => ({ ...it, index }));
		return {
			scanId: `scan-${photoPlans.size + 1}`,
			location,
			...answeredBy(ai.photos),
			totalMs: 2000,
			tiles: [0, 1, 2, 3].map((index) => ({ index, ms: 500, items: 2 })),
			items,
			missing: here.slice(2, 3)
		};
	};
	// SYNTHETIC recipe templates, seeded like the store's defaults.
	let templates: TemplateView[] = [
		['Use it up', 'Use as many of the use-soon items as possible. Simple, no-waste home cooking.'],
		['Quick weeknight', 'Dinner for two, ready in 30 minutes, one pan if possible.'],
		['High protein', 'High in protein, filling, not too heavy.'],
		['Vegetarian', 'Vegetarian (no meat or fish), hearty and colourful.'],
		['Something new', 'Surprise me: a dish from a cuisine I may not cook often.']
	].map(([name, body], i) => ({ id: i + 1, name, body }));
	let nextTemplate = templates.length + 1;
	const fail = (kind: 'bad_input' | 'not_found', detail?: string): Promise<Result<never>> =>
		Promise.resolve({
			status: 'error',
			error: kind === 'bad_input' ? { kind, detail: detail ?? '' } : { kind }
		} as Result<never>);
	const named = (...names: string[]) =>
		names.flatMap((n) => {
			const l = lots.find((x) => x.item === n);
			return l ? [{ ingredient: n, lot: l }] : [];
		});
	// SYNTHETIC suggestions: invented, no model involved.
	const sampleSuggestions = (): SuggestionsView => ({
		id: `sg-${++n}`,
		...answeredBy(ai.recipes),
		ms: 1500,
		items: [
			{
				index: 0,
				name: 'Spinach and halloumi omelette',
				why: 'Uses the spinach that needs eating today, with eggs and halloumi to hold it together.',
				minutes: 15,
				have: named('spinach', 'eggs', 'halloumi'),
				need: [],
				expired: [],
				usesSoon: 1,
				score: 6
			},
			{
				index: 1,
				name: 'Creamy spinach and chickpea curry',
				why: 'Spinach and yoghurt are close to their date; chickpeas and rice make it a full meal.',
				minutes: 30,
				have: named('spinach', 'greek yoghurt', 'chickpeas', 'basmati rice'),
				need: ['garam masala', 'onion'],
				expired: lots.filter((l) => l.item === 'cream'),
				usesSoon: 2,
				score: 6
			},
			{
				index: 2,
				name: 'Beef and pea rice bowl',
				why: 'A quick use of the freezer mince and peas, with yoghurt on the side.',
				minutes: 25,
				have: named('minced beef', 'peas', 'basmati rice', 'greek yoghurt'),
				need: ['soy sauce'],
				expired: [],
				usesSoon: 1,
				score: 5
			},
			{
				index: 3,
				name: 'Halloumi and chickpea traybake',
				why: 'Everything goes in one tray; nothing to buy.',
				minutes: 35,
				have: named('halloumi', 'chickpeas', 'olive oil'),
				need: ['lemon', 'smoked paprika'],
				expired: [],
				usesSoon: 0,
				score: 1
			}
		]
	});
	const sampleRecipe = (title: string): string =>
		[
			`# ${title}`,
			'Serves: 2',
			'Time: 25 min',
			'',
			'## Ingredients',
			'- 250 g spinach',
			'- 4 eggs',
			'- 100 g halloumi, grated',
			'- 1 tbsp olive oil',
			'- salt and pepper',
			'',
			'## Steps',
			'1. Heat the oil in a wide pan and wilt the spinach for two minutes.',
			'2. Beat the eggs with a pinch of salt and pour them over the spinach.',
			'3. Scatter the halloumi on top and cook gently until the edges set.',
			'4. Fold, rest for a minute, and serve with the yoghurt.'
		].join('\n');
	const parseSample = (raw: string, title: string, ms: number): RecipeView => ({
		...answeredBy(ai.recipes),
		title,
		serves: 2,
		minutes: 25,
		ingredients: raw
			.split('\n')
			.filter((l) => l.startsWith('- '))
			.map((l) => l.slice(2)),
		steps: raw
			.split('\n')
			.filter((l) => /^\d+\. /.test(l))
			.map((l) => l.replace(/^\d+\. /, '')),
		raw,
		ms
	});
	const suggestionSets = new Map<string, SuggestionsView>();
	const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: 'ok', data });
	return {
		getInventory: (): Promise<Result<InventoryView>> =>
			ok({ lots: [...lots], events: lots.length, openMs: 0.4, queryMs: 0.05 }),
		addItem: (input: AddInput): Promise<Result<LotView>> => {
			const l = lot(input.item, input.qty || '1', input.location, null, { store: input.store });
			lots.push(l);
			return ok(l);
		},
		parseReceipt: (text: string, store: string | null): Promise<Result<ReceiptView>> =>
			text.trim()
				? ok(sampleReceipt(text, store))
				: Promise.resolve({ status: 'error', error: { kind: 'bad_input', detail: 'text' } }),
		importReceipt: (input: ReceiptImportInput): Promise<Result<ImportResultView>> => {
			imported.add('a1b2c3d4e5f60718');
			for (const l of input.lines)
				lots.push(lot(l.item, l.qty, l.location, null, { store: input.store }));
			return ok({ added: input.lines.length, learned: input.lines.length });
		},
		scanPhoto: async (
			imageBase64: string,
			location: string,
			onProgress: Channel<ScanProgress>
		): Promise<Result<PhotoReviewView>> => {
			if (!imageBase64) return { status: 'error', error: { kind: 'bad_input', detail: 'image' } };
			if (ai.photos === 'cloud' && !signedIn) return signInError();
			for (let done = 1; done <= 4; done++) {
				await new Promise((r) => setTimeout(r, 500));
				onProgress.onmessage({ done, total: 4 });
			}
			const review = sampleReview(location);
			photoPlans.set(review.scanId, review.items.length);
			return { status: 'ok', data: review };
		},
		applyPhoto: (input: ApplyPhotoInput): Promise<Result<ApplyPhotoResult>> => {
			if (!photoPlans.has(input.scanId))
				return Promise.resolve({ status: 'error', error: { kind: 'not_found' } });
			photoPlans.delete(input.scanId);
			const count = (a: string) => input.decisions.filter((d) => d.action === a);
			for (const d of count('add'))
				lots.push(lot(d.item ?? 'item', d.qty || '1', 'fridge', null, { store: null }));
			return ok({ seen: count('seen').length, added: count('add').length, learned: 0 });
		},
		listTemplates: (): Promise<Result<TemplateView[]>> => ok(templates.map((t) => ({ ...t }))),
		saveTemplate: (input: SaveTemplateInput): Promise<Result<TemplateView>> => {
			const name = input.name.trim();
			const body = input.body.trim();
			if (!name) return fail('bad_input', 'name');
			if (!body) return fail('bad_input', 'body');
			if (templates.some((t) => t.name === name && t.id !== input.id))
				return fail('bad_input', 'name');
			if (input.id === null) {
				const t = { id: nextTemplate++, name, body };
				templates.push(t);
				return ok({ ...t });
			}
			const t = templates.find((x) => x.id === input.id);
			if (!t) return fail('not_found');
			t.name = name;
			t.body = body;
			return ok({ ...t });
		},
		deleteTemplate: (id: number): Promise<Result<null>> => {
			if (!templates.some((t) => t.id === id)) return fail('not_found');
			templates = templates.filter((t) => t.id !== id);
			return ok(null);
		},
		orderTemplates: (ids: number[]): Promise<Result<null>> => {
			const same = ids.length === templates.length && templates.every((t) => ids.includes(t.id));
			if (!same) return fail('not_found');
			templates = ids.map((id) => templates.find((t) => t.id === id)!);
			return ok(null);
		},
		suggestRecipes: async (input: SuggestInput): Promise<Result<SuggestionsView>> => {
			await new Promise((r) => setTimeout(r, 1500));
			if (ai.recipes === 'cloud' && !signedIn) return signInError();
			const s = sampleSuggestions();
			s.items = s.items.slice(0, Math.max(1, input.count));
			suggestionSets.set(s.id, s);
			return { status: 'ok', data: s };
		},
		writeRecipe: async (
			input: WriteRecipeInput,
			onText: Channel<string>
		): Promise<Result<RecipeView>> => {
			const s = suggestionSets.get(input.suggestionsId);
			const item = s?.items[input.index];
			if (!item) return { status: 'error', error: { kind: 'not_found' } };
			const raw = sampleRecipe(item.name);
			const chunks = raw.match(/[\s\S]{1,14}/g) ?? [];
			for (const c of chunks) {
				await new Promise((r) => setTimeout(r, 2000 / chunks.length));
				onText.onmessage(c);
			}
			return { status: 'ok', data: parseSample(raw, item.name, 2000) };
		},
		getAiSettings: (): Promise<Result<AiSettingsView>> => ok({ ...ai }),
		setAiSettings: (input: AiSettingsView): Promise<Result<AiSettingsView>> => {
			ai = { ...input, cloudModel: input.cloudModel.trim() || 'gemma4:31b-cloud' };
			return ok({ ...ai });
		},
		checkCloud: async (): Promise<Result<CloudCheckView>> => {
			await new Promise((r) => setTimeout(r, 600));
			const was = signedIn;
			signedIn = true;
			return {
				status: 'ok',
				data: { signedIn: was, model: ai.cloudModel, ms: was ? 800 : 400, detail: null }
			};
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
