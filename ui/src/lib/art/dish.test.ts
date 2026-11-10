import { describe, expect, it } from 'vitest';
import { hash, isHot, mulberry32, plan, toppingKinds, vesselFor } from './dish.js';

describe('vesselFor', () => {
	it.each([
		['Spaghetti aglio e olio', 'plate'],
		['Creamy mushroom pasta', 'plate'],
		['Chickpea and spinach curry', 'deep'],
		['Tomato soup', 'deep'],
		['Linsen Eintopf', 'deep'],
		['Greek salad', 'salad'],
		['Pasta salad', 'plate'],
		['Spinach and halloumi omelette', 'pan'],
		['Shakshuka', 'pan'],
		['Veggie stir-fry', 'wok'],
		['Avocado toast', 'board'],
		['Black bean burrito', 'wrap'],
		['Margherita pizza', 'round'],
		['Apple porridge', 'small'],
		['Skyr parfait', 'glass'],
		['Plum crumble', 'slice'],
		['Buddha bowl', 'bowl'],
		['Something mysterious', 'bowl']
	])('%s -> %s', (title, vessel) => {
		expect(vesselFor(title)).toBe(vessel);
	});
});

describe('plan', () => {
	it('is the same every time for the same title', () => {
		expect(plan('Tomato soup', ['vegetables'])).toEqual(plan('Tomato soup', ['vegetables']));
		expect(plan('Tomato soup', ['vegetables'])).toEqual(plan(' tomato SOUP ', ['vegetables']));
	});
	it('differs between titles', () => {
		const a = plan('Mushroom pasta', ['vegetables']);
		const b = plan('Garlic pasta', ['vegetables']);
		expect(a.items).not.toEqual(b.items);
	});
	it('keeps toppings on the food surface', () => {
		const p = plan('Big bowl', ['vegetables', 'eggs', 'herbs', 'cheese']);
		for (const i of p.items) {
			const dx = (i.x - p.surface.cx) / p.surface.rx;
			const dy = (i.y - p.surface.cy) / p.surface.ry;
			expect(dx * dx + dy * dy).toBeLessThanOrEqual(1);
		}
	});
	it('works with no categories', () => {
		expect(plan('Mystery', []).items.length).toBeGreaterThan(0);
	});
	it('steams only hot dishes', () => {
		expect(plan('Tomato soup', []).steam).toHaveLength(3);
		expect(plan('Greek salad', []).steam).toHaveLength(0);
		expect(isHot('plate', 'cold noodle plate')).toBe(false);
	});
});

describe('toppingKinds', () => {
	it('maps categories to distinct toppings', () => {
		expect(toppingKinds(['eggs', 'cheese', 'eggs'])).toEqual(['egg', 'cube']);
		expect(toppingKinds(['drinks'])).toEqual([]);
	});
});

describe('prng', () => {
	it('is deterministic and in range', () => {
		const a = mulberry32(hash('x'));
		const b = mulberry32(hash('x'));
		for (let i = 0; i < 20; i++) {
			const v = a();
			expect(v).toBe(b());
			expect(v).toBeGreaterThanOrEqual(0);
			expect(v).toBeLessThan(1);
		}
	});
});
