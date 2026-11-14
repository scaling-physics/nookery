// Tiny SVG string helpers for the dish drawing (vessels.ts, bases.ts).
// Colours are CSS expressions (var(--c-*), color-mix) so they follow light
// and dark. Pure.

export type Ellipse = { cx: number; cy: number; rx: number; ry: number };

export const r1 = (n: number): number => Math.round(n * 10) / 10;
export const col = (name: string): string => `var(--c-${name})`;
/** Darker (towards ink). */
export const dk = (c: string, p = 55): string => `color-mix(in srgb, ${c} ${p}%, #1a1008)`;
/** Lighter (towards cream). */
export const lt = (c: string, p = 65): string => `color-mix(in srgb, ${c} ${p}%, #fffaf0)`;
export const mixc = (a: string, b: string, p: number): string =>
	`color-mix(in srgb, ${a} ${p}%, ${b})`;

export type O = {
	/** Outline colour; null = none. Default: a darker shade of the fill. */
	stroke?: string | null;
	sw?: number;
	op?: number;
	/** Vessel-style outline (var(--ol), 1.1). */
	ink?: boolean;
	/** rotate(a cx cy) */
	rot?: number;
	dash?: string;
};

function attrs(fill: string, o: O, cx = 0, cy = 0): string {
	let a = `fill="${fill}"`;
	if (o.ink) a += ` stroke="var(--ol)" stroke-width="1.1" stroke-linejoin="round"`;
	else if (o.stroke !== null) {
		const s = o.stroke ?? (fill === 'none' ? 'var(--ol)' : dk(fill));
		a += ` stroke="${s}" stroke-width="${o.sw ?? 0.6}" stroke-linejoin="round" stroke-linecap="round"`;
	}
	if (o.op !== undefined) a += ` opacity="${o.op}"`;
	if (o.dash) a += ` stroke-dasharray="${o.dash}"`;
	if (o.rot) a += ` transform="rotate(${r1(o.rot)} ${r1(cx)} ${r1(cy)})"`;
	return a;
}

export const E = (
	cx: number,
	cy: number,
	rx: number,
	ry: number,
	fill: string,
	o: O = {}
): string =>
	`<ellipse cx="${r1(cx)}" cy="${r1(cy)}" rx="${r1(rx)}" ry="${r1(ry)}" ${attrs(fill, o, cx, cy)}/>`;

export const P = (d: string, fill: string, o: O = {}): string =>
	`<path d="${d}" ${attrs(fill, o)}/>`;

/** An open stroke. */
export const L = (d: string, stroke: string, sw = 1, op = 1): string =>
	`<path d="${d}" fill="none" stroke="${stroke}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round"${op < 1 ? ` opacity="${op}"` : ''}/>`;

/** A soft highlight stroke. */
export const HI = (d: string, op = 0.35, sw = 1.4): string => L(d, 'var(--c-milk)', sw, op);

export const G = (inner: string, transform: string): string =>
	`<g transform="${transform}">${inner}</g>`;

/** Elements in a markup string (for the size budget). */
export function countEls(s: string): number {
	return (s.match(/<(?!\/)[a-z]/g) ?? []).length;
}
