import { describe, expect, it } from 'vitest';
import { hash, mulberry32 } from './dish.js';

describe('prng', () => {
	it('hashes stably', () => {
		expect(hash('x')).toBe(hash('x'));
		expect(hash('x')).not.toBe(hash('y'));
	});
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
