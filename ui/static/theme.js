// Applies the saved Light / Dark choice before the first paint, so a pinned
// theme never flashes the system one. Same key as src/lib/theme.svelte.ts.
// A file, not an inline script: the app's CSP only allows scripts from 'self'.
try {
	var t = localStorage.getItem('nookery.theme');
	if (t === 'light' || t === 'dark') document.documentElement.dataset.theme = t;
} catch (e) {
	// storage unavailable: follow the system
}
