// The Mac's pairing QR code holds `nookery://pair?code=<CODE>&addr=<IP:PORT>&name=<device>`
// (built by `pairing_link` in src-tauri/src/sync.rs). This parses and checks it.

export type PairingLink = { code: string; addr: string; name: string };

const CODE = /^[A-Z2-7]{4}(-[A-Z2-7]{4}){5}-[A-Z2-7]{2}$/;
const ADDR = /^[A-Za-z0-9.-]+(:\d{1,5})?$/;

/** Reads a pairing link, or null if `text` isn't a valid one. Typed text
 *  (a code or a different link) is never mistaken for it. */
export function parsePairingLink(text: string): PairingLink | null {
	let url: URL;
	try {
		url = new URL(text.trim());
	} catch {
		return null;
	}
	// `new URL('nookery://pair?...')`: host is "pair" for special-less schemes in
	// most engines, but some put it in the pathname; accept both.
	if (url.protocol !== 'nookery:') return null;
	const target = url.hostname || url.pathname.replace(/^\/+/, '');
	if (target !== 'pair') return null;
	const code = (url.searchParams.get('code') ?? '').trim().toUpperCase();
	const addr = (url.searchParams.get('addr') ?? '').trim();
	const name = (url.searchParams.get('name') ?? '').trim().slice(0, 60);
	if (!CODE.test(code)) return null;
	if (!ADDR.test(addr)) return null;
	const port = addr.split(':')[1];
	if (port !== undefined && (Number(port) < 1 || Number(port) > 65535)) return null;
	return { code, addr, name };
}
