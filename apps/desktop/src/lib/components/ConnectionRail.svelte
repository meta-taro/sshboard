<script lang="ts" module>
	/** 並びの 1 件。**接続の識別子と、人が読む名前と札だけ。** */
	export type RailItem = {
		id: string;
		name: string;
		tag?: string | null;
	};
</script>

<script lang="ts">
	/**
	 * **繋がっている接続の並びと、AI の柵。**
	 *
	 * ## なぜ部品にしたか
	 *
	 * 案（UI 案・2026-10-21）は、これを**端末の左に縦の列として常設**します。
	 * 実物はいま**横並びのタブ**です。
	 *
	 * **写して 2 つ置きません**（D39 —— 片方だけ直る日が来る）。
	 * 1 つの部品にして、`layout` で**横にも縦にも置けるように**します。
	 * 中身・文言・印の付け方は 1 か所です。
	 *
	 * ## 柵を一緒に持つ理由
	 *
	 * 案では列の底に柵のカードが付きます。**どの接続を見ているか**と
	 * **その接続で AI が書ける所**は、**並べて読むもの**です ——
	 * 離すと「いまどの接続の柵を見ているのか」が分からなくなります。
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	import Icon from './Icon.svelte';

	let {
		items,
		activeId,
		/** **AI が書いてよいディレクトリ**（D22）。**空 ＝ AI は読むだけ。** */
		roots = [],
		/** `row` は横並び（いまの形）、`rail` は縦の列（案の形）。 */
		layout = 'row',
		onpick,
		onadd
	}: {
		items: readonly RailItem[];
		activeId: string | null;
		roots?: readonly string[];
		layout?: 'row' | 'rail';
		onpick: (id: string) => void;
		onadd: () => void;
	} = $props();

	/**
	 * 柵のカードの見出し。
	 *
	 * **文言は増やしません**（`files.scope.some` を使い回す・11 言語ぶん訳が揃っている）。
	 * ただし案は**道を別の行に等幅で**出すので、見出しからは道を抜きます。
	 * `{roots}` が末尾に在ることは `messages.test.ts` が見張っています ——
	 * 崩れたら「AI が書けるのは:」のような尻切れが出ます。
	 */
	function fenceHead(): string {
		return i18n.t('files.scope.some', { roots: '' }).replace(/[\s:：,，]+$/u, '');
	}
</script>

<nav class="rail" class:vertical={layout === 'rail'} aria-label={i18n.t('tab.connections')}>
	<div class="picks" role="tablist" aria-label={i18n.t('tab.connections')}>
		{#each items as item (item.id)}
			<button
				type="button"
				role="tab"
				class="pick"
				class:active={item.id === activeId}
				aria-selected={item.id === activeId}
				onclick={() => onpick(item.id)}
			>
				<!-- **接続先は伏せられる所**（D26 / 禁止事項 4）。 -->
				<span data-secret>{item.name}</span>
				{#if item.tag}<span class="tag" data-secret>{item.tag}</span>{/if}
			</button>
		{/each}
	</div>

	<!--
		**別の接続を開く**（実機の指摘・2026-10-04）。
		ファイルの面と**同じ鍵の文言**を使います。
		**言葉を出します** —— アイコンだけだと、触れる前の人には存在しません
		（Issue #9 で一度やった失敗）。
	-->
	<button type="button" class="add" onclick={onadd} title={i18n.t('files.another')}>
		<Icon name="plus" size={13} />
		{i18n.t('files.another')}
	</button>

	<!--
		**AI の柵**（D22）。案は列の底に置きました。

		**既定は空＝AI は読むだけ。**それが普通の状態なので**警告には見せません。**
		色が付くのは柵が在るときだけ。
	-->
	<div class="fence" class:open={roots.length > 0}>
		<span class="fence-head">
			<Icon name="lock" size={11} />
			{roots.length > 0 ? fenceHead() : i18n.t('files.scope.none')}
		</span>
		{#each roots as root (root)}
			<span class="fence-root" data-secret>{root}</span>
		{/each}
		<!-- **人は制限されません**（D22）。書かないと自分が縛られていると読まれる。 -->
		<span class="fence-note">{i18n.t('files.scope.help')}</span>
	</div>
</nav>

<style>
	.rail {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
		flex: 0 0 auto;
		min-width: 0;
	}

	/*
	 * **縦の列**（案の形）。
	 *
	 * 横並びのときは柵が右端に流れますが、縦なら**底に落ちます** ——
	 * 案がそう置いているのは、**接続を選んでから柵を読む**順になるからです。
	 */
	.rail.vertical {
		flex-direction: column;
		align-items: stretch;
		flex-wrap: nowrap;
		gap: 0.3rem;
	}

	.picks {
		display: flex;
		align-items: center;
		gap: 0.2rem;
		flex-wrap: wrap;
		min-width: 0;
	}

	.rail.vertical .picks {
		flex-direction: column;
		align-items: stretch;
		flex-wrap: nowrap;
	}

	.pick {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font: inherit;
		font-size: var(--text-sm);
		color: var(--fg-control);
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--r-pill);
		padding: 0.2rem 0.6rem;
		cursor: pointer;
		min-width: 0;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease);
	}

	.rail.vertical .pick {
		justify-content: flex-start;
		border-radius: var(--r-control);
	}

	.pick:hover {
		background: var(--surface-2);
	}

	/*
	 * **選ばれている接続。**地と字を入れ替えます（UI 案）——
	 * 浮かせるだけだと**白黒で撮ったときに消えます。**
	 */
	.pick.active {
		color: var(--selected-fg);
		background: var(--selected);
		font-weight: 700;
	}

	.pick:focus-visible,
	.add:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.tag {
		font-size: var(--text-xs);
		padding: 0 0.3rem;
		border-radius: var(--r-pill);
		background: var(--shell);
		color: var(--fg-muted);
		flex: none;
	}

	.pick.active .tag {
		background: var(--ground);
		color: var(--fg-control);
	}

	.add {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		flex: none;
		font: inherit;
		font-size: var(--text-sm);
		color: var(--fg-control);
		background: transparent;
		border: 1px solid var(--hairline);
		border-radius: var(--r-pill);
		padding: 0.14rem 0.5rem;
		cursor: pointer;
	}

	.add:hover {
		background: var(--surface-2);
	}

	/* --- AI の柵（D22） --- */

	.fence {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 0 0 auto;
		padding: 0.5rem 0.6rem;
		border: 1px solid var(--hairline);
		border-radius: var(--r-core);
		background: var(--surface-2);
		font-size: var(--text-sm);
		min-width: 0;
	}

	/* **橙は AI の色。**「これは AI にかかっている制限」が色で読める。 */
	.fence.open {
		border-color: var(--ai);
		background: var(--ai-soft);
	}

	.fence-head {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-weight: 700;
		color: var(--fg-muted);
	}

	.fence.open .fence-head {
		color: var(--warning);
	}

	/* **道は等幅で、1 本ずつ別の行。**長い道が折り返して読めなくならないように。 */
	.fence-root {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--fg);
		overflow-wrap: anywhere;
	}

	.fence-note {
		color: var(--fg-dim);
		line-height: 1.5;
	}
</style>
