// Android's back button closes the topmost open menu or dialog. The Tauri
// back-button hook is held only while something is open, so otherwise Back
// behaves as Android's default (history, then leave the app). Browser
// history is never touched.
import { onBackButtonPress } from '@tauri-apps/api/app';
import type { PluginListener } from '@tauri-apps/api/core';
import { isAndroid } from '#lib/platform.js';

const stack: Array<() => void> = [];
let hook: PluginListener | null = null;
let hooking = false;

function sync() {
	if (!isAndroid) return;
	if (stack.length && !hook && !hooking) {
		hooking = true;
		onBackButtonPress(() => stack[stack.length - 1]?.())
			.then((l) => {
				hooking = false;
				if (stack.length) hook = l;
				else void l.unregister();
			})
			.catch(() => {
				hooking = false;
			});
	} else if (!stack.length && hook) {
		void hook.unregister();
		hook = null;
	}
}

/** Call while something closable is open; returns the function to call when it closes. */
export function pushBack(close: () => void): () => void {
	stack.push(close);
	sync();
	return () => {
		const i = stack.lastIndexOf(close);
		if (i >= 0) stack.splice(i, 1);
		sync();
	};
}

/** Any native `<dialog>` that opens (modal) is closed by Back. */
export function watchDialogs(): () => void {
	if (!isAndroid) return () => {};
	const open = new Map<HTMLDialogElement, () => void>();
	const scan = () => {
		for (const d of document.querySelectorAll('dialog')) {
			if (d.open && !open.has(d))
				open.set(
					d,
					pushBack(() => d.close())
				);
			else if (!d.open && open.has(d)) {
				open.get(d)?.();
				open.delete(d);
			}
		}
		for (const [d, off] of open)
			if (!d.isConnected) {
				off();
				open.delete(d);
			}
	};
	const mo = new MutationObserver(scan);
	mo.observe(document.body, {
		subtree: true,
		childList: true,
		attributes: true,
		attributeFilter: ['open']
	});
	return () => {
		mo.disconnect();
		for (const off of open.values()) off();
	};
}
