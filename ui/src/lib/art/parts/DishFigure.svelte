<script lang="ts" module>
	let uid = 0;
</script>

<script lang="ts">
	// The drawing of one dish: the dish kind gives vessel, base and tint; compose.ts places the recipe's ingredient pieces, seeded by
	// the title so a recipe always looks the same. Procedural inline SVG;
	// colours are layout.css tokens, so it follows light and dark.
	import { compose } from '../compose.js';
	import type { DishKind, Piece as PieceKind } from '../types.js';
	import Base from './Base.svelte';
	import Piece from './Piece.svelte';
	import Vessel from './Vessel.svelte';

	let {
		kind,
		pieces = [],
		title,
		size,
		hero = false,
		decorative = false
	}: {
		kind: DishKind;
		pieces?: PieceKind[];
		title: string;
		size?: number;
		hero?: boolean;
		/** Next to the title already: hide from assistive tech. */
		decorative?: boolean;
	} = $props();

	const id = `da${++uid}`;
	const c = $derived(compose(kind, pieces, title));
	const px = $derived(size ?? (hero ? 320 : 96));
	const detail = $derived(hero || px >= 120);
</script>

{#snippet nook()}
	{#if c.bg.shape === 'arch'}
		<path d="M12 98V52a38 38 0 0 1 76 0v46z" />
	{:else if c.bg.shape === 'blob'}
		<path d="M50 7C76 5 95 26 93 52C91 79 71 95 46 93C21 91 5 72 7 47C9 23 27 9 50 7Z" />
	{:else}
		<circle cx="50" cy="52" r="43" />
	{/if}
{/snippet}

<svg
	class="dish"
	class:hero
	viewBox="0 0 100 100"
	width={px}
	height={px}
	role={decorative ? undefined : 'img'}
	aria-label={decorative ? undefined : `Picture of ${kind.label.toLowerCase()}: ${title}`}
	aria-hidden={decorative ? 'true' : undefined}
	focusable="false"
	xmlns="http://www.w3.org/2000/svg"
	style="--bgc: var(--c-{c.bg.tone}); --base: var(--c-{c.tint}); --ol: var(--art-line);"
>
	{#if hero}
		<defs>
			<filter id="{id}-g" x="0" y="0" width="100%" height="100%">
				<feTurbulence
					type="fractalNoise"
					baseFrequency=".85"
					numOctaves="2"
					seed={c.seed % 100}
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
	<g transform={c.bg.flip ? 'translate(100 0) scale(-1 1)' : undefined}>
		<g class="nook">{@render nook()}</g>
		{#if hero}<g class="grain" filter="url(#{id}-g)">{@render nook()}</g>{/if}
		<g class="nook-line" transform="translate(50 52) scale(.9) translate(-50 -52)"
			>{@render nook()}</g
		>
	</g>

	{#if hero}
		{#each c.crumbs as p, i (i)}<Piece
				kind={p.kind}
				x={p.x}
				y={p.y}
				rot={p.rot}
				s={p.s}
				v={p.v}
			/>{/each}
	{/if}

	<g transform="rotate({c.tilt} 50 70)">
		<ellipse class="shadow" cx="50" cy="81" rx="40" ry="7" />
		<Vessel vessel={c.vessel} layer="back" />
		<Base comp={c} {detail} />
		<Base comp={c} {detail} which="side" />
		{#each c.pieces as p, i (i)}<Piece
				kind={p.kind}
				x={p.x}
				y={p.y}
				rot={p.rot}
				s={p.s}
				v={p.v}
			/>{/each}
		<Vessel vessel={c.vessel} layer="front" />

		{#if c.steam.length}
			<g class="steam" aria-hidden="true">
				{#each c.steam as s, i (i)}
					<path
						class="wisp"
						d="M{s.x} {c.surface.cy - 8}C{s.x - 3.5} {c.surface.cy - 12} {s.x + 3.5} {c.surface.cy -
							16} {s.x} {c.surface.cy - 20}C{s.x - 3.5} {c.surface.cy - 24} {s.x + 3} {c.surface
							.cy - 27} {s.x + 1} {c.surface.cy - 30}"
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
