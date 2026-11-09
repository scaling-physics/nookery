import { describe, expect, it } from 'vitest';
import {
	formatCents,
	formatDiff,
	frozenLabel,
	itemName,
	parseFieldError,
	statusLabel,
	urgent
} from './format.js';
import type { LotView } from './bindings.js';

describe('statusLabel', () => {
	it('words days naturally', () => {
		expect(statusLabel({ status: 'use_soon', days: 0 }).text).toBe('Use today');
		expect(statusLabel({ status: 'use_soon', days: 1 }).text).toBe('Use by tomorrow');
		expect(statusLabel({ status: 'expired', days: 1 }).text).toBe('Expired yesterday');
		expect(statusLabel({ status: 'fresh', days: 30 }).text).toBe('4 weeks left');
		expect(statusLabel({ status: 'fresh', days: 300 }).text).toBe('10 months left');
	});

	it('colours by urgency', () => {
		expect(statusLabel({ status: 'expired', days: 3 }).tone).toBe('expired');
		expect(statusLabel({ status: 'fresh', days: 5 }).tone).toBe('fresh');
		expect(statusLabel({ status: 'undated', days: null }).tone).toBe('muted');
	});
});

describe('urgent', () => {
	it('keeps only expired and use-soon lots, in order', () => {
		const lot = (item: string, status: LotView['status']) => ({ item, status }) as LotView;
		const got = urgent([lot('a', 'expired'), lot('b', 'fresh'), lot('c', 'use_soon')]);
		expect(got.map((l) => l.item)).toEqual(['a', 'c']);
	});
});

describe('itemName', () => {
	it('capitalises the first letter only', () => {
		expect(itemName('greek yoghurt')).toBe('Greek yoghurt');
	});
});

describe('frozenLabel', () => {
	it('counts days, then weeks', () => {
		const today = new Date(2026, 10, 8);
		expect(frozenLabel('2026-11-08', today)).toBe('Frozen today');
		expect(frozenLabel('2026-11-07', today)).toBe('Frozen yesterday');
		expect(frozenLabel('2026-10-27', today)).toBe('Frozen 12 days ago');
		expect(frozenLabel('2026-09-29', today)).toBe('Frozen 6 weeks ago');
	});
});

describe('formatCents', () => {
	it('prints euros with a decimal comma and keeps the sign', () => {
		expect(formatCents(977)).toBe('9,77 €');
		expect(formatCents(5)).toBe('0,05 €');
		expect(formatCents(0)).toBe('0,00 €');
		expect(formatCents(-25)).toBe('-0,25 €');
		expect(formatCents(123456)).toBe('1234,56 €');
	});

	it('shows an explicit plus for positive differences', () => {
		expect(formatDiff(5)).toBe('+0,05 €');
		expect(formatDiff(-5)).toBe('-0,05 €');
	});
});

describe('parseFieldError', () => {
	it('splits a field and a line number', () => {
		expect(parseFieldError('qty:3')).toEqual({ field: 'qty', line: 3 });
		expect(parseFieldError('total')).toEqual({ field: 'total', line: null });
	});
});
