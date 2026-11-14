// Pure composition of one dish picture: which vessel and base, where every
// piece goes, steam, nook and tilt. Seeded by the title (FNV-1a ->
// mulberry32) so a recipe always looks the same. No DOM (unit-tested); the
// drawing is parts/Vessel.svelte, parts/Base.svelte and parts/Piece.svelte.
// Canvas: 100 x 100, table at y ~ 80.

import { hash, mulberry32 } from './dish.js';
import type { Base, Colour, DishKind, Piece, Vessel } from './types.js';
import { r1, type Ellipse } from './svg.js';

export type { Ellipse };

/** Where the food sits, per vessel (100 x 100 canvas). */
export const SURFACE: Record<Vessel, Ellipse> = {
	plate: { cx: 50, cy: 60, rx: 26, ry: 8.5 },
	salad: { cx: 50, cy: 60, rx: 29, ry: 9.5 },
	bowl: { cx: 50, cy: 48, rx: 33, ry: 8 },
	deep: { cx: 50, cy: 46, rx: 34, ry: 7.4 },
	small: { cx: 50, cy: 54.4, rx: 26, ry: 6.4 },
	glass: { cx: 50, cy: 31, rx: 15.5, ry: 3.6 },
	pan: { cx: 48, cy: 60, rx: 29, ry: 10 },
	wok: { cx: 50, cy: 47, rx: 32, ry: 7 },
	board: { cx: 50, cy: 64, rx: 32, ry: 9 },
	wrap: { cx: 74, cy: 57, rx: 6, ry: 15 },
	round: { cx: 50, cy: 60, rx: 36, ry: 14 },
	slice: { cx: 54, cy: 44, rx: 16, ry: 4 },
	pot: { cx: 50, cy: 49, rx: 30, ry: 6.4 },
	tray: { cx: 50, cy: 56, rx: 32, ry: 7 },
	skewer: { cx: 50, cy: 63, rx: 28, ry: 8 },
	bun: { cx: 50, cy: 62, rx: 27, ry: 3.5 },
	taco: { cx: 50, cy: 50, rx: 28, ry: 6 },
	stack: { cx: 50, cy: 70, rx: 24, ry: 6 },
	mug: { cx: 50, cy: 42, rx: 18, ry: 4.2 },
	cup: { cx: 50, cy: 58, rx: 19, ry: 4.5 },
	steamer: { cx: 50, cy: 51, rx: 28, ry: 7 },
	sushi: { cx: 42, cy: 62, rx: 26, ry: 8 },
	jar: { cx: 50, cy: 38, rx: 16, ry: 3.4 },
	loaf: { cx: 50, cy: 50, rx: 22, ry: 5 }
};

/** Tall vessels whose body the base fills (glass, jar) or hides (mug, cup). */
export const COLUMN: ReadonlySet<Vessel> = new Set<Vessel>(['glass', 'jar', 'mug', 'cup']);
export const GLASS_BODY = 'M32.5 29L36 84Q36 88 40 88H60Q64 88 64 84L67.5 29Z';
export const JAR_BODY = 'M31 38V82Q31 85 34 85H66Q69 85 69 82V38Z';
/** Inside of the baking dish (trapezoid). */
export const TRAY_INNER = 'M26 48H74L86 62H14Z';
/** Three skewers lying across the plate: x1 y1 x2 y2. */
export const SKEWERS: readonly (readonly [number, number, number, number])[] = [
	[14, 58, 86, 49],
	[12, 65, 88, 60],
	[16, 72, 84, 70]
];

/** Bases that are a liquid surface: a side dish sits on top, not beside. */
const LIQUID: ReadonlySet<Base> = new Set<Base>([
	'broth',
	'thick',
	'creamy',
	'ramen',
	'smoothie',
	'porridge',
	'yoghurt'
]);
/** How far the pieces sit above the surface (mounds, stacks). */
const LIFT: Partial<Record<Base, number>> = {
	rice: 6,
	'fried-rice': 6,
	grain: 5,
	mash: 6,
	pancake: 19,
	fries: 4,
	roast: 4,
	patty: 0
};

export type Placed = { kind: Piece; x: number; y: number; rot: number; s: number; v: number };
export type Steam = { x: number; delay: number; dur: number };
export type BgTone = 'leaf' | 'lemon' | 'blue' | 'terra' | 'pumpkin' | 'plum' | 'wheat';

export interface Composition {
	id: string;
	vessel: Vessel;
	base: Base;
	tint: Colour;
	side?: Base;
	sideTint?: Colour;
	hot: boolean;
	/** Where the main base is drawn. */
	surface: Ellipse;
	/** Where the side base is drawn. */
	sideSurface?: Ellipse;
	/** Back to front. */
	pieces: Placed[];
	steam: Steam[];
	/** Loose bits on the table (hero only). */
	crumbs: Placed[];
	bg: { shape: 'arch' | 'blob' | 'disc'; tone: BgTone; flip: boolean };
	tilt: number;
	seed: number;
}

const TONES: BgTone[] = ['leaf', 'lemon', 'blue', 'terra', 'pumpkin', 'plum', 'wheat'];

const COUNT: Record<Vessel, [number, number]> = {
	plate: [5, 9],
	salad: [7, 12],
	bowl: [5, 9],
	deep: [3, 5],
	small: [3, 5],
	glass: [2, 3],
	pan: [4, 8],
	wok: [5, 8],
	board: [4, 7],
	wrap: [3, 4],
	round: [8, 12],
	slice: [2, 4],
	pot: [3, 5],
	tray: [8, 12],
	skewer: [15, 15],
	bun: [4, 5],
	taco: [4, 5],
	stack: [2, 3],
	mug: [2, 3],
	cup: [1, 2],
	steamer: [3, 4],
	sushi: [3, 5],
	jar: [2, 4],
	loaf: [3, 4]
};

type Layout = 'scatter' | 'sectors' | 'even' | 'skewer' | 'peek' | 'rows' | 'salad' | 'soup';

export function layoutFor(vessel: Vessel, base: Base): Layout {
	if (base === 'ramen') return 'sectors';
	if (vessel === 'skewer') return 'skewer';
	if (vessel === 'round' || base === 'pizza') return 'even';
	if (vessel === 'taco' || vessel === 'wrap' || vessel === 'bun') return 'peek';
	if (vessel === 'tray') return 'rows';
	if (vessel === 'salad' || base === 'greens') return 'salad';
	if (LIQUID.has(base) && (vessel === 'deep' || vessel === 'pot' || vessel === 'mug'))
		return 'soup';
	return 'scatter';
}

const inEllipse = (e: Ellipse, a: number, r: number) => ({
	x: e.cx + Math.cos(a) * r * e.rx,
	y: e.cy + Math.sin(a) * r * e.ry
});

function shuffle<T>(a: T[], rnd: () => number): T[] {
	const out = [...a];
	for (let i = out.length - 1; i > 0; i--) {
		const j = Math.floor(rnd() * (i + 1));
		[out[i], out[j]] = [out[j], out[i]];
	}
	return out;
}

export function compose(kind: DishKind, pieces: readonly Piece[], title: string): Composition {
	const seed = hash(title.trim().toLowerCase());
	const rnd = mulberry32(seed);
	const { vessel, base } = kind;

	// --- surfaces (a side dish shrinks a solid main base, or sits on a liquid)
	let surface = { ...SURFACE[vessel] };
	let sideSurface: Ellipse | undefined;
	const side = kind.side && kind.side !== 'none' ? kind.side : undefined;
	if (side && base !== 'rice-side' && !COLUMN.has(vessel)) {
		const s = SURFACE[vessel];
		if (LIQUID.has(base)) {
			sideSurface = {
				cx: s.cx + s.rx * 0.42,
				cy: s.cy - s.ry * 0.2,
				rx: s.rx * 0.4,
				ry: s.ry * 0.75
			};
		} else {
			surface = { cx: s.cx - s.rx * 0.26, cy: s.cy, rx: s.rx * 0.7, ry: s.ry };
			sideSurface = {
				cx: s.cx + s.rx * 0.6,
				cy: s.cy - s.ry * 0.1,
				rx: s.rx * 0.36,
				ry: s.ry * 0.8
			};
		}
	}
	const lift = LIFT[base] ?? 0;
	const area: Ellipse = {
		cx: surface.cx,
		cy: surface.cy - lift,
		rx: surface.rx * (lift > 10 ? 0.55 : 0.9),
		ry: surface.ry * 0.9
	};

	// --- which pieces: signature first, then the recipe's, distinct
	const seq: Piece[] = [];
	for (const p of [...kind.signature, ...pieces]) if (!seq.includes(p)) seq.push(p);
	const noGarnish = ['glass', 'jar', 'cup', 'slice', 'sushi', 'skewer', 'steamer'].includes(vessel);
	const garnishRoll = rnd();
	if (!noGarnish && !seq.includes('herb') && garnishRoll < 0.6 && seq.length < 6) seq.push('herb');

	const layout = layoutFor(vessel, base);
	const [lo, hi] = COUNT[vessel];
	const K = seq.length;
	let n = K === 0 ? 0 : Math.min(hi, Math.max(Math.min(lo, K * 3), K * 2));
	if (K === 1) n = Math.min(n, 3);
	if (layout === 'sectors') n = Math.min(Math.max(K, 3), 6);
	if (layout === 'soup') n = Math.min(Math.max(3, Math.min(K, 4)), 4);
	if (layout === 'skewer') n = K === 0 ? 0 : 15;

	const items: Placed[] = [];
	const mk = (kindP: Piece, x: number, y: number, rot?: number, s?: number): Placed => ({
		kind: kindP,
		x: r1(x),
		y: r1(y),
		rot: rot ?? Math.round(rnd() * 360),
		s: r1(s ?? 0.85 + rnd() * 0.4),
		v: Math.floor(rnd() * 3)
	});
	const kindAt = (i: number) => seq[i % K];

	if (n > 0) {
		if (layout === 'sectors') {
			const base0 = rnd() * Math.PI * 2;
			for (let i = 0; i < n; i++) {
				const a = base0 + (i / n) * Math.PI * 2;
				const p = inEllipse(surface, a, 0.68);
				items.push(mk(kindAt(i), p.x, p.y, undefined, 1.05));
			}
		} else if (layout === 'even') {
			const off = rnd() * Math.PI * 2;
			const order = shuffle(
				Array.from({ length: n }, (_, i) => i),
				rnd
			);
			for (let i = 0; i < n; i++) {
				const r = Math.sqrt((i + 0.5) / n) * 0.8;
				const p = inEllipse(surface, off + i * 2.39996, r);
				items.push(mk(kindAt(order[i]), p.x, p.y));
			}
		} else if (layout === 'skewer') {
			SKEWERS.forEach(([x1, y1, x2, y2], li) => {
				const ang = (Math.atan2(y2 - y1, x2 - x1) * 180) / Math.PI;
				for (let j = 0; j < 5; j++) {
					const t = 0.2 + j * 0.15;
					items.push(
						mk(
							kindAt(j + (li % 2)),
							x1 + (x2 - x1) * t,
							y1 + (y2 - y1) * t,
							ang + (rnd() - 0.5) * 20,
							0.95
						)
					);
				}
			});
		} else if (layout === 'peek') {
			for (let i = 0; i < n; i++) {
				const t = (i + 0.5) / n;
				if (vessel === 'bun') {
					const sgn = i % 2 ? 1 : -1;
					items.push(
						mk(
							kindAt(i),
							area.cx + sgn * area.rx * (0.62 + 0.38 * rnd()),
							area.cy + (rnd() - 0.3) * 2,
							undefined,
							0.9
						)
					);
				} else if (vessel === 'wrap') {
					const p = inEllipse(surface, Math.PI * 2 * t + rnd(), 0.45);
					items.push(mk(kindAt(i), p.x, p.y, undefined, 0.8));
				} else {
					const a = Math.PI + t * Math.PI;
					const p = inEllipse(surface, a, 0.78);
					items.push(mk(kindAt(i), p.x, p.y - 2.2, undefined, 0.95));
				}
			}
		} else if (layout === 'rows') {
			const R = n <= 8 ? 2 : 3;
			const C = Math.ceil(n / R);
			let i = 0;
			for (let r = 0; r < R; r++) {
				for (let c = 0; c < C && i < n; c++, i++) {
					const stagger = r % 2 ? 0.5 : 0;
					const x = area.cx + (c + stagger - (C - 1) / 2) * ((2 * area.rx * 0.95) / C);
					const y = area.cy + (r - (R - 1) / 2) * area.ry * 1.15;
					items.push(
						mk(kindAt(i), x + (rnd() - 0.5) * 1.5, y + (rnd() - 0.5) * 1, undefined, 0.95)
					);
				}
			}
		} else {
			// scatter / salad / soup: best-of-N spreads without clumping
			const kinds = layout === 'salad' ? shuffle(seq, rnd) : seq;
			const reach = layout === 'soup' ? 0.6 : 0.82;
			for (let i = 0; i < n; i++) {
				let best = { x: area.cx, y: area.cy };
				let bestD = -1;
				for (let k = 0; k < 6; k++) {
					const a = rnd() * Math.PI * 2;
					const r = Math.sqrt(rnd()) * reach;
					const p = inEllipse(area, a, r);
					let d = Infinity;
					for (const o of items) d = Math.min(d, Math.hypot((o.x - p.x) * 0.6, (o.y - p.y) * 1.4));
					if (d > bestD) {
						bestD = d;
						best = p;
					}
				}
				items.push(mk(kinds[i % K], best.x, best.y));
			}
		}
		// Back to front, so nearer pieces overlap farther ones (skewers keep order).
		if (layout !== 'skewer') items.sort((a, b) => a.y - b.y);
	}

	// --- steam
	const hot = kind.hot;
	const sc = SURFACE[vessel];
	const steam: Steam[] = hot
		? [0, 1, 2].map((i) => ({
				x: r1(sc.cx - 9 + i * 9 + (rnd() - 0.5) * 4),
				delay: r1(i * 0.7 + rnd() * 0.4),
				dur: r1(2.8 + rnd() * 1.2)
			}))
		: [];

	// --- crumbs on the table (hero only)
	const crumbKinds: Piece[] = ['herb', 'dot', 'sprinkle', 'lemon', 'leaf', ...seq.slice(0, 2)];
	const crumbs: Placed[] = [];
	for (let i = 0; i < 5; i++) {
		const left = rnd() < 0.5;
		crumbs.push(
			mk(
				crumbKinds[Math.floor(rnd() * crumbKinds.length)],
				left ? 6 + rnd() * 14 : 80 + rnd() * 14,
				66 + rnd() * 22,
				undefined,
				0.7 + rnd() * 0.5
			)
		);
	}

	return {
		id: kind.id,
		vessel,
		base,
		tint: kind.tint,
		side,
		sideTint: kind.sideTint,
		hot,
		surface,
		sideSurface,
		pieces: items,
		steam,
		crumbs,
		bg: {
			shape: (['arch', 'blob', 'disc'] as const)[Math.floor(rnd() * 3)],
			tone: TONES[Math.floor(rnd() * TONES.length)],
			flip: rnd() < 0.5
		},
		tilt: r1((rnd() - 0.5) * 6),
		seed
	};
}
