// The vessel drawings (SVG markup strings, 100 x 100 canvas). `back` is
// drawn before the food, `front` after it (rims, glass, a top bun). Colours:
// --art-* tokens; `--bgc` (the nook tone) and `--base` (the dish tint) are
// set on the root <svg> by DishArt. Pure.

import type { Vessel } from './types.js';
import { E, G, HI, L, P, col, dk, mixc } from './svg.js';
import { GLASS_BODY, JAR_BODY, SKEWERS, TRAY_INNER } from './compose.js';

const CER = 'var(--art-ceramic)';
const CIN = 'var(--art-ceramic-in)';
const IRON = 'var(--art-iron)';
const IIN = 'var(--art-iron-in)';
const IHI = 'var(--art-iron-hi)';
const WOOD = 'var(--art-wood)';
const WDK = 'var(--art-wood-dk)';
const BG = 'var(--bgc)';
const INK = { ink: true } as const;
const BUN = mixc(col('pumpkin'), col('wheat'), 55);
const SHELL = mixc(col('wheat'), col('pumpkin'), 70);

function plateAt(cx: number, cy: number, rx: number, ry: number): string {
	return (
		E(cx, cy + 4, rx, ry, CIN, INK) +
		E(cx, cy, rx, ry, CER, INK) +
		E(cx, cy, rx * 0.75, ry * 0.7, CIN, { stroke: null, op: 0.6 }) +
		E(cx, cy + 1, rx * 0.725, ry * 0.667, 'none', { stroke: BG, sw: 1.4, op: 0.55 })
	);
}

function bowlBack(): string {
	return (
		P('M40 85H60L62 91H38Z', CIN, INK) +
		P('M13 48C13 74 29 87 50 87C71 87 87 74 87 48Z', CER, INK) +
		L('M15 60Q50 74 85 60', BG, 2.4) +
		L('M15 65Q50 79 85 65', BG, 1, 0.6) +
		E(50, 48, 37, 9.5, CIN, INK)
	);
}

function steamerTier(): string {
	return (
		P('M16 62V74A34 9 0 0 0 84 74V62Z', WOOD, INK) +
		L('M16 68A34 9 0 0 0 84 68', WDK, 1) +
		P('M16 50V62A34 9 0 0 0 84 62V50Z', WOOD, INK) +
		L('M16 56A34 9 0 0 0 84 56', WDK, 1)
	);
}

function tacoShell(): string {
	return (
		P('M14 50Q50 24 86 50Q50 58 14 50Z', dk(SHELL, 80), INK) +
		P('M12 50Q50 66 88 50Q82 80 50 83Q18 80 12 50Z', SHELL, INK) +
		HI('M18 54Q50 66 82 54', 0.5, 1.2) +
		L('M30 66Q50 74 70 66', dk(SHELL, 75), 0.8, 0.6)
	);
}

export function vesselSvg(v: Vessel, layer: 'back' | 'front'): string {
	return layer === 'back' ? back(v) : front(v);
}

function back(v: Vessel): string {
	switch (v) {
		case 'plate':
		case 'salad':
			return plateAt(50, 62, 40, 15);
		case 'stack':
			return plateAt(50, 72, 38, 11);
		case 'bowl':
			return bowlBack();
		case 'small':
			return G(
				bowlBack() +
					L('M78 60L96 34', IHI, 2.4) +
					E(97, 31, 3.4, 5, IHI, { stroke: null, rot: 35 }),
				'translate(10 16) scale(.8)'
			);
		case 'deep':
			return (
				P('M40 85H60L62 91H38Z', CIN, INK) +
				P('M11 45C11 73 28 87 50 87C72 87 89 73 89 45Z', CER, INK) +
				L('M13 58Q50 74 87 58', BG, 2.4) +
				E(50, 45, 39, 9.5, CIN, INK)
			);
		case 'glass':
			return (
				L('M60 40L80 8', IHI, 2.4) +
				E(81.4, 5.4, 3, 4.3, IHI, { stroke: null, rot: 30 }) +
				P(GLASS_BODY, CER, { stroke: null, op: 0.18 })
			);
		case 'pan':
			return (
				P('M72 59L95 52Q99 51 99 55L98 58Q97 61 93 62L72 66Z', IRON, {
					stroke: 'var(--ol)',
					sw: 1
				}) +
				E(94, 56.5, 1.4, 1.4, 'var(--bg)', { stroke: null }) +
				E(48, 66, 37, 15, IIN, INK) +
				E(48, 61, 37, 15, IRON, INK) +
				E(48, 61, 32, 12, IIN, { stroke: null }) +
				L('M22 56Q48 50 74 56', IHI, 1.2, 0.7)
			);
		case 'wok':
			return (
				L('M88 48L99 33', 'var(--ol)', 6.6) +
				L('M88 48L99 33', IRON, 4.6) +
				P('M10 47C12 74 30 86 50 86C70 86 88 74 90 47Z', IRON, INK) +
				L('M20 62Q50 78 80 62', IHI, 1.4, 0.7) +
				E(50, 47, 40, 9, IIN, INK)
			);
		case 'board':
			return (
				P('M10 56Q50 46 90 56L94 80Q50 90 6 80Z', WDK, INK) +
				P('M10 52Q50 42 90 52L94 76Q50 86 6 76Z', WOOD, INK) +
				L('M16 66Q50 60 84 66', WDK, 0.8, 0.6) +
				E(84, 56, 1.6, 1.6, WDK, { stroke: null })
			);
		case 'wrap':
			return (
				P('M8 78L16 56L88 50L94 76Z', 'color-mix(in srgb, var(--c-cream) 80%, var(--surface-2))', {
					ink: true
				}) +
				L('M22 66L40 64M24 70L44 68', col('wheat'), 0.8, 0.6) +
				P('M12 44H72V72H12Q8 58 12 44Z', col('wheat'), INK) +
				L('M18 50Q45 45 68 50', col('milk'), 1.6, 0.5) +
				P('M12 62Q12 72 26 72H50L44 56Q26 58 12 62Z', BG, {
					stroke: 'var(--ol)',
					sw: 1,
					op: 0.85
				}) +
				E(74, 58, 6.5, 14.5, dk(col('wheat'), 80), INK)
			);
		case 'round':
			return (
				E(50, 66, 45, 19, WDK, INK) +
				E(50, 62, 45, 19, WOOD, INK) +
				L('M16 64Q50 76 84 64', WDK, 0.8, 0.5)
			);
		case 'slice':
			return E(50, 76, 38, 9, CER, INK) + E(50, 75, 29, 6, CIN, { stroke: null, op: 0.7 });
		case 'pot':
			return (
				E(15, 53, 4, 3.4, IRON, INK) +
				E(85, 53, 4, 3.4, IRON, INK) +
				P('M16 48C16 76 30 87 50 87C70 87 84 76 84 48Z', 'var(--art-enamel)', INK) +
				L('M22 62Q50 76 78 62', 'var(--c-milk)', 1.2, 0.3) +
				E(50, 48, 34, 8.4, 'var(--art-enamel-in)', INK)
			);
		case 'tray':
			return (
				P('M8 64H92V72Q92 77 87 77H13Q8 77 8 72Z', CER, INK) +
				P('M22 44H78L92 64H8Z', CER, INK) +
				P(TRAY_INNER, CIN, { stroke: 'var(--ol)', sw: 0.8 }) +
				L('M12 70H88', BG, 1.6, 0.9)
			);
		case 'skewer':
			return (
				plateAt(50, 64, 40, 12) +
				SKEWERS.map(
					([x1, y1, x2, y2]) =>
						L(`M${x1} ${y1}L${x2} ${y2}`, 'var(--ol)', 2.2) +
						L(`M${x1} ${y1}L${x2} ${y2}`, WOOD, 1.3)
				).join('')
			);
		case 'bun':
			return (
				E(50, 81, 39, 6, WDK, INK) +
				E(50, 79, 39, 6, WOOD, INK) +
				P('M23 71Q23 64 50 64Q77 64 77 71Q77 78 50 78Q23 78 23 71Z', BUN, INK) +
				HI('M30 74Q50 78 70 74', 0.4)
			);
		case 'taco':
			return (
				G(tacoShell(), 'translate(-6 -8) translate(50 60) scale(.8) translate(-50 -60)') +
				P('M14 50Q50 24 86 50Q50 58 14 50Z', dk(SHELL, 80), INK)
			);
		case 'mug':
			return (
				L('M70 50Q88 50 86 62Q84 74 70 72', 'var(--ol)', 6.6) +
				L('M70 50Q88 50 86 62Q84 74 70 72', CER, 4.4) +
				P('M30 40H70V76Q70 84 62 84H38Q30 84 30 76Z', CER, INK) +
				L('M31 62H69', BG, 2.4) +
				E(50, 40, 20, 5.5, CIN, INK)
			);
		case 'cup':
			return (
				P('M30 60H70L66 84H34Z', mixc(col('pink'), CER, 45), INK) +
				['M38 60L39.5 84', 'M46 60L46.5 84', 'M54 60L53.5 84', 'M62 60L60.5 84']
					.map((d) => L(d, dk(col('pink'), 60), 0.7, 0.55))
					.join('') +
				E(50, 60, 20, 4.5, mixc(col('pink'), CER, 45), INK)
			);
		case 'steamer':
			return (
				steamerTier() + E(50, 50, 34, 9, WOOD, INK) + E(50, 50.5, 30, 7.5, WDK, { stroke: null })
			);
		case 'sushi':
			return (
				P('M6 70H94V74Q94 77 90 77H10Q6 77 6 74Z', 'var(--art-slate-dk)', INK) +
				P('M14 50H86L94 70H6Z', 'var(--art-slate)', INK) +
				E(83, 59, 8, 2.8, CER, INK) +
				E(83, 59.4, 5.8, 1.8, col('dark'), { stroke: null }) +
				L('M58 69L92 65', WOOD, 1.5) +
				L('M58 72L92 68', WOOD, 1.5)
			);
		case 'jar':
			return (
				E(14, 82, 9, 3.2, IRON, INK) +
				E(14, 80, 9, 3.2, IHI, INK) +
				P(JAR_BODY, CER, { stroke: null, op: 0.16 })
			);
		case 'loaf':
			return (
				E(50, 78, 41, 9, WDK, INK) +
				E(50, 75, 41, 9, WOOD, INK) +
				P(
					'M20 70Q16 48 34 40Q50 34 66 40Q84 48 80 70Q50 78 20 70Z',
					mixc('var(--base)', col('brown'), 50),
					INK
				) +
				L('M34 46L38 60M46 43L49 62M58 43L60 62', dk(col('brown'), 70), 0.9, 0.55) +
				HI('M28 52Q32 44 44 41', 0.4)
			);
	}
}

function front(v: Vessel): string {
	switch (v) {
		case 'glass':
			return (
				L(GLASS_BODY.replace('Z', ''), 'var(--ol)', 1.1) +
				L('M37 34L39.5 78', '#fff', 1.6, 0.45) +
				E(50, 29, 17.5, 4, 'none', { ink: true }) +
				E(50, 29, 15.5, 3.4, col('milk'), { stroke: null, op: 0.0 })
			);
		case 'jar':
			return (
				L(
					'M33 29H67V37Q69 38 69 42V82Q69 85 66 85H34Q31 85 31 82V42Q31 38 33 37Z',
					'var(--ol)',
					1.1
				) +
				E(50, 29, 17, 3.2, 'none', INK) +
				P('M34 62H66V74H34Z', col('milk'), { stroke: 'var(--ol)', sw: 0.6, op: 0.85 }) +
				L('M38 66H56M38 70H48', col('muted'), 0.8, 0.8) +
				L('M35 46L36.4 80', '#fff', 1.6, 0.45)
			);
		case 'mug':
			return L('M35 52L36 76', '#fff', 1.6, 0.4);
		case 'bun':
			return (
				P('M21 59Q19 34 50 33Q81 34 79 59Q50 64 21 59Z', mixc(BUN, col('brown'), 80), INK) +
				HI('M28 52Q30 40 44 37', 0.45, 1.6) +
				[
					[40, 42],
					[52, 39],
					[62, 44],
					[46, 49],
					[58, 52],
					[34, 52]
				]
					.map(([x, y]) => E(x, y, 1.5, 0.8, col('milk'), { stroke: dk(BUN), sw: 0.3, rot: x * 5 }))
					.join('')
			);
		case 'taco':
			return (
				P('M12 50Q50 66 88 50Q82 80 50 83Q18 80 12 50Z', SHELL, INK) +
				HI('M18 54Q50 66 82 54', 0.5, 1.2) +
				L('M30 68Q50 76 70 68', dk(SHELL, 75), 0.8, 0.6)
			);
		case 'steamer':
			return (
				P('M16 50A34 9 0 0 0 84 50L80 50A30 7.5 0 0 1 20 50Z', WOOD, { ink: true }) +
				L('M16 50A34 9 0 0 0 84 50', WDK, 0.8, 0.6)
			);
		default:
			return '';
	}
}
