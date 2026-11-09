/** Duration for the row collapse; 0 when the user prefers reduced motion. */
export function motionMs(ms = 150): number {
	if (typeof window === 'undefined' || !window.matchMedia) return ms;
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : ms;
}
