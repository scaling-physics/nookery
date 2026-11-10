<script lang="ts" module>
	let uid = 0;
</script>

<script lang="ts">
	// A small hand-drawn-feeling picture of a dish: a vessel chosen from the
	// title, toppings chosen from the food categories, laid out by a PRNG
	// seeded with the title, so a recipe always looks the same. Procedural
	// inline SVG; colours come from layout.css tokens, so it follows light
	// and dark. Pure planning is in dish.ts.
	import type { FoodCategoryView } from '../bindings.js';
	import { plan, type PlacedTopping, type Topping } from './dish.js';

	let {
		title,
		categories,
		size,
		hero = false,
		decorative = false
	}: {
		title: string;
		categories: FoodCategoryView[];
		size?: number;
		hero?: boolean;
		/** Next to the title already: hide from assistive tech. */
		decorative?: boolean;
	} = $props();

	const id = `da${++uid}`;
	const p = $derived(plan(title, categories));
	const px = $derived(size ?? (hero ? 320 : 96));

	const FILL: Record<Topping, string> = {
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
		dot: 'muted'
	};
	function fillOf(t: PlacedTopping): string {
		if (t.kind === 'leaf' && t.v === 1) return 'pea';
		if (t.kind === 'bean' && t.v === 1) return 'rust';
		return FILL[t.kind];
	}

	/** Layer polygon inside the glass, y1 to y2, with a wavy top edge. */
	function layer(y1: number, y2: number, wave = 1.6): string {
		const l = (y: number) => 32 + (y - 26) * 0.069;
		const r = (y: number) => 68 - (y - 26) * 0.069;
		const mid = (l(y1) + r(y1)) / 2;
		return `M${l(y1)} ${y1} Q${(l(y1) + mid) / 2} ${y1 - wave} ${mid} ${y1} T${r(y1)} ${y1} L${r(y2)} ${y2} L${l(y2)} ${y2}Z`;
	}

	const wokBits = [
		[30, 44, 14],
		[44, 40, -20],
		[58, 41, 35],
		[70, 44, -10]
	] as const;
</script>

{#snippet piece(t: PlacedTopping)}
	<g
		class="t"
		transform="translate({t.x} {t.y}) rotate({t.rot}) scale({t.s})"
		style="--f: var(--c-{fillOf(t)})"
	>
		{#if t.kind === 'leaf'}
			<path d="M-5.5 0Q0-6.5 5.5 0Q0 6.5-5.5 0Z" fill="var(--f)" />
			<path d="M-4 0H4" class="vein" />
		{:else if t.kind === 'tomato'}
			<circle r="4.2" fill="var(--f)" />
			<circle r="2.6" fill="var(--c-milk)" opacity=".45" stroke="none" />
			<circle cx="-.9" cy="-.6" r=".5" fill="var(--fd)" stroke="none" />
			<circle cx="1" cy=".9" r=".5" fill="var(--fd)" stroke="none" />
		{:else if t.kind === 'wedge'}
			<path d="M-5 2.6A5.8 5.8 0 0 1 5 2.6Z" fill="var(--f)" />
			<path d="M-3 2.2A3.6 3.6 0 0 1 3 2.2" class="vein" />
		{:else if t.kind === 'herb'}
			<path d="M0 5V-1M0 2L-3.6-.4M0 2L3.6-.4" class="stem" />
			<g transform="translate(-3.4 -.5) rotate(-52) scale(.15)"
				><path
					id="{id}-l{t.x}"
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
		{:else if t.kind === 'lemon'}
			<circle r="5.2" fill="var(--f)" />
			<circle r="4.2" fill="var(--c-milk)" opacity=".55" stroke="none" />
			<path d="M0-3.8V3.8M-3.3-1.9L3.3 1.9M-3.3 1.9L3.3-1.9" class="vein" />
		{:else if t.kind === 'cube'}
			<rect x="-3.4" y="-3.4" width="6.8" height="6.8" rx="1.2" fill="var(--f)" />
			<circle cx="-1" cy="-1" r=".6" fill="var(--fd)" stroke="none" />
			<circle cx="1.2" cy="1.3" r=".5" fill="var(--fd)" stroke="none" />
		{:else if t.kind === 'dollop'}
			<path d="M-5 2Q-6-3 -1-3.4Q0-6 3-4Q7-3.5 5.6 1Q4 3.6-1 3.4Q-4 3.6-5 2Z" fill="var(--f)" />
		{:else if t.kind === 'egg'}
			<path d="M-6 0Q-7-5-2-5.5Q3-7 6-3Q8 2 3 4.5Q-3 6-6 0Z" fill="var(--f)" />
			<circle cy="-.3" r="2.7" fill="var(--c-lemon)" />
			<circle cx="-.8" cy="-1.2" r=".7" fill="var(--c-milk)" opacity=".7" stroke="none" />
		{:else if t.kind === 'slice'}
			<ellipse rx="5.8" ry="3.8" fill="var(--f)" />
			<path d="M-3.4-.8Q0-2.2 3.4-.8M-3-.4Q0 1 3 1.8" class="vein" />
		{:else if t.kind === 'fillet'}
			<path d="M-6 0Q-3-4 2-3.5Q6-3 6 1Q2 4-3 3Q-6 2-6 0Z" fill="var(--f)" />
			<path d="M-3-1Q0-2 3-1M-3 1Q0 0 3 1" class="vein" />
		{:else if t.kind === 'grain'}
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
		{:else if t.kind === 'bean'}
			<path d="M-3-1.6Q-1-3.4 1.4-2.4Q3.4-1 2 1.4Q.4 3-1.8 2.2Q-4 1-3-1.6Z" fill="var(--f)" />
			<path d="M-1.8-.8Q-.4-1.8 .8-1" class="vein" />
			<path
				d="M2.6 2.2Q4.2 .8 5.6 2.2Q6.2 4 4.4 4.6Q2.6 4.4 2.6 2.2Z"
				fill="var(--f)"
				opacity=".9"
			/>
		{:else if t.kind === 'swoosh'}
			<path d="M-7 1Q-3.5-3.4 0 0T7-1" fill="none" stroke="var(--f)" stroke-width="2" />
			<path
				d="M-7 1Q-3.5-3.4 0 0T7-1"
				fill="none"
				stroke="var(--c-milk)"
				stroke-width=".5"
				opacity=".4"
			/>
		{:else if t.kind === 'sprinkle'}
			{#each [[-4, -1], [-1.5, 2.5], [1.5, -2], [4, 1.4], [0, 0.2]] as g, i (i)}
				<circle
					cx={g[0]}
					cy={g[1]}
					r=".9"
					fill={i % 2 ? 'var(--c-rust)' : 'var(--f)'}
					stroke="none"
				/>
			{/each}
		{:else if t.kind === 'nut'}
			<path
				d="M-4.5-.5Q-4.5-4 -.5-3.4Q0-4.4 1-3.4Q4.5-3.6 4.4 0Q4.4 3.6 0 3.6Q-4.5 3.6-4.5-.5Z"
				fill="var(--f)"
			/>
			<path d="M0-3V3.2M-2.6-1.2Q-1.4 0-2.4 1.8" class="vein" />
		{:else if t.kind === 'berry'}
			<circle r="2.8" fill="var(--f)" />
			<circle cx="-.9" cy="-.9" r=".7" fill="var(--c-milk)" opacity=".7" stroke="none" />
			<circle cx="3.6" cy="1.8" r="2.2" fill="var(--f)" />
		{:else if t.kind === 'crouton'}
			<rect x="-3.2" y="-3.2" width="6.4" height="6.4" rx="1.4" fill="var(--f)" />
			<circle cx="-.8" cy=".6" r=".5" fill="var(--fd)" stroke="none" />
		{:else}
			<circle cx="-2.4" cy="-.4" r="1.2" fill="var(--f)" />
			<circle cx="1.2" cy="1.4" r="1.5" fill="var(--f)" />
			<circle cx="2.6" cy="-1.8" r=".9" fill="var(--f)" />
		{/if}
	</g>
{/snippet}

{#snippet toppings()}
	{#each p.items as t, i (i)}{@render piece(t)}{/each}
{/snippet}

<svg
	class="dish"
	class:hero
	viewBox="0 0 100 100"
	width={px}
	height={px}
	role={decorative ? undefined : 'img'}
	aria-label={decorative ? undefined : `Illustration: ${title}`}
	aria-hidden={decorative ? 'true' : undefined}
	focusable="false"
	xmlns="http://www.w3.org/2000/svg"
	style="--bgc: var(--c-{p.bg.tone}); --base: var(--c-{p.base}); --ol: var(--art-line);"
>
	{#if hero}
		<defs>
			<filter id="{id}-g" x="0" y="0" width="100%" height="100%">
				<feTurbulence
					type="fractalNoise"
					baseFrequency=".85"
					numOctaves="2"
					seed={p.seed % 100}
					result="n"
				/>
				<feColorMatrix
					in="n"
					type="matrix"
					values="0 0 0 0 .35  0 0 0 0 .25  0 0 0 0 .15  0 0 0 .55 -.12"
					result="g"
				/>
				<feComposite in="g" in2="SourceAlpha" operator="in" />
			</filter>
		</defs>
	{/if}

	<!-- the nook behind the dish -->
	<g transform={p.bg.flip ? 'translate(100 0) scale(-1 1)' : undefined}>
		{#snippet nook()}
			{#if p.bg.shape === 'arch'}
				<path d="M12 98V52a38 38 0 0 1 76 0v46z" />
			{:else if p.bg.shape === 'blob'}
				<path d="M50 7C76 5 95 26 93 52C91 79 71 95 46 93C21 91 5 72 7 47C9 23 27 9 50 7Z" />
			{:else}
				<circle cx="50" cy="52" r="43" />
			{/if}
		{/snippet}
		<g class="nook">{@render nook()}</g>
		{#if hero}<g class="grain" filter="url(#{id}-g)">{@render nook()}</g>{/if}
		<g class="nook-line" transform="translate(50 52) scale(.9) translate(-50 -52)"
			>{@render nook()}</g
		>
	</g>

	{#if hero}
		{#each p.crumbs as c, i (i)}{@render piece(c)}{/each}
	{/if}

	<g transform="rotate({p.tilt} 50 70)">
		<ellipse class="shadow" cx="50" cy="81" rx="40" ry="7" />

		{#if p.vessel === 'plate' || p.vessel === 'salad'}
			<ellipse class="ink" cx="50" cy="66" rx="40" ry="15" fill="var(--art-ceramic-in)" />
			<ellipse class="ink" cx="50" cy="62" rx="40" ry="15" fill="var(--art-ceramic)" />
			<ellipse cx="50" cy="62" rx="30" ry="10.5" fill="var(--art-ceramic-in)" opacity=".6" />
			<ellipse
				cx="50"
				cy="63"
				rx="29"
				ry="10"
				fill="none"
				stroke="var(--bgc)"
				stroke-width="1.4"
				opacity=".55"
			/>
			{#if p.vessel === 'plate'}
				<!-- the twirl of pasta -->
				{#each Array.from({ length: p.twirl + 2 }, (_, i) => i) as i (i)}
					<ellipse
						class="strand"
						cx={50 + (i % 2 ? 1.2 : -1.2)}
						cy={59 - i * 3.6}
						rx={19 - i * 3.4}
						ry={6.2 - i * 0.9}
						fill="var(--c-wheat)"
					/>
				{/each}
				<path d="M33 54Q50 62 67 54" class="hi" />
			{:else}
				{#each [[-18, 60, -25, 0], [-8, 56, 20, 1], [4, 58, -10, 0], [14, 56, 35, 1], [22, 60, 10, 0], [0, 52, 50, 1], [-12, 52, -45, 0]] as l, i (i)}
					<g
						class="t"
						style="--f: var(--c-{l[3] ? 'pea' : 'leaf'})"
						transform="translate({50 + l[0]} {l[1]}) rotate({l[2]}) scale(1.9 1.5)"
					>
						<path d="M-5.5 0Q0-6.5 5.5 0Q0 6.5-5.5 0Z" fill="var(--f)" />
						<path d="M-4 0H4" class="vein" />
					</g>
				{/each}
			{/if}
			{@render toppings()}
		{:else if p.vessel === 'bowl' || p.vessel === 'small'}
			<g transform={p.vessel === 'small' ? 'translate(10 16) scale(.8)' : undefined}>
				<path class="ink" d="M40 85H60L62 91H38Z" fill="var(--art-ceramic-in)" />
				<path
					class="ink"
					d="M13 48C13 74 29 87 50 87C71 87 87 74 87 48Z"
					fill="var(--art-ceramic)"
				/>
				<path
					d="M15 60Q50 74 85 60"
					fill="none"
					stroke="var(--bgc)"
					stroke-width="2.4"
					stroke-linecap="round"
				/>
				<path
					d="M15 65Q50 79 85 65"
					fill="none"
					stroke="var(--bgc)"
					stroke-width="1"
					opacity=".6"
				/>
				<ellipse class="ink" cx="50" cy="48" rx="37" ry="9.5" fill="var(--art-ceramic-in)" />
				<path class="ink" d="M20 49Q22 33 50 32Q78 33 80 49Q50 57 20 49Z" fill="var(--base)" />
				{#if p.vessel === 'small'}
					<path d="M30 44Q50 49 70 44" class="hi" />
					<!-- spoon -->
					<path
						d="M78 60L96 34"
						stroke="var(--art-iron-hi)"
						stroke-width="2.4"
						stroke-linecap="round"
					/>
					<ellipse
						cx="97"
						cy="31"
						rx="3.4"
						ry="5"
						transform="rotate(35 97 31)"
						fill="var(--art-iron-hi)"
					/>
				{:else}
					<path d="M30 42Q50 48 70 42" class="hi" />
				{/if}
				{@render toppings()}
			</g>
		{:else if p.vessel === 'deep'}
			<path class="ink" d="M40 85H60L62 91H38Z" fill="var(--art-ceramic-in)" />
			<path class="ink" d="M11 45C11 73 28 87 50 87C72 87 89 73 89 45Z" fill="var(--art-ceramic)" />
			<path
				d="M13 58Q50 74 87 58"
				fill="none"
				stroke="var(--bgc)"
				stroke-width="2.4"
				stroke-linecap="round"
			/>
			<ellipse class="ink" cx="50" cy="45" rx="39" ry="9.5" fill="var(--art-ceramic-in)" />
			<ellipse
				cx="50"
				cy="46"
				rx="34"
				ry="7.4"
				fill="var(--base)"
				stroke="var(--ol)"
				stroke-width=".9"
			/>
			<ellipse cx="44" cy="45" rx="14" ry="2.6" fill="var(--c-milk)" opacity=".25" />
			<path d="M26 47Q50 53 74 47" class="hi" />
			{@render toppings()}
		{:else if p.vessel === 'glass'}
			<path d={layer(66, 86)} fill="var(--c-plum)" opacity=".85" />
			<path d={layer(48, 68, 2)} fill="var(--c-milk)" />
			<path d={layer(38, 50, 2.4)} fill="var(--c-wheat)" />
			<path d={layer(30, 40, 2)} fill="var(--c-milk)" />
			<path
				d="M32 26L36 84Q36 88 40 88H60Q64 88 64 84L68 26"
				fill="var(--art-ceramic)"
				opacity=".18"
			/>
			<path class="ink" d="M32 26L36 84Q36 88 40 88H60Q64 88 64 84L68 26" fill="none" />
			<path
				d="M37 34L39.5 78"
				stroke="#fff"
				stroke-width="1.6"
				stroke-linecap="round"
				opacity=".45"
			/>
			<ellipse class="ink" cx="50" cy="26" rx="18" ry="4" fill="none" />
			<ellipse cx="50" cy="29" rx="15.5" ry="3.4" fill="var(--c-milk)" opacity=".9" />
			<path d="M60 40L80 8" stroke="var(--art-iron-hi)" stroke-width="2.4" stroke-linecap="round" />
			<ellipse
				cx="81.4"
				cy="5.4"
				rx="3"
				ry="4.3"
				transform="rotate(30 81.4 5.4)"
				fill="var(--art-iron-hi)"
			/>
			{@render toppings()}
		{:else if p.vessel === 'pan'}
			<path
				d="M72 59L95 52Q99 51 99 55L98 58Q97 61 93 62L72 66Z"
				fill="var(--art-iron)"
				stroke="var(--ol)"
				stroke-width="1"
				stroke-linejoin="round"
			/>
			<circle cx="94" cy="56.5" r="1.4" fill="var(--bg)" />
			<ellipse class="ink" cx="48" cy="66" rx="37" ry="15" fill="var(--art-iron-in)" />
			<ellipse class="ink" cx="48" cy="61" rx="37" ry="15" fill="var(--art-iron)" />
			<ellipse cx="48" cy="61" rx="32" ry="12" fill="var(--art-iron-in)" />
			<ellipse
				cx="48"
				cy="61"
				rx="29.5"
				ry="10.5"
				fill="color-mix(in srgb, var(--base) 55%, var(--c-lemon))"
				opacity=".95"
			/>
			<path
				d="M22 56Q48 50 74 56"
				fill="none"
				stroke="var(--art-iron-hi)"
				stroke-width="1.2"
				stroke-linecap="round"
				opacity=".7"
			/>
			<path d="M30 64Q48 70 66 64" class="hi" />
			{@render toppings()}
		{:else if p.vessel === 'wok'}
			<path d="M88 48L99 33" stroke="var(--ol)" stroke-width="6.6" stroke-linecap="round" />
			<path d="M88 48L99 33" stroke="var(--art-iron)" stroke-width="4.6" stroke-linecap="round" />
			<path class="ink" d="M10 47C12 74 30 86 50 86C70 86 88 74 90 47Z" fill="var(--art-iron)" />
			<path
				d="M20 62Q50 78 80 62"
				fill="none"
				stroke="var(--art-iron-hi)"
				stroke-width="1.4"
				stroke-linecap="round"
				opacity=".7"
			/>
			<ellipse class="ink" cx="50" cy="47" rx="40" ry="9" fill="var(--art-iron-in)" />
			<path
				class="ink"
				d="M18 48Q24 32 50 31Q76 32 82 48Q50 56 18 48Z"
				fill="color-mix(in srgb, var(--base) 50%, var(--c-wheat))"
			/>
			{#each wokBits as w, i (i)}
				<ellipse
					cx={w[0] + 6}
					cy={w[1] + 1}
					rx="6"
					ry="2"
					transform="rotate({w[2]} {w[0] + 6} {w[1] + 1})"
					fill="none"
					stroke="var(--c-wheat)"
					stroke-width="1.8"
					stroke-linecap="round"
					opacity=".8"
				/>
			{/each}
			{@render toppings()}
		{:else if p.vessel === 'board'}
			<path d="M10 56Q50 46 90 56L94 80Q50 90 6 80Z" fill="var(--art-wood-dk)" class="ink" />
			<path d="M10 52Q50 42 90 52L94 76Q50 86 6 76Z" fill="var(--art-wood)" class="ink" />
			<path
				d="M16 66Q50 60 84 66"
				fill="none"
				stroke="var(--art-wood-dk)"
				stroke-width=".8"
				opacity=".6"
			/>
			<circle cx="84" cy="56" r="1.6" fill="var(--art-wood-dk)" />
			{#each [[36, 58, -5], [64, 57, 4]] as s, i (i)}
				<g transform="translate({s[0]} {s[1]}) rotate({s[2]}) scale(1.05)">
					<path
						class="ink"
						d="M-13 6C-17-6-9-13-3-12C0-15 6-15 9-12C15-13 17-5 13 6L12 14Q0 17-12 14Z"
						fill="var(--c-brown)"
					/>
					<path
						d="M-10 6C-13-3-7-9-2-8.4C1-10.6 5-10.6 7-8.4C11-9 13-3 10 6L9 11Q0 13.4-9 11Z"
						fill="var(--c-wheat)"
					/>
				</g>
			{/each}
			{@render toppings()}
		{:else if p.vessel === 'wrap'}
			<g transform="rotate(-14 50 58)">
				<rect class="ink" x="14" y="44" width="62" height="28" rx="14" fill="var(--c-wheat)" />
				<path
					d="M20 50Q45 45 70 50"
					fill="none"
					stroke="var(--c-milk)"
					stroke-width="1.6"
					stroke-linecap="round"
					opacity=".5"
				/>
				<path
					d="M14 62Q14 72 28 72H46L40 56Q26 58 14 62Z"
					fill="var(--bgc)"
					opacity=".85"
					stroke="var(--ol)"
					stroke-width="1"
					stroke-linejoin="round"
				/>
				<path d="M22 66L34 66M24 69L36 69" stroke="var(--c-milk)" stroke-width=".8" opacity=".6" />
				<ellipse class="ink" cx="75" cy="55" rx="6.5" ry="13" fill="var(--c-pea)" />
				<ellipse cx="75" cy="55" rx="3.6" ry="9" fill="var(--c-leaf)" opacity=".7" />
				{@render toppings()}
			</g>
		{:else if p.vessel === 'round'}
			<ellipse class="ink" cx="50" cy="63" rx="41" ry="19" fill="var(--c-brown)" />
			<ellipse class="ink" cx="50" cy="59" rx="41" ry="19" fill="var(--c-wheat)" />
			<ellipse
				cx="50"
				cy="58.5"
				rx="34.5"
				ry="15.5"
				fill="var(--base)"
				stroke="var(--ol)"
				stroke-width=".8"
			/>
			<path
				d="M26 54Q34 50 40 55Q36 60 28 59ZM58 52Q70 50 74 56Q68 62 58 59ZM42 62Q52 60 58 65Q48 68 40 66Z"
				fill="color-mix(in srgb, var(--c-lemon) 55%, var(--c-milk))"
				opacity=".9"
			/>
			{@render toppings()}
			<path d="M50 44V73M20 52L80 66" stroke="var(--c-brown)" stroke-width=".8" opacity=".5" />
		{:else if p.vessel === 'slice'}
			<ellipse class="ink" cx="50" cy="76" rx="38" ry="9" fill="var(--art-ceramic)" />
			<ellipse cx="50" cy="75" rx="29" ry="6" fill="var(--art-ceramic-in)" opacity=".7" />
			<path class="ink" d="M82 40L82 64L66 72L66 48Z" fill="var(--c-brown)" />
			<path class="ink" d="M26 48H66V72H26Z" fill="var(--c-wheat)" />
			<path d="M26 56H66V60H26Z" fill="var(--c-milk)" />
			<path d="M26 66H66V69H26Z" fill="var(--base)" opacity=".9" />
			<path class="ink" d="M26 48L42 40H82L66 48Z" fill="var(--c-milk)" />
			<path
				d="M28 49Q34 54 40 49Q46 55 52 49Q58 54 64 49"
				fill="none"
				stroke="var(--bgc)"
				stroke-width="1.4"
				stroke-linecap="round"
				opacity=".8"
			/>
			{@render toppings()}
		{/if}

		{#if p.steam.length}
			<g class="steam" aria-hidden="true">
				{#each p.steam as s, i (i)}
					<path
						class="wisp"
						d="M{s.x} {p.surface.cy - 8}C{s.x - 3.5} {p.surface.cy - 12} {s.x + 3.5} {p.surface.cy -
							16} {s.x} {p.surface.cy - 20}C{s.x - 3.5} {p.surface.cy - 24} {s.x + 3} {p.surface
							.cy - 27} {s.x + 1} {p.surface.cy - 30}"
						style="animation-delay:{s.delay}s;animation-duration:{s.dur}s"
					/>
				{/each}
			</g>
		{/if}
	</g>
</svg>

<style>
	.dish {
		display: block;
		flex: none;
		overflow: visible;
	}
	.dish.hero {
		max-width: 100%;
		height: auto;
	}
	.nook {
		fill: color-mix(in srgb, var(--bgc) 26%, var(--surface-2));
	}
	.nook-line {
		fill: none;
		stroke: color-mix(in srgb, var(--bgc) 55%, transparent);
		stroke-width: 0.8;
		stroke-dasharray: 0.1 2.2;
		stroke-linecap: round;
	}
	.grain {
		opacity: 0.4;
		mix-blend-mode: multiply;
	}
	.shadow {
		fill: color-mix(in srgb, var(--ink) 16%, transparent);
	}
	.ink {
		stroke: var(--ol);
		stroke-width: 1.1;
		stroke-linejoin: round;
	}
	.t {
		--fd: color-mix(in srgb, var(--f) 60%, #1a1008);
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
	.strand {
		stroke: color-mix(in srgb, var(--c-wheat) 55%, #1a1008);
		stroke-width: 0.9;
	}
	.hi {
		fill: none;
		stroke: var(--c-milk);
		stroke-width: 1.4;
		stroke-linecap: round;
		opacity: 0.35;
	}
	.steam {
		fill: none;
		stroke: var(--ink-2);
		stroke-width: 1.5;
		stroke-linecap: round;
		opacity: 0.55;
	}
	.wisp {
		opacity: 0;
		animation: rise 3.4s ease-in-out infinite;
	}
	@keyframes rise {
		0% {
			opacity: 0;
			transform: translateY(5px);
		}
		35% {
			opacity: 0.7;
		}
		100% {
			opacity: 0;
			transform: translateY(-7px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wisp {
			animation: none;
			opacity: 0.5;
		}
	}
</style>
