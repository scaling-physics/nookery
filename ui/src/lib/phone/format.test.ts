import { describe, expect, it } from 'vitest';
import { formatBytes, formatEta, formatSpeed } from './format.js';

describe('phone formats', () => {
	it('formats sizes', () => {
		expect(formatBytes(2_600_000_000)).toBe('2.6 GB');
		expect(formatBytes(850_000)).toBe('850 kB');
		expect(formatBytes(12)).toBe('12 B');
		expect(formatSpeed(12_000_000)).toBe('12 MB/s');
	});
	it('formats time left', () => {
		expect(formatEta(30)).toBe('about 30 s left');
		expect(formatEta(600)).toBe('about 10 min left');
		expect(formatEta(Infinity)).toBe('');
	});
});
