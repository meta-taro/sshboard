<script lang="ts">
	import { onMount } from 'svelte';

	import favicon from '$lib/assets/favicon.svg';
	import '$lib/styles/tokens.css';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { theme } from '$lib/theme/theme.svelte';

	let { children } = $props();

	onMount(() => {
		theme.init();
		i18n.init();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
