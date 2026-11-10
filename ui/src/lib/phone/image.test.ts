import { describe, expect, it } from 'vitest';
import { fitWithin } from './image.js';

describe('fitWithin', () => {
	it('scales the long side down to 2000 and keeps the ratio', () => {
		expect(fitWithin(4000, 3000)).toEqual({ w: 2000, h: 1500 });
		expect(fitWithin(3000, 4000)).toEqual({ w: 1500, h: 2000 });
	});
	it('never scales up', () => {
		expect(fitWithin(924, 2000)).toEqual({ w: 924, h: 2000 });
		expect(fitWithin(500, 400)).toEqual({ w: 500, h: 400 });
	});
});
