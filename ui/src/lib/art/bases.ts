// The base of a dish (what fills the vessel), as SVG markup strings drawn
// inside the vessel's food surface (an ellipse on the 100 x 100 canvas).
// Calm, flat-ish shapes: one shade, one highlight. `detail` is off at small
// sizes (fewer elements). Pure.

import type { Base, Vessel } from './types.js';
import { COLUMN, GLASS_BODY, JAR_BODY, TRAY_INNER, type Ellipse } from './compose.js';
import { E, HI, L, P, col, dk, lt, mixc, r1 } from './svg.js';

export interface BaseCtx {
	s: Ellipse;
	/** Colour name of the tint. */
	tint: string;
	sideTint?: string;
	vessel: Vessel;
	rnd: () => number;
	detail: boolean;
}

const OL = 'var(--ol)';
const MILK = col('milk');
const BROWN = col('brown');

/** A dome (mound) sitting on y=by, h high, with a rounded front. */
function dome(
	cx: number,
	by: number,
	rx: number,
	h: number,
	depth: number,
	fill: string,
	sw = 0.7
): string {
	return P(
		`M${r1(cx - rx)} ${r1(by)}C${r1(cx - rx)} ${r1(by - h * 1.35)} ${r1(cx + rx)} ${r1(by - h * 1.35)} ${r1(cx + rx)} ${r1(by)}Q${r1(cx)} ${r1(by + depth)} ${r1(cx - rx)} ${r1(by)}Z`,
		fill,
		{ stroke: dk(fill), sw }
	);
}

/** A leaf (lens) centred at x,y of length len. */
function leaf(
	x: number,
	y: number,
	len: number,
	wid: number,
	ang: number,
	fill: string,
	vein: boolean
): string {
	const a = (ang * Math.PI) / 180;
	const vx = Math.cos(a) * len * 0.7;
	const vy = Math.sin(a) * len * 0.7;
	return (
		P(
			`M${r1(x - len)} ${r1(y)}Q${r1(x)} ${r1(y - wid)} ${r1(x + len)} ${r1(y)}Q${r1(x)} ${r1(y + wid)} ${r1(x - len)} ${r1(y)}Z`,
			fill,
			{ rot: ang }
		) +
		(vein ? L(`M${r1(x - vx)} ${r1(y - vy)}L${r1(x + vx)} ${r1(y + vy)}`, dk(fill), 0.45, 0.5) : '')
	);
}

export function baseSvg(base: Base, c: BaseCtx): string {
	if (base === 'none') return '';
	if (COLUMN.has(c.vessel)) return column(base, c);
	switch (base) {
		case 'broth':
		case 'thick':
		case 'creamy':
			return liquid(base, c);
		case 'ramen':
			return ramen(c);
		case 'rice':
		case 'fried-rice':
		case 'grain':
			return mound(base, c);
		case 'rice-side':
			return riceSide(c);
		case 'noodles':
			return noodles(c);
		case 'spaghetti':
			return spaghetti(c);
		case 'short-pasta':
			return shortPasta(c);
		case 'lasagne':
			return lasagne(c);
		case 'gnocchi':
			return gnocchi(c);
		case 'greens':
			return greens(c);
		case 'mash':
			return mash(c);
		case 'potatoes':
			return potatoes(c);
		case 'fries':
			return fries(c);
		case 'omelette':
		case 'tortilla':
			return fold(base, c);
		case 'scramble':
			return scramble(c);
		case 'fried-egg':
			return friedEgg(c);
		case 'pancake':
			return pancake(c);
		case 'toast':
			return toast(c);
		case 'pizza':
			return pizza(c);
		case 'flatbread':
			return flatbread(c);
		case 'dumpling':
			return dumplings(c);
		case 'porridge':
		case 'yoghurt':
		case 'smoothie':
			return creamBowl(base, c);
		case 'cake':
		case 'crumble':
		case 'pie':
			return c.vessel === 'slice' ? wedge(base, c) : round(base, c);
		case 'fillet':
			return fillet(c);
		case 'patty':
			return patty(c);
		case 'roast':
			return roast(c);
		case 'traybake':
		case 'gratin':
			return bake(base, c);
		case 'sushi-roll':
			return sushiRolls(c);
	}
}

// ---------------------------------------------------------------- liquids

function liquid(base: 'broth' | 'thick' | 'creamy', { s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const X = (a: number) => cx + a * rx;
	const Y = (b: number) => cy + b * ry;
	let out = E(cx, cy, rx, ry, base === 'creamy' ? lt(T, 82) : T, { stroke: OL, sw: 0.9 });
	if (base === 'broth') {
		out += E(X(-0.15), Y(-0.2), rx * 0.5, ry * 0.3, MILK, { stroke: null, op: 0.22 });
		const n = detail ? 7 : 4;
		for (let i = 0; i < n; i++) {
			const a = rnd() * 6.28;
			const r = 0.25 + rnd() * 0.55;
			out += E(
				X(Math.cos(a) * r),
				Y(Math.sin(a) * r),
				1.5 + rnd(),
				0.7 + rnd() * 0.3,
				col('lemon'),
				{ stroke: null, op: 0.5 }
			);
		}
	} else if (base === 'thick') {
		out += E(cx, cy + 0.4, rx * 0.88, ry * 0.8, 'none', { stroke: lt(T, 50), sw: 1, op: 0.5 });
		out += L(
			`M${r1(X(-0.55))} ${r1(Y(0.15))}Q${r1(X(-0.1))} ${r1(Y(-0.6))} ${r1(X(0.4))} ${r1(Y(0))}T${r1(X(0.7))} ${r1(Y(0.1))}`,
			dk(T, 70),
			1.4,
			0.4
		);
		const n = detail ? 6 : 3;
		for (let i = 0; i < n; i++) {
			const a = rnd() * 6.28;
			const r = 0.2 + rnd() * 0.6;
			out += E(X(Math.cos(a) * r), Y(Math.sin(a) * r), 1.2, 0.8, dk(T, 65), {
				stroke: null,
				op: 0.55
			});
		}
		out += HI(
			`M${r1(X(-0.6))} ${r1(Y(0.45))}Q${r1(cx)} ${r1(Y(0.95))} ${r1(X(0.6))} ${r1(Y(0.45))}`,
			0.3
		);
	} else {
		out += L(
			`M${r1(X(-0.6))} ${r1(Y(0.1))}Q${r1(X(-0.3))} ${r1(Y(-0.75))} ${r1(X(0.1))} ${r1(Y(-0.15))}T${r1(X(0.58))} ${r1(Y(0.12))}`,
			MILK,
			1.6,
			0.7
		);
		out += HI(
			`M${r1(X(-0.5))} ${r1(Y(0.5))}Q${r1(cx)} ${r1(Y(0.95))} ${r1(X(0.5))} ${r1(Y(0.5))}`,
			0.4
		);
		if (detail)
			for (let i = 0; i < 4; i++)
				out += E(X(rnd() * 1.2 - 0.6), Y(rnd() * 1.2 - 0.6), 0.6, 0.4, dk(T, 60), {
					stroke: null,
					op: 0.6
				});
	}
	return out;
}

function ramen({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = E(cx, cy, rx, ry, T, { stroke: OL, sw: 0.9 });
	out += E(cx, cy, rx * 0.4, ry * 0.5, mixc(col('wheat'), MILK, 70), {
		stroke: dk(col('wheat')),
		sw: 0.6
	});
	const n = detail ? 5 : 3;
	for (let i = 0; i < n; i++) {
		const y = cy + (i - (n - 1) / 2) * ry * 0.22;
		out += L(
			`M${r1(cx - rx * 0.34)} ${r1(y)}Q${r1(cx - rx * 0.15)} ${r1(y - ry * 0.25 - rnd() * 1.2)} ${r1(cx)} ${r1(y)}T${r1(cx + rx * 0.34)} ${r1(y)}`,
			dk(col('wheat'), 75),
			0.8,
			0.7
		);
	}
	out += E(cx - rx * 0.55, cy + ry * 0.1, rx * 0.12, ry * 0.12, col('lemon'), {
		stroke: null,
		op: 0.5
	});
	return out;
}

// --------------------------------------------------------------- mounds

function mound(base: 'rice' | 'fried-rice' | 'grain', { s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const h = Math.min(rx * 0.55, ry * 2.8);
	const by = cy + ry * 0.35;
	const fill = base === 'rice' ? lt(T, 70) : T;
	const w = rx * 0.92;
	let out = dome(cx, by, w, h, ry * 1.4, fill);
	const n = detail ? 20 : 8;
	const colors = base === 'fried-rice' ? [col('pea'), col('rust'), col('lemon'), BROWN] : [];
	for (let i = 0; i < n; i++) {
		const a = rnd() * Math.PI;
		const rr = Math.sqrt(rnd());
		const x = cx + Math.cos(a) * w * 0.85 * rr * (rnd() < 0.5 ? -1 : 1);
		const y = by - rr * h * 0.62 * rnd() - 0.5 + ry * 0.2;
		if (base === 'grain') {
			out += E(x, y, 0.8, 0.8, i % 2 ? lt(fill, 60) : dk(fill, 75), { stroke: null, op: 0.9 });
		} else if (colors.length && i % 3 === 0) {
			out += E(x, y, 1.4, 0.9, colors[i % colors.length], { stroke: null, rot: rnd() * 90 });
		} else {
			out += E(x, y, 1.5, 0.7, base === 'rice' ? MILK : lt(fill, 70), {
				stroke: dk(fill, 85),
				sw: 0.3,
				rot: rnd() * 180
			});
		}
	}
	out += HI(
		`M${r1(cx - w * 0.6)} ${r1(by - h * 0.5)}Q${r1(cx - w * 0.3)} ${r1(by - h * 0.95)} ${r1(cx)} ${r1(by - h * 0.95)}`,
		0.5,
		1.6
	);
	return out;
}

function riceSide(c: BaseCtx): string {
	const { s, tint, rnd, detail } = c;
	const T = col(tint);
	const sauce: Ellipse = {
		cx: s.cx - s.rx * 0.28,
		cy: s.cy + s.ry * 0.1,
		rx: s.rx * 0.66,
		ry: s.ry * 0.9
	};
	const rice: Ellipse = {
		cx: s.cx + s.rx * 0.5,
		cy: s.cy - s.ry * 0.05,
		rx: s.rx * 0.46,
		ry: s.ry * 0.8
	};
	return (
		liquid('thick', { ...c, s: sauce, rnd, detail }) +
		mound('rice', { ...c, s: rice, tint: 'milk', rnd, detail: detail && T !== '' })
	);
}

function noodles({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = E(cx, cy, rx * 0.95, ry * 0.9, dk(T, 70), { stroke: null, op: 0.5 });
	const n = detail ? 8 : 5;
	for (let i = 0; i < n; i++) {
		const x0 = cx - rx * (0.8 - rnd() * 0.2);
		const y0 = cy + (rnd() - 0.5) * ry * 1.4;
		const x1 = cx + rx * (0.8 - rnd() * 0.2);
		const y1 = cy + (rnd() - 0.5) * ry * 1.4;
		const d = `M${r1(x0)} ${r1(y0)}C${r1(x0 + rx * 0.5)} ${r1(y0 - ry * 1.5)} ${r1(x1 - rx * 0.5)} ${r1(y1 + ry * 1.5)} ${r1(x1)} ${r1(y1)}`;
		out += L(d, dk(T, 70), 2.4, 0.9) + L(d, T, 1.5);
	}
	out += HI(
		`M${r1(cx - rx * 0.5)} ${r1(cy - ry * 0.2)}Q${r1(cx)} ${r1(cy - ry * 0.7)} ${r1(cx + rx * 0.4)} ${r1(cy - ry * 0.2)}`,
		0.4
	);
	return out;
}

function mash({ s, tint, detail }: BaseCtx): string {
	const T = lt(col(tint), 80);
	const { cx, cy, rx, ry } = s;
	const h = Math.min(rx * 0.5, ry * 2.6);
	const by = cy + ry * 0.35;
	const w = rx * 0.85;
	let out = dome(cx, by, w, h, ry * 1.4, T);
	out += L(
		`M${r1(cx - w * 0.6)} ${r1(by - h * 0.2)}Q${r1(cx - w * 0.3)} ${r1(by - h * 0.7)} ${r1(cx)} ${r1(by - h * 0.3)}T${r1(cx + w * 0.6)} ${r1(by - h * 0.2)}`,
		dk(T, 75),
		0.8,
		0.6
	);
	if (detail)
		out += L(
			`M${r1(cx - w * 0.4)} ${r1(by + ry * 0.2)}Q${r1(cx)} ${r1(by - h * 0.2)} ${r1(cx + w * 0.4)} ${r1(by + ry * 0.2)}`,
			dk(T, 75),
			0.7,
			0.5
		);
	out += P(
		`M${r1(cx - 3.5)} ${r1(by - h * 0.7)}L${r1(cx + 3.5)} ${r1(by - h * 0.78)}L${r1(cx + 4.2)} ${r1(by - h * 0.45)}L${r1(cx - 2.8)} ${r1(by - h * 0.38)}Z`,
		col('lemon')
	);
	out += HI(
		`M${r1(cx - w * 0.55)} ${r1(by - h * 0.45)}Q${r1(cx - w * 0.35)} ${r1(by - h * 0.85)} ${r1(cx - w * 0.1)} ${r1(by - h * 0.9)}`,
		0.5,
		1.5
	);
	return out;
}

// ----------------------------------------------------------------- pasta

function spaghetti({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const pasta = mixc(col('wheat'), T, 62);
	const { cx, cy, rx, ry } = s;
	const rows = detail ? 5 : 4;
	let out = '';
	for (let i = 0; i < rows; i++) {
		const k = i / rows;
		out += E(
			cx + (i % 2 ? 1.2 : -1.2),
			cy + ry * 0.35 - i * ry * 0.5,
			rx * (0.86 - k * 0.62),
			ry * (0.8 - k * 0.5),
			pasta,
			{ stroke: dk(pasta), sw: 0.9 }
		);
	}
	const top = cy + ry * 0.35 - (rows - 1) * ry * 0.5;
	if (tint !== 'wheat' && tint !== 'cream')
		out += E(cx, top - 0.5, rx * 0.2, ry * 0.2, T, { stroke: dk(T), sw: 0.5 });
	out += L(
		`M${r1(cx - rx * 0.6)} ${r1(cy + ry * 0.1)}Q${r1(cx)} ${r1(cy + ry * 0.9)} ${r1(cx + rx * 0.6)} ${r1(cy + ry * 0.1)}`,
		dk(pasta, 70),
		0.7,
		0.55
	);
	if (detail)
		out += L(
			`M${r1(cx - rx * 0.4)} ${r1(cy - ry * 0.3)}Q${r1(cx)} ${r1(cy + ry * 0.3)} ${r1(cx + rx * 0.4)} ${r1(cy - ry * 0.3)}`,
			dk(pasta, 70),
			0.6,
			0.5
		);
	out += L(
		`M${r1(cx + rx * 0.7)} ${r1(cy + ry * 0.5)}q${r1(rx * 0.15)} ${r1(-ry * 0.4)} ${r1(rx * 0.3)} ${r1(ry * 0.1)}`,
		dk(pasta, 70),
		0.8,
		0.7
	);
	out += HI(
		`M${r1(cx - rx * 0.4)} ${r1(top + ry * 0.2)}Q${r1(cx - rx * 0.2)} ${r1(top - ry * 0.1)} ${r1(cx)} ${r1(top - ry * 0.1)}`,
		0.45
	);
	void rnd;
	return out;
}

function shortPasta({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const pasta = mixc(col('wheat'), T, 70);
	const { cx, cy, rx, ry } = s;
	let out = E(cx, cy, rx * 0.92, ry * 0.88, T, { stroke: dk(T), sw: 0.7, op: 0.92 });
	const n = detail ? 14 : 9;
	const pts: [number, number][] = [];
	for (let i = 0; i < n * 3 && pts.length < n; i++) {
		const a = rnd() * 6.28;
		const r = Math.sqrt(rnd()) * 0.78;
		const x = cx + Math.cos(a) * r * rx;
		const y = cy + Math.sin(a) * r * ry;
		if (pts.every(([px, py]) => Math.hypot(px - x, (py - y) * 2) > 5.2)) pts.push([x, y]);
	}
	pts.sort((a, b) => a[1] - b[1]);
	for (const [x, y] of pts) {
		out += P(
			`M${r1(x - 3.2)} ${r1(y - 1.2)}L${r1(x + 2.6)} ${r1(y - 1.9)}L${r1(x + 3.3)} ${r1(y + 1.1)}L${r1(x - 2.5)} ${r1(y + 1.9)}Z`,
			pasta,
			{ rot: rnd() * 180 }
		);
	}
	return out;
}

function gnocchi({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = E(cx, cy, rx * 0.92, ry * 0.88, T, { stroke: dk(T), sw: 0.7, op: 0.9 });
	const n = detail ? 9 : 6;
	const pts: [number, number][] = [];
	for (let i = 0; i < n * 4 && pts.length < n; i++) {
		const a = rnd() * 6.28;
		const r = Math.sqrt(rnd()) * 0.75;
		const x = cx + Math.cos(a) * r * rx;
		const y = cy + Math.sin(a) * r * ry;
		if (pts.every(([px, py]) => Math.hypot(px - x, (py - y) * 2) > 6)) pts.push([x, y]);
	}
	pts.sort((a, b) => a[1] - b[1]);
	for (const [x, y] of pts) {
		const rot = rnd() * 60 - 30;
		out += E(x, y, 3.6, 2.4, col('cream'), { rot });
		if (detail)
			out += L(
				`M${r1(x - 1.2)} ${r1(y - 1)}v2M${r1(x)} ${r1(y - 1.2)}v2.4M${r1(x + 1.2)} ${r1(y - 1)}v2`,
				dk(col('cream'), 70),
				0.4,
				0.7
			);
	}
	return out;
}

function boxGeometry(c: BaseCtx): {
	x0: number;
	x1: number;
	yf: number;
	yb: number;
	inset: number;
	h: number;
} {
	const { s, vessel } = c;
	if (vessel === 'tray') return { x0: 17, x1: 83, yf: 60, yb: 50, inset: 8, h: 6 };
	const w = s.rx * 0.85;
	return {
		x0: s.cx - w,
		x1: s.cx + w,
		yf: s.cy + s.ry * 0.2,
		yb: s.cy - s.ry * 0.8,
		inset: w * 0.18,
		h: Math.max(5, s.ry * 0.8)
	};
}

function lasagne(c: BaseCtx): string {
	const T = col(c.tint);
	const g = boxGeometry(c);
	const { x0, x1, yf, yb, inset, h } = g;
	const sheet = col('cream');
	let out = '';
	// front face: layers
	const lay = [sheet, T, sheet, T, sheet];
	const lh = h / lay.length;
	out += P(`M${r1(x0)} ${r1(yf)}H${r1(x1)}V${r1(yf + h)}H${r1(x0)}Z`, T, { ink: true });
	lay.forEach((f, i) => {
		out += P(`M${r1(x0)} ${r1(yf + i * lh)}H${r1(x1)}V${r1(yf + (i + 1) * lh)}H${r1(x0)}Z`, f, {
			stroke: null,
			op: 0.95
		});
	});
	// top: browned cheese
	const gold = mixc(col('lemon'), col('pumpkin'), 55);
	out += P(
		`M${r1(x0)} ${r1(yf)}L${r1(x0 + inset)} ${r1(yb)}H${r1(x1 - inset)}L${r1(x1)} ${r1(yf)}Z`,
		gold,
		{ stroke: OL, sw: 0.9 }
	);
	for (let i = 0; i < (c.detail ? 5 : 3); i++) {
		out += E(
			x0 + (x1 - x0) * (0.15 + c.rnd() * 0.7),
			yb + (yf - yb) * (0.2 + c.rnd() * 0.6),
			2.6,
			1.2,
			mixc(BROWN, gold, 45),
			{ stroke: null, op: 0.7 }
		);
	}
	// the cut corner
	const cw = (x1 - x0) * 0.26;
	out += P(
		`M${r1(x1 - cw)} ${r1(yf - (yf - yb) * 0.45)}H${r1(x1 - inset * 0.4)}L${r1(x1)} ${r1(yf)}H${r1(x1 - cw)}Z`,
		'var(--art-ceramic-in)',
		{ stroke: null }
	);
	out += P(
		`M${r1(x1 - cw)} ${r1(yf - (yf - yb) * 0.45)}V${r1(yf + h)}H${r1(x1)}V${r1(yf)}L${r1(x1 - inset * 0.4)} ${r1(yf - (yf - yb) * 0.45)}Z`,
		T,
		{ ink: true }
	);
	lay.forEach((f, i) => {
		out += P(
			`M${r1(x1 - cw)} ${r1(yf + i * lh)}H${r1(x1)}V${r1(yf + (i + 1) * lh)}H${r1(x1 - cw)}Z`,
			f,
			{ stroke: null, op: 0.95 }
		);
	});
	return out;
}

// --------------------------------------------------------------- salads

function greens({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = '';
	const n = detail ? 10 : 8;
	const pts: [number, number, number][] = [];
	for (let i = 0; i < n; i++) {
		const a = (i / n) * 6.28 + rnd();
		const r = i < 4 ? 0.25 + rnd() * 0.2 : 0.55 + rnd() * 0.35;
		pts.push([cx + Math.cos(a) * r * rx, cy + Math.sin(a) * r * ry, rnd() * 180 - 90]);
	}
	pts.sort((a, b) => a[1] - b[1]);
	pts.forEach(([x, y, ang], i) => {
		out += leaf(
			x,
			y,
			6.5 + rnd() * 2,
			3.6,
			ang,
			i % 3 === 1 ? col('pea') : i % 3 === 2 ? mixc(T, col('pea'), 60) : T,
			detail
		);
	});
	return out;
}

// ---------------------------------------------------------------- sides

function potatoes({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const n = detail ? 8 : 6;
	const pts: [number, number][] = [];
	for (let i = 0; i < n * 4 && pts.length < n; i++) {
		const a = rnd() * 6.28;
		const r = Math.sqrt(rnd()) * 0.78;
		const x = cx + Math.cos(a) * r * rx;
		const y = cy + Math.sin(a) * r * ry;
		if (pts.every(([px, py]) => Math.hypot(px - x, (py - y) * 1.8) > 7)) pts.push([x, y]);
	}
	pts.sort((a, b) => a[1] - b[1]);
	let out = '';
	for (const [x, y] of pts) {
		const k = 3.6 + rnd() * 1.2;
		const d = `M${r1(x - k)} ${r1(y)}Q${r1(x - k)} ${r1(y - k * 0.8)} ${r1(x)} ${r1(y - k * 0.8)}Q${r1(x + k)} ${r1(y - k * 0.8)} ${r1(x + k)} ${r1(y + 0.4)}Q${r1(x + k * 0.7)} ${r1(y + k * 0.8)} ${r1(x)} ${r1(y + k * 0.75)}Q${r1(x - k)} ${r1(y + k * 0.7)} ${r1(x - k)} ${r1(y)}Z`;
		out += P(d, T, { rot: rnd() * 40 - 20 });
		out += E(x - 1, y - 0.8, 1.2, 0.8, BROWN, { stroke: null, op: 0.45 });
	}
	return out;
}

function fries({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const n = detail ? 12 : 8;
	let out = '';
	const sticks: [number, number, number, number][] = [];
	for (let i = 0; i < n; i++) {
		const bx = cx + (rnd() - 0.5) * rx * 0.9;
		const by = cy + ry * 0.4 + (rnd() - 0.2) * ry * 0.5;
		const ang = (rnd() - 0.5) * 1.0;
		const len = 12 + rnd() * 6;
		sticks.push([bx, by, bx + Math.sin(ang) * len, by - Math.cos(ang) * len]);
	}
	sticks.sort((a, b) => a[1] - b[1]);
	for (const [x1, y1, x2, y2] of sticks) {
		out += P(
			`M${r1(x1)} ${r1(y1)}L${r1(x2)} ${r1(y2)}L${r1(x2 + 2.2)} ${r1(y2)}L${r1(x1 + 2.2)} ${r1(y1)}Z`,
			mixc(T, col('lemon'), 70),
			{ stroke: dk(T, 65), sw: 0.5 }
		);
	}
	return out;
}

function fold(base: 'omelette' | 'tortilla', c: BaseCtx): string {
	const { s, tint, rnd, detail, vessel } = c;
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	if (vessel === 'wrap') {
		let out = E(cx, cy, rx * 0.9, ry * 0.92, T, { stroke: dk(T), sw: 0.6 });
		out += E(cx, cy, rx * 0.5, ry * 0.55, 'none', { stroke: dk(T, 70), sw: 0.6, op: 0.6 });
		return out;
	}
	const X = (a: number) => r1(cx + a * rx);
	const Y = (b: number) => r1(cy + b * ry);
	const fill = base === 'omelette' ? T : col('wheat');
	let out = P(
		`M${X(-0.88)} ${Y(0.2)}C${X(-0.9)} ${Y(-1.25)} ${X(0.9)} ${Y(-1.25)} ${X(0.88)} ${Y(0.2)}C${X(0.55)} ${Y(1)} ${X(-0.55)} ${Y(1)} ${X(-0.88)} ${Y(0.2)}Z`,
		fill,
		{ stroke: dk(mixc(fill, BROWN, 60)), sw: 0.9 }
	);
	out += L(
		`M${X(-0.8)} ${Y(0.18)}Q${X(0)} ${Y(-0.35)} ${X(0.8)} ${Y(0.18)}`,
		dk(fill, 70),
		0.9,
		0.6
	);
	const n = detail ? 6 : 3;
	for (let i = 0; i < n; i++) {
		out += E(
			cx + (rnd() - 0.5) * rx * 1.3,
			cy + (rnd() * 0.9 - 0.1) * ry,
			1.5,
			0.9,
			base === 'omelette' ? mixc(BROWN, T, 40) : dk(fill, 60),
			{ stroke: null, op: 0.55 }
		);
	}
	out += HI(`M${X(-0.5)} ${Y(-0.55)}Q${X(-0.1)} ${Y(-0.9)} ${X(0.3)} ${Y(-0.7)}`, 0.5);
	return out;
}

function scramble({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const n = detail ? 9 : 6;
	const pts: [number, number][] = [];
	for (let i = 0; i < n; i++) {
		const a = rnd() * 6.28;
		const r = Math.sqrt(rnd()) * 0.72;
		pts.push([cx + Math.cos(a) * r * rx, cy + Math.sin(a) * r * ry]);
	}
	pts.sort((a, b) => a[1] - b[1]);
	let out = '';
	for (const [x, y] of pts) {
		out += E(x, y, 4.4 + rnd() * 1.4, 2.8 + rnd(), T, {
			rot: rnd() * 60 - 30,
			stroke: dk(T, 65),
			sw: 0.6
		});
		if (detail) out += E(x - 1.2, y - 0.9, 1.6, 0.7, MILK, { stroke: null, op: 0.45 });
	}
	return out;
}

function friedEgg({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = '';
	if (!['milk', 'cream', 'lemon', 'wheat'].includes(tint)) {
		out += E(cx, cy, rx * 0.95, ry * 0.92, T, { stroke: OL, sw: 0.9 });
		out += E(cx, cy + 0.4, rx * 0.84, ry * 0.8, 'none', { stroke: lt(T, 50), sw: 0.9, op: 0.45 });
		if (detail)
			out += L(
				`M${r1(cx - rx * 0.6)} ${r1(cy + ry * 0.2)}Q${r1(cx - rx * 0.2)} ${r1(cy - ry * 0.5)} ${r1(cx + rx * 0.1)} ${r1(cy + ry * 0.1)}`,
				dk(T, 70),
				1.2,
				0.4
			);
	}
	const spots: [number, number][] =
		rx > 18
			? [
					[cx - rx * 0.32, cy - ry * 0.05],
					[cx + rx * 0.32, cy + ry * 0.1]
				]
			: [[cx, cy]];
	for (const [x, y] of spots) {
		const w = Math.min(10, rx * 0.34);
		const k = w / 10;
		out += P(
			`M${r1(x - w)} ${r1(y + 0.5)}Q${r1(x - w * 1.15)} ${r1(y - 5 * k - ry * 0.2)} ${r1(x - w * 0.3)} ${r1(y - 6 * k - ry * 0.3)}Q${r1(x + w * 0.5)} ${r1(y - 8 * k - ry * 0.3)} ${r1(x + w)} ${r1(y - 2 * k)}Q${r1(x + w * 1.1)} ${r1(y + 4 * k)} ${r1(x + w * 0.2)} ${r1(y + 5 * k)}Q${r1(x - w * 0.8)} ${r1(y + 6 * k)} ${r1(x - w)} ${r1(y + 0.5)}Z`,
			MILK,
			{ stroke: dk(MILK, 75), sw: 0.7 }
		);
		out += E(x + rnd() * 1.5 - 0.7, y - 0.6, w * 0.38, w * 0.34, col('lemon'), {
			stroke: dk(col('pumpkin'), 80),
			sw: 0.6
		});
		out += E(x - w * 0.15, y - 1.7, w * 0.1, w * 0.07, '#fff', { stroke: null, op: 0.8 });
	}
	return out;
}

function pancake({ s, tint }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const w = Math.min(rx * 0.95, 22);
	const e = Math.max(3.2, ry * 0.62);
	const n = 4;
	const step = 5.2;
	let out = '';
	for (let i = 0; i < n; i++) {
		const y = cy + ry * 0.2 - i * step;
		const fill = i % 2 ? mixc(T, col('wheat'), 80) : T;
		out += P(
			`M${r1(cx - w)} ${r1(y)}V${r1(y + 4)}Q${r1(cx)} ${r1(y + 4 + e * 1.8)} ${r1(cx + w)} ${r1(y + 4)}V${r1(y)}Z`,
			fill,
			{ stroke: dk(fill, 60), sw: 0.7 }
		);
		if (i === n - 1) {
			out += E(cx, y, w, e, lt(fill, 78), { stroke: dk(fill, 60), sw: 0.7 });
		} else {
			out += L(
				`M${r1(cx - w + 1)} ${r1(y + 1.5)}Q${r1(cx)} ${r1(y + 1.5 + e * 1.8)} ${r1(cx + w - 1)} ${r1(y + 1.5)}`,
				MILK,
				0.8,
				0.3
			);
		}
	}
	const top = cy + ry * 0.2 - (n - 1) * step;
	const syr = dk(col('pumpkin'), 55);
	out += P(
		`M${r1(cx - w * 0.7)} ${r1(top - e * 0.3)}Q${r1(cx)} ${r1(top - e * 1.1)} ${r1(cx + w * 0.7)} ${r1(top - e * 0.3)}Q${r1(cx + w * 0.7)} ${r1(top + e * 0.9)} ${r1(cx + w * 0.4)} ${r1(top + e * 0.8)}L${r1(cx + w * 0.38)} ${r1(top + 6)}Q${r1(cx + w * 0.3)} ${r1(top + 7)} ${r1(cx + w * 0.25)} ${r1(top + 4)}Q${r1(cx)} ${r1(top + e * 0.9)} ${r1(cx - w * 0.5)} ${r1(top + e * 0.8)}Z`,
		syr,
		{ stroke: null, op: 0.9 }
	);
	out += P(
		`M${r1(cx - 3)} ${r1(top - 2.2)}H${r1(cx + 3)}V${r1(top + 0.4)}H${r1(cx - 3)}Z`,
		col('lemon')
	);
	out += HI(
		`M${r1(cx - w * 0.5)} ${r1(top - e * 0.4)}Q${r1(cx - w * 0.2)} ${r1(top - e * 0.9)} ${r1(cx + w * 0.1)} ${r1(top - e * 0.9)}`,
		0.5
	);
	return out;
}

function toast({ s, tint, detail, vessel }: BaseCtx): string {
	const T = col(tint);
	const k = Math.min(1.05, s.rx / 31);
	const slice = (x: number, y: number, rot: number) =>
		`<g transform="translate(${r1(x)} ${r1(y)}) rotate(${rot}) scale(${r1(k)})">` +
		P('M-13 6C-17-6-9-13-3-12C0-15 6-15 9-12C15-13 17-5 13 6L12 14Q0 17-12 14Z', BROWN, {
			ink: true
		}) +
		P('M-10 6C-13-3-7-9-2-8.4C1-10.6 5-10.6 7-8.4C11-9 13-3 10 6L9 11Q0 13.4-9 11Z', T, {
			stroke: null
		}) +
		(detail
			? E(-3, 2, 1.1, 0.7, dk(T, 75), { stroke: null, op: 0.5 }) +
				E(4, 5, 1, 0.6, dk(T, 75), { stroke: null, op: 0.5 })
			: '') +
		`</g>`;
	if (vessel === 'board')
		return slice(s.cx - s.rx * 0.52, s.cy - 3, -5) + slice(s.cx + s.rx * 0.52, s.cy - 4, 4);
	return slice(s.cx, s.cy - 3, -3);
}

function pizza({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const cheese = mixc(col('lemon'), MILK, 60);
	let out = E(cx, cy + 2.5, rx, ry, BROWN, { ink: true });
	out += E(cx, cy, rx, ry, col('wheat'), { ink: true });
	out += E(cx, cy, rx * 0.84, ry * 0.84, T, { stroke: OL, sw: 0.7 });
	const n = detail ? 7 : 4;
	for (let i = 0; i < n; i++) {
		const a = (i / n) * 6.28 + rnd();
		const r = 0.15 + rnd() * 0.5;
		out += E(
			cx + Math.cos(a) * r * rx,
			cy + Math.sin(a) * r * ry,
			4.5 + rnd() * 3,
			1.8 + rnd(),
			cheese,
			{ stroke: null, op: 0.85, rot: rnd() * 40 - 20 }
		);
	}
	for (let i = 0; i < 3; i++) {
		const a = (i / 3) * Math.PI + 0.3;
		out += L(
			`M${r1(cx - Math.cos(a) * rx)} ${r1(cy - Math.sin(a) * ry)}L${r1(cx + Math.cos(a) * rx)} ${r1(cy + Math.sin(a) * ry)}`,
			BROWN,
			0.7,
			0.4
		);
	}
	return out;
}

function flatbread({ s, tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	let out = E(cx, cy + 2, rx * 0.92, ry * 0.9, dk(T, 70), { stroke: OL, sw: 0.9 });
	out += E(cx, cy, rx * 0.92, ry * 0.9, T, { stroke: OL, sw: 0.9 });
	const n = detail ? 8 : 5;
	for (let i = 0; i < n; i++) {
		const a = rnd() * 6.28;
		const r = Math.sqrt(rnd()) * 0.75;
		out += E(
			cx + Math.cos(a) * r * rx,
			cy + Math.sin(a) * r * ry,
			2.4 + rnd() * 1.6,
			1.2 + rnd() * 0.6,
			mixc(BROWN, T, 55),
			{ stroke: null, op: 0.6, rot: rnd() * 60 }
		);
	}
	out += HI(
		`M${r1(cx - rx * 0.6)} ${r1(cy - ry * 0.3)}Q${r1(cx - rx * 0.2)} ${r1(cy - ry * 0.75)} ${r1(cx + rx * 0.2)} ${r1(cy - ry * 0.7)}`,
		0.4
	);
	return out;
}

function dumplings({ s, tint, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const spots: [number, number][] = detail
		? [
				[-0.55, -0.35],
				[0, -0.45],
				[0.55, -0.35],
				[-0.3, 0.3],
				[0.3, 0.3],
				[-0.75, 0.25]
			]
		: [
				[-0.5, -0.3],
				[0.1, -0.4],
				[0.55, -0.2],
				[-0.2, 0.3],
				[0.35, 0.35]
			];
	let out = '';
	for (const [a, b] of spots) {
		const x = cx + a * rx;
		const y = cy + b * ry;
		out += P(
			`M${r1(x - 4.4)} ${r1(y + 1.8)}Q${r1(x - 4.4)} ${r1(y - 3.8)} ${r1(x)} ${r1(y - 4)}Q${r1(x + 4.4)} ${r1(y - 3.8)} ${r1(x + 4.4)} ${r1(y + 1.8)}Q${r1(x)} ${r1(y + 3.6)} ${r1(x - 4.4)} ${r1(y + 1.8)}Z`,
			T,
			{ stroke: dk(T, 60), sw: 0.7 }
		);
		out += L(
			`M${r1(x - 3)} ${r1(y - 1.8)}v1.4M${r1(x - 1.4)} ${r1(y - 3)}v1.6M${r1(x + 0.2)} ${r1(y - 3.3)}v1.6M${r1(x + 1.8)} ${r1(y - 3)}v1.6M${r1(x + 3.2)} ${r1(y - 1.8)}v1.4`,
			dk(T, 70),
			0.5,
			0.8
		);
	}
	return out;
}

// -------------------------------------------------------- creamy / sweet

function creamBowl(
	base: 'porridge' | 'yoghurt' | 'smoothie',
	{ s, tint, rnd, detail }: BaseCtx
): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const fill = base === 'smoothie' ? T : lt(T, 85);
	let out = E(cx, cy, rx * 0.95, ry * 0.95, fill, { stroke: OL, sw: 0.9 });
	out += L(
		`M${r1(cx - rx * 0.6)} ${r1(cy + ry * 0.05)}Q${r1(cx - rx * 0.3)} ${r1(cy - ry * 0.7)} ${r1(cx + rx * 0.05)} ${r1(cy - ry * 0.1)}T${r1(cx + rx * 0.6)} ${r1(cy + ry * 0.05)}`,
		dk(fill, 80),
		1.2,
		0.35
	);
	if (base === 'yoghurt')
		out += E(cx, cy - ry * 0.35, rx * 0.35, ry * 0.4, MILK, {
			stroke: dk(MILK, 85),
			sw: 0.5,
			op: 0.9
		});
	if (detail || base === 'porridge') {
		for (let i = 0; i < 5; i++)
			out += E(
				cx + (rnd() - 0.5) * rx * 1.3,
				cy + (rnd() - 0.5) * ry * 1.2,
				0.8,
				0.5,
				dk(fill, 70),
				{ stroke: null, op: 0.5 }
			);
	}
	out += HI(
		`M${r1(cx - rx * 0.55)} ${r1(cy + ry * 0.45)}Q${r1(cx)} ${r1(cy + ry * 0.9)} ${r1(cx + rx * 0.55)} ${r1(cy + ry * 0.45)}`,
		0.35
	);
	return out;
}

/** Slice of cake / pie / crumble on the small plate (vessel 'slice'). */
function wedge(base: 'cake' | 'crumble' | 'pie', { tint, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const crust = mixc(col('wheat'), BROWN, 55);
	let out = P('M82 40L82 64L66 72L66 48Z', base === 'cake' ? dk(T, 85) : crust, { ink: true });
	out += P('M26 48H66V72H26Z', base === 'cake' ? T : crust, { ink: true });
	if (base === 'cake') {
		out += P('M26 56H66V60H26Z', MILK, { stroke: null });
		out += P('M26 66H66V69H26Z', MILK, { stroke: null, op: 0.9 });
		out += P('M26 48L42 40H82L66 48Z', MILK, { ink: true });
		out += L('M28 49Q34 54 40 49Q46 55 52 49Q58 54 64 49', 'var(--bgc)', 1.4, 0.8);
	} else if (base === 'pie') {
		out += P('M26 54H66V66H26Z', T, { stroke: null });
		out += L('M26 54H66M26 66H66', dk(T, 70), 0.6, 0.6);
		out += P('M26 48L42 40H82L66 48Z', crust, { ink: true });
		if (detail)
			out += L('M34 48L46 41M44 48L56 41M54 48L66 41M60 48L72 41', dk(crust, 70), 0.6, 0.6);
	} else {
		out += P('M26 52H66V66H26Z', T, { stroke: null });
		out += P('M26 48L42 40H82L66 48Z', mixc(col('wheat'), BROWN, 70), { ink: true });
		for (let i = 0; i < (detail ? 12 : 6); i++)
			out += E(30 + rnd() * 48, 42 + rnd() * 5, 1.5, 1, dk(crust, 80), { stroke: null, op: 0.7 });
		out += E(27, 50, 3, 3, mixc(crust, T, 65), { stroke: null });
		out += E(36, 49.5, 3.6, 3, mixc(crust, T, 70), { stroke: null });
	}
	return out;
}

/** A whole cake / pie / crumble on the surface. */
function round(
	base: 'cake' | 'crumble' | 'pie',
	{ s, tint, rnd, detail, vessel }: BaseCtx
): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const crust = mixc(col('wheat'), BROWN, 55);
	if (vessel === 'cup') {
		// muffin dome over the paper case
		let out = dome(cx, cy + 1, rx * 1.12, 12, 4, mixc(T, BROWN, 75), 0.9);
		out += HI(
			`M${r1(cx - rx * 0.7)} ${r1(cy - 5)}Q${r1(cx - rx * 0.4)} ${r1(cy - 11)} ${r1(cx - 2)} ${r1(cy - 11)}`,
			0.45,
			1.5
		);
		for (let i = 0; i < 4; i++)
			out += E(cx + (rnd() - 0.5) * rx * 1.2, cy - 2 - rnd() * 6, 0.9, 0.7, dk(T, 60), {
				stroke: null,
				op: 0.6
			});
		return out;
	}
	if (vessel === 'loaf') return '';
	if (base === 'pie') {
		let out = E(cx, cy + 3, rx, ry, crust, { ink: true });
		out += E(cx, cy, rx, ry, crust, { ink: true });
		out += E(cx, cy, rx * 0.84, ry * 0.8, T, { stroke: OL, sw: 0.6 });
		for (const k of [-1, 0, 1]) {
			out += L(
				`M${r1(cx - rx * 0.8)} ${r1(cy + (k * 0.3 - 0.3) * ry)}L${r1(cx + rx * 0.8)} ${r1(cy + (k * 0.3 + 0.3) * ry)}`,
				crust,
				2.2
			);
			out += L(
				`M${r1(cx - rx * 0.8)} ${r1(cy + (k * 0.3 + 0.3) * ry)}L${r1(cx + rx * 0.8)} ${r1(cy + (k * 0.3 - 0.3) * ry)}`,
				crust,
				2.2
			);
		}
		out += E(cx, cy, rx, ry, 'none', { stroke: dk(crust, 70), sw: 0.8 });
		return out;
	}
	if (base === 'crumble') {
		let out = E(cx, cy + 2, rx, ry, 'var(--art-ceramic)', { ink: true });
		out += E(cx, cy, rx, ry, 'var(--art-ceramic)', { ink: true });
		out += E(cx, cy, rx * 0.88, ry * 0.85, T, { stroke: OL, sw: 0.6 });
		out += E(cx, cy - 0.5, rx * 0.8, ry * 0.76, mixc(col('wheat'), BROWN, 70), { stroke: null });
		for (let i = 0; i < (detail ? 16 : 8); i++) {
			const a = rnd() * 6.28;
			const r = Math.sqrt(rnd()) * 0.75;
			out += E(
				cx + Math.cos(a) * r * rx,
				cy + Math.sin(a) * r * ry,
				1.5 + rnd(),
				1,
				i % 2 ? crust : lt(crust, 70),
				{ stroke: dk(crust, 70), sw: 0.3 }
			);
		}
		return out;
	}
	// cake: two layers, icing
	const w = Math.min(rx * 0.85, 22);
	const e = Math.max(3, ry * 0.6);
	const top = cy - 2;
	let out = P(
		`M${r1(cx - w)} ${r1(top)}V${r1(top + 14)}Q${r1(cx)} ${r1(top + 14 + e * 1.8)} ${r1(cx + w)} ${r1(top + 14)}V${r1(top)}Z`,
		T,
		{ ink: true }
	);
	out += P(
		`M${r1(cx - w)} ${r1(top + 5)}Q${r1(cx)} ${r1(top + 5 + e * 1.8)} ${r1(cx + w)} ${r1(top + 5)}V${r1(top + 8)}Q${r1(cx)} ${r1(top + 8 + e * 1.8)} ${r1(cx - w)} ${r1(top + 8)}Z`,
		MILK,
		{ stroke: null }
	);
	out += E(cx, top, w, e, MILK, { ink: true });
	out += E(cx, top, w * 0.6, e * 0.55, 'none', { stroke: dk(MILK, 85), sw: 0.5, op: 0.7 });
	return out;
}

// -------------------------------------------------------- meat and bakes

function fillet({ s, tint, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const w = rx * 0.7;
	const h = ry * 1.15;
	let out = P(
		`M${r1(cx - w)} ${r1(cy)}Q${r1(cx - w * 0.9)} ${r1(cy - h)} ${r1(cx + w * 0.1)} ${r1(cy - h * 0.9)}Q${r1(cx + w)} ${r1(cy - h * 0.8)} ${r1(cx + w)} ${r1(cy)}Q${r1(cx + w * 0.6)} ${r1(cy + h * 0.9)} ${r1(cx - w * 0.2)} ${r1(cy + h * 0.8)}Q${r1(cx - w)} ${r1(cy + h * 0.6)} ${r1(cx - w)} ${r1(cy)}Z`,
		T,
		{ stroke: dk(T, 60), sw: 0.9 }
	);
	const n = detail ? 4 : 3;
	for (let i = 0; i < n; i++) {
		const x = cx - w * 0.55 + i * w * 0.38;
		out += L(
			`M${r1(x)} ${r1(cy - h * 0.55)}L${r1(x + w * 0.28)} ${r1(cy + h * 0.45)}`,
			dk(T, 55),
			1,
			0.5
		);
	}
	out += HI(
		`M${r1(cx - w * 0.6)} ${r1(cy - h * 0.55)}Q${r1(cx - w * 0.2)} ${r1(cy - h * 0.85)} ${r1(cx + w * 0.2)} ${r1(cy - h * 0.8)}`,
		0.5
	);
	return out;
}

function patty({ s, tint, vessel, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	if (vessel === 'bun') {
		const w = rx * 1.02;
		let out = P(
			`M${r1(cx - w)} ${r1(cy - 1)}Q${r1(cx - w)} ${r1(cy - 4.5)} ${r1(cx - w + 3)} ${r1(cy - 4.5)}H${r1(cx + w - 3)}Q${r1(cx + w)} ${r1(cy - 4.5)} ${r1(cx + w)} ${r1(cy - 1)}V${r1(cy + 2.5)}Q${r1(cx + w)} ${r1(cy + 5.5)} ${r1(cx + w - 3)} ${r1(cy + 5.5)}H${r1(cx - w + 3)}Q${r1(cx - w)} ${r1(cy + 5.5)} ${r1(cx - w)} ${r1(cy + 2.5)}Z`,
			T,
			{ ink: true }
		);
		out += P(
			`M${r1(cx - w - 1)} ${r1(cy - 2)}H${r1(cx + w + 1)}L${r1(cx + w - 3)} ${r1(cy + 1)}L${r1(cx + w - 8)} ${r1(cy - 1)}L${r1(cx + 4)} ${r1(cy + 2.5)}L${r1(cx - 6)} ${r1(cy - 1)}L${r1(cx - w + 4)} ${r1(cy + 1.5)}Z`,
			col('lemon'),
			{ stroke: dk(col('pumpkin'), 80), sw: 0.5 }
		);
		return out;
	}
	let out = '';
	const spots: [number, number][] = [
		[-0.5, -0.25],
		[0.05, 0.25],
		[0.55, -0.2]
	];
	for (const [a, b] of spots) {
		const x = cx + a * rx;
		const y = cy + b * ry;
		out += E(x, y + 1.6, 7.5, 3.4, dk(T, 70), { stroke: dk(T, 50), sw: 0.7 });
		out += E(x, y, 7.5, 3.4, T, { stroke: dk(T, 55), sw: 0.7 });
		if (detail)
			out +=
				E(x - 1, y - 0.6, 2.2, 0.9, dk(T, 70), { stroke: null, op: 0.5 }) +
				E(x + 2.5 + rnd(), y + 0.4, 1.3, 0.7, lt(T, 70), { stroke: null, op: 0.5 });
	}
	return out;
}

function roast({ s, tint }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const w = Math.min(rx * 0.55, 17);
	const h = Math.min(ry * 1.5, 11);
	const y = cy - 1;
	let out = E(cx - w * 0.7, y + h * 0.65, w * 0.34, h * 0.28, T, {
		stroke: dk(T, 55),
		sw: 0.8,
		rot: 25
	});
	out += E(cx + w * 0.7, y + h * 0.65, w * 0.34, h * 0.28, T, {
		stroke: dk(T, 55),
		sw: 0.8,
		rot: -25
	});
	out += E(cx - w * 1.0, y + h * 0.95, 1.4, 1.1, MILK, { stroke: dk(MILK, 70), sw: 0.4 });
	out += E(cx + w * 1.0, y + h * 0.95, 1.4, 1.1, MILK, { stroke: dk(MILK, 70), sw: 0.4 });
	out += E(cx, y, w, h, T, { stroke: dk(T, 55), sw: 0.9 });
	out += E(cx + w * 0.1, y - h * 0.1, w * 0.62, h * 0.55, mixc(T, col('pumpkin'), 70), {
		stroke: null,
		op: 0.7
	});
	out += L(
		`M${r1(cx - w * 0.4)} ${r1(y + h * 0.1)}Q${r1(cx)} ${r1(y + h * 0.5)} ${r1(cx + w * 0.4)} ${r1(y + h * 0.1)}`,
		dk(T, 55),
		0.7,
		0.55
	);
	out += HI(
		`M${r1(cx - w * 0.6)} ${r1(y - h * 0.4)}Q${r1(cx - w * 0.3)} ${r1(y - h * 0.75)} ${r1(cx + w * 0.1)} ${r1(y - h * 0.7)}`,
		0.5,
		1.5
	);
	return out;
}

function bake(base: 'traybake' | 'gratin', { s, tint, vessel, rnd, detail }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const tray = vessel === 'tray';
	const gold = base === 'gratin' ? mixc(T, col('pumpkin'), 65) : mixc(T, col('wheat'), 70);
	const inner = 'M28 49.5H72L83.5 61H16.5Z';
	let out = tray
		? P(inner, gold, { stroke: OL, sw: 0.8 })
		: E(cx, cy, rx * 0.95, ry * 0.92, gold, { stroke: OL, sw: 0.9 });
	const n = detail ? 8 : 5;
	for (let i = 0; i < n; i++) {
		const x = tray ? 24 + rnd() * 52 : cx + (rnd() - 0.5) * rx * 1.5;
		const y = tray ? 51 + rnd() * 9 : cy + (rnd() - 0.5) * ry * 1.4;
		out += E(
			x,
			y,
			base === 'gratin' ? 3.4 + rnd() * 2 : 2.4,
			base === 'gratin' ? 1.5 : 1.1,
			mixc(BROWN, gold, 55),
			{ stroke: null, op: 0.6, rot: rnd() * 40 - 20 }
		);
	}
	if (base === 'gratin' && detail)
		out += E(cx - rx * 0.2, cy - ry * 0.2, 1.6, 1.1, lt(gold, 70), { stroke: null, op: 0.9 });
	out += HI(
		tray
			? 'M32 52Q50 48.5 66 52'
			: `M${r1(cx - rx * 0.6)} ${r1(cy - ry * 0.4)}Q${r1(cx)} ${r1(cy - ry * 0.85)} ${r1(cx + rx * 0.5)} ${r1(cy - ry * 0.4)}`,
		0.4
	);
	void TRAY_INNER;
	return out;
}

function sushiRolls({ s, tint }: BaseCtx): string {
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	const nori = col('dark');
	const fills = [T, col('pea'), col('pink')];
	let out = '';
	let k = 0;
	for (const row of [0, 1]) {
		for (const c of [0, 1, 2]) {
			const x = cx + (c - 1) * rx * 0.64 + (row ? rx * 0.32 : -rx * 0.1);
			const y = cy + (row - 0.5) * ry * 1.0 - 1;
			const w = rx * 0.22;
			out += P(
				`M${r1(x - w)} ${r1(y)}V${r1(y + 4)}Q${r1(x)} ${r1(y + 4 + ry * 0.9)} ${r1(x + w)} ${r1(y + 4)}V${r1(y)}Z`,
				nori,
				{ stroke: null }
			);
			out += E(x, y, w, ry * 0.42, MILK, { stroke: nori, sw: 1.3 });
			out += E(x, y, w * 0.4, ry * 0.17, fills[k++ % 3], { stroke: null });
		}
	}
	return out;
}

// ------------------------------------------------------------- columns

function lay(
	xl: (y: number) => number,
	xr: (y: number) => number,
	y1: number,
	y2: number,
	wave = 1.6
): string {
	const mid = (xl(y1) + xr(y1)) / 2;
	return `M${r1(xl(y1))} ${y1}Q${r1((xl(y1) + mid) / 2)} ${r1(y1 - wave)} ${r1(mid)} ${y1}T${r1(xr(y1))} ${y1}L${r1(xr(y2))} ${y2}L${r1(xl(y2))} ${y2}Z`;
}

/** Contents of glass / jar / mug / cup. */
function column(base: Base, c: BaseCtx): string {
	const { s, tint, sideTint, vessel } = c;
	const T = col(tint);
	const { cx, cy, rx, ry } = s;
	if (vessel === 'mug')
		return (
			liquid(base === 'creamy' || base === 'broth' ? base : 'thick', c) +
			HI(
				`M${r1(cx - rx * 0.5)} ${r1(cy + ry * 0.4)}Q${r1(cx)} ${r1(cy + ry * 0.9)} ${r1(cx + rx * 0.5)} ${r1(cy + ry * 0.4)}`,
				0.3
			)
		);
	if (vessel === 'cup') return round('cake', c);
	const glass = vessel === 'glass';
	const xl = glass ? (y: number) => 32.5 + (y - 29) * 0.0636 : () => 31;
	const xr = glass ? (y: number) => 67.5 - (y - 29) * 0.0636 : () => 69;
	const bottom = glass ? 88 : 85;
	const topY = glass ? 29 : 38;
	const body = glass ? GLASS_BODY : JAR_BODY;
	const wave = 1.8;
	let out = '';
	let topColour = T;
	if (base === 'smoothie') {
		out += P(body, T, { stroke: null, op: 0.92 });
		out += P(lay(xl, xr, topY + 14, bottom, wave), dk(T, 82), { stroke: null, op: 0.5 });
		topColour = lt(T, 72);
	} else if (base === 'yoghurt' || base === 'porridge') {
		const berry = col(sideTint ?? 'plum');
		const oat = base === 'porridge' ? lt(T, 80) : MILK;
		out += P(lay(xl, xr, bottom - 22, bottom), berry, { stroke: null, op: 0.88 });
		out += P(lay(xl, xr, bottom - 38, bottom - 20, 2), oat, { stroke: null });
		out += P(lay(xl, xr, bottom - 50, bottom - 36, 2.2), col('wheat'), { stroke: null });
		out += P(lay(xl, xr, topY + 1, bottom - 48, 2), oat, { stroke: null });
		topColour = oat;
	} else {
		out += P(body, T, { stroke: null, op: 0.9 });
		out += P(lay(xl, xr, topY + 10, bottom), dk(T, 85), { stroke: null, op: 0.5 });
		topColour = lt(T, 80);
	}
	out += E(cx, cy, rx, ry, topColour, { stroke: dk(topColour, 70), sw: 0.6 });
	if (base === 'smoothie')
		out +=
			E(cx - 4, cy - 0.4, 1.1, 0.7, MILK, { stroke: null, op: 0.7 }) +
			E(cx + 3, cy + 0.6, 0.8, 0.5, MILK, { stroke: null, op: 0.7 });
	return out;
}
