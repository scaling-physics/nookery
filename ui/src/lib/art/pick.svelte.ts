// Settings "Recipe pictures": matched by words (default) or picked by the
// recipe model. In 'model' mode `pickFor` asks the backend (batched, cached,
// local only) and returns undefined until an answer is there, so the caller
// shows the word match meanwhile and whenever the model fails.
import { api } from '../api.js';
import { DISHES } from './dishes.js';
import type { ArtPick } from './types.js';

export type ArtMode = 'matched' | 'model';

export const art = $state<{ mode: ArtMode }>({ mode: 'matched' });

const STORAGE_KEY = 'nookery.art.picks';
const MAX_STORED = 300;
const MAX_BATCH = 8;
const BATCH_MS = 50;

type Dish = { title: string; ingredients: string[] };

/** Answers by key; `null` = the model had no usable answer. Reactive. */
const answers = $state<Record<string, ArtPick | null>>(readStored());
/** Keys that failed this session: not asked again (until reload). */
const failed = new Set<string>();
const queued = new Map<string, Dish>();
const asked = new Set<string>();
let timer: ReturnType<typeof setTimeout> | undefined;
let busy = false;

function hash(s: string): string {
	let h = 0x811c9dc5;
	for (let i = 0; i < s.length; i++) {
		h ^= s.charCodeAt(i);
		h = Math.imul(h, 0x01000193) >>> 0;
	}
	return h.toString(36);
}

function keyOf(title: string, ingredients: readonly string[]): string {
	return hash(`${title}\n${ingredients.join('\n')}`);
}

function readStored(): Record<string, ArtPick | null> {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		const v = raw ? JSON.parse(raw) : {};
		return v && typeof v === 'object' && !Array.isArray(v) ? v : {};
	} catch {
		return {};
	}
}

function store() {
	try {
		const keys = Object.keys(answers);
		const keep = keys.slice(Math.max(0, keys.length - MAX_STORED));
		const out: Record<string, ArtPick | null> = {};
		for (const k of keep) out[k] = answers[k];
		localStorage.setItem(STORAGE_KEY, JSON.stringify(out));
	} catch {
		// no storage: the in-memory cache still works
	}
}

async function flush() {
	timer = undefined;
	if (busy || queued.size === 0) return;
	busy = true;
	const batch = [...queued.entries()].slice(0, MAX_BATCH);
	for (const [k] of batch) queued.delete(k);
	try {
		const options = DISHES.map((d) => ({ id: d.id, label: d.label }));
		const r = await api.pickDishArt(
			batch.map(([, d]) => d),
			options
		);
		if (r.status === 'ok') {
			batch.forEach(([k], i) => {
				answers[k] = r.data[i] ?? null;
			});
			store();
		} else {
			for (const [k] of batch) failed.add(k);
		}
	} catch {
		for (const [k] of batch) failed.add(k);
	} finally {
		busy = false;
		if (queued.size > 0) schedule();
	}
}

function schedule() {
	if (timer === undefined) timer = setTimeout(flush, BATCH_MS);
}

/** Reads the saved choice once at start; call from the root layout. */
export async function loadArtMode() {
	try {
		const r = await api.getAiSettings();
		if (r.status === 'ok') art.mode = r.data.recipeArt;
	} catch {
		// stays 'matched'
	}
}

/**
 * The model's pick for a recipe, when the mode is 'model' and it has
 * answered (cached); undefined otherwise (the caller then uses matchDish).
 * Reactive: reading it inside $derived re-runs when the answer arrives.
 */
export function pickFor(title: string, ingredients: readonly string[]): ArtPick | undefined {
	if (art.mode !== 'model' || !title.trim()) return undefined;
	const key = keyOf(title, ingredients);
	const a = answers[key];
	if (a) return a;
	if (a === null || failed.has(key) || asked.has(key)) return undefined;
	asked.add(key);
	queued.set(key, { title, ingredients: [...ingredients] });
	schedule();
	return undefined;
}

/** For tests: forget everything held in memory. */
export function resetArtPicks() {
	for (const k of Object.keys(answers)) delete answers[k];
	failed.clear();
	queued.clear();
	asked.clear();
	clearTimeout(timer);
	timer = undefined;
	busy = false;
}
