import { describe, expect, it } from 'vitest';
import type { PhoneAiView } from '#lib/bindings.js';
import { ranOnPhone } from './where.js';

const v = (pref: PhoneAiView['pref'], downloaded: boolean): PhoneAiView => ({
	available: true,
	pref,
	paired: true,
	model: {
		downloaded,
		bytes: 0,
		partialBytes: 0,
		freeBytes: null,
		expectedBytes: 1,
		downloading: false
	}
});

describe('ranOnPhone', () => {
	it('follows the choice, and automatic follows the download', () => {
		expect(ranOnPhone(v('phone', false))).toBe('on this phone');
		expect(ranOnPhone(v('mac', true))).toBe('on your Mac');
		expect(ranOnPhone(v('auto', true))).toBe('on this phone');
		expect(ranOnPhone(v('auto', false))).toBe('on your Mac');
	});
});
