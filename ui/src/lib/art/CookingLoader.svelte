<script lang="ts">
	// A pan with tossed carrot coins and bubbles, for the waiting moments.
	// Decorative; the caller supplies the status text. Still under
	// prefers-reduced-motion.
	let { size = 140 }: { size?: number } = $props();
</script>

<svg
	viewBox="0 0 120 90"
	width={size}
	height={size * 0.75}
	aria-hidden="true"
	focusable="false"
	class="loader"
>
	<ellipse cx="58" cy="78" rx="42" ry="6" fill="color-mix(in srgb, var(--ink) 14%, transparent)" />
	<path
		d="M84 62L110 54Q115 53 115 58L114 62Q113 65 108 66L84 70Z"
		fill="var(--art-iron)"
		stroke="var(--art-line)"
		stroke-width="1.1"
		stroke-linejoin="round"
	/>
	<g class="pan">
		<ellipse
			cx="58"
			cy="68"
			rx="40"
			ry="13"
			fill="var(--art-iron-in)"
			stroke="var(--art-line)"
			stroke-width="1.1"
		/>
		<ellipse
			cx="58"
			cy="63"
			rx="40"
			ry="13"
			fill="var(--art-iron)"
			stroke="var(--art-line)"
			stroke-width="1.1"
		/>
		<ellipse cx="58" cy="63" rx="34" ry="10" fill="var(--art-iron-in)" />
		<path
			d="M30 59Q58 52 86 59"
			fill="none"
			stroke="var(--art-iron-hi)"
			stroke-width="1.2"
			stroke-linecap="round"
			opacity=".7"
		/>
	</g>
	{#each [[40, 0], [58, 0.5], [74, 1]] as c, i (i)}
		<g class="toss" style="animation-delay:{c[1]}s; --x:{c[0]}px">
			<g transform="translate({c[0]} 52)">
				<ellipse
					rx="6"
					ry="3.4"
					fill="var(--c-pumpkin)"
					stroke="color-mix(in srgb, var(--c-pumpkin) 55%, #1a1008)"
					stroke-width=".7"
				/>
				<ellipse rx="2.6" ry="1.3" fill="var(--c-milk)" opacity=".4" />
			</g>
		</g>
	{/each}
	{#each [[46, 0], [66, 0.9], [56, 1.7]] as b, i (i)}
		<circle class="bubble" cx={b[0]} cy="60" r="2" style="animation-delay:{b[1]}s" />
	{/each}
</svg>

<style>
	.loader {
		display: block;
		overflow: visible;
	}
	.pan {
		transform-origin: 58px 68px;
		animation: shake 1.8s ease-in-out infinite;
	}
	.toss {
		animation: toss 1.8s ease-in-out infinite;
	}
	.bubble {
		fill: none;
		stroke: var(--c-milk);
		stroke-width: 0.9;
		opacity: 0;
		transform-box: fill-box;
		transform-origin: center;
		animation: pop 2.2s ease-out infinite;
	}
	@keyframes shake {
		0%,
		100% {
			transform: rotate(0);
		}
		50% {
			transform: rotate(-2.5deg);
		}
	}
	@keyframes toss {
		0%,
		100% {
			transform: translateY(8px) rotate(0);
		}
		45% {
			transform: translateY(-14px) rotate(25deg);
		}
	}
	@keyframes pop {
		0% {
			opacity: 0;
			transform: translateY(3px) scale(0.6);
		}
		40% {
			opacity: 0.9;
		}
		100% {
			opacity: 0;
			transform: translateY(-4px) scale(1.2);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pan,
		.toss,
		.bubble {
			animation: none;
		}
		.toss {
			transform: translateY(-3px);
		}
		.bubble {
			opacity: 0.6;
		}
	}
</style>
