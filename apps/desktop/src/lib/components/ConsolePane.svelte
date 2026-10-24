<script lang="ts">
	/**
	 * **端末 1 枚。**番号で指す面（DESIGN.md「端末の割り方」・2026-10-06）。
	 *
	 * ## なぜ部品にしたか
	 *
	 * `+page.svelte` に 1 枚分の配線（xterm・追従・検索・クリップボード）が
	 * 全部入っていて、**2 枚にするには写すしかありませんでした。**
	 * 写した日に、片方だけ直る日が来ます（D39）。
	 *
	 * 部品にしたので、**端末の面を描いて確かめるテストが初めて書けます** ——
	 * 端末の面は 2026-09-01 から 3 日間 1 バイトも映しておらず、
	 * **型検査は 0 errors のまま**でした（Issue #10）。
	 *
	 * ## 打鍵は番号で送る
	 *
	 * **接続の名前で送ると、同じサーバに 2 枚開いた日に
	 * どちらへ打ったか言えません。**`console_type_into(consoleId, bytes)` です。
	 */
	import { invoke } from '@tauri-apps/api/core';

	import type { ConsolePane } from '$lib/console-panes';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { attachClipboard } from '$lib/terminal-clipboard';
	import type { ClipboardOptions, ClipboardPorts, TerminalPlatform } from '$lib/terminal-clipboard';
	import type { TerminalSearch } from '$lib/terminal-search';
	import { attachFit, attachSearch, createTerminal, writeChunk } from '$lib/terminal.svelte';
	import type { Terminal } from '@xterm/xterm';

	let {
		pane,
		/** 作った端に書き戻すもの（**見ていない間の分**・Issue #14 / #11）。 */
		backlog = [],
		fontPx = 12,
		/**
		 * **接続の印の色**（`red` などの名前。無ければ色を出しません）。
		 *
		 * **端末のための色をもう 1 つ作りません**（DESIGN.md）——
		 * 接続の一覧とファイルの面と同じ札です。
		 */
		mark = null,
		/**
		 * **この面を止める**（UI 案・2026-10-21）。
		 *
		 * 案は AI が握っている面の肩に［止める］を置きました。
		 * **人はいつでも取り返せる**（D29）という約束が、
		 * **その面の上に在ること**が要ります —— 別の所に在ると、
		 * 2 枚開いた日に「どちらを止めるのか」が言えません。
		 */
		onstop,
		/** いま打鍵が行く面か。**人が見ている面を 1 つに決めるため。** */
		active = false,
		/** 人が握っているか。**握っていなければ打たない**（往復させずに止める）。 */
		iHold = false,
		clipboard,
		platform,
		/** 検索の取っ手を親へ渡す。**閉じるときは `undefined`。** */
		onsearch = () => {},
		/**
		 * **流れてくる出力を書く口**を親へ渡す（**閉じるときは `undefined`**）。
		 *
		 * **これが無いと、開いたままの面には何も出ません。**
		 * 作ったときに控えを書き戻すだけでは、**マウント時の 1 回**しか描かれず、
		 * **そのあと流れてくる分が落ちます** —— タブを切り替えると出るのは、
		 * 作り直しのときに控えを書き戻しているからです。
		 *
		 * 2026-10-07 に実運用で踏みました。**AI が `rm` を打っても人の画面は静かなまま**で、
		 * 「人が常に見ている」（D29）が崩れていました。
		 */
		onready = () => {},
		/** この面が選ばれた。**親が「いま見ている面」を移します。** */
		onpick = () => {},
		onfailure = () => {},
		/**
		 * 検索の近道を、親が横取りするか。
		 *
		 * **`attachClipboard` の `handledElsewhere` と同じ形**にします ——
		 * 型を合わせずに包むと、押しても何も起きない日が来ます。
		 */
		findIsOpen = () => false
	}: {
		pane: ConsolePane;
		backlog?: number[];
		fontPx?: number;
		mark?: string | null;
		active?: boolean;
		iHold?: boolean;
		onstop?: (id: number) => void;
		clipboard: ClipboardPorts;
		platform: TerminalPlatform;
		onsearch?: (id: number, search: TerminalSearch | undefined) => void;
		onready?: (id: number, write: ((chunk: number[]) => void) | undefined) => void;
		onpick?: (id: number) => void;
		onfailure?: (message: string) => void;
		findIsOpen?: ClipboardOptions['handledElsewhere'];
	} = $props();

	let host: HTMLDivElement | undefined = $state();
	let term = $state<Terminal | undefined>();
	/** 外すもの。**溜めっぱなしにすると監視が二重に走る。** */
	let detach: Array<() => void> = [];

	/**
	 * 端末は**要素が現れてから**作る。
	 *
	 * `onMount` で作ろうとすると、そのときのタブが別の面なので貼る先がまだ無く、
	 * **一度も作られません**（実際にそうなっていました・2026-09-01）。
	 *
	 * 貼り直しでは戻らなかったので（実測）、要素が入れ替わったら作り直します。
	 * 表示は消えますが、**シェルは実行体の側で生き続けます。**
	 */
	$effect(() => {
		const at = host;
		if (!at) return;
		if (!term) {
			// **打てる面。**握っていないときは実行体が断るので、
			// ここで打てること自体は塞ぎません（断り方で伝える）。
			term = createTerminal(at, fontPx, true);
			// **作った端に書き戻す**（Issue #14 / #11）。
			writeChunk(term, backlog);
			term.onData((data) => {
				// **握っていなければ打たない。**往復させて断られるより、画面で止める方が速い。
				if (!iHold) return;
				const bytes = Array.from(new TextEncoder().encode(data));
				// **番号で送る。**接続の名前だと、2 枚開いた日にどちらか言えません。
				invoke('console_type_into', { consoleId: pane.id, bytes }).catch((error: unknown) => {
					onfailure(String(error));
				});
			});
			term.onResize(({ cols, rows }) => {
				invoke('console_resize', { cols, rows }).catch(() => {
					/* まだ開いていないだけ。**開いてから効く。** */
				});
			});
			// **窓に追従させる。**無いと 80×24 で固定され、上の `onResize` も一度も出ません。
			detach.push(attachFit(term, at));
			onsearch(
				pane.id,
				attachSearch(term, (error: unknown) => onfailure(String(error)))
			);
			// **流れてくる分を書く口を渡す。**作った端に書き戻すだけでは、
			// **マウント時の 1 回**しか描かれません。
			const live = term;
			onready(pane.id, (chunk) => writeChunk(live, chunk));
			// **なぞるだけでコピー**。右クリックで貼り付け（PuTTY / TeraTerm の形）。
			// **素の Ctrl+C は横取りしません**（走っているものを止められなくなるため）。
			detach.push(
				attachClipboard(term, clipboard, platform, {
					handledElsewhere: findIsOpen,
					host: at
				})
			);
		} else if (!at.contains(term.element ?? null)) {
			detach.forEach((off) => off());
			detach = [];
			onsearch(pane.id, undefined);
			onready(pane.id, undefined);
			term.dispose();
			term = undefined;
		}
	});

	// **字の大きさを揃える。**xterm.js は自前で描くので `rem` が効きません。
	$effect(() => {
		if (term) term.options.fontSize = fontPx;
	});
</script>

<!--
	**番号・題・色を出す**（人が決めた形・2026-10-06）——

	> 番号とかタイトルとかいろとかできると尚良い w

	番号は実行体が振るもの。**人が口で「②を見て」と言えて、AI も同じ番号で指せます**
	（PRD §4-0 の「同じ視点」）。
-->
<section
	class="pane"
	class:active
	class:mine={pane.holder === 'human'}
	class:ai={pane.holder === 'ai'}
	data-console={pane.id}
	style:--mark={mark ? `var(--mark-${mark})` : 'transparent'}
>
	<header>
		<!-- **色は帯で出します。**字の色にすると、読みにくい組み合わせが出ます（D57）。 -->
		<span class="mark-bar" aria-hidden="true"></span>
		<span class="num">#{pane.id}</span>
		<span class="on" data-secret>{pane.connection}</span>
		<!--
			**誰が握っているかを、面の肩に書く**（UI 案・2026-10-21）。
			以前はここに `'AI'` / `'人'` が**日本語のまま直書き**されていました
			（画面は 11 言語あります）。文言は `console.held.*` に既に在ります。
		-->
		<span class="holder" class:ai={pane.holder === 'ai'}>
			<span class="dot" aria-hidden="true"></span>
			{pane.holder === 'ai' ? i18n.t('console.held.ai') : i18n.t('console.held.me')}
		</span>
		{#if pane.holder === 'ai' && onstop}
			<!--
				**止まらない停止ボタンは、無い方がまし。**
				`console_stop_of` は面の番号で止めるので、**2 枚開いていても
				どちらを止めたのかが言えます。**
			-->
			<button type="button" class="stop" onclick={() => onstop(pane.id)}>
				{i18n.t('console.stop')}
			</button>
		{/if}
	</header>
	<!--
		**触ったら、この面へ移る。**打鍵の行き先を、人が見ている面に合わせます。
		`focusin` も拾います —— キーボードで入る人をマウス前提で落とさないため。
	-->
	<div
		class="screen"
		bind:this={host}
		role="presentation"
		onmousedown={() => onpick(pane.id)}
		onfocusin={() => onpick(pane.id)}
	></div>
</section>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1 1 0;
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		overflow: hidden;
	}

	/* **いま打鍵が行く面を、はっきり分ける。**
	   どちらに打っているか分からないのがいちばん危ない（`rm` の相手を取り違える）。 */
	.pane.active {
		border-color: var(--accent);
	}

	/*
	 * **面の枠を、握っている人の色にする**（UI 案・2026-10-21）。
	 *
	 * 案は人の面を青、AI の面を橙で縁取ります。**2 枚並んだとき、
	 * どちらが AI のものかを枠だけで言える**ようにするためです。
	 *
	 * `active`（いま打鍵が行く面）とは別の合図です ——
	 * `active` は「次に打つ先」、こちらは「誰のものか」。
	 * **両方同時に要ります**（人の面が 2 枚のときもある）。
	 */
	.pane.mine {
		border-color: var(--human);
	}

	.pane.ai {
		border-color: var(--ai);
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.2rem 0.45rem;
		border-bottom: 1px solid var(--hairline);
		font-size: var(--text-sm);
	}

	/* **印の色。**無ければ透明で、場所だけ残します（並びが動かないように）。 */
	.mark-bar {
		flex: 0 0 auto;
		width: 3px;
		align-self: stretch;
		border-radius: var(--r-pill);
		background: var(--mark, transparent);
	}

	.num {
		font-variant-numeric: tabular-nums;
		font-weight: 600;
		color: var(--accent);
	}

	.on {
		flex: 1 1 auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/*
	 * **誰が握っているか。**丸い印と文字を並べます。
	 *
	 * **色だけに頼りません** —— 案の人（青）と AI（橙）は明るさがほぼ同じで、
	 * 色が見えない人には見分けられないので、**文字が本体**です。
	 * 印は「同じ仲間か」を一目で言うための添え物。
	 */
	.holder {
		display: inline-flex;
		align-items: center;
		gap: 0.26rem;
		flex: none;
		color: var(--human);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 14rem;
	}

	.holder.ai {
		color: var(--ai);
	}

	.holder .dot {
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background: currentColor;
		flex: none;
	}

	/* **この面を止める。**肩に置くので、2 枚開いていても相手を取り違えません。 */
	.stop {
		flex: none;
		font: inherit;
		font-size: var(--text-sm);
		color: var(--danger);
		background: var(--danger-soft);
		border: 1px solid var(--danger);
		border-radius: var(--r-pill);
		padding: 0.06rem 0.44rem;
		cursor: pointer;
	}

	.stop:hover {
		background: color-mix(in srgb, var(--danger) 22%, transparent);
	}

	.stop:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.holder {
		flex: 0 0 auto;
		padding: 0 0.3rem;
		border-radius: var(--r-control);
		border: 1px solid var(--hairline);
	}

	/*
	 * **AI は橙**（UI 案・2026-10-21）。
	 *
	 * ここは `--accent` でした。フェーズ A で `--accent` を**人の青**にしたので、
	 * **AI の名乗りが青くなっていました** —— 案とちょうど逆です。
	 * 「人と AI を色で見分ける」が案の要点なのに、取り違えていました。
	 */
	.holder.ai {
		border-color: var(--ai);
		color: var(--ai);
	}

	.screen {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
