<script lang="ts">
	/**
	 * **診断の面。**繋がらなかったときに「なぜ」を読む所。
	 *
	 * **握り潰さないための面です** —— 失敗が画面に出ないと、
	 * 人は「壊れている」としか言えません。段（`stage`）と
	 * どうすれば通るか（`hint`）を並べます。
	 *
	 * ## なぜ部品にしたか
	 *
	 * `+page.svelte` が 3100 行を超えており、**この面を読むために
	 * 全部を開く**必要がありました。
	 *
	 * 案（UI 案・2026-10-21）は端末の横に接続と帯を常設するので、
	 * **面を入れ替えられる形**にしておく必要があります。
	 */
	import type { DiagEvent } from '$lib/diagnostics';
	import { i18n } from '$lib/i18n/i18n.svelte';

	import Icon from './Icon.svelte';

	/**
	 * **`events` という名前は使いません。**
	 *
	 * `@testing-library/svelte` の `render` は、第 2 引数で props と
	 * Svelte のオプションを兼ねます。`events` は**そのオプションの予約名**なので、
	 * props に使うと `UnknownSvelteOptionsError` で**テストが 1 本も走りません**
	 * （実際にそうなりました）。
	 */
	let { found, onrefresh }: { found: readonly DiagEvent[]; onrefresh: () => void } = $props();
</script>

<section class="diag" aria-label={i18n.t('tab.diag')}>
	<div class="diag-head">
		<span class="label">{i18n.t('diag.label')}</span>
		<span class="scaffold">{i18n.t('diag.help')}</span>
		<button type="button" onclick={onrefresh}>
			<Icon name="refresh" size={13} />
			{i18n.t('diag.refresh')}
		</button>
	</div>
	{#if found.length === 0}
		<p class="empty">{i18n.t('diag.empty')}</p>
	{:else}
		<ol class="diag-list">
			{#each found as event (event.seq)}
				<li class={event.level}>
					<!-- **起動からの経過。**日付ではありません（秒に直して出します）。 -->
					<span class="at">{(event.atMs / 1000).toFixed(1)}s</span>
					<span class="stage">{event.stage}</span>
					<!-- **接続の名前は伏せられる所**（D26）。画面に接続先が写り込むため。 -->
					{#if event.connection}<span class="who" data-secret>{event.connection}</span>{/if}
					<span class="detail">
						{event.message}
						{#if event.hint}<em class="hint">→ {event.hint}</em>{/if}
					</span>
				</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.diag {
		display: flex;
		flex-direction: column;
		min-height: 0;
		flex: 1 1 auto;
		gap: 0.4rem;
	}

	.diag-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.label {
		font-size: 0.72rem;
		color: var(--fg);
	}

	.scaffold {
		flex: 1 1 auto;
		min-width: 0;
		font-size: 0.68rem;
		color: var(--fg-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	button {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		flex: none;
		font: inherit;
		font-size: 0.7rem;
		color: var(--fg-control);
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-pill);
		padding: 0.16rem 0.6rem;
		cursor: pointer;
	}

	button:hover {
		background: var(--surface-2);
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.empty {
		margin: 0;
		font-size: 0.76rem;
		color: var(--fg-faint);
	}

	.diag-list {
		list-style: none;
		margin: 0;
		padding: 0.3rem;
		overflow: auto;
		flex: 1 1 auto;
		min-height: 0;
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		font-size: 0.7rem;
		line-height: 1.6;
	}

	.diag-list li {
		display: flex;
		gap: 0.45rem;
		padding: 0.12rem 0.4rem;
		align-items: baseline;
	}

	.diag-list li.error {
		color: var(--danger);
	}

	.diag-list li.warn {
		color: var(--warning);
	}

	.at {
		font-family: var(--font-mono);
		font-size: 0.62rem;
		color: var(--fg-faint);
		font-variant-numeric: tabular-nums;
		flex: none;
		min-width: 3.2rem;
		text-align: right;
	}

	.stage,
	.who {
		font-size: 0.62rem;
		color: var(--fg-muted);
		flex: none;
		white-space: nowrap;
	}

	.who {
		padding: 0 0.3rem;
		border: 1px solid var(--hairline);
		border-radius: var(--r-pill);
	}

	/*
	 * 診断の 1 行の中身。
	 *
	 * **`+page.svelte` では `.what`（面の説明文）と名前が衝突していました**
	 * （Issue #22）—— 同じ名前が 2 か所で定義され、`flex: 1 1 auto` が
	 * 説明文にも効いて **250px の空白**を作っていました。
	 * 部品に出したので、**名前はこのファイルの中だけ**に閉じます。
	 */
	.detail {
		flex: 1 1 auto;
		min-width: 0;
		overflow-wrap: anywhere;
	}

	/* **どうすれば通るか。**握り潰さないための一言。 */
	.hint {
		color: var(--fg-muted);
		font-style: normal;
	}
</style>
