/** 2_600_000_000 -> "2.6 GB" (decimal units, like the phone's own storage screen). */
export function formatBytes(n: number): string {
	if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
	if (n >= 1e6) return `${Math.round(n / 1e6)} MB`;
	if (n >= 1e3) return `${Math.round(n / 1e3)} kB`;
	return `${Math.round(n)} B`;
}

/** Bytes per second -> "12 MB/s". */
export function formatSpeed(bps: number): string {
	return `${formatBytes(bps)}/s`;
}

/** Seconds left -> "about 3 min left". */
export function formatEta(seconds: number): string {
	if (!isFinite(seconds) || seconds < 0) return '';
	if (seconds < 90) return `about ${Math.max(1, Math.round(seconds))} s left`;
	return `about ${Math.round(seconds / 60)} min left`;
}
