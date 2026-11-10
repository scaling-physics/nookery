import { describe, expect, it } from 'vitest';
import type { DayView, PlanView, ShoppingItemView } from '../bindings.js';
import {
	addDays,
	dayHeading,
	daySuggestion,
	groupShopping,
	nextMonday,
	planNeeds,
	shortDate,
	weekdayName,
	weekLabel
} from './helpers.js';

const item = (name: string, category: ShoppingItemView['category'], done = false) =>
	({ id: name, item: name, qty: null, category, done, from: 'manual' }) as ShoppingItemView;

const day = (date: string, need: string[], cooked = false): DayView => ({
	date,
	name: 'x',
	why: '',
	minutes: 20,
	have: [],
	need,
	needCategories: need.map(() => 'other'),
	usesSoon: 1,
	cooked
});

describe('dates', () => {
	it('names weekdays and short dates without time zone drift', () => {
		expect(weekdayName('2026-11-09')).toBe('Monday');
		expect(weekdayName('2026-11-15')).toBe('Sunday');
		expect(shortDate('2026-11-09')).toBe('9 Nov');
		expect(weekLabel('2026-12-28')).toBe('Week of 28 Dec');
	});
	it('adds days across month ends', () => {
		expect(addDays('2026-11-29', 3)).toBe('2026-12-02');
		expect(addDays('2026-03-01', -1)).toBe('2026-02-28');
	});
	it('finds the next Monday', () => {
		expect(nextMonday('2026-11-09')).toBe('2026-11-09');
		expect(nextMonday('2026-11-11')).toBe('2026-11-16');
		expect(nextMonday('2026-11-15')).toBe('2026-11-16');
	});
	it('says today and tomorrow', () => {
		expect(dayHeading('2026-11-09', '2026-11-09')).toBe('Today');
		expect(dayHeading('2026-11-10', '2026-11-09')).toBe('Tomorrow');
		expect(dayHeading('2026-11-12', '2026-11-09')).toBe('Thursday');
	});
});

describe('shopping groups', () => {
	it('orders by aisle, keeps ticked items last and sorts by name', () => {
		const g = groupShopping([
			item('rice', 'grains'),
			item('milk', 'dairy', true),
			item('kale', 'vegetables'),
			item('butter', 'dairy'),
			item('apple', 'fruit')
		]);
		expect(g.map((x) => x.category)).toEqual(['vegetables', 'fruit', 'dairy', 'grains']);
		expect(g[2].items.map((i) => i.item)).toEqual(['butter', 'milk']);
	});
	it('is empty for nothing', () => expect(groupShopping([])).toEqual([]));
});

describe('plan helpers', () => {
	it('lists needs once and skips cooked days', () => {
		const plan = {
			week: '2026-11-09',
			days: [
				day('2026-11-09', ['Onion', 'lemon']),
				day('2026-11-10', ['onion ']),
				day('2026-11-11', ['kale'], true)
			],
			spares: [],
			template: null,
			note: '',
			createdAt: ''
		} as PlanView;
		expect(planNeeds(plan)).toEqual(['Onion', 'lemon']);
	});
	it('turns a day into a suggestion', () => {
		const s = daySuggestion(day('2026-11-09', ['kale']), 2);
		expect(s).toMatchObject({ index: 2, need: ['kale'], usesSoon: 1, expired: [] });
	});
});
