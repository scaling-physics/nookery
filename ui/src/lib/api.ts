// The UI's only door to the backend. Inside Tauri it calls the generated
// commands; in a plain browser (`pnpm -C ui dev`) it serves SYNTHETIC sample
// data so screens can be previewed and screenshotted without the app.

import { Channel } from '@tauri-apps/api/core';
import { commands } from './bindings.js';
import type {
	AddedToShoppingView,
	AddShoppingInput,
	CookedInput,
	CookedView,
	DayRef,
	DayView,
	FavouriteView,
	MakePlanInput,
	PlanResultView,
	PlanSummaryView,
	PlanView,
	PutAwayView,
	SaveFavouriteInput,
	SetDayCookedInput,
	SetShoppingDoneInput,
	ShoppingItemView,
	ShoppingView,
	AiModeView,
	ArtDishInput,
	ArtOption,
	ArtPickView,
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
	RecipeChatInput,
	RecipeChatView,
	RecipeView,
	ReceiptFileView,
	ReceiptImportInput,
	ReceiptLineView,
	ReceiptView,
	SaveTemplateInput,
	DownloadProgress,
	PhoneAiPref,
	PhoneAiView,
	ScanProgress,
	SyncStatusView,
	SuggestInput,
	SuggestionsView,
	TemplateView,
	WriteRecipeInput
} from './bindings.js';

import { matchDish } from './art/dishes.js';
import { addDays, groupShopping, nextMonday, isoDate } from './plan/helpers.js';

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

/** A progress channel for `downloadPhoneModel` (same idea as `progressChannel`). */
export function downloadChannel(handler: (p: DownloadProgress) => void): Channel<DownloadProgress> {
	if (inTauri) {
		const c = new Channel<DownloadProgress>();
		c.onmessage = handler;
		return c;
	}
	return { onmessage: handler } as unknown as Channel<DownloadProgress>;
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
	const CATEGORY: Record<string, LotView['category']> = {
		cream: 'dairy',
		spinach: 'vegetables',
		'greek yoghurt': 'dairy',
		pesto: 'sauces',
		eggs: 'eggs',
		halloumi: 'cheese',
		broccoli: 'vegetables',
		milk: 'dairy',
		butter: 'dairy',
		carrots: 'vegetables',
		'yellow box': 'other',
		'yellow sauce': 'other',
		cheddar: 'cheese',
		'leftover rice': 'leftovers',
		lemons: 'fruit',
		hummus: 'legumes',
		ketchup: 'sauces',
		'white tub': 'other',
		'minced beef': 'meat',
		peas: 'vegetables',
		'bread rolls': 'bread',
		'basmati rice': 'grains',
		chickpeas: 'legumes',
		'olive oil': 'sauces',
		buckwheat: 'grains',
		spaghetti: 'grains',
		oats: 'grains',
		'tinned tomatoes': 'vegetables',
		honey: 'sweets',
		lentils: 'legumes',
		'green jar': 'other',
		'coffee beans': 'drinks',
		flour: 'grains'
	};
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
			category: CATEGORY[item.toLowerCase()] ?? 'other',
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
	/** Remove `qty` ("200g", "2") from a left amount when the units agree and
	 *  something is left; otherwise null (all of it is gone). */
	const subtractQty = (left: string, qty: string): string | null => {
		const parse = (t: string) => {
			const m = /^\s*(\d+(?:[.,]\d+)?)\s*([a-z]*)\s*$/i.exec(t);
			return m ? { n: Number(m[1].replace(',', '.')), u: m[2].toLowerCase() } : null;
		};
		const a = parse(left);
		const b = parse(qty);
		if (!a || !b || a.u !== b.u || b.n >= a.n) return null;
		return `${Math.round((a.n - b.n) * 1000) / 1000}${a.u}`;
	};
	const lots: LotView[] = [
		// Fridge
		lot('cream', '200ml', 'fridge', -1, { opened: true }),
		lot('Spinach', '250g', 'fridge', 0, { store: 'REWE' }),
		lot('greek yoghurt', '500g', 'fridge', 1, { store: 'Lidl' }),
		lot('pesto', '1', 'fridge', 2, { opened: true }),
		lot('Eggs', '4', 'fridge', 9, { store: 'Lidl' }),
		lot('halloumi', '225g', 'fridge', 21),
		lot('broccoli', '1', 'fridge', 4),
		lot('milk', '1l', 'fridge', 6, { store: 'REWE' }),
		lot('Butter', '250g', 'fridge', 28, { opened: true }),
		lot('carrots', '600g', 'fridge', 12),
		lot('yellow box', '1', 'fridge', null),
		lot('yellow sauce', '1', 'fridge', null, { opened: true }),
		lot('Cheddar', '200g', 'fridge', 18, { store: 'Lidl' }),
		lot('leftover rice', '2', 'fridge', 1),
		lot('lemons', '3', 'fridge', 10),
		lot('hummus', '200g', 'fridge', null, { status: 'unconfirmed', days: 15 }),
		lot('ketchup', '1', 'fridge', 90, { opened: true }),
		lot('white tub', '2', 'fridge', null, { status: 'unconfirmed', days: 22 }),
		// Freezer
		lot('minced beef', '500g', 'freezer', null, { frozenOn: iso(-12) }),
		lot('Peas', '750g', 'freezer', null, { frozenOn: iso(-40) }),
		lot('bread rolls', '6', 'freezer', null, { frozenOn: iso(-3) }),
		// Pantry
		lot('basmati rice', '1.5kg', 'pantry', 300),
		lot('chickpeas', '2', 'pantry', 500),
		lot('olive oil', '750ml', 'pantry', null, { opened: true }),
		lot('Buckwheat', '500g', 'pantry', 200),
		lot('spaghetti', '500g', 'pantry', 400, { store: 'REWE' }),
		lot('oats', '1kg', 'pantry', 150, { opened: true }),
		lot('tinned tomatoes', '3', 'pantry', 600),
		lot('honey', '350g', 'pantry', null),
		lot('lentils', '500g', 'pantry', 320),
		lot('green jar', '1', 'pantry', null, { status: 'unconfirmed', days: 40 }),
		lot('Coffee beans', '250g', 'pantry', 60, { opened: true }),
		lot('flour', '1kg', 'pantry', -3)
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
	// SYNTHETIC receipt text (the REWE test fixture), "read from a photo".
	const SAMPLE_RECEIPT_TEXT = `REWE Markt GmbH
Beispielstrasse 12
50667 Koeln
Tel.: 0221 000000
UID Nr.: DE000000000
--------------------------------
BIO H-MILCH 1,5%             1,19 B
JOGHURT NATUR                1,98 B
   2 Stk x      0,99
BANANEN                      1,53 B
   0,512 kg x   2,99 EUR/kg
MINERALWASSER 1,5L           0,79 A
PFAND EINWEG                 0,25 A
KAESE GOUDA                  2,49 B
Sofort-Rabatt               -0,50
BROT DINKEL                  2,29 B
LEERGUTBON                  -0,25
--------------------------------
SUMME                EUR     9,77
Geg. BAR                    20,00
Rueckgeld                   10,23
Steuer %     Netto    Steuer   Brutto
A 19%         0,87      0,17     1,04
B  7%         7,67      0,54     8,21
08.11.26 14:32
TSE-Signatur: 0000000000
Vielen Dank fuer Ihren Einkauf!
`;
	// SYNTHETIC photo review: invented, no image involved.
	// SYNTHETIC AI settings; "signed in" flips on the first cloud check, so
	// both states can be previewed.
	let ai: AiSettingsView = {
		recipeArt: 'matched',
		photos: 'local',
		recipes: 'local',
		cloudModel: 'gemma4:31b-cloud'
	};
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
				needCategories: [],
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
				needCategories: ['spices', 'vegetables'],
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
				needCategories: ['sauces'],
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
				needCategories: ['fruit', 'spices'],
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
		ingredientCategories: ['vegetables', 'eggs', 'dairy', 'cheese', 'sauces', 'spices'],
		steps: raw
			.split('\n')
			.filter((l) => /^\d+\. /.test(l))
			.map((l) => l.replace(/^\d+\. /, '')),
		raw,
		ms
	});

	// SYNTHETIC favourites, plans and shopping (in memory, gone on reload).
	let favourites: FavouriteView[] = [];
	let shopping: ShoppingItemView[] = [];
	let shoppingId = 0;
	const plans = new Map<string, PlanView>();
	const FRESH = new Set([
		'vegetables',
		'fruit',
		'herbs',
		'dairy',
		'cheese',
		'eggs',
		'meat',
		'fish'
	]);
	const categoryOf = (name: string): LotView['category'] => {
		const n = name.toLowerCase();
		const known = CATEGORY[n];
		if (known) return known;
		if (/onion|leek|kale|tomato|pepper|cucumber|garlic|lettuce|mushroom/.test(n))
			return 'vegetables';
		if (/lemon|apple|banana|lime|orange/.test(n)) return 'fruit';
		if (/basil|parsley|coriander|dill|mint/.test(n)) return 'herbs';
		if (/masala|paprika|cumin|chilli|spice/.test(n)) return 'spices';
		if (/soy|sauce|vinegar|tahini/.test(n)) return 'sauces';
		if (/cheese|feta|parmesan/.test(n)) return 'cheese';
		if (/bread|bun/.test(n)) return 'bread';
		return 'other';
	};
	const shoppingView = (): ShoppingView => ({
		groups: groupShopping(shopping),
		open: shopping.filter((i) => !i.done).length,
		done: shopping.filter((i) => i.done).length
	});
	const planDays = (week: string, count: number): { days: DayView[]; spares: DayView[] } => {
		const dish = (
			name: string,
			why: string,
			minutes: number,
			have: string[],
			need: string[],
			soon: number
		): DayView => ({
			date: '',
			name,
			why,
			minutes,
			have: named(...have),
			need,
			needCategories: need.map(categoryOf),
			usesSoon: soon,
			cooked: false
		});
		const all = [
			dish(
				'Spinach and halloumi omelette',
				'Uses the spinach that needs eating first.',
				15,
				['spinach', 'Eggs', 'halloumi'],
				[],
				1
			),
			dish(
				'Creamy spinach and chickpea curry',
				'Yoghurt and spinach are close to their date.',
				30,
				['greek yoghurt', 'chickpeas', 'basmati rice'],
				['garam masala', 'onion'],
				1
			),
			dish(
				'Beef and pea rice bowl',
				'The freezer mince and peas, with yoghurt on the side.',
				25,
				['minced beef', 'Peas', 'basmati rice'],
				['soy sauce'],
				0
			),
			dish(
				'Pesto spaghetti with broccoli',
				'The opened pesto wants finishing.',
				20,
				['pesto', 'spaghetti', 'broccoli'],
				['parmesan'],
				1
			),
			dish(
				'Lentil and carrot soup',
				'Cheap, filling, and uses the carrots.',
				35,
				['lentils', 'carrots'],
				['onion', 'vegetable stock'],
				0
			),
			dish(
				'Halloumi and chickpea traybake',
				'One tray; nothing much to buy.',
				35,
				['halloumi', 'chickpeas', 'olive oil'],
				['lemon'],
				0
			),
			dish(
				'Cheddar and leftover rice bake',
				'Rescues the leftover rice.',
				25,
				['Cheddar', 'leftover rice', 'Eggs'],
				['spring onion'],
				1
			),
			dish(
				'Buckwheat porridge with honey',
				'A light supper.',
				15,
				['Buckwheat', 'milk', 'honey'],
				['apple'],
				0
			),
			dish('Oat pancakes', 'Eggs, oats and milk.', 20, ['oats', 'Eggs', 'milk'], ['banana'], 0)
		];
		const days = all.slice(0, count).map((d, i) => ({ ...d, date: addDays(week, i) }));
		return { days, spares: all.slice(count) };
	};
	const fakePlanView = (p: PlanView): PlanView => structuredClone(p);
	const suggestionSets = new Map<string, SuggestionsView>();
	const phone =
		typeof location !== 'undefined' && new URLSearchParams(location.search).has('phone');
	const sync: {
		sharing: boolean;
		code: string;
		peer: string | null;
		last: SyncStatusView['last'];
		clash: boolean;
	} = {
		sharing: false,
		code: 'K7QX-2MDA-PW4R-HNBE-3TYC-ZUA2-MD',
		peer: null,
		last: null,
		clash: true
	};
	const syncView = (): SyncStatusView => ({
		role: phone ? 'phone' : 'mac',
		sharing: sync.sharing,
		addresses: phone ? [] : ['192.168.1.20'],
		port: 47480,
		code: phone ? null : sync.code,
		link:
			!phone && sync.sharing
				? `nookery://pair?code=${sync.code}&addr=192.168.1.20%3A47480&name=Mac`
				: null,
		qr: !phone && sync.sharing ? fakeQr() : null,
		peer: phone ? sync.peer : null,
		peerName: phone && sync.peer ? 'Mac' : null,
		last: sync.last
	});
	// A stand-in picture for the QR code (not scannable; the app draws a real one).
	const fakeQr = () => {
		const n = 25;
		let rects = '';
		let x = 7;
		for (let r = 0; r < n; r++)
			for (let c = 0; c < n; c++) {
				x = (x * 1103515245 + 12345) & 0x7fffffff;
				const finder = (r < 8 && (c < 8 || c > n - 9)) || (r > n - 9 && c < 8);
				if (
					finder
						? r % 7 === 0 || c % 7 === 0 || (r % 7 > 1 && r % 7 < 5 && c % 7 > 1 && c % 7 < 5)
						: x % 3 === 0
				)
					rects += `<rect x="${c + 2}" y="${r + 2}" width="1" height="1"/>`;
			}
		return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n + 4} ${n + 4}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="#fffdf8"/><g fill="#2a2118">${rects}</g></svg>`;
	};
	const phoneAi: { pref: PhoneAiPref; downloaded: boolean; part: number; running: boolean } = {
		pref: 'auto',
		downloaded: false,
		part: 0,
		running: false
	};
	let stopDownload = false;
	const MODEL_BYTES = 2_497_281_120;
	const phoneAiView = (): PhoneAiView => ({
		available: phone,
		pref: phoneAi.pref,
		paired: !!sync.peer,
		model: {
			downloaded: phoneAi.downloaded,
			bytes: phoneAi.downloaded ? MODEL_BYTES : 0,
			partialBytes: phoneAi.part,
			freeBytes: 41_300_000_000,
			expectedBytes: MODEL_BYTES,
			downloading: phoneAi.running
		}
	});
	const ok = <T>(data: T): Promise<Result<T>> => Promise.resolve({ status: 'ok', data });
	return {
		// ---- the phone's own AI (SYNTHETIC; `?phone` shows it) ----
		getPhoneAi: (): Promise<Result<PhoneAiView>> => ok(phoneAiView()),
		setPhoneAi: (pref: PhoneAiPref): Promise<Result<PhoneAiView>> => {
			phoneAi.pref = pref;
			return ok(phoneAiView());
		},
		downloadPhoneModel: async (
			onProgress: Channel<DownloadProgress>
		): Promise<Result<PhoneAiView>> => {
			if (!sync.peer)
				return {
					status: 'error',
					error: { kind: 'mac_unreachable', detail: 'No Mac is paired yet.' }
				};
			phoneAi.running = true;
			stopDownload = false;
			while (phoneAi.part < MODEL_BYTES) {
				await new Promise((r) => setTimeout(r, 120));
				if (stopDownload) {
					phoneAi.running = false;
					return { status: 'error', error: { kind: 'bad_input', detail: 'cancelled' } };
				}
				phoneAi.part = Math.min(MODEL_BYTES, phoneAi.part + 60_000_000);
				onProgress.onmessage({ done: phoneAi.part, total: MODEL_BYTES });
			}
			phoneAi.running = false;
			phoneAi.downloaded = true;
			phoneAi.part = 0;
			return ok(phoneAiView()).then((r) => r);
		},
		cancelPhoneDownload: (): Promise<Result<null>> => {
			stopDownload = true;
			return ok(null);
		},
		deletePhoneModel: (): Promise<Result<PhoneAiView>> => {
			phoneAi.downloaded = false;
			phoneAi.part = 0;
			return ok(phoneAiView());
		},
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
		readReceiptFile: async (file: string): Promise<Result<ReceiptFileView>> => {
			if (!file) return { status: 'error', error: { kind: 'bad_input', detail: 'file' } };
			await new Promise((r) => setTimeout(r, 700));
			return ok({
				text: SAMPLE_RECEIPT_TEXT,
				method: 'ocr',
				ms: 142,
				pages: 1,
				readBy: 'this Mac'
			});
		},
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
		// SYNTHETIC cook: canned answers, no model. Questions get a plain reply;
		// requests for a change also get a revised recipe.
		recipeChat: async (
			input: RecipeChatInput,
			onText: Channel<string>
		): Promise<Result<RecipeChatView>> => {
			const m = input.message.toLowerCase();
			if (!m.trim()) return fail('bad_input', 'message');
			const r = input.recipe;
			const change = (
				reply: string,
				title: string,
				ing: string[],
				steps: string[],
				serves = r.serves
			) => ({
				reply,
				raw:
					`${reply}\n---\n# ${title}\nServes: ${serves ?? 2}\nTime: ${r.minutes ?? 25} min\n## Ingredients\n` +
					ing.map((i) => `- ${i}`).join('\n') +
					'\n## Steps\n' +
					steps.map((s, i) => `${i + 1}. ${s}`).join('\n'),
				title,
				ing,
				steps,
				serves
			});
			let a: ReturnType<typeof change> | { reply: string; raw: string };
			if (/vegetarian|vegan|meat/.test(m)) {
				a = change(
					'Easy: I swapped the cheese for chickpeas so it stays hearty and meat-free.',
					r.title.replace(/halloumi/i, 'chickpea'),
					[
						...r.ingredients.filter((i) => !/halloumi|beef|chicken/i.test(i)),
						'1 tin chickpeas, drained'
					],
					r.steps.map((s) => s.replace(/halloumi/gi, 'chickpeas'))
				);
			} else if (/people|serves|for \d/.test(m)) {
				const n = Number(m.match(/\d+/)?.[0] ?? 4);
				a = change(
					`Scaled up for ${n} people: I doubled the amounts and added a few minutes.`,
					r.title,
					r.ingredients.map((i) => i.replace(/^(\d+)/, (d) => String(Number(d) * 2))),
					r.steps,
					n
				);
			} else if (/oven|no oven/.test(m)) {
				a = change(
					'No oven needed: everything happens in one pan on the hob.',
					r.title,
					r.ingredients,
					[
						...r.steps.slice(0, -1),
						'Cover the pan and let it finish gently on low heat for 5 minutes.'
					]
				);
			} else if (/fewer|simpler|less/.test(m)) {
				a = change(
					'Simplified: down to the essentials, same idea.',
					r.title,
					r.ingredients.slice(0, Math.max(2, r.ingredients.length - 2)),
					r.steps
				);
			} else if (/spic|hot|chilli|zucchini|use the/.test(m)) {
				a = change(
					'Spiced it up with chilli flakes and a pinch of cumin.',
					r.title,
					[...r.ingredients, '1 tsp chilli flakes', '1/2 tsp cumin'],
					r.steps.map((s, i) => (i === 0 ? `${s} Add the chilli and cumin now.` : s))
				);
			} else if (/swap|instead|replace|substitut/.test(m)) {
				const reply =
					'From your kitchen: feta or a firm goat cheese works for halloumi, and spinach swaps happily for chard or kale. Greek yoghurt stands in for cream.';
				a = { reply, raw: reply };
			} else if (/freez/.test(m)) {
				const reply =
					'Yes. Let it cool, then freeze in a sealed box for up to 2 months. Reheat gently in a pan; the eggs stay a little firmer.';
				a = { reply, raw: reply };
			} else {
				const reply =
					'Good question. I would keep it simple and taste as you go; the recipe is forgiving. Tell me what you would like to change and I will rewrite it.';
				a = { reply, raw: reply };
			}
			const chunks = a.raw.match(/[\s\S]{1,12}/g) ?? [];
			for (const c of chunks) {
				await new Promise((res) => setTimeout(res, 1600 / chunks.length));
				onText.onmessage(c);
			}
			const revised: RecipeView | null =
				'ing' in a
					? {
							...answeredBy(ai.recipes),
							title: a.title,
							serves: a.serves,
							minutes: r.minutes,
							ingredients: a.ing,
							ingredientCategories: a.ing.map((i) =>
								categoryOf(i.replace(/^[\d./,\s]*(g|kg|ml|l|tbsp|tsp|tin)?\s+/i, ''))
							),
							steps: a.steps,
							raw: a.raw,
							ms: 1600
						}
					: null;
			return {
				status: 'ok',
				data: { reply: a.reply, recipe: revised, ...answeredBy(ai.recipes), ms: 1600 }
			};
		},
		saveFavourite: (input: SaveFavouriteInput): Promise<Result<FavouriteView>> => {
			const title = input.title.trim();
			if (!title) return fail('bad_input', 'title');
			const same = favourites.find((f) => f.recipe.title.toLowerCase() === title.toLowerCase());
			if (same) return ok(same);
			const f: FavouriteView = {
				id: `fav-${favourites.length + 1}`,
				recipe: {
					title,
					serves: input.serves,
					minutes: input.minutes,
					ingredients: input.ingredients,
					ingredientCategories: input.ingredients.map((i) =>
						categoryOf(i.replace(/^[\d.,\s]*(g|kg|ml|l|tbsp|tsp)?\s+/i, ''))
					),
					steps: input.steps
				},
				why: input.why,
				savedAt: new Date().toISOString(),
				cooked: 0
			};
			favourites = [f, ...favourites];
			return ok(f);
		},
		updateFavourite: (id: string, input: SaveFavouriteInput): Promise<Result<FavouriteView>> => {
			const f = favourites.find((x) => x.id === id);
			if (!f) return fail('not_found');
			const title = input.title.trim();
			if (!title) return fail('bad_input', 'title');
			f.recipe = {
				title,
				serves: input.serves,
				minutes: input.minutes,
				ingredients: [...input.ingredients],
				ingredientCategories: input.ingredients.map((i) =>
					categoryOf(i.replace(/^[\d.,\s]*(g|kg|ml|l|tbsp|tsp)?\s+/i, ''))
				),
				steps: [...input.steps]
			};
			return ok(structuredClone(f));
		},
		listFavourites: (): Promise<Result<FavouriteView[]>> => ok(structuredClone(favourites)),
		deleteFavourite: (id: string): Promise<Result<null>> => {
			if (!favourites.some((f) => f.id === id)) return fail('not_found');
			favourites = favourites.filter((f) => f.id !== id);
			return ok(null);
		},
		cooked: (input: CookedInput): Promise<Result<CookedView>> => {
			// Check everything first, then write (all or nothing).
			const plan: { i: number; qty: string | null }[] = [];
			for (const [k, it] of input.items.entries()) {
				const i = lots.findIndex((l) => l.short === it.lot || l.id === it.lot);
				if (i < 0) return fail('bad_input', `lot:${k}`);
				if (it.amount !== null) {
					const m = /^\s*(\d+(?:[.,]\d+)?)\s*([a-z]*)\s*$/i.exec(it.amount);
					const l = /^\s*(\d+(?:[.,]\d+)?)\s*([a-z]*)\s*$/i.exec(lots[i].left);
					const unit = (u: string) => (u === 'kg' ? 'g' : u === 'l' ? 'ml' : u);
					const amt = (x: RegExpExecArray) =>
						Number(x[1].replace(',', '.')) * (/^(kg|l)$/i.test(x[2]) ? 1000 : 1);
					if (
						!m ||
						!l ||
						unit(m[2].toLowerCase()) !== unit(l[2].toLowerCase()) ||
						amt(m) <= 0 ||
						amt(m) > amt(l)
					)
						return fail('bad_input', `amount:${k}`);
				}
				plan.push({ i, qty: it.amount });
			}
			let usedUp = 0;
			const gone: LotView[] = [];
			for (const { i, qty } of plan) {
				const l = lots[i];
				const rest = qty ? subtractQty(l.left, qty) : null;
				if (rest) l.left = rest;
				else {
					gone.push(l);
					usedUp++;
				}
			}
			for (const l of gone) lots.splice(lots.indexOf(l), 1);
			const t = input.title?.trim().toLowerCase();
			for (const f of favourites) if (t && f.recipe.title.toLowerCase() === t) f.cooked++;
			return ok({ lots: plan.length, usedUp });
		},
		makePlan: async (input: MakePlanInput): Promise<Result<PlanResultView>> => {
			if (input.days < 1 || input.days > 7) return fail('bad_input', 'days');
			await new Promise((r) => setTimeout(r, 1800));
			if (ai.recipes === 'cloud' && !signedIn) return signInError();
			const start = input.start ?? nextMonday(isoDate(new Date()));
			const monday = nextMonday(addDays(start, -6));
			const { days, spares } = planDays(start, input.days);
			const t = templates.find((x) => x.id === input.templateId);
			const plan: PlanView = {
				week: monday,
				days,
				spares,
				template: t?.name ?? null,
				note: input.note.trim(),
				createdAt: new Date().toISOString()
			};
			plans.set(monday, plan);
			return ok({ plan: fakePlanView(plan), ...answeredBy(ai.recipes), ms: 1800 });
		},
		getPlan: (week: string): Promise<Result<PlanView>> => {
			const p = plans.get(week);
			return p ? ok(fakePlanView(p)) : fail('not_found');
		},
		listPlans: (): Promise<Result<PlanSummaryView[]>> =>
			ok(
				[...plans.values()]
					.sort((a, b) => b.week.localeCompare(a.week))
					.map((p) => ({
						week: p.week,
						days: p.days.length,
						cooked: p.days.filter((d) => d.cooked).length,
						createdAt: p.createdAt
					}))
			),
		swapDay: (input: DayRef): Promise<Result<PlanView>> => {
			const p = plans.get(input.week);
			const i = p?.days.findIndex((d) => d.date === input.date && !d.cooked) ?? -1;
			if (!p || i < 0 || p.spares.length === 0) return fail('not_found');
			const next = p.spares.shift()!;
			const old = p.days[i];
			p.days[i] = { ...next, date: input.date };
			p.spares.push({ ...old, date: '' });
			return ok(fakePlanView(p));
		},
		setDayCooked: (input: SetDayCookedInput): Promise<Result<PlanView>> => {
			const p = plans.get(input.week);
			const d = p?.days.find((x) => x.date === input.date);
			if (!p || !d) return fail('not_found');
			d.cooked = input.cooked;
			return ok(fakePlanView(p));
		},
		deletePlan: (week: string): Promise<Result<null>> =>
			plans.delete(week) ? ok(null) : fail('not_found'),
		listShopping: (): Promise<Result<ShoppingView>> => ok(structuredClone(shoppingView())),
		addShopping: (input: AddShoppingInput): Promise<Result<ShoppingItemView>> => {
			const item = input.item.trim();
			if (!item) return fail('bad_input', 'item');
			const same = shopping.find((s) => !s.done && s.item.toLowerCase() === item.toLowerCase());
			if (same) return ok({ ...same });
			const s: ShoppingItemView = {
				id: `shop-${++shoppingId}`,
				item,
				qty: input.qty?.trim() || null,
				category: categoryOf(item),
				done: false,
				from: 'manual'
			};
			shopping.push(s);
			return ok({ ...s });
		},
		addPlanToShopping: (week: string): Promise<Result<AddedToShoppingView>> => {
			const p = plans.get(week);
			if (!p) return fail('not_found');
			const wanted = new Map<string, { name: string; cat: LotView['category'] }>();
			for (const d of p.days.filter((x) => !x.cooked))
				d.need.forEach((n, i) => {
					const k = n.toLowerCase();
					if (!wanted.has(k)) wanted.set(k, { name: n, cat: d.needCategories[i] ?? 'other' });
				});
			const added: ShoppingItemView[] = [];
			let skipped = 0;
			for (const { name, cat } of wanted.values()) {
				const k = name.toLowerCase();
				const listed = shopping.some((s) => !s.done && s.item.toLowerCase().includes(k));
				const inStock = lots.some((l) => l.item.toLowerCase().includes(k));
				if (listed || inStock) {
					skipped++;
					continue;
				}
				const s: ShoppingItemView = {
					id: `shop-${++shoppingId}`,
					item: name,
					qty: null,
					category: cat,
					done: false,
					from: `plan:${week}`
				};
				shopping.push(s);
				added.push({ ...s });
			}
			return ok({ added, skipped });
		},
		setShoppingDone: (input: SetShoppingDoneInput): Promise<Result<ShoppingItemView>> => {
			const s = shopping.find((x) => x.id === input.id);
			if (!s) return fail('not_found');
			s.done = input.done;
			return ok({ ...s });
		},
		deleteShopping: (id: string): Promise<Result<null>> => {
			if (!shopping.some((x) => x.id === id)) return fail('not_found');
			shopping = shopping.filter((x) => x.id !== id);
			return ok(null);
		},
		clearDoneShopping: (): Promise<Result<number>> => {
			const n = shopping.filter((x) => x.done).length;
			shopping = shopping.filter((x) => !x.done);
			return ok(n);
		},
		putAwayShopping: (): Promise<Result<PutAwayView>> => {
			const done = shopping.filter((x) => x.done);
			const added = done.map((x) =>
				lot(x.item, x.qty ?? '1', FRESH.has(x.category) ? 'fridge' : 'pantry', null)
			);
			lots.push(...added);
			shopping = shopping.filter((x) => !x.done);
			return ok({ lots: added, items: done.map((x) => ({ ...x })) });
		},
		// SYNTHETIC picture picks: the word matcher's kind and the first three
		// ingredients, after a short wait.
		pickDishArt: async (
			dishes: ArtDishInput[],
			options: ArtOption[]
		): Promise<Result<(ArtPickView | null)[]>> => {
			await new Promise((r) => setTimeout(r, 300));
			const known = new Set(options.map((o) => o.id));
			return {
				status: 'ok',
				data: dishes.map((d) => {
					const kind = matchDish(d.title, d.ingredients).id;
					return known.has(kind) ? { kind, show: d.ingredients.slice(0, 3) } : null;
				})
			};
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
		// ---- sync (SYNTHETIC; `?phone` previews the phone's side) ----
		getSyncStatus: (): Promise<Result<SyncStatusView>> => ok(syncView()),
		setSharing: (on: boolean): Promise<Result<SyncStatusView>> => {
			sync.sharing = on;
			return ok(syncView());
		},
		newPairingCode: (): Promise<Result<SyncStatusView>> => {
			sync.code =
				sync.code === 'K7QX-2MDA-PW4R-HNBE-3TYC-ZUA2-MD'
					? 'HG5V-9RLE-QJ2N-XA6D-4WPS-MKR2-ET'
					: 'K7QX-2MDA-PW4R-HNBE-3TYC-ZUA2-MD';
			return ok(syncView());
		},
		pairWithMac: async (code: string, address: string): Promise<Result<SyncStatusView>> => {
			await new Promise((r) => setTimeout(r, 500));
			if (code.replace(/[-\s]/g, '').length !== 26)
				return Promise.resolve({ status: 'error', error: { kind: 'sync_bad_code' } });
			if (address.includes('bad'))
				return Promise.resolve({
					status: 'error',
					error: {
						kind: 'sync_unreachable',
						detail:
							'Couldn’t reach the Mac. Is Nookery open there, sharing switched on, and are both on the same Wi-Fi?'
					}
				});
			sync.peer = address.trim();
			return ok(syncView());
		},
		forgetMac: (): Promise<Result<SyncStatusView>> => {
			sync.peer = null;
			sync.last = null;
			return ok(syncView());
		},
		syncNow: async (): Promise<Result<SyncStatusView>> => {
			await new Promise((r) => setTimeout(r, 700));
			sync.last = {
				at: new Date().toISOString(),
				ok: true,
				pushed: 1,
				pulled: 17,
				docsChanged: 2,
				dropped: sync.clash ? ['Used milk'] : [],
				error: null
			};
			sync.clash = !sync.clash;
			return ok(syncView());
		},
		lotAction: (id: string, action: LotAction): Promise<Result<LotView>> => {
			const i = lots.findIndex((l) => l.short === id);
			if (i < 0) return Promise.resolve({ status: 'error', error: { kind: 'not_found' } });
			const l = lots[i];
			switch (action.type) {
				case 'use': {
					const rest = action.qty ? subtractQty(l.left, action.qty) : null;
					if (rest) l.left = rest;
					else lots.splice(i, 1);
					break;
				}
				case 'bin':
				case 'wrong':
					lots.splice(i, 1);
					break;
				case 'open':
					l.opened = true;
					break;
				case 'seen':
					if (l.status === 'unconfirmed') {
						l.status = 'undated';
						l.days = null;
					}
					break;
				case 'move':
					l.location = action.to;
					l.frozenOn = action.to === 'freezer' ? iso(0) : null;
					break;
			}
			return ok(l);
		}
	};
}
