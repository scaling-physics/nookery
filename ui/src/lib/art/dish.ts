// Pure planning for DishArt: which vessel a recipe title suggests, which
// toppings its food categories suggest, and a seeded layout so the same
// recipe always gets the same picture. No DOM here (unit-tested).

import type { FoodCategoryView } from '../bindings.js';

export type Vessel =
	| 'plate'
	| 'salad'
	| 'bowl'
	| 'deep'
	| 'small'
	| 'glass'
	| 'pan'
	| 'wok'
	| 'board'
	| 'wrap'
	| 'round'
	| 'slice';

export type Topping =
	| 'leaf'
	| 'wedge'
	| 'tomato'
	| 'herb'
	| 'lemon'
	| 'cube'
	| 'dollop'
	| 'egg'
	| 'slice'
	| 'fillet'
	| 'grain'
	| 'bean'
	| 'swoosh'
	| 'sprinkle'
	| 'nut'
	| 'berry'
	| 'crouton'
	| 'dot';

/** FNV-1a, 32 bit. */
export function hash(s: string): number {
	let h = 0x811c9dc5;
	for (let i = 0; i < s.length; i++) {
		h ^= s.charCodeAt(i);
		h = Math.imul(h, 0x01000193);
	}
	return h >>> 0;
}

/** mulberry32: a tiny seeded PRNG returning [0, 1). */
export function mulberry32(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

// Ordered: the first rule that matches wins ("pasta salad" is a plate of
// pasta, "chicken curry bowl" is a curry).
const RULES: [RegExp, Vessel][] = [
	[/\b(wrap|wraps|taco|tacos|burrito|quesadilla|fajita|gyros?|d[oö]ner|kebab)\b/, 'wrap'],
	[/\b(pizza|flammkuchen|flatbread|focaccia|naan|pide)\b/, 'round'],
	[
		/\b(soup|stew|curry|dal|dahl|daal|chili|chilli|chowder|broth|ramen|pho|minestrone|eintopf|suppe|gulasch|goulash|risotto|congee)\b/,
		'deep'
	],
	[/\b(stir[- ]?fr(y|ied)|wok|chow mein|pad thai|fried rice|lo mein)\b/, 'wok'],
	[
		/\b(pasta|spaghetti|noodles?|penne|linguine|tagliatelle|fusilli|lasagn?e|gnocchi|carbonara|nudeln|ravioli|mac and cheese|orzo)\b/,
		'plate'
	],
	[/\b(salad|salat|slaw|tabbouleh|caprese)\b/, 'salad'],
	[
		/\b(skillet|pan|frittata|omelet+e?|shakshuka|hash|tortilla espanola|pfanne|r[oö]sti|pancakes?|fritters?|scramble|scrambled|baked|bake|gratin|casserole|auflauf)\b/,
		'pan'
	],
	[
		/\b(toast|toasts|sandwich|bruschetta|crostini|tartine|panini|sub|bagel|stulle|brot|schnitte|melt)\b/,
		'board'
	],
	[/\b(porridge|oats|oatmeal|muesli|m[uü]sli|granola|haferbrei|overnight)\b/, 'small'],
	[/\b(yogh?urt|skyr|parfait|smoothie|pudding|quark|lassi)\b/, 'glass'],
	[/\b(cake|dessert|crumble|pie|tart|brownie|cheesecake|kuchen|muffin|cobbler|torte)\b/, 'slice'],
	[/\b(bowl|rice|buddha|grain|poke|pilaf|quinoa|couscous|reis|schale|bulgur|medley)\b/, 'bowl']
];

export function vesselFor(title: string): Vessel {
	const t = title.toLowerCase();
	for (const [re, v] of RULES) if (re.test(t)) return v;
	return 'bowl';
}

/** Dishes that steam. */
export function isHot(vessel: Vessel, title = ''): boolean {
	if (['deep', 'pan', 'wok', 'small'].includes(vessel)) return true;
	if (['salad', 'glass', 'slice', 'board'].includes(vessel)) return false;
	return !/\b(cold|chilled|salad|overnight|iced)\b/i.test(title);
}

const TOPPINGS: Partial<Record<FoodCategoryView, Topping[]>> = {
	vegetables: ['leaf', 'tomato', 'wedge'],
	herbs: ['herb'],
	fruit: ['lemon', 'berry'],
	cheese: ['cube'],
	dairy: ['dollop'],
	eggs: ['egg'],
	meat: ['slice'],
	fish: ['fillet'],
	grains: ['grain'],
	bread: ['crouton'],
	legumes: ['bean'],
	sauces: ['swoosh'],
	spices: ['sprinkle'],
	nuts: ['nut'],
	sweets: ['berry'],
	leftovers: ['dot'],
	other: ['dot']
};

/** Distinct topping kinds for the categories, in order, at most `max`. */
export function toppingKinds(categories: readonly FoodCategoryView[], max = 6): Topping[] {
	const out: Topping[] = [];
	for (const c of categories) {
		for (const t of TOPPINGS[c] ?? []) {
			if (!out.includes(t)) out.push(t);
			if (out.length >= max) return out;
		}
	}
	return out;
}

export type Ellipse = { cx: number; cy: number; rx: number; ry: number };

/** Where food sits, on a 100 x 100 canvas, per vessel. */
export const SURFACE: Record<Vessel, Ellipse> = {
	plate: { cx: 50, cy: 52, rx: 17, ry: 7 },
	salad: { cx: 50, cy: 56, rx: 28, ry: 10 },
	bowl: { cx: 50, cy: 49, rx: 28, ry: 8 },
	deep: { cx: 50, cy: 46, rx: 29, ry: 7.5 },
	small: { cx: 50, cy: 44, rx: 24, ry: 7 },
	glass: { cx: 50, cy: 34, rx: 15, ry: 4 },
	pan: { cx: 48, cy: 58, rx: 27, ry: 9.5 },
	wok: { cx: 50, cy: 49, rx: 27, ry: 7 },
	board: { cx: 36, cy: 58, rx: 9, ry: 6 },
	wrap: { cx: 72, cy: 47, rx: 6, ry: 7 },
	round: { cx: 50, cy: 57, rx: 27, ry: 11 },
	slice: { cx: 54, cy: 41, rx: 14, ry: 3.2 }
};

export type PlacedTopping = {
	kind: Topping;
	x: number;
	y: number;
	rot: number;
	s: number;
	v: number;
};
export type Steam = { x: number; delay: number; dur: number };
export type BgTone = 'leaf' | 'lemon' | 'blue' | 'terra' | 'pumpkin' | 'plum' | 'wheat';
export type Plan = {
	vessel: Vessel;
	hot: boolean;
	bg: { shape: 'arch' | 'blob' | 'disc'; tone: BgTone; flip: boolean };
	surface: Ellipse;
	items: PlacedTopping[];
	steam: Steam[];
	/** Loose bits on the table (hero only). */
	crumbs: PlacedTopping[];
	/** Colour name (a --c-* tone) for broth / base. */
	base: 'wheat' | 'terra' | 'pumpkin' | 'brown' | 'leaf' | 'milk';
	/** Twirl turns, 2-3. */
	twirl: number;
	garnish: boolean;
	tilt: number;
	seed: number;
};

const TONES: BgTone[] = ['leaf', 'lemon', 'blue', 'terra', 'pumpkin', 'plum', 'wheat'];

function baseFor(categories: readonly FoodCategoryView[], vessel: Vessel): Plan['base'] {
	const has = (c: FoodCategoryView) => categories.includes(c);
	if (vessel === 'deep') {
		if (has('sauces')) return 'terra';
		if (has('spices') || has('vegetables')) return 'pumpkin';
		if (has('legumes')) return 'brown';
		if (has('herbs')) return 'leaf';
		return 'wheat';
	}
	if (vessel === 'glass' || vessel === 'small') return has('fruit') ? 'milk' : 'wheat';
	if (vessel === 'round') return has('sauces') || has('vegetables') ? 'terra' : 'wheat';
	if (vessel === 'plate') return 'wheat';
	if (has('grains')) return 'wheat';
	return 'milk';
}

export function plan(title: string, categories: readonly FoodCategoryView[]): Plan {
	const seed = hash(title.trim().toLowerCase());
	const rnd = mulberry32(seed);
	const vessel = vesselFor(title);
	const hot = isHot(vessel, title);
	const surface = SURFACE[vessel];
	const kinds = toppingKinds(categories);
	const garnish = kinds.length > 0 ? !kinds.includes('herb') && rnd() < 0.6 : true;
	const all: Topping[] = kinds.length ? [...kinds] : ['sprinkle', 'dot'];
	if (garnish) all.push('herb');

	const wrap = vessel === 'wrap';
	// A board carries two slices; everything else one surface.
	const surfaces = vessel === 'board' ? [surface, { cx: 64, cy: 57, rx: 9, ry: 6 }] : [surface];
	const n = wrap
		? 3
		: vessel === 'deep'
			? 4
			: vessel === 'board'
				? 6
				: Math.min(11, 4 + all.length * 2);
	const items: PlacedTopping[] = [];
	for (let i = 0; i < n; i++) {
		const kind = all[i % all.length];
		const sf = surfaces[i % surfaces.length];
		let best = { x: sf.cx, y: sf.cy };
		let bestD = -1;
		// Best of a few random candidates: spreads pieces without clumping.
		for (let k = 0; k < 6; k++) {
			const a = rnd() * Math.PI * 2;
			const r = Math.sqrt(rnd()) * 0.82;
			const x = sf.cx + Math.cos(a) * r * sf.rx;
			const y = sf.cy + Math.sin(a) * r * sf.ry;
			let d = Infinity;
			for (const o of items) d = Math.min(d, Math.hypot((o.x - x) * 0.6, (o.y - y) * 1.4));
			if (d > bestD) {
				bestD = d;
				best = { x, y };
			}
		}
		items.push({
			kind,
			x: round(best.x),
			y: round(best.y),
			rot: Math.round(rnd() * 360),
			s: round(0.85 + rnd() * 0.4),
			v: Math.floor(rnd() * 3)
		});
	}
	// Back to front, so nearer pieces overlap farther ones.
	items.sort((a, b) => a.y - b.y);

	const steam: Steam[] = hot
		? [0, 1, 2].map((i) => ({
				x: round(surface.cx - 9 + i * 9 + (rnd() - 0.5) * 4),
				delay: round(i * 0.7 + rnd() * 0.4),
				dur: round(2.8 + rnd() * 1.2)
			}))
		: [];

	const crumbKinds: Topping[] = ['herb', 'dot', 'sprinkle', 'lemon', 'leaf'];
	const crumbs: PlacedTopping[] = [];
	for (let i = 0; i < 5; i++) {
		const left = rnd() < 0.5;
		crumbs.push({
			kind: crumbKinds[Math.floor(rnd() * crumbKinds.length)],
			x: round(left ? 6 + rnd() * 14 : 80 + rnd() * 14),
			y: round(66 + rnd() * 22),
			rot: Math.round(rnd() * 360),
			s: round(0.7 + rnd() * 0.5),
			v: Math.floor(rnd() * 3)
		});
	}

	return {
		vessel,
		hot,
		bg: {
			shape: (['arch', 'blob', 'disc'] as const)[Math.floor(rnd() * 3)],
			tone: TONES[Math.floor(rnd() * TONES.length)],
			flip: rnd() < 0.5
		},
		surface,
		items,
		steam,
		crumbs,
		base: baseFor(categories, vessel),
		twirl: 2 + Math.floor(rnd() * 2),
		garnish,
		tilt: round((rnd() - 0.5) * 6),
		seed
	};
}

function round(n: number): number {
	return Math.round(n * 10) / 10;
}
