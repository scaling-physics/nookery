// Where the UI is running. `isAndroid` = the Tauri Android app (back button
// hook, no local AI). The phone *layout* is decided by CSS alone (narrow
// width), so it can be previewed in a browser.
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const isAndroid =
	typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent) && isTauri;

// Android WebViews often report no bottom inset under the gesture bar: CSS
// uses this class to keep a minimum.
if (isAndroid) document.documentElement.classList.add('android');
