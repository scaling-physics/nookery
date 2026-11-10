<script lang="ts">
	// A little nook: shelf, cooking pot (steaming), cookbook, lemons and a
	// sprig of parsley. Decorative; the page text carries the meaning.
	let { size = 120, steam = true }: { size?: number; steam?: boolean } = $props();
</script>

<svg
	viewBox="0 0 120 100"
	width={size}
	height={size * (100 / 120)}
	aria-hidden="true"
	focusable="false"
	class="scene"
>
	<path
		d="M14 98V48a46 46 0 0 1 92 0v50z"
		fill="color-mix(in srgb, var(--c-wheat) 30%, var(--surface-2))"
		stroke="color-mix(in srgb, var(--c-wheat) 70%, var(--ink-3))"
		stroke-width="1.4"
	/>
	<path
		d="M22 98V50a38 38 0 0 1 76 0v48z"
		fill="none"
		stroke="color-mix(in srgb, var(--c-wheat) 60%, transparent)"
		stroke-width=".9"
		stroke-dasharray=".1 2.4"
		stroke-linecap="round"
	/>
	<!-- shelf -->
	<rect
		x="8"
		y="80"
		width="104"
		height="7"
		rx="2.5"
		fill="var(--art-wood)"
		stroke="var(--art-line)"
		stroke-width="1.1"
	/>
	<rect x="12" y="87" width="96" height="3" rx="1.5" fill="var(--art-wood-dk)" />
	<!-- cookbook -->
	<g transform="translate(70 80)">
		<rect
			x="0"
			y="-22"
			width="30"
			height="22"
			rx="2"
			fill="var(--c-terra)"
			stroke="var(--art-line)"
			stroke-width="1.1"
		/>
		<rect
			x="3"
			y="-19"
			width="24"
			height="16"
			rx="1.5"
			fill="none"
			stroke="var(--c-milk)"
			stroke-width=".9"
			opacity=".7"
		/>
		<circle
			cx="15"
			cy="-11"
			r="4"
			fill="var(--c-lemon)"
			stroke="var(--art-line)"
			stroke-width=".8"
		/>
		<path d="M12-11H18M15-14V-8" stroke="var(--art-line)" stroke-width=".6" opacity=".5" />
	</g>
	<!-- pot -->
	<g>
		<path
			d="M26 56H62V70Q62 80 52 80H36Q26 80 26 70Z"
			fill="var(--art-iron)"
			stroke="var(--art-line)"
			stroke-width="1.1"
			stroke-linejoin="round"
		/>
		<rect
			x="22"
			y="58"
			width="6"
			height="4"
			rx="2"
			fill="var(--art-iron)"
			stroke="var(--art-line)"
			stroke-width=".9"
		/>
		<rect
			x="60"
			y="58"
			width="6"
			height="4"
			rx="2"
			fill="var(--art-iron)"
			stroke="var(--art-line)"
			stroke-width=".9"
		/>
		<path
			d="M28 62Q44 66 60 62"
			fill="none"
			stroke="var(--art-iron-hi)"
			stroke-width="1.2"
			stroke-linecap="round"
			opacity=".7"
		/>
		<path
			d="M24 56Q24 46 44 46Q64 46 64 56Z"
			fill="var(--art-iron-in)"
			stroke="var(--art-line)"
			stroke-width="1.1"
			stroke-linejoin="round"
		/>
		<circle
			cx="44"
			cy="44.5"
			r="2.6"
			fill="var(--c-lemon)"
			stroke="var(--art-line)"
			stroke-width=".9"
		/>
		{#if steam}
			{#each [34, 44, 54] as x, i (x)}
				<path
					class="wisp"
					d="M{x} 40C{x - 3} 36 {x + 3} 32 {x} 28C{x - 3} 24 {x + 3} 21 {x + 1} 18"
					style="animation-delay:{i * 0.6}s"
				/>
			{/each}
		{/if}
	</g>
	<!-- lemons and parsley on the shelf -->
	<g>
		<g transform="translate(103 76)">
			<ellipse rx="6" ry="4.6" fill="var(--c-lemon)" stroke="var(--art-line)" stroke-width=".9" />
			<path
				d="M-5.6-1Q-7-2-6.4-3"
				fill="none"
				stroke="var(--art-line)"
				stroke-width=".9"
				stroke-linecap="round"
			/>
		</g>
		<g
			transform="translate(14 79)"
			fill="var(--c-leaf)"
			stroke="color-mix(in srgb, var(--c-leaf) 60%, #1a1008)"
			stroke-width=".5"
			stroke-linejoin="round"
		>
			<path d="M4 0C4-8 3-14 1-20" fill="none" stroke-width=".9" />
			<path d="M1-20C-5-21-8-16-4-13Q-5-9 0-10Q1-6 4-9Q8-9 6-13Q10-15 7-19Q4-22 1-20Z" />
			<path d="M3-10C-1-9-3-5 0-3Q-1 0 3-1Q6 0 6-3Q9-4 7-8Q5-11 3-10Z" opacity=".85" />
		</g>
	</g>
</svg>

<style>
	.scene {
		display: block;
		flex: none;
	}
	.wisp {
		fill: none;
		stroke: var(--ink-3);
		stroke-width: 1.4;
		stroke-linecap: round;
		opacity: 0;
		animation: rise 3.2s ease-in-out infinite;
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
			transform: translateY(-6px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.wisp {
			animation: none;
			opacity: 0.45;
		}
	}
</style>
