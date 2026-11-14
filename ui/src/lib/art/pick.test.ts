import { beforeEach, describe, expect, it, vi } from 'vitest';

const pickDishArt = vi.fn();
const getAiSettings = vi.fn();
vi.mock('../api.js', () => ({ api: { pickDishArt, getAiSettings } }));
vi.mock('./dishes.js', () => ({
	DISHES: [
		{ id: 'ramen', label: 'Ramen' },
		{ id: 'shakshuka', label: 'Shakshuka' }
	]
}));

const { art, pickFor, resetArtPicks, loadArtMode } = await import('./pick.svelte.js');

const answerAll =
	(kind = 'ramen') =>
	async (dishes: { ingredients: string[] }[]) => ({
		status: 'ok',
		data: dishes.map((d) => ({ kind, show: d.ingredients.slice(0, 1) }))
	});

beforeEach(() => {
	vi.useFakeTimers();
	pickDishArt.mockReset();
	getAiSettings.mockReset();
	resetArtPicks();
	art.mode = 'model';
});

describe('pickFor', () => {
	it('does nothing in matched mode', async () => {
		art.mode = 'matched';
		expect(pickFor('Ramen', ['egg'])).toBeUndefined();
		await vi.advanceTimersByTimeAsync(200);
		expect(pickDishArt).not.toHaveBeenCalled();
	});

	it('batches requests of the same moment, at most 8 per call, one in flight', async () => {
		pickDishArt.mockImplementation(answerAll());
		for (let i = 0; i < 10; i++) pickFor(`Dish ${i}`, ['egg']);
		await vi.advanceTimersByTimeAsync(60);
		expect(pickDishArt).toHaveBeenCalledTimes(1);
		expect(pickDishArt.mock.calls[0][0]).toHaveLength(8);
		expect(pickDishArt.mock.calls[0][1]).toEqual([
			{ id: 'ramen', label: 'Ramen' },
			{ id: 'shakshuka', label: 'Shakshuka' }
		]);
		await vi.advanceTimersByTimeAsync(200);
		expect(pickDishArt).toHaveBeenCalledTimes(2);
		expect(pickDishArt.mock.calls[1][0]).toHaveLength(2);
	});

	it('caches answers and does not ask twice', async () => {
		pickDishArt.mockImplementation(answerAll('shakshuka'));
		expect(pickFor('Eggs', ['egg', 'tomato'])).toBeUndefined();
		await vi.advanceTimersByTimeAsync(100);
		expect(pickFor('Eggs', ['egg', 'tomato'])).toEqual({ kind: 'shakshuka', show: ['egg'] });
		expect(pickFor('Eggs', ['egg', 'tomato'])).toBeDefined();
		await vi.advanceTimersByTimeAsync(100);
		expect(pickDishArt).toHaveBeenCalledTimes(1);
	});

	it('falls back (undefined) and does not retry after an error', async () => {
		pickDishArt.mockResolvedValue({ status: 'error', error: { kind: 'recipe_model_missing' } });
		pickFor('Soup', ['leek']);
		await vi.advanceTimersByTimeAsync(100);
		expect(pickFor('Soup', ['leek'])).toBeUndefined();
		await vi.advanceTimersByTimeAsync(200);
		expect(pickDishArt).toHaveBeenCalledTimes(1);
	});

	it('survives a thrown call and a null answer', async () => {
		pickDishArt.mockRejectedValueOnce(new Error('boom'));
		pickFor('A', ['x']);
		await vi.advanceTimersByTimeAsync(100);
		expect(pickFor('A', ['x'])).toBeUndefined();
		pickDishArt.mockResolvedValueOnce({ status: 'ok', data: [null] });
		pickFor('B', ['x']);
		await vi.advanceTimersByTimeAsync(100);
		expect(pickFor('B', ['x'])).toBeUndefined();
		await vi.advanceTimersByTimeAsync(200);
		expect(pickDishArt).toHaveBeenCalledTimes(2);
	});
});

describe('loadArtMode', () => {
	it('reads the saved mode', async () => {
		getAiSettings.mockResolvedValue({ status: 'ok', data: { recipeArt: 'matched' } });
		await loadArtMode();
		expect(art.mode).toBe('matched');
	});
});
