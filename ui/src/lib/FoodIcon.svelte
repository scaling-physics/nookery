<script lang="ts">
	// Category glyphs on a 24 grid: soft shapes on a tinted rounded tile, in
	// the same language as the logo. Colours come from the --c-* tokens in
	// layout.css, so the set follows light and dark.
	import type { FoodCategoryView } from './bindings.js';

	let { category, size = 20 }: { category: FoodCategoryView; size?: number } = $props();

	const TONE: Record<FoodCategoryView, string> = {
		vegetables: 'leaf',
		fruit: 'lemon',
		herbs: 'leaf',
		dairy: 'wheat',
		cheese: 'lemon',
		eggs: 'lemon',
		meat: 'rust',
		fish: 'blue',
		bread: 'wheat',
		grains: 'wheat',
		legumes: 'brown',
		spices: 'pumpkin',
		sauces: 'terra',
		sweets: 'plum',
		drinks: 'blue',
		nuts: 'brown',
		leftovers: 'terra',
		other: 'muted'
	};
	const tone = $derived(TONE[category] ?? 'muted');
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	aria-hidden="true"
	focusable="false"
	data-category={category}
	style="--k: var(--c-{tone}); --kd: color-mix(in srgb, var(--k) 68%, #1a1008); --kt: color-mix(in srgb, var(--k) 20%, var(--surface));"
>
	<rect x=".5" y=".5" width="23" height="23" rx="7.5" fill="var(--kt)" />
	<g transform="translate(12 12) scale(1.12) translate(-12 -12)">
		{#if category === 'vegetables'}
			<path
				d="M17 8.2c.4 1.4 0 2.6-1 3.6L8.6 18.6c-1.2 1-2.6.4-2.4-1 .1-.9.6-1.6 1.2-2.4l5.1-6.3c1.3-1.6 3.6-2.1 4.5-.7Z"
				fill="var(--c-pumpkin)"
			/>
			<path
				d="M10 14.2l2-1.4M8.8 16.2l1.8-1.2"
				stroke="var(--kd)"
				stroke-width="1.3"
				stroke-linecap="round"
				opacity=".45"
			/>
			<path d="M14.8 8.4c-.5-2.6.6-4.2 3-4.4.4 2.4-.6 4-3 4.4Z" fill="var(--c-leaf)" />
			<path
				d="M15.6 9.2c1.6-1.8 3.4-2.2 5-1.2-.8 2-2.8 2.6-5 1.2Z"
				fill="var(--c-leaf)"
				opacity=".8"
			/>
		{:else if category === 'fruit'}
			<path
				d="M5 13.4c0-3.4 3.2-5.6 7-5.6 3.2 0 5.6 1.4 6.4 3.6l1.2.6-1.2.9c-.8 3-3.4 4.6-6.4 4.6-4 0-7-1.8-7-4.1Z"
				fill="var(--k)"
				transform="rotate(-18 12 13)"
			/>
			<path
				d="M8.5 10.8c1-.8 2.2-1.2 3.4-1.2"
				stroke="#fff"
				stroke-opacity=".6"
				stroke-width="1.5"
				stroke-linecap="round"
				fill="none"
			/>
			<path d="M13.6 7.6c.2-2 1.4-3 3.4-3.2 0 2-1.2 3.2-3.4 3.2Z" fill="var(--c-leaf)" />
		{:else if category === 'herbs'}
			<path
				d="M12 19.5v-8M12 15l-3.2-2.4M12 15l3.2-2.4"
				stroke="var(--kd)"
				stroke-width="1.4"
				stroke-linecap="round"
				fill="none"
			/>
			<path
				d="M12 11.6c-2.8-.4-3.2-3.2-1.8-5 .6-.8 1.4-1.2 1.8-1.8.4.6 1.2 1 1.8 1.8 1.4 1.8 1 4.6-1.8 5Z"
				fill="var(--k)"
			/>
			<ellipse cx="8" cy="11.4" rx="2.3" ry="3" transform="rotate(-50 8 11.4)" fill="var(--k)" />
			<ellipse cx="16" cy="11.4" rx="2.3" ry="3" transform="rotate(50 16 11.4)" fill="var(--k)" />
			<path
				d="M12 11V7.2"
				stroke="#fff"
				stroke-opacity=".4"
				stroke-width="1.1"
				stroke-linecap="round"
			/>
		{:else if category === 'dairy'}
			<rect x="9.2" y="4.6" width="5.6" height="2.6" rx="1.2" fill="var(--c-blue)" />
			<path
				d="M9.2 7.2h5.6l2.2 3.2V18a1.6 1.6 0 0 1-1.6 1.6H8.6A1.6 1.6 0 0 1 7 18v-7.6Z"
				fill="var(--c-milk)"
			/>
			<rect x="7" y="12.4" width="10" height="4" fill="var(--c-blue)" opacity=".85" />
		{:else if category === 'cheese'}
			<path
				d="M4.4 14.2 19.6 8.4v8.6a1.4 1.4 0 0 1-1.4 1.4H5.8a1.4 1.4 0 0 1-1.4-1.4Z"
				fill="var(--k)"
			/>
			<path d="M4.4 14.2 19.6 8.4 13 6.2Z" fill="#fff" fill-opacity=".38" />
			<circle cx="9" cy="15.8" r="1.5" fill="var(--kt)" />
			<circle cx="14.4" cy="14.2" r="1.2" fill="var(--kt)" />
			<circle cx="16.6" cy="16.8" r=".9" fill="var(--kt)" />
		{:else if category === 'eggs'}
			<path
				d="M12 4.4c3.5 0 5.6 5.2 5.6 8.8a5.6 5.6 0 0 1-11.2 0c0-3.6 2.1-8.8 5.6-8.8Z"
				fill="var(--c-milk)"
				stroke="var(--kd)"
				stroke-opacity=".55"
				stroke-width="1.1"
			/>
			<path
				d="M8.6 14.2a3.6 3.6 0 0 0 2.6 3.2"
				stroke="var(--k)"
				stroke-width="1.6"
				stroke-linecap="round"
				fill="none"
			/>
			<path
				d="M10.2 7.6c-.8 1.2-1.4 2.6-1.6 3.8"
				stroke="var(--k)"
				stroke-width="1.4"
				stroke-linecap="round"
				fill="none"
				opacity=".7"
			/>
		{:else if category === 'meat'}
			<path
				d="M10.6 14.4 6.6 18.4"
				stroke="var(--c-milk)"
				stroke-width="2.4"
				stroke-linecap="round"
			/>
			<circle cx="5.6" cy="17.4" r="1.7" fill="var(--c-milk)" />
			<circle cx="7.4" cy="19.2" r="1.7" fill="var(--c-milk)" />
			<circle cx="14" cy="9.8" r="5.6" fill="var(--k)" />
			<path
				d="M11.4 8.2c.8-1.2 2-1.8 3.2-1.8"
				stroke="#fff"
				stroke-opacity=".5"
				stroke-width="1.5"
				stroke-linecap="round"
				fill="none"
			/>
		{:else if category === 'fish'}
			<path d="M16 12 20.4 8v8Z" fill="var(--kd)" />
			<path
				d="M3.8 12c2-3.2 5-4.4 7.4-4.4 2.4 0 4.2 1.4 5.2 4.4-1 3-2.8 4.4-5.2 4.4-2.4 0-5.4-1.2-7.4-4.4Z"
				fill="var(--k)"
			/>
			<path d="M9.6 15.8 12 18.6l1.2-2.2Z" fill="var(--kd)" />
			<circle cx="7.6" cy="11" r="1.1" fill="var(--c-milk)" />
			<path
				d="M11.6 9.4c.8 1.6.8 3.6 0 5.2"
				stroke="#fff"
				stroke-opacity=".45"
				stroke-width="1.3"
				stroke-linecap="round"
				fill="none"
			/>
		{:else if category === 'bread'}
			<path
				d="M4.4 16.2V13c0-3.4 3.2-5.4 7.6-5.4s7.6 2 7.6 5.4v3.2c0 1.2-.9 2-2 2H6.4c-1.1 0-2-.8-2-2Z"
				fill="var(--k)"
			/>
			<path
				d="M8 11.6 9.4 14M11.6 10.8 13 13.6M15.2 11.6 16.6 14"
				stroke="var(--c-milk)"
				stroke-width="1.5"
				stroke-linecap="round"
				opacity=".7"
			/>
		{:else if category === 'grains'}
			<path d="M12 19.6V10" stroke="var(--kd)" stroke-width="1.4" stroke-linecap="round" />
			<g fill="var(--k)">
				<ellipse cx="12" cy="5.6" rx="1.5" ry="2.1" />
				<ellipse cx="9.4" cy="8.4" rx="1.5" ry="2.3" transform="rotate(-32 9.4 8.4)" />
				<ellipse cx="14.6" cy="8.4" rx="1.5" ry="2.3" transform="rotate(32 14.6 8.4)" />
				<ellipse cx="9.4" cy="12.2" rx="1.5" ry="2.3" transform="rotate(-32 9.4 12.2)" />
				<ellipse cx="14.6" cy="12.2" rx="1.5" ry="2.3" transform="rotate(32 14.6 12.2)" />
				<ellipse cx="9.6" cy="16" rx="1.4" ry="2.1" transform="rotate(-32 9.6 16)" opacity=".8" />
				<ellipse cx="14.4" cy="16" rx="1.4" ry="2.1" transform="rotate(32 14.4 16)" opacity=".8" />
			</g>
		{:else if category === 'legumes'}
			<path
				d="M4.6 15.4c-.6-3.4 2.4-7.6 8-8.6 3.2-.6 6 .4 6.8 1.6-.2 4.8-3.6 9-8.4 9.4-2.8.2-5.8-.6-6.4-2.4Z"
				fill="var(--c-leaf)"
				transform="rotate(-6 12 12)"
			/>
			<circle cx="9.4" cy="12.6" r="2" fill="var(--c-pea)" />
			<circle cx="13.4" cy="11" r="2" fill="var(--c-pea)" />
			<circle cx="16.2" cy="9.4" r="1.4" fill="var(--c-pea)" />
		{:else if category === 'spices'}
			<rect x="8" y="4.6" width="8" height="3.2" rx="1.2" fill="var(--kd)" />
			<rect x="6.8" y="7.8" width="10.4" height="11.6" rx="2.6" fill="var(--k)" />
			<rect x="8.4" y="11.6" width="7.2" height="4.4" rx="1.2" fill="var(--c-milk)" opacity=".92" />
			<circle cx="10.4" cy="13.8" r=".9" fill="var(--kd)" />
			<circle cx="12.4" cy="13.8" r=".9" fill="var(--kd)" />
			<circle cx="14.2" cy="13.8" r=".9" fill="var(--kd)" />
		{:else if category === 'sauces'}
			<rect x="10.2" y="4.4" width="3.6" height="2.8" rx="1" fill="var(--kd)" />
			<path
				d="M10.4 7.2h3.2l1 2.2c1.6.6 2.4 1.8 2.4 3.4v4.4a2.2 2.2 0 0 1-2.2 2.2H9.2A2.2 2.2 0 0 1 7 17.2v-4.4c0-1.6.8-2.8 2.4-3.4Z"
				fill="var(--k)"
			/>
			<rect x="8.6" y="12.2" width="6.8" height="4" rx="1.2" fill="var(--c-milk)" opacity=".92" />
			<circle cx="12" cy="14.2" r="1.1" fill="var(--c-lemon)" />
		{:else if category === 'sweets'}
			<path d="M8 12 3.6 8.6v6.8Z" fill="var(--kd)" />
			<path d="M16 12l4.4-3.4v6.8Z" fill="var(--kd)" />
			<circle cx="12" cy="12" r="5" fill="var(--k)" />
			<path
				d="M9 9.6c1.4-1 3.2-1.2 4.8-.4"
				stroke="#fff"
				stroke-opacity=".55"
				stroke-width="1.5"
				stroke-linecap="round"
				fill="none"
			/>
			<path
				d="M11 15.8c1.8.4 3.4-.2 4.2-1.6"
				stroke="var(--c-milk)"
				stroke-width="1.4"
				stroke-linecap="round"
				fill="none"
				opacity=".7"
			/>
		{:else if category === 'drinks'}
			<path d="M13.4 7 15 3.6" stroke="var(--c-lemon)" stroke-width="1.7" stroke-linecap="round" />
			<rect x="5.8" y="6.4" width="12.4" height="2.4" rx="1.2" fill="var(--kd)" />
			<path
				d="M6.8 8.8h10.4l-1 9.4a1.6 1.6 0 0 1-1.6 1.4H9.4a1.6 1.6 0 0 1-1.6-1.4Z"
				fill="var(--k)"
			/>
			<path d="M7.2 12.4h9.6" stroke="var(--c-milk)" stroke-width="1.5" opacity=".6" />
		{:else if category === 'nuts'}
			<path d="M12 4.2v2.4" stroke="var(--kd)" stroke-width="1.6" stroke-linecap="round" />
			<path d="M7.8 11.6c-1.2-3.6 1-5.4 4.2-5.4s5.4 1.8 4.2 5.4Z" fill="var(--kd)" />
			<path d="M8.4 12.6h7.2c-.2 3.4-1.6 5.4-3.6 7-2-1.6-3.4-3.6-3.6-7Z" fill="var(--k)" />
			<path
				d="M10 14c.2 1.2.7 2.2 1.4 3"
				stroke="#fff"
				stroke-opacity=".4"
				stroke-width="1.4"
				stroke-linecap="round"
				fill="none"
			/>
		{:else if category === 'leftovers'}
			<path
				d="M9.4 3.6c-1 1-1 2 0 3s1 1.8 0 2.6M14.6 3.6c-1 1-1 2 0 3s1 1.8 0 2.6"
				stroke="var(--kd)"
				stroke-width="1.4"
				stroke-linecap="round"
				fill="none"
				opacity=".7"
			/>
			<rect x="4.4" y="10.4" width="15.2" height="3" rx="1.5" fill="var(--kd)" />
			<path d="M5.6 13.4h12.8l-.8 4.2a2 2 0 0 1-2 1.6H8.4a2 2 0 0 1-2-1.6Z" fill="var(--k)" />
		{:else}
			<path
				d="M8.6 11.4C8.6 6.2 15.4 6.2 15.4 11.4"
				stroke="var(--kd)"
				stroke-width="1.7"
				stroke-linecap="round"
				fill="none"
			/>
			<path
				d="M4.8 11.4h14.4L17.6 18a1.6 1.6 0 0 1-1.6 1.2H8a1.6 1.6 0 0 1-1.6-1.2Z"
				fill="var(--k)"
			/>
			<path
				d="M6 14.4h12M10 11.6l.6 7.4M14 11.6l-.6 7.4"
				stroke="var(--kt)"
				stroke-width="1.2"
				opacity=".8"
			/>
		{/if}
	</g>
</svg>
