<script lang="ts">
	import './layout.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Nookery</title>
</svelte:head>

{@render children()}
