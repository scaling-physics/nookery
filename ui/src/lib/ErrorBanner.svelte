<script lang="ts">
	import { CLOUD_CREDITS, CLOUD_SIGN_IN } from '#lib/format.js';

	let { message, class: cls = '' }: { message: string; class?: string } = $props();
</script>

<p
	role="alert"
	class="rounded-2xl bg-[var(--terracotta-bg)] p-3 text-sm text-[var(--terracotta)] {cls}"
>
	{message}
	{#if message === CLOUD_SIGN_IN || message === CLOUD_CREDITS}
		<a href="/settings" class="font-medium underline underline-offset-2">Open Settings</a>
	{/if}
</p>
