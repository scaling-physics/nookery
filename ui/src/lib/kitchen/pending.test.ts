import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PendingRemovals } from './pending.svelte.js';
import type { LotView } from '../bindings.js';

const lot = (id: string, item = id) => ({ id, short: id, item }) as LotView;

describe('PendingRemovals', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	function make() {
		const write = vi.fn(async () => {});
		const onSettled = vi.fn();
		return { write, onSettled, p: new PendingRemovals({ write, onSettled, ms: 6000 }) };
	}

	it('hides at once and writes after the timeout', async () => {
		const { p, write, onSettled } = make();
		p.add([lot('a', 'pesto')], 'used');
		expect(p.hiddenIds.has('a')).toBe(true);
		expect(p.message).toBe('Pesto used up');
		await vi.advanceTimersByTimeAsync(5900);
		expect(write).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(200);
		expect(write).toHaveBeenCalledExactlyOnceWith('a', { type: 'use', qty: null });
		expect(onSettled).toHaveBeenCalledOnce();
		expect(p.hiddenIds.size).toBe(0);
	});

	it('undo cancels: nothing is written', async () => {
		const { p, write } = make();
		p.add([lot('a')], 'binned');
		p.undo();
		expect(p.hiddenIds.size).toBe(0);
		await vi.advanceTimersByTimeAsync(10_000);
		expect(write).not.toHaveBeenCalled();
	});

	it('flush writes everything once, and again does nothing', async () => {
		const { p, write } = make();
		p.add([lot('a')], 'used');
		p.add([lot('b')], 'wrong');
		await p.flush();
		await p.flush();
		await vi.advanceTimersByTimeAsync(10_000);
		expect(write).toHaveBeenCalledTimes(2);
		expect(write).toHaveBeenCalledWith('b', { type: 'wrong' });
	});

	it('merges the toast and ignores a lot already pending', () => {
		const { p } = make();
		p.add([lot('a', 'x')], 'used');
		p.add([lot('b'), lot('c')], 'binned');
		p.add([lot('a')], 'used');
		expect(p.queued).toHaveLength(3);
		expect(p.message).toBe('3 removed');
	});

	it('restarts the timer on each removal', async () => {
		const { p, write } = make();
		p.add([lot('a')], 'used');
		await vi.advanceTimersByTimeAsync(4000);
		p.add([lot('b')], 'used');
		await vi.advanceTimersByTimeAsync(4000);
		expect(write).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(2100);
		expect(write).toHaveBeenCalledTimes(2);
	});

	it('reports failed writes and keeps going', async () => {
		const write = vi.fn(async (id: string) => {
			if (id === 'a') throw new Error('x');
		});
		const onError = vi.fn();
		const p = new PendingRemovals({ write, onError });
		p.add([lot('a'), lot('b')], 'used');
		await p.flush();
		expect(write).toHaveBeenCalledTimes(2);
		expect(onError).toHaveBeenCalledWith(1);
	});
});
