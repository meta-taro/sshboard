<script lang="ts">
	import { onMount } from 'svelte';

	import favicon from '$lib/assets/favicon.svg';
	import '$lib/styles/tokens.css';
	import { theme } from '$lib/theme/theme.svelte';

	let { children } = $props();

	onMount(() => {
		theme.init();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
