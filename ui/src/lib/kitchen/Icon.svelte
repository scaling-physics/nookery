<script lang="ts">
	type Name = 'check' | 'dots' | 'snow' | 'search' | 'plus' | 'x' | 'camera' | 'receipt';
	let { name, size = 16 }: { name: Name; size?: number } = $props();
	const PATHS: Record<Name, string> = {
		check: 'M5 12.5l4.5 4.5L19 7.5',
		dots: 'M5 12h.01M12 12h.01M19 12h.01',
		snow: 'M12 3v18M4.2 7.5l15.6 9M19.8 7.5l-15.6 9',
		search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-3.5-3.5',
		plus: 'M12 5v14M5 12h14',
		x: 'M6 6l12 12M18 6L6 18',
		camera: 'M4 8h3l1.5-2h7L17 8h3v11H4zM12 11a3.2 3.2 0 1 0 0 6.4 3.2 3.2 0 0 0 0-6.4z',
		receipt: 'M6 3h12v18l-3-2-3 2-3-2-3 2zM9 8h6M9 12h6'
	};
</script>

<svg
	viewBox="0 0 24 24"
	width={size}
	height={size}
	fill="none"
	stroke="currentColor"
	stroke-width={name === 'dots' ? 3 : 1.8}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<path d={PATHS[name]} />
</svg>
