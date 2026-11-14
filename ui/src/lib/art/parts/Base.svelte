<script lang="ts">
	// The food body inside its vessel's surface: the main base and, smaller,
	// the side. Markup comes from ../bases.ts. `detail` is off at small sizes.
	import type { Composition } from '../compose.js';
	import { baseSvg } from '../bases.js';
	import { hash, mulberry32 } from '../dish.js';

	let {
		comp,
		detail = true,
		which = 'main'
	}: { comp: Composition; detail?: boolean; which?: 'main' | 'side' } = $props();

	const markup = $derived.by(() => {
		const side = which === 'side';
		const base = side ? comp.side : comp.base;
		const s = side ? comp.sideSurface : comp.surface;
		if (!base || !s) return '';
		return baseSvg(base, {
			s,
			tint: side ? (comp.sideTint ?? 'wheat') : comp.tint,
			sideTint: comp.sideTint,
			vessel: comp.vessel,
			rnd: mulberry32((comp.seed ^ hash(`${which}:${base}`)) >>> 0),
			detail
		});
	});
</script>

{#if markup}<g class="base">{@html markup}</g>{/if}
