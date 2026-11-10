/** Longest side the phone sends to the Mac for a photo scan. */
export const MAX_SIDE = 2000;

/** The size to scale `w` x `h` to so the long side is at most `max` (never up). */
export function fitWithin(w: number, h: number, max = MAX_SIDE): { w: number; h: number } {
	const long = Math.max(w, h);
	if (long <= max) return { w, h };
	const k = max / long;
	return { w: Math.max(1, Math.round(w * k)), h: Math.max(1, Math.round(h * k)) };
}

/** Reads an image file and returns it as a JPEG data URL, at most 2000 px on the
 *  long side (EXIF rotation is applied by the browser). Falls back to the file as is. */
export async function resizeForMac(file: Blob): Promise<string> {
	const raw = () =>
		new Promise<string>((resolve) => {
			const r = new FileReader();
			r.onload = () => resolve(String(r.result));
			r.readAsDataURL(file);
		});
	try {
		const bitmap = await createImageBitmap(file, { imageOrientation: 'from-image' });
		const { w, h } = fitWithin(bitmap.width, bitmap.height);
		const canvas = document.createElement('canvas');
		canvas.width = w;
		canvas.height = h;
		const ctx = canvas.getContext('2d');
		if (!ctx) return await raw();
		ctx.drawImage(bitmap, 0, 0, w, h);
		bitmap.close();
		return canvas.toDataURL('image/jpeg', 0.88);
	} catch {
		return await raw();
	}
}
