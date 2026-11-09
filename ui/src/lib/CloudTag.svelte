<script lang="ts">
	let { show }: { show: boolean } = $props();
</script>

{#if show}
	<span
		class="ml-1 inline-block rounded-full bg-[var(--ice-bg)] px-2 py-px align-middle text-xs font-medium text-[var(--ice)]"
		>cloud</span
	>
{/if}
