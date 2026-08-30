<script lang="ts">
	import { onMount } from 'svelte';

	import favicon from '$lib/assets/favicon.svg';
	// **フォントは同梱する。**CSP が default-src 'self' で、
	// デスクトップアプリはオフラインでも動く必要がある。
	import '@fontsource/zen-kaku-gothic-new/400.css';
	import '@fontsource/zen-kaku-gothic-new/500.css';
	import '@fontsource/zen-kaku-gothic-new/700.css';
	import '@fontsource/jetbrains-mono/400.css';
	import '@fontsource/jetbrains-mono/500.css';
	import '$lib/styles/tokens.css';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { redaction } from '$lib/redaction/redaction.svelte';
	import { textSize } from '$lib/text-size/text-size.svelte';
	import { theme } from '$lib/theme/theme.svelte';

	let { children } = $props();

	onMount(() => {
		theme.init();
		// **`app.html` が既に当てている。**ここは状態を読み戻すためで、
		// 片方だけにすると「保存はされるが次の起動で標準に戻る」になる。
		textSize.init();
		i18n.init();

		// **撮る前に伏せる**（D26）。ここを繋がないと、伏せる仕組みが在っても働かない
		// （実際に、書いたまま誰も呼んでいない状態が続いた）。
		const stops: Array<() => void> = [];
		redaction
			.watch()
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* ブラウザで開いているだけ。**画面は普通に動く。** */
			});
		return () => stops.forEach((stop) => stop());
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
