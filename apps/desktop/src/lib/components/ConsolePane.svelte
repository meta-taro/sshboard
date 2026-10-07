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
		/** いま打鍵が行く面か。**人が見ている面を 1 つに決めるため。** */
		active = false,
		/** 人が握っているか。**握っていなければ打たない**（往復させずに止める）。 */
		iHold = false,
		clipboard,
		platform,
		/** 検索の取っ手を親へ渡す。**閉じるときは `undefined`。** */
		onsearch = () => {},
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
		active?: boolean;
		iHold?: boolean;
		clipboard: ClipboardPorts;
		platform: TerminalPlatform;
		onsearch?: (id: number, search: TerminalSearch | undefined) => void;
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
<section class="pane" class:active data-console={pane.id}>
	<header>
		<span class="num">#{pane.id}</span>
		<span class="on" data-secret>{pane.connection}</span>
		<span class="holder" class:ai={pane.holder === 'ai'}>
			{pane.holder === 'ai' ? 'AI' : '人'}
		</span>
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

	header {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.2rem 0.45rem;
		border-bottom: 1px solid var(--hairline);
		font-size: 0.72rem;
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

	.holder {
		flex: 0 0 auto;
		padding: 0 0.3rem;
		border-radius: var(--r-control);
		border: 1px solid var(--hairline);
	}

	.holder.ai {
		border-color: var(--accent);
		color: var(--accent);
	}

	.screen {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
