import { isAndroid, isTauri } from '#lib/platform.js';

export type ThemeMode = 'auto' | 'light' | 'dark';

/** Also read by `static/theme.js`, which applies it before the first paint. */
export const THEME_KEY = 'nookery.theme';

export function parseTheme(v: unknown): ThemeMode {
	return v === 'light' || v === 'dark' ? v : 'auto';
}

/** Appearance, per device: kept in browser storage, not synced. `layout.css`
 *  follows the system unless `<html data-theme>` pins light or dark. */
class Theme {
	mode = $state<ThemeMode>('auto');

	init() {
		try {
			this.mode = parseTheme(localStorage.getItem(THEME_KEY));
		} catch {
			// storage unavailable: follow the system
		}
		this.#apply();
	}

	set(mode: ThemeMode) {
		this.mode = mode;
		try {
			if (mode === 'auto') localStorage.removeItem(THEME_KEY);
			else localStorage.setItem(THEME_KEY, mode);
		} catch {
			// not remembered, still applied for now
		}
		this.#apply();
	}

	#apply() {
		const root = document.documentElement;
		if (this.mode === 'auto') delete root.dataset.theme;
		else root.dataset.theme = this.mode;
		// The Mac's title bar follows the window's appearance, not the page.
		if (isTauri && !isAndroid) {
			const t = this.mode === 'auto' ? null : this.mode;
			import('@tauri-apps/api/window')
				.then(({ getCurrentWindow }) => getCurrentWindow().setTheme(t))
				.catch(() => {});
		}
	}
}

export const theme = new Theme();
