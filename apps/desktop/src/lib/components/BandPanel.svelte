<script lang="ts">
	/**
	 * **帯。**人と AI の操作が、時間順に 1 本で並ぶ所。
	 *
	 * **これがこの道具の要です**（PRD §4-1「見えないことが最大の危険」）。
	 * 見えない所で AI が動いている、を作らないために在ります。
	 *
	 * ## なぜ部品にしたか
	 *
	 * `+page.svelte` が 3300 行あり（script 1241 / style 1162）、
	 * **この節を 1 つ読むために全部を開かないといけませんでした。**
	 * 案（UI 案・2026-10-21）は帯を端末の横に常設するので、
	 * **2 か所から同じものを描けること**が先に要ります ——
	 * 写して置くと、**片方だけ直る日**が来ます（D39）。
	 *
	 * ## 絞り込みはここが持ちます
	 *
	 * **どの絞りで見ているかは、見ている人の都合**です。
	 * 画面ぜんたいの状態ではないので、ここに置きます。
	 */
	import { filterBand, type BandLine, type BandWho } from '$lib/band';
	import { i18n } from '$lib/i18n/i18n.svelte';

	let { lines }: { lines: readonly BandLine[] } = $props();

	/**
	 * **既定は「全部」。**
	 *
	 * 1 本に並んでいることが要なので、分けるのは人が選んだときだけ。
	 * **混ぜるのをやめません** —— 片方だけを見たいときに分けて見せるだけ。
	 */
	let who = $state<BandWho>('all');

	const shown = $derived(filterBand(lines, who));

	/** 札の並び。**`band.ts` の `BandWho` と対。** */
	const PICKS: readonly {
		who: BandWho;
		key: 'band.who.all' | 'band.who.ai' | 'band.who.human';
	}[] = [
		{ who: 'all', key: 'band.who.all' },
		{ who: 'ai', key: 'band.who.ai' },
		{ who: 'human', key: 'band.who.human' }
	];
</script>

<section class="band" aria-label={i18n.t('band.label')}>
	<!-- **帯が何なのかを書く。**「ここに何が出るのか」が分からないと、
	     空のときに壊れているのか、何も起きていないのかが区別できない。 -->
	<p class="what">{i18n.t('band.what')}</p>

	<!--
		**誰の行かで絞る**（UI 案・2026-10-21）。
		色だけで分けません（人と AI の色は明るさがほぼ同じ）——
		**選んだ札は地と字を入れ替えます。**
	-->
	{#if lines.length > 0}
		<div class="band-who" role="group" aria-label={i18n.t('band.label')}>
			{#each PICKS as pick (pick.who)}
				<button
					type="button"
					class="band-who-pick"
					class:active={who === pick.who}
					class:ai={pick.who === 'ai'}
					class:human={pick.who === 'human'}
					aria-pressed={who === pick.who}
					onclick={() => (who = pick.who)}
				>
					{i18n.t(pick.key)}
				</button>
			{/each}
		</div>
	{/if}

	{#if lines.length === 0}
		<p class="empty">{i18n.t('band.empty')}</p>
	{:else if shown.length === 0}
		<!--
			**絞った結果が空なのを、「何も起きていない」と見せない。**
			**`band.empty` を使い回しません** —— 言っている事実が違います。
		-->
		<p class="empty">{i18n.t('band.filtered.empty')}</p>
	{:else}
		<ol>
			{#each shown as line (line.seq)}
				<li class:ai={line.tag === '[AI]'}>{line.rendered}</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.band {
		flex: 1;
		/* **下端で切れないように。**`min-height: 0` だけだと、中身が伸びて
		   ウィンドウの外へはみ出します（Issue #22）。 */
		min-height: 6rem;
		overflow-y: auto;
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		padding: 0.6rem 0.75rem;
	}

	/* 面の説明文。**伸びません**（Issue #22 —— 伸びて 250px の空白を作っていました）。 */
	.what {
		flex: 0 0 auto;
		margin: 0 0 0.35rem;
		font-size: 0.72rem;
		line-height: 1.6;
		color: var(--fg-muted);
	}

	.empty {
		margin: 0;
		font-size: 0.76rem;
		color: var(--fg-faint);
	}

	ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		white-space: pre;
		font-family: var(--font-mono);
		font-size: 0.76rem;
		line-height: 1.75;
		color: var(--fg-muted);
	}

	/*
	 * **AI の行は橙**（UI 案・2026-10-21）。
	 *
	 * ここは `--accent` でした。`--accent` は**人の青**なので、
	 * **AI の行が青く**なっていました —— 案とちょうど逆です。
	 * 帯は「どちらが触ったか」を読む所なので、ここを取り違えると帯が読めません。
	 * `check-css-tokens.py` が見張っています。
	 */
	li.ai {
		color: var(--ai);
	}

	/*
	 * **絞り込みの札。**選んだものは地と字を入れ替えます ——
	 * 色だけで分けません（人と AI の色は明るさがほぼ同じ）。
	 */
	.band-who {
		display: flex;
		gap: 0.3rem;
		flex: 0 0 auto;
		margin: 0 0 0.45rem;
	}

	.band-who-pick {
		font: inherit;
		font-size: 0.68rem;
		color: var(--fg-control);
		background: transparent;
		border: 1px solid var(--hairline);
		border-radius: var(--r-pill);
		padding: 0.14rem 0.6rem;
		cursor: pointer;
		white-space: nowrap;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease);
	}

	.band-who-pick:hover {
		background: var(--surface-2);
	}

	.band-who-pick.ai {
		border-color: var(--ai);
		color: var(--warning);
	}

	.band-who-pick.human {
		border-color: var(--human);
		color: var(--human);
	}

	.band-who-pick.active {
		color: var(--selected-fg);
		background: var(--selected);
		border-color: transparent;
		font-weight: 700;
	}

	.band-who-pick.active.ai {
		background: var(--ai);
		color: var(--ground);
	}

	.band-who-pick.active.human {
		background: var(--human);
		color: var(--ground);
	}

	.band-who-pick:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
</style>
