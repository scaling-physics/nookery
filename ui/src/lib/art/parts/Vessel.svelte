<script lang="ts">
	// One vessel layer. `back` is drawn before the food, `front` after it
	// (glass, rims, a top bun). Needs --bgc and --base set on the root <svg>.
	// Markup comes from ../vessels.ts (internal strings, no user text).
	import type { Vessel } from '../types.js';
	import { vesselSvg } from '../vessels.js';

	let { vessel, layer }: { vessel: Vessel; layer: 'back' | 'front' } = $props();
	const markup = $derived(vesselSvg(vessel, layer));
</script>

{#if markup}<g class="vessel {layer}">{@html markup}</g>{/if}
