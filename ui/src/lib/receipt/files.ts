import type { CommandError } from '#lib/bindings.js';

/** Base64 of the bytes, in chunks (spreading 10 MB into one call overflows the stack). */
export function bytesToBase64(bytes: Uint8Array): string {
	const chunk = 0x8000;
	let bin = '';
	for (let i = 0; i < bytes.length; i += chunk) {
		bin += String.fromCharCode(...bytes.subarray(i, i + chunk));
	}
	return btoa(bin);
}

export async function fileToBase64(file: File): Promise<string> {
	return bytesToBase64(new Uint8Array(await file.arrayBuffer()));
}

/** A friendly sentence for a receipt file that couldn't be read. */
export function readError(e: CommandError): string {
	switch (e.kind) {
		case 'receipt_unreadable':
			switch (e.detail) {
				case 'unknown_format':
					return 'That file isn’t a receipt Nookery can read. Use a photo (PNG, JPEG, HEIC), a PDF or a text file.';
				case 'too_large':
					return 'That file is too large (over 20 MB). Try a smaller photo.';
				case 'no_text':
					return 'No text found. Try a sharper, well-lit photo with the whole receipt in view.';
				case 'ocr_unavailable':
					return 'Reading photos needs your Mac. Pair this phone with it in Settings, or paste the text.';
				case 'pdf':
					return 'Couldn’t open that PDF. Is it damaged or password protected?';
				case 'image':
					return 'Couldn’t open that image. Try another photo.';
			}
			return 'Couldn’t read that file.';
		case 'mac_unreachable':
			return e.detail;
		case 'bad_input':
			return 'Couldn’t read that file.';
		default:
			return 'Something went wrong reading the file. Try again.';
	}
}

/** The note above the review for text that came from OCR or a PDF. */
export function readNote(method: 'text' | 'pdf_text' | 'ocr', readBy: string): string | null {
	if (method === 'ocr') return `Read from a photo on ${readBy}: check the amounts.`;
	if (method === 'pdf_text') return 'Read from the PDF.';
	return null;
}
