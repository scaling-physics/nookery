// The camera, for the Mac's pairing QR code. Phone only (Tauri's barcode-scanner
// plugin is registered under cfg(mobile)); everywhere else this says "no".
import { isAndroid } from '#lib/platform.js';
import { parsePairingLink, type PairingLink } from './pairing.js';

export type ScanOutcome =
	| { kind: 'ok'; link: PairingLink }
	| { kind: 'denied' }
	| { kind: 'cancelled' }
	| { kind: 'wrong' }
	| { kind: 'unavailable' };

export const canScanQr = isAndroid;

/** Opens the camera and waits for a Nookery pairing QR code. */
export async function scanPairingQr(): Promise<ScanOutcome> {
	if (!canScanQr) return { kind: 'unavailable' };
	try {
		const { scan, Format, checkPermissions, requestPermissions } =
			await import('@tauri-apps/plugin-barcode-scanner');
		let perm = await checkPermissions();
		if (perm !== 'granted') perm = await requestPermissions();
		if (perm !== 'granted') return { kind: 'denied' };
		document.documentElement.classList.add('scanning');
		try {
			const r = await scan({ windowed: true, formats: [Format.QRCode] });
			const link = parsePairingLink(r.content);
			return link ? { kind: 'ok', link } : { kind: 'wrong' };
		} finally {
			document.documentElement.classList.remove('scanning');
		}
	} catch {
		// Cancelled (cancel() rejects the pending scan) or no camera.
		return { kind: 'cancelled' };
	}
}

export async function cancelScan(): Promise<void> {
	try {
		const { cancel } = await import('@tauri-apps/plugin-barcode-scanner');
		await cancel();
	} catch {
		/* nothing to cancel */
	}
}
