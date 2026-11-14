<script lang="ts">
	// One ingredient glyph, centred at 0,0 (~ +-5 units), placed by the
	// caller. Simple filled shapes with a thin darker outline of their own
	// tint, one highlight or vein; readable at 6-10 px. Colours are the
	// `--c-*` tokens, so it follows light and dark and needs nothing from
	// the parent's styles.
	import type { Piece } from '../types.js';

	let {
		kind,
		x,
		y,
		rot = 0,
		s = 1,
		v = 0
	}: {
		kind: Piece;
		x: number;
		y: number;
		rot?: number;
		/** Scale, ~0.7-1.3. */
		s?: number;
		/** Variant 0-2 (colour or shape), seeded by the caller. */
		v?: number;
	} = $props();

	const FILL: Record<Piece, string> = {
		leaf: 'leaf',
		tomato: 'rust',
		wedge: 'pumpkin',
		herb: 'leaf',
		lemon: 'lemon',
		cube: 'lemon',
		dollop: 'milk',
		egg: 'milk',
		slice: 'rust',
		fillet: 'terra',
		grain: 'wheat',
		bean: 'brown',
		swoosh: 'terra',
		sprinkle: 'pumpkin',
		nut: 'brown',
		berry: 'plum',
		crouton: 'wheat',
		dot: 'muted',
		'fried-egg': 'milk',
		chickpea: 'wheat',
		lentil: 'brown',
		mushroom: 'brown',
		pepper: 'rust',
		chili: 'rust',
		onion: 'cream',
		'spring-onion': 'leaf',
		garlic: 'cream',
		carrot: 'pumpkin',
		broccoli: 'leaf',
		spinach: 'leaf',
		basil: 'pea',
		avocado: 'olive',
		cucumber: 'leaf',
		corn: 'lemon',
		pea: 'pea',
		olive: 'dark',
		potato: 'wheat',
		prawn: 'pink',
		salmon: 'pink',
		chicken: 'wheat',
		bacon: 'rust',
		sausage: 'brown',
		mince: 'brown',
		cheese: 'lemon',
		mozzarella: 'milk',
		sesame: 'cream',
		seed: 'dark',
		banana: 'lemon',
		apple: 'rust',
		citrus: 'pumpkin',
		zucchini: 'olive',
		aubergine: 'plum',
		cabbage: 'pea',
		pumpkin: 'pumpkin',
		beet: 'plum'
	};
	const fill = $derived.by(() => {
		if (kind === 'leaf' && v === 1) return 'pea';
		if (kind === 'bean' && v === 1) return 'rust';
		if (kind === 'pepper') return ['rust', 'lemon', 'leaf'][v % 3];
		if (kind === 'olive') return v === 0 ? 'dark' : 'olive';
		if (kind === 'onion' && v === 1) return 'plum';
		if (kind === 'cabbage' && v === 1) return 'plum';
		if (kind === 'lentil' && v === 1) return 'rust';
		if (kind === 'sesame' && v === 1) return 'wheat';
		return FILL[kind];
	});
</script>

<g
	class="t"
	transform="translate({x} {y}) rotate({rot}) scale({s})"
	style="--f: var(--c-{fill}); --fd: color-mix(in srgb, var(--f) 60%, #1a1008)"
	data-piece={kind}
	data-v={v}
>
	{#if kind === 'leaf'}
		<path d="M-5.5 0Q0-6.5 5.5 0Q0 6.5-5.5 0Z" fill="var(--f)" />
		<path d="M-4 0H4" class="vein" />
	{:else if kind === 'tomato'}
		<circle r="4.2" fill="var(--f)" />
		<circle r="2.6" fill="var(--c-milk)" opacity=".45" stroke="none" />
		<circle cx="-.9" cy="-.6" r=".5" fill="var(--fd)" stroke="none" />
		<circle cx="1" cy=".9" r=".5" fill="var(--fd)" stroke="none" />
	{:else if kind === 'wedge'}
		<path d="M-5 2.6A5.8 5.8 0 0 1 5 2.6Z" fill="var(--f)" />
		<path d="M-3 2.2A3.6 3.6 0 0 1 3 2.2" class="vein" />
	{:else if kind === 'herb'}
		<path d="M0 5V-1M0 2L-3.6-.4M0 2L3.6-.4" class="stem" />
		<g transform="translate(-3.4 -.5) rotate(-52) scale(.15)"
			><path
				d="M0 0C-5-8-13-15-15-25Q-18-33-10-33Q-8-41-3-38Q0-45 3-38Q8-41 10-33Q18-33 15-25C13-15 5-8 0 0Z"
				fill="var(--f)"
			/></g
		>
		<g transform="translate(3.4 -.5) rotate(52) scale(.15)"
			><path
				d="M0 0C-5-8-13-15-15-25Q-18-33-10-33Q-8-41-3-38Q0-45 3-38Q8-41 10-33Q18-33 15-25C13-15 5-8 0 0Z"
				fill="var(--f)"
			/></g
		>
		<g transform="translate(0 -1) scale(.19)"
			><path
				d="M0 0C-5-8-13-15-15-25Q-18-33-10-33Q-8-41-3-38Q0-45 3-38Q8-41 10-33Q18-33 15-25C13-15 5-8 0 0Z"
				fill="var(--f)"
			/></g
		>
	{:else if kind === 'lemon'}
		<circle r="5.2" fill="var(--f)" />
		<circle r="4.2" fill="var(--c-milk)" opacity=".55" stroke="none" />
		<path d="M0-3.8V3.8M-3.3-1.9L3.3 1.9M-3.3 1.9L3.3-1.9" class="vein" />
	{:else if kind === 'cube'}
		<rect x="-3.4" y="-3.4" width="6.8" height="6.8" rx="1.2" fill="var(--f)" />
		<circle cx="-1" cy="-1" r=".6" fill="var(--fd)" stroke="none" />
		<circle cx="1.2" cy="1.3" r=".5" fill="var(--fd)" stroke="none" />
	{:else if kind === 'dollop'}
		<path d="M-5 2Q-6-3 -1-3.4Q0-6 3-4Q7-3.5 5.6 1Q4 3.6-1 3.4Q-4 3.6-5 2Z" fill="var(--f)" />
	{:else if kind === 'egg'}
		<path d="M-6 0Q-7-5-2-5.5Q3-7 6-3Q8 2 3 4.5Q-3 6-6 0Z" fill="var(--f)" />
		<circle cy="-.3" r="2.7" fill="var(--c-lemon)" />
		<circle cx="-.8" cy="-1.2" r=".7" fill="var(--c-milk)" opacity=".7" stroke="none" />
	{:else if kind === 'slice'}
		<ellipse rx="5.8" ry="3.8" fill="var(--f)" />
		<path d="M-3.4-.8Q0-2.2 3.4-.8M-3-.4Q0 1 3 1.8" class="vein" />
	{:else if kind === 'fillet'}
		<path d="M-6 0Q-3-4 2-3.5Q6-3 6 1Q2 4-3 3Q-6 2-6 0Z" fill="var(--f)" />
		<path d="M-3-1Q0-2 3-1M-3 1Q0 0 3 1" class="vein" />
	{:else if kind === 'grain'}
		{#each [[-3, -1.5, 20], [0, 1, -30], [3, -1, 45], [-1.5, 3.4, 5], [2.6, 3, -20]] as g, i (i)}
			<ellipse
				cx={g[0]}
				cy={g[1]}
				rx="1.4"
				ry=".8"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill="var(--f)"
			/>
		{/each}
	{:else if kind === 'bean'}
		<path d="M-3-1.6Q-1-3.4 1.4-2.4Q3.4-1 2 1.4Q.4 3-1.8 2.2Q-4 1-3-1.6Z" fill="var(--f)" />
		<path d="M-1.8-.8Q-.4-1.8 .8-1" class="vein" />
		<path d="M2.6 2.2Q4.2 .8 5.6 2.2Q6.2 4 4.4 4.6Q2.6 4.4 2.6 2.2Z" fill="var(--f)" opacity=".9" />
	{:else if kind === 'swoosh'}
		<path d="M-7 1Q-3.5-3.4 0 0T7-1" fill="none" stroke="var(--f)" stroke-width="2" />
		<path
			d="M-7 1Q-3.5-3.4 0 0T7-1"
			fill="none"
			stroke="var(--c-milk)"
			stroke-width=".5"
			opacity=".4"
		/>
	{:else if kind === 'sprinkle'}
		{#each [[-4, -1], [-1.5, 2.5], [1.5, -2], [4, 1.4], [0, 0.2]] as g, i (i)}
			<circle
				cx={g[0]}
				cy={g[1]}
				r=".9"
				fill={i % 2 ? 'var(--c-rust)' : 'var(--f)'}
				stroke="none"
			/>
		{/each}
	{:else if kind === 'nut'}
		<path
			d="M-4.5-.5Q-4.5-4 -.5-3.4Q0-4.4 1-3.4Q4.5-3.6 4.4 0Q4.4 3.6 0 3.6Q-4.5 3.6-4.5-.5Z"
			fill="var(--f)"
		/>
		<path d="M0-3V3.2M-2.6-1.2Q-1.4 0-2.4 1.8" class="vein" />
	{:else if kind === 'berry'}
		<circle r="2.8" fill="var(--f)" />
		<circle cx="-.9" cy="-.9" r=".7" fill="var(--c-milk)" opacity=".7" stroke="none" />
		<circle cx="3.6" cy="1.8" r="2.2" fill="var(--f)" />
	{:else if kind === 'crouton'}
		<rect x="-3.2" y="-3.2" width="6.4" height="6.4" rx="1.4" fill="var(--f)" />
		<circle cx="-.8" cy=".6" r=".5" fill="var(--fd)" stroke="none" />
	{:else if kind === 'dot'}
		<circle cx="-2.4" cy="-.4" r="1.2" fill="var(--f)" />
		<circle cx="1.2" cy="1.4" r="1.5" fill="var(--f)" />
		<circle cx="2.6" cy="-1.8" r=".9" fill="var(--f)" />
	{:else if kind === 'fried-egg'}
		<path
			d="M-5.6 .6Q-6.6-3.4 -2.6-4.4Q0-6.2 3-4.4Q7-3.6 6 .4Q6.4 4.6 1.6 4.8Q-1 6 -3.4 4.4Q-6 3.6-5.6 .6Z"
			fill="var(--c-milk)"
			stroke="var(--c-muted)"
			stroke-width=".5"
		/>
		<circle cx="-.4" cy=".2" r="2.5" fill="var(--c-lemon)" />
		<circle cx="-1.3" cy="-.7" r=".7" fill="var(--c-milk)" opacity=".7" stroke="none" />
	{:else if kind === 'chickpea'}
		<circle cx="-1.8" cy="1" r="3" fill="var(--f)" />
		<circle cx="3" cy="-1.8" r="2.4" fill="var(--f)" />
		<path d="M-3.8-.6Q-2.6-1.8-1.4-1.4" class="vein" />
		<circle cx="2.2" cy="-2.6" r=".5" fill="var(--c-cream)" stroke="none" />
	{:else if kind === 'lentil'}
		{#each [[-3, -1.6, 10], [0.4, -2.6, -25], [3, 0.4, 40], [-1.4, 2.2, -10], [2.6, 3.2, 20]] as g, i (i)}
			<ellipse
				cx={g[0]}
				cy={g[1]}
				rx="1.9"
				ry="1.3"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill={i === 2 ? 'var(--c-pumpkin)' : 'var(--f)'}
			/>
		{/each}
	{:else if kind === 'mushroom'}
		<path d="M-1.6 .6H1.6L1.2 4.6Q0 5.4-1.2 4.6Z" fill="var(--c-cream)" />
		<path d="M-5.6 1Q-5.4-4.8 0-4.8Q5.4-4.8 5.6 1Q0 2.2-5.6 1Z" fill="var(--f)" />
		<path d="M-3.4-1.6Q-2-3 0-3" class="hi" />
	{:else if kind === 'pepper'}
		<path
			d="M-5.4 2.4Q-5-3.6 .4-4.2Q4.4-4.4 5.6-1.4L3.4-.6Q2.6-2.2 .2-2Q-2.4-1.6-3.2 3Q-4.4 3.8-5.4 2.4Z"
			fill="var(--f)"
		/>
		<path d="M-1.2-3.4Q2-3.6 3.4-1.8" class="hi" />
	{:else if kind === 'chili'}
		<path
			d="M-5.6 3.2Q-6-1.4-1-1.6Q3-1.6 4.4-4.4L5.4-3.4Q4.6 1.4-.4 2.8Q-3.8 4.6-5.6 3.2Z"
			fill="var(--f)"
		/>
		<path d="M4.4-4.4L5.4-3.4L6.2-5.2Q5.4-5.4 4.4-4.4Z" fill="var(--c-leaf)" />
		<path d="M-3.8 1.4Q-1.4.2 1-.4" class="hi" />
	{:else if kind === 'onion'}
		<path
			fill-rule="evenodd"
			d="M-5.2 0A5.2 4.4 0 1 1 5.2 0A5.2 4.4 0 1 1-5.2 0ZM-2.8 0A2.8 2.2 0 1 0 2.8 0A2.8 2.2 0 1 0-2.8 0Z"
			fill="var(--f)"
		/>
		<path d="M-3.8-2.4Q-1.4-3.8 1.4-3.4" class="hi" />
	{:else if kind === 'spring-onion'}
		{#each [[-3.2, 1.6, 2.2], [0.8, -2.4, 2.4], [3.4, 2, 1.9]] as g, i (i)}
			<circle cx={g[0]} cy={g[1]} r={g[2]} fill={i === 1 ? 'var(--c-pea)' : 'var(--f)'} />
			<circle
				cx={g[0]}
				cy={g[1]}
				r={g[2] * 0.45}
				fill="var(--c-milk)"
				opacity=".75"
				stroke="none"
			/>
		{/each}
	{:else if kind === 'garlic'}
		<path d="M-1.6-4.4Q1-5.4 2.4-2.6Q5 1 2 4.2Q-1.6 5.6-3.4 2.4Q-4.4-1-1.6-4.4Z" fill="var(--f)" />
		<path d="M-.4-3Q1.6 0 .4 3.6" class="vein" />
		<path d="M-1.6-4.4L-1.4-5.6L-.2-4.8Z" fill="var(--c-wheat)" />
	{:else if kind === 'carrot'}
		<circle r="4.4" fill="var(--f)" />
		<circle r="2.5" fill="var(--c-lemon)" opacity=".5" stroke="none" />
		<circle r=".8" fill="var(--fd)" opacity=".6" stroke="none" />
		<path d="M-3 -2.2Q-1.6 -3.4 0-3.4" class="hi" />
	{:else if kind === 'broccoli'}
		<path d="M-1.2 1.4H1.4L2 5.2H-2Z" fill="var(--c-pea)" />
		<circle cx="-2.8" cy="-.8" r="2.8" fill="var(--f)" />
		<circle cx="2.8" cy="-.8" r="2.8" fill="var(--f)" />
		<circle cx="0" cy="-2.8" r="3" fill="var(--f)" />
		<circle cx="-.8" cy="-3.4" r=".6" fill="var(--c-pea)" stroke="none" />
		<circle cx="2.4" cy="-1.2" r=".5" fill="var(--c-pea)" stroke="none" />
		<circle cx="-2.8" cy="-.8" r=".5" fill="var(--c-pea)" stroke="none" />
	{:else if kind === 'spinach'}
		<path
			d="M-5.6 1Q-6.4-2.6-2.6-3.2Q-1-5 1.8-3.6Q5.4-3.8 5.8-.4Q6.4 3 2.6 3.8Q-.6 5.6-3 3.6Q-5.4 3.6-5.6 1Z"
			fill="color-mix(in srgb, var(--f) 78%, #1a1008)"
		/>
		<path d="M-4.4 1.4Q0 0 4.6-.2M-1 .6Q-2-1-1.8-2.4M1.4 .1Q2.4 1.6 2 3" class="vein" />
	{:else if kind === 'basil'}
		<path d="M-5.4 1.4Q-4-4.8 1-4.6Q5-4.2 6 .4Q1.8 4.4-3 3.4Q-5 3-5.4 1.4Z" fill="var(--f)" />
		<path d="M-4.6 1.4Q0-.4 5 .2M-1.6 .4Q-1.8-1.6-1-3M1.4-.1Q1.2 1.8 2 2.8" class="vein" />
		<path d="M-5.4 1.4L-6.4 2.4" class="stem" />
	{:else if kind === 'avocado'}
		<path d="M-5.4 0Q-5-5 0-5Q5-5 5.4 0Q5 5 0 5Q-5 5-5.4 0Z" fill="var(--f)" />
		<path
			d="M-4.2 0Q-3.8-3.8 0-3.8Q3.8-3.8 4.2 0Q3.8 3.8 0 3.8Q-3.8 3.8-4.2 0Z"
			fill="var(--c-pea)"
			stroke="none"
		/>
		<circle cx="0" cy="1" r="1.6" fill="var(--c-brown)" stroke="none" />
	{:else if kind === 'cucumber'}
		<circle r="4.8" fill="var(--f)" />
		<circle r="3.7" fill="var(--c-pea)" stroke="none" />
		<circle r="1.4" fill="var(--c-milk)" opacity=".6" stroke="none" />
		<circle cx="-1.6" cy="1.4" r=".4" fill="var(--fd)" stroke="none" />
		<circle cx="1.6" cy="-1.4" r=".4" fill="var(--fd)" stroke="none" />
		<circle cx="1.8" cy="1.6" r=".4" fill="var(--fd)" stroke="none" />
	{:else if kind === 'corn'}
		{#each [[-3, -2, 20], [0.4, -3.2, -10], [3.4, -1.4, 35], [-1.6, 1.4, -20], [2, 2.4, 10], [-4.2, 2.6, 30]] as g, i (i)}
			<ellipse
				cx={g[0]}
				cy={g[1]}
				rx="1.5"
				ry="1.9"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill="var(--f)"
			/>
		{/each}
	{:else if kind === 'pea'}
		{#each [[-2.8, -1.4], [2.4, -2.2], [0.2, 2.4]] as g, i (i)}
			<circle cx={g[0]} cy={g[1]} r="2.2" fill="var(--f)" />
			<circle
				cx={g[0] - 0.7}
				cy={g[1] - 0.7}
				r=".5"
				fill="var(--c-milk)"
				opacity=".7"
				stroke="none"
			/>
		{/each}
	{:else if kind === 'olive'}
		<ellipse rx="4.2" ry="3.2" fill="var(--f)" />
		{#if v === 0}
			<circle r="1.2" fill="var(--c-leaf)" opacity=".0" stroke="none" />
			<path d="M-2.6-1.4Q-1-2.4 .6-2.2" class="hi" />
			<circle r="1" fill="var(--c-muted)" opacity=".5" stroke="none" />
		{:else if v === 1}
			<circle r="1.2" fill="var(--c-milk)" opacity=".55" stroke="none" />
			<path d="M-2.6-1.4Q-1-2.4 .6-2.2" class="hi" />
		{:else}
			<circle r="1.4" fill="var(--c-rust)" stroke="none" />
			<path d="M-2.6-1.4Q-1-2.4 .6-2.2" class="hi" />
		{/if}
	{:else if kind === 'potato'}
		<path
			d="M-4.6-1.2Q-4-4.2 -.6-4Q3-4.6 4.4-1.6Q5.4 2 2.6 3.8Q-1 5-3.6 3.2Q-5 1.4-4.6-1.2Z"
			fill="var(--f)"
		/>
		<path
			d="M-3-.8Q-2.4-2.8-.2-2.6Q2.6-3 3.2-.6Q3.4 1.8 1.4 2.6Q-1.6 3.2-2.6 1.6Q-3.2 .6-3-.8Z"
			fill="var(--c-cream)"
			stroke="none"
		/>
		<circle cx="-.8" cy=".2" r=".4" fill="var(--fd)" stroke="none" />
		<circle cx="1.4" cy="-.8" r=".4" fill="var(--fd)" stroke="none" />
	{:else if kind === 'prawn'}
		<path
			d="M-4.8 3.2Q-6.4-2.6-1.6-4.4Q3-5.6 5-2Q4.8 .8 2.6 1.6Q3.4-1.8 .6-2.2Q-2.4-2-2.6 1Q-2.8 3-1.2 4.2Q-3.6 5-4.8 3.2Z"
			fill="var(--f)"
		/>
		<path d="M-2 -3.2Q-.2 -2.6 .4 -1.8M2.4-4.2Q3.4-3.2 3.4-2.2" class="vein" />
		<path d="M-4.8 3.2L-6.4 4.6L-3.8 5.2Z" fill="var(--c-pumpkin)" />
	{:else if kind === 'salmon'}
		<path d="M-5.8 .4Q-4-3.6 .4-3.8Q5-4 6 .2Q4.8 4.2-.4 4Q-5 4.4-5.8 .4Z" fill="var(--f)" />
		<path
			d="M-4-.6Q0-2.4 4-.8M-4 1.8Q0 .2 4.2 1.8"
			fill="none"
			stroke="var(--c-milk)"
			stroke-width=".8"
			opacity=".7"
		/>
	{:else if kind === 'chicken'}
		<path
			d="M-5 -.6Q-4.6-4.4 -.4-4.2Q4.2-5 5.2-1Q6 2.8 2 4Q-2 5.4-4.4 3Q-5.6 1.2-5 -.6Z"
			fill="var(--f)"
		/>
		<path
			d="M-3 -.6Q-2.6-2.6-.4-2.4Q2.4-2.8 3 -.6Q3.2 1.6 1 2Q-1.6 2.6-2.8 1.2Q-3.2 .4-3 -.6Z"
			fill="var(--c-cream)"
			stroke="none"
		/>
		<path d="M-1.4-.4Q0 .6 1.6-.2" class="vein" />
	{:else if kind === 'bacon'}
		<path d="M-6-1.6Q-3-4 0-1.8T6-2.2L6.4 1.6Q3.4 3.6 0 1.6T-6.2 2Z" fill="var(--f)" />
		<path d="M-5.2-.4Q-2.6-2.4 0-.6T5.4-.8" fill="none" stroke="var(--c-pink)" stroke-width="1.1" />
	{:else if kind === 'sausage'}
		<circle r="4.4" fill="var(--f)" />
		<circle r="3.2" fill="var(--c-pink)" stroke="none" />
		<circle cx="-1.2" cy="-.8" r=".5" fill="var(--c-cream)" stroke="none" />
		<circle cx="1.2" cy=".6" r=".5" fill="var(--c-cream)" stroke="none" />
		<circle cx="-.4" cy="1.6" r=".4" fill="var(--c-cream)" stroke="none" />
	{:else if kind === 'mince'}
		{#each [[-3.4, -1.8, 1.7], [0.4, -2.8, 1.5], [3.4, -0.6, 1.8], [-1.2, 1.8, 1.9], [2.6, 3, 1.4], [-4, 2.6, 1.2]] as g, i (i)}
			<path
				d="M{g[0] - g[2]} {g[1]}Q{g[0] - g[2] * 0.6} {g[1] - g[2]} {g[0]} {g[1] -
					g[2] * 0.9}Q{g[0] + g[2]} {g[1] - g[2] * 0.4} {g[0] + g[2] * 0.9} {g[1] +
					g[2] * 0.4}Q{g[0]} {g[1] + g[2]} {g[0] - g[2]} {g[1]}Z"
				fill={i % 3 === 1 ? 'var(--c-terra)' : 'var(--f)'}
			/>
		{/each}
	{:else if kind === 'cheese'}
		{#each [[-4.4, -2.6, 20], [-0.6, -3.4, -15], [2, -0.6, 40], [-3.8, 1.8, -30], [0.4, 2.8, 10], [3, 3.4, -20]] as g, i (i)}
			<path
				d="M{g[0]} {g[1]}q1.2 1.2 2.8 .2"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill="none"
				stroke="var(--f)"
				stroke-width="1.3"
				stroke-linecap="round"
			/>
		{/each}
	{:else if kind === 'mozzarella'}
		<circle r="4.8" fill="var(--f)" stroke="var(--c-muted)" stroke-width=".5" />
		<path d="M-3.2-2Q-1.4-3.8 1.2-3.4" class="hi" />
		<circle cx="1.6" cy="1.6" r="1.2" fill="var(--c-cream)" opacity=".7" stroke="none" />
	{:else if kind === 'sesame'}
		{#each [[-3.6, -1.4, 20], [-0.4, -3, -30], [3, -0.8, 50], [-1.6, 2, -10], [2.2, 3, 30]] as g, i (i)}
			<path
				d="M{g[0]} {g[1] - 1.2}Q{g[0] + 0.9} {g[1]} {g[0]} {g[1] + 1.2}Q{g[0] -
					0.9} {g[1]} {g[0]} {g[1] - 1.2}Z"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill="var(--f)"
				stroke="var(--c-wheat)"
				stroke-width=".3"
			/>
		{/each}
	{:else if kind === 'seed'}
		{#each [[-3.2, -1.2, 20], [0, -2.6, -40], [3.2, -0.4, 60], [-1.4, 2, -10], [2, 3, 30]] as g, i (i)}
			<ellipse
				cx={g[0]}
				cy={g[1]}
				rx="1.5"
				ry=".95"
				transform="rotate({g[2]} {g[0]} {g[1]})"
				fill={i === 1 ? 'var(--c-olive)' : 'var(--f)'}
			/>
		{/each}
	{:else if kind === 'banana'}
		<circle r="4.6" fill="var(--c-wheat)" />
		<circle r="3.8" fill="var(--c-cream)" stroke="none" />
		<circle r="2" fill="var(--c-lemon)" opacity=".35" stroke="none" />
		<circle cx="-.6" cy="-.2" r=".45" fill="var(--c-dark)" opacity=".7" stroke="none" />
		<circle cx=".8" cy=".8" r=".4" fill="var(--c-dark)" opacity=".7" stroke="none" />
	{:else if kind === 'apple'}
		<path d="M-5.2 -1.4A5.4 5.4 0 0 1 5.2 -1.4Q5 3.6 0 4.8Q-5 3.6-5.2-1.4Z" fill="var(--f)" />
		<path
			d="M-3.8 -1.2A4 4 0 0 1 3.8 -1.2Q3.6 2.6 0 3.6Q-3.6 2.6-3.8-1.2Z"
			fill="var(--c-cream)"
			stroke="none"
		/>
		<ellipse cx="0" cy=".4" rx=".8" ry="1.5" fill="var(--c-brown)" stroke="none" />
	{:else if kind === 'citrus'}
		<circle r="5.2" fill="var(--f)" />
		<circle r="4.3" fill="var(--c-milk)" opacity=".7" stroke="none" />
		<circle r="3.6" fill="var(--c-pumpkin)" opacity=".85" stroke="none" />
		<path
			d="M0-3.6V3.6M-3.1-1.8L3.1 1.8M-3.1 1.8L3.1-1.8"
			fill="none"
			stroke="var(--c-milk)"
			stroke-width=".7"
			opacity=".8"
		/>
	{:else if kind === 'zucchini'}
		<circle r="4.8" fill="var(--f)" />
		<circle r="3.8" fill="var(--c-cream)" stroke="none" />
		{#each [0, 60, 120, 180, 240, 300] as a (a)}
			<circle
				cx={Math.cos((a * Math.PI) / 180) * 1.8}
				cy={Math.sin((a * Math.PI) / 180) * 1.8}
				r=".5"
				fill="var(--c-pea)"
				stroke="none"
			/>
		{/each}
	{:else if kind === 'aubergine'}
		<circle r="4.8" fill="var(--f)" />
		<circle r="3.7" fill="var(--c-cream)" stroke="none" />
		<circle r="1.1" fill="var(--c-wheat)" opacity=".6" stroke="none" />
		<circle cx="-1.6" cy="1" r=".4" fill="var(--c-dark)" stroke="none" />
		<circle cx="1.6" cy="-1" r=".4" fill="var(--c-dark)" stroke="none" />
		<circle cx="1.4" cy="1.8" r=".4" fill="var(--c-dark)" stroke="none" />
	{:else if kind === 'cabbage'}
		{#each [[-4.6, -2.6], [0.4, -3.4], [-3.8, 0.8], [1, 0.4], [-1.6, 3.6], [3, 3]] as g, i (i)}
			<path
				d="M{g[0]} {g[1]}q1.2-1.6 2.4 0t2.4 0"
				fill="none"
				stroke="var(--f)"
				stroke-width="1.3"
				stroke-linecap="round"
				opacity={i % 2 ? 0.85 : 1}
			/>
		{/each}
	{:else if kind === 'pumpkin'}
		<rect x="-3.8" y="-3" width="7.6" height="7" rx="1.4" fill="var(--f)" />
		<path
			d="M-3.8-1.6Q0-3.6 3.8-1.6"
			fill="none"
			stroke="var(--c-lemon)"
			stroke-width="1.2"
			opacity=".55"
		/>
		<circle cx="1.2" cy="1.6" r=".5" fill="var(--fd)" stroke="none" />
	{:else if kind === 'beet'}
		<rect x="-3.6" y="-3.6" width="7.2" height="7.2" rx="1.6" fill="var(--f)" />
		<rect
			x="-2.2"
			y="-2.2"
			width="4.4"
			height="4.4"
			rx="1"
			fill="none"
			stroke="var(--c-pink)"
			stroke-width=".6"
			opacity=".7"
		/>
		<path d="M-2.6-1.4Q-1-2.8 1-2.6" class="hi" />
	{/if}
</g>

<style>
	.t {
		stroke: var(--fd);
		stroke-width: 0.5;
		stroke-linejoin: round;
		stroke-linecap: round;
	}
	.vein {
		fill: none;
		stroke: var(--fd);
		stroke-width: 0.45;
		opacity: 0.5;
	}
	.stem {
		fill: none;
		stroke: var(--fd);
		stroke-width: 0.8;
	}
	.hi {
		fill: none;
		stroke: var(--c-milk);
		stroke-width: 0.8;
		opacity: 0.45;
	}
</style>
