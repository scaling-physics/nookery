import { describe, expect, it } from 'vitest';
import { bytesToBase64, readError, readNote } from './files.js';

describe('bytesToBase64', () => {
	it('matches btoa for small input', () => {
		expect(bytesToBase64(new TextEncoder().encode('Nookery'))).toBe(btoa('Nookery'));
	});
	it('handles 10 MB without overflowing the stack', () => {
		const big = new Uint8Array(10 * 1024 * 1024).fill(65);
		const b64 = bytesToBase64(big);
		expect(b64.length).toBe(Math.ceil(big.length / 3) * 4);
		expect(atob(b64.slice(0, 8))).toBe('AAAAAA');
	});
});

describe('messages', () => {
	it('says what to do for each unreadable case', () => {
		for (const detail of [
			'unknown_format',
			'too_large',
			'no_text',
			'ocr_unavailable',
			'pdf',
			'image'
		])
			expect(readError({ kind: 'receipt_unreadable', detail })).not.toMatch(/went wrong/);
		expect(readError({ kind: 'internal' })).toMatch(/went wrong/);
	});
	it('notes only OCR and PDF reads', () => {
		expect(readNote('ocr', 'this Mac')).toMatch(/photo on this Mac/);
		expect(readNote('pdf_text', 'this Mac')).toBe('Read from the PDF.');
		expect(readNote('text', 'this Mac')).toBeNull();
	});
});
