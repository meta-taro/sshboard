<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import ConnectionManager from '$lib/components/ConnectionManager.svelte';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { session } from '$lib/session.svelte';
	import { menuLabels } from '$lib/i18n/messages-menu';
	import { titlebar } from '$lib/window/titlebar.svelte';
	import AboutDialog from '$lib/components/AboutDialog.svelte';
	import { updater } from '$lib/update/updater.svelte';
	import { listenForPageShot } from '$lib/capture/page-shot';
	import { LOCALES } from '$lib/i18n/locales';
	import { textSize } from '$lib/text-size/text-size.svelte';
	import { theme, type ThemeMode } from '$lib/theme/theme.svelte';
	import { attachFit, attachSearch, createTerminal, writeChunk } from '$lib/terminal.svelte';
	import { attachClipboard, browserClipboard, detectPlatform } from '$lib/terminal-clipboard';
	import { isFindShortcut, type TerminalSearch } from '$lib/terminal-search';
	import '@xterm/xterm/css/xterm.css';
	import type { Terminal } from '@xterm/xterm';

	type McpAccess = { url: string; token: string };
	type McpFailure = { port: number; detail: string };

	/** 何が起きたかの記録。Rust 側の `Event` と対。**接続先は入っていない。** */
	type DiagEvent = {
		seq: number;
		atMs: number;
		level: 'info' | 'warn' | 'error';
		stage: string;
		connection?: string;
		message: string;
		hint?: string;
	};

	let lines = $state<BandLine[]>([]);
	let mcp = $state<McpAccess | null>(null);
	/** **立ち上がらなかったこと。**出さないと「起動中」のまま止まって見える。 */
	let mcpFailure = $state<McpFailure | null>(null);

	/**
	 * 「表示」を開いているか。
	 *
	 * **自前タイトルバー（D17）にしたので、Windows では OS のメニューが消えました。**
	 * 文字サイズは右上のボタンに残っていましたが、**そこが見つからなかった**という
	 * 報告が実機から 2 回出ています（MCP のコピーボタンでも同じことが起きた）。
	 * **失った「表示」メニューを、自分の帯の中に作り直します。**
	 */
	let viewMenuOpen = $state(false);

	/**
	 * 「sshboard について」を開いているか。
	 *
	 * **版を確かめる場所はここ 1 つ**です。自前タイトルバー（D17）で
	 * Windows から OS のメニューが消え、確かめる場所が無くなっていました。
	 * **「表示」や「診断」へ散らしたのは誤り**で、普通に探すのは
	 * 「〜について」／ヘルプです（dbboard も同じ形）。
	 */
	let aboutOpen = $state(false);

	/** メニューの文言は**既にある**（`messages-menu`）。新しい言い方を増やさない。 */
	const menu = $derived(menuLabels(i18n.locale));
	let mcpCopied = $state(false);
	let failure = $state<string | null>(null);
	let streaming = $state(false);
	/** **既定は接続。**繋がないとファイルも端末も空です。 */
	let view = $state<'connections' | 'files' | 'console' | 'band' | 'diag'>('connections');

	// --- 端末（D29）------------------------------------------------------------
	// **同時に触れるのは 1 人。**AI が握っている間、人の入力は締まる。
	let consoleHost: HTMLDivElement | undefined = $state();
	// **`$state` にする。**そうしないと、文字サイズの効果が
	// 「端末ができたこと」を追えず、片方だけ取り残される（実際に取り残された）。
	let consoleTerm = $state<Terminal | undefined>();
	/** 誰が握っているか。`null` は誰も握っていない。 */
	let holder = $state<'human' | 'ai' | null>(null);
	/** **どの接続の端末か。**タブを移しても端末は付いてこない（D25）。 */
	let consoleOn = $state<string | null>(null);
	const iHold = $derived(holder === 'human');
	let diag = $state<DiagEvent[]>([]);

	// --- コピー & ペースト --------------------------------------------------------
	// **なぞるだけでコピー**は、端末では当たり前の挙動。無いと毎回ためらう。
	// ショートカットは `terminal-clipboard.ts` を見ること（**Ctrl+C は横取りしない**）。
	const platform = detectPlatform(
		// `navigator.platform` は古い口だが、**要るのは「⌘ があるか」だけ。**
		typeof navigator === 'undefined' ? '' : navigator.platform
	);
	const clipboard = browserClipboard((error: unknown) => {
		// 押したのに何も起きない、を作らない。**断られたら画面に出す。**
		failure = String(error);
	});
	/** 端末を作り直すときに外すもの。**溜めっぱなしにすると監視が二重に走る。** */
	let detachConsole: Array<() => void> = [];
	let detachOutput: Array<() => void> = [];

	// --- 検索 --------------------------------------------------------------------
	// `⌘F` / `Ctrl+Shift+F`。**素の `Ctrl+F` は端末の「1 文字進む」なので取りません。**
	type SearchPane = 'console' | 'output';
	let searchOpen = $state<SearchPane | null>(null);
	let searchTerm = $state('');
	/** 打った語が無かったか。**「押したのに何も起きない」を作らないため。** */
	let searchMissed = $state(false);
	let searchInput: HTMLInputElement | undefined = $state();
	let consoleSearch: TerminalSearch | undefined;
	let outputSearch: TerminalSearch | undefined;

	function searchFor(pane: SearchPane): TerminalSearch | undefined {
		return pane === 'console' ? consoleSearch : outputSearch;
	}

	function openSearch(pane: SearchPane) {
		searchOpen = pane;
		searchMissed = false;
		// 開いた先に入力があるので、**そこへ手を渡す。**開いて空振りさせない。
		tick().then(() => searchInput?.focus());
	}

	function closeSearch() {
		// **強調を消してから閉じる。**残ると、いま何を見ているのか分からなくなる。
		if (searchOpen) searchFor(searchOpen)?.close();
		searchOpen = null;
		searchMissed = false;
	}

	function runSearch(backwards = false) {
		if (!searchOpen) return;
		const search = searchFor(searchOpen);
		if (!search) return;
		const found = backwards ? search.previous(searchTerm) : search.next(searchTerm);
		// 空の語は「見つからない」ではなく「まだ何も打っていない」。**区別する。**
		searchMissed = !found && searchTerm.trim() !== '';
	}

	function onSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			closeSearch();
			event.preventDefault();
		} else if (event.key === 'Enter') {
			// Shift で戻る。**ブラウザの検索と同じ手つき。**
			runSearch(event.shiftKey);
			event.preventDefault();
		}
	}

	/** 端末のキー処理は 1 本しか付かないので、検索はここから割り込む。 */
	function findFrom(pane: SearchPane) {
		return (event: { type: string; key: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }) => {
			if (!isFindShortcut(event, platform)) return false;
			openSearch(pane);
			return true;
		};
	}

	/** 記録を取り直す。**押したときと、診断を開いたときに読む。** */
	async function loadDiagnostics() {
		try {
			diag = await invoke<DiagEvent[]>('diagnostics_recent', { limit: 200 });
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	// 診断を開いたら、その場で読む。**古い記録を見せない。**
	$effect(() => {
		if (view === 'diag') loadDiagnostics();
	});

	/**
	 * エージェントへ貼る 1 行。
	 *
	 * **合言葉は起動ごとに変わる**（D23）ので、覚えて書き写す形にしない。
	 * ここを写して貼るだけで済むようにする。
	 */
	const mcpCommand = $derived(
		mcp
			? `claude mcp add --transport http sshboard ${mcp.url} --header "Authorization: Bearer ${mcp.token}"`
			: ''
	);

	async function copyMcpCommand() {
		if (!mcpCommand) return;
		try {
			await navigator.clipboard.writeText(mcpCommand);
			mcpCopied = true;
			setTimeout(() => (mcpCopied = false), 2000);
		} catch (error: unknown) {
			// 写せないことを黙らない。**押したのに何も起きない**が一番困る。
			failure = String(error);
		}
	}

	let terminalHost: HTMLDivElement | undefined = $state();
	let terminal = $state<Terminal | undefined>();

	// **端末の字も一緒に変える。**xterm.js は自前で描くので `rem` が効かない。
	// 画面だけ大きくなって端末が小さいままだと、同じ 1 つの道具に見えない。
	/**
	 * 端末は**要素が現れてから**作る。
	 *
	 * `onMount` で作ろうとすると、そのときのタブが「ファイル」なので
	 * 貼る先がまだ無く、**一度も作られません**（実際にそうなっていて、
	 * 帯と出力の面は何も映していませんでした・2026-09-01）。
	 *
	 * タブを行き来すると要素が作り直されるので、**同じ端末を貼り直します。**
	 * 作り直すと、それまでの表示が消えます。
	 */
	$effect(() => {
		const host = consoleHost;
		if (!host) return;
		if (!consoleTerm) {
			// **打てる面。**握っていないときは Rust 側が断るので、
			// ここで打てること自体は塞がない（断り方で伝える）。
			consoleTerm = createTerminal(host, textSize.terminalPx, true);
			consoleTerm.onData((data) => {
				// **握っていなければ打たない。**往復させて断られるより、
				// 画面で止める方が速い（Rust 側でも同じ判断をしている）。
				if (!iHold) return;
				const bytes = Array.from(new TextEncoder().encode(data));
				invoke('console_type', { bytes }).catch((error: unknown) => {
					failure = String(error);
				});
			});
			consoleTerm.onResize(({ cols, rows }) => {
				invoke('console_resize', { cols, rows }).catch(() => {
					/* まだ開いていないだけ。**開いてから効く。** */
				});
			});
			// **窓に追従させる。**無いと 80×24 で固定され、上の `onResize` も一度も出ない。
			detachConsole.push(attachFit(consoleTerm, host));
			// **なぞるだけでコピー**。ショートカットは ⌘C / Ctrl+Shift+C / Ctrl+Shift+V。
			// **素の Ctrl+C は横取りしません**（走っているものを止められなくなるため）。
			consoleSearch = attachSearch(consoleTerm, (error: unknown) => {
				failure = String(error);
			});
			detachConsole.push(
				attachClipboard(consoleTerm, clipboard, platform, {
					handledElsewhere: findFrom('console')
				})
			);
		} else if (!host.contains(consoleTerm.element ?? null)) {
			// **貼り直しでは戻らなかった**（実測）。作り直す。
			// 表示は消えますが、**シェルは Engine 側で生き続けます。**
			detachConsole.forEach((detach) => detach());
			detachConsole = [];
			consoleTerm.dispose();
			consoleTerm = undefined;
		}
	});

	$effect(() => {
		const host = terminalHost;
		if (!host) return;
		if (!terminal) {
			terminal = createTerminal(host, textSize.terminalPx);
			detachOutput.push(attachFit(terminal, host));
			// **見るだけの面でも、なぞればコピーできます。**ログを拾うのはここ。
			// 貼り付けは付けません（`disableStdin` の面から文字が出ると嘘になる）。
			outputSearch = attachSearch(terminal, (error: unknown) => {
				failure = String(error);
			});
			detachOutput.push(
				attachClipboard(terminal, clipboard, platform, {
					allowPaste: false,
					handledElsewhere: findFrom('output')
				})
			);
		} else if (!host.contains(terminal.element ?? null)) {
			detachOutput.forEach((detach) => detach());
			detachOutput = [];
			terminal.dispose();
			terminal = undefined;
		}
	});

	$effect(() => {
		const px = textSize.terminalPx;
		if (terminal) terminal.options.fontSize = px;
		// **端末タブにも効かせる。**片方だけ変わると、同じ 1 つの道具に見えない
		// （実際に端末タブだけ取り残されていた）。
		if (consoleTerm) consoleTerm.options.fontSize = px;
	});

	/**
	 * `⌘ +` / `⌘ −` / `⌘ 0`。**この操作は覚えなくても手が知っている。**
	 *
	 * WebView 側の既定の拡大は効かない（アプリなので）。
	 * 押しても何も起きないより、**期待どおりに動く方**へ寄せる。
	 */
	function onKeydown(event: KeyboardEvent) {
		if (!(event.metaKey || event.ctrlKey)) return;
		// `+` は Shift 付きだったり `=` だったりする。**両方受ける。**
		if (event.key === '+' || event.key === '=' || event.key === ';') {
			textSize.step(1);
		} else if (event.key === '-' || event.key === '_') {
			textSize.step(-1);
		} else if (event.key === '0') {
			textSize.set('normal');
		} else {
			return;
		}
		event.preventDefault();
	}

	/** 追いたいログのパス。**サーバー側**（右のペインで見ているのと同じ場所）。 */
	let followPath = $state('/var/log/messages');

	/**
	 * サーバーのログを追う（`tail -f`）。
	 *
	 * **GUI へは色付き・MCP へは素**で流れます（Issue 005）。
	 * コマンドは Rust 側が組み立てるので、**任意の文字列はシェルへ渡りません**（D3）。
	 */
	/**
	 * 端末を開いて握る（D29）。
	 *
	 * **打鍵は帯へ載せません。**1 キーずつ載せると帯が溢れ、
	 * 受け取り待ちを挟むと端末が使い物になりません。開始と終了は載ります。
	 */
	async function openConsole() {
		if (!consoleTerm) return;
		try {
			await invoke('console_open', { cols: consoleTerm.cols, rows: consoleTerm.rows });
			holder = 'human';
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 握りを取り返す。**人は常に勝ちます**（D29）。 */
	async function takeConsole() {
		try {
			await invoke('console_take');
			holder = 'human';
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 止める。**失敗しません**（D29 の停止ボタン）。 */
	async function stopConsole() {
		try {
			await invoke('console_stop');
			holder = null;
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	async function follow() {
		const path = followPath.trim();
		if (!path) return;
		try {
			await invoke('stream_follow', { path, lines: null });
			streaming = true;
		} catch (error: unknown) {
			failure = i18n.t('err.stream.start', { detail: String(error) });
		}
	}

	async function stopStream() {
		try {
			await invoke('stop_stream');
			streaming = false;
		} catch (error: unknown) {
			failure = i18n.t('err.stream.stop', { detail: String(error) });
		}
	}

	/**
	 * 「帯へ入れた」と返す。
	 *
	 * **描画（requestAnimationFrame）を待たないこと。**
	 * WKWebView はウィンドウが前面に無いと rAF を止める。この製品は
	 * 「人はエディタを見ていて、AI が裏で動く」使い方が普通なので、
	 * 描画を待つ作りにすると背面にした瞬間に MCP が全部失敗する。
	 *
	 * 待つのは DOM に入るところまで（`tick()`）。そこまで来ていれば、
	 * 人がウィンドウを見た瞬間にその行がある。**応答が先に返ることは無い。**
	 */
	function ackRendered(seq: number) {
		invoke('band_ack', { seq }).catch((error: unknown) => {
			failure = i18n.t('err.ack', { detail: String(error) });
		});
	}

	onMount(() => {
		// **この帯が OS のタイトルバー**（D17）。最大化の状態を取り込む。
		titlebar.init();

		// **画面を撮る受け口**（D36）。OS の画面収録の許可が要らない方の道。
		listenForPageShot()
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 受け口が開けなくても、OS のキャプチャへ落ちる */
			});
		// **起動時に 1 回だけ。**定期的に叩きません（D34）。
		updater.check();

		const stops: Array<() => void> = [];

		type ConsoleState = { holder: 'human' | 'ai' | null; connection: string | null };
		invoke<ConsoleState>('console_holder')
			.then((state) => {
				holder = state.holder;
				consoleOn = state.connection;
			})
			.catch(() => {
				/* 取れなくても画面は出す */
			});

		// **AI が握った瞬間に、人の側の入力が締まる**（D29）。
		listen<ConsoleState>('console://holder', (event) => {
			holder = event.payload.holder;
			consoleOn = event.payload.connection;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});


		// **ANSI を落とさずに渡す。**色は人の側にだけ残す（Issue 005）。
		listen<number[]>('stream://raw', (event) => {
			if (terminal) writeChunk(terminal, event.payload);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe.stream', { detail: String(error) });
			});

		listen<BandLine>('band://line', async (event) => {
			lines = appendLine(lines, event.payload);
			await tick();
			ackRendered(event.payload.seq);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe', { detail: String(error) });
			});

		// OS のメニュー（表示 > 文字を大きく）から。**大きさを持っているのは画面側。**
		listen<'larger' | 'smaller' | 'reset'>('menu://text-size', (event) => {
			if (event.payload === 'reset') textSize.set('normal');
			else textSize.step(event.payload === 'larger' ? 1 : -1);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe', { detail: String(error) });
			});

		listen<McpAccess>('mcp://ready', (event) => {
			mcp = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 下の取り直しで拾う */
			});

		// **立たなかったときも受け取る。**番号を固定した分、ぶつかる場面が増える。
		listen<McpFailure>('mcp://failed', (event) => {
			mcpFailure = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 失敗の通知が届かないこと自体は、下の帯で気づける */
			});

		// 起動が速いと 'mcp://ready' を購読より先に取りこぼす。取り直す。
		invoke<McpAccess | null>('mcp_url')
			.then((access) => {
				if (access) mcp = access;
			})
			.catch((error: unknown) => {
				failure = i18n.t('err.mcp', { detail: String(error) });
			});

		return () => {
			stops.forEach((stop) => stop());
			terminal?.dispose();
			consoleTerm?.dispose();
		};
	});
</script>

<svelte:window onkeydown={onKeydown} />

<!--
	端末の検索バー。**2 つの面で同じものを出す**ので 1 か所に書く。
	出したままにしない — 閉じると強調も消える（`closeSearch`）。
-->
{#snippet searchBar(pane: SearchPane)}
	{#if searchOpen === pane}
		<div class="search-bar">
			<input
				bind:this={searchInput}
				bind:value={searchTerm}
				oninput={() => runSearch()}
				onkeydown={onSearchKeydown}
				placeholder={i18n.t('search.placeholder')}
				aria-label={i18n.t('search.placeholder')}
				spellcheck="false"
			/>
			<button type="button" onclick={() => runSearch(true)}>{i18n.t('search.prev')}</button>
			<button type="button" onclick={() => runSearch()}>{i18n.t('search.next')}</button>
			<!-- **無かったことを言う。**黙って何も起きないと、壊れたのかと思う。 -->
			{#if searchMissed}
				<span class="missed" role="status">{i18n.t('search.none')}</span>
			{/if}
			<button type="button" onclick={closeSearch}>{i18n.t('search.close')}</button>
		</div>
	{/if}
{/snippet}

<main>
	<!-- **上の帯は 1 行に収める。**6 段積むと、道具として使う面積が消える。 -->
	<!--
		**この帯が OS のタイトルバーです**（`decorations: false`・D17）。
		標準の帯は配色がテーマに追従せず、暗い画面の上に明るい帯が乗ります。

		帯そのものを掴んで動かせるようにし（`data-tauri-drag-region`）、
		中のボタン類は各自が押せるままにします。
	-->
	<header data-tauri-drag-region>
		<!--
			**製品名だけ。**説明文は外しました。

			「1 本の SSH を、人と AI で共有します。」は、**毎日 8 時間見る画面**に
			常時出しておく文ではありません（実機で「要る？」と言われた）。
			同じ説明は「〜について」と README にあります。

			**この要素自体を掴める領域にします。**`pointer-events: none` で
			下へ落とす手もありますが、それだと吹き出しが出なくなります。
		-->
		<span class="phase" data-tauri-drag-region title={i18n.t('app.driven')}>sshboard</span>

		<!-- **ボタンとボタンの隙間でも掴める。**
		     付けないと、並びの余白は「掴めない帯」になります。 -->
		<nav class="tabs" data-tauri-drag-region>
			<!--
				**接続が 1 番目。**繋がないとファイルも端末も空です。
				以前はファイルが先頭で、**「接続マネージャーが 2 番目なのはおかしい」**
				と実機で言われました（2026-09-03）。そのとおりです。
			-->
			<button
				type="button"
				class:active={view === 'connections'}
				onclick={() => (view = 'connections')}
			>
				<Icon name="server" />
				{i18n.t('tab.connections')}
			</button>
			<button type="button" class:active={view === 'files'} onclick={() => (view = 'files')}>
				<Icon name="folder" />
				{i18n.t('tab.files')}
			</button>
			<button type="button" class:active={view === 'console'} onclick={() => (view = 'console')}>
				<Icon name="terminal" />
				{i18n.t('tab.console')}
			</button>
			<button type="button" class:active={view === 'band'} onclick={() => (view = 'band')}>
				<Icon name="activity" />
				{i18n.t('tab.band')}
			</button>
			<button type="button" class:active={view === 'diag'} onclick={() => (view = 'diag')}>
				<Icon name="warning" />
				{i18n.t('tab.diag')}
			</button>
		</nav>

		<!--
			**「表示」。**自前タイトルバー（D17）にした結果、
			Windows では OS のメニューが消えました。文字サイズは右上に出ていましたが、
			**「変更できるようにしたい」と実機から言われた** — つまり見つからなかった。
			**失ったメニューを、自分の帯の中に作り直します。**
			文言は `messages-menu` に既にあるものを使い、言い方を増やしません。
		-->
		<!-- **「〜について」はここ。**版を確かめる場所を 1 つに決めます（dbboard と同じ形）。 -->
		<button
			type="button"
			class="about-open"
			onclick={() => (aboutOpen = true)}
			title={i18n.t('about.title')}
			aria-label={i18n.t('about.title')}
		>
			?
		</button>

		<div class="view-menu" data-tauri-drag-region>
			<button
				type="button"
				class="view-trigger"
				aria-haspopup="true"
				aria-expanded={viewMenuOpen}
				onclick={() => (viewMenuOpen = !viewMenuOpen)}
			>
				{menu['menu.view']}
			</button>

			{#if viewMenuOpen}
				<!-- **外を押したら閉じる。**開きっぱなしは邪魔。 -->
				<div class="view-backdrop" role="presentation" onclick={() => (viewMenuOpen = false)}></div>
				<div class="view-panel" role="group" aria-label={menu['menu.view']}>
					<div class="view-row">
						<span class="view-label">{i18n.t('text.label')}</span>
						<div class="view-controls">
							<button
								type="button"
								class="icon-only text-step"
								onclick={() => textSize.step(-1)}
								disabled={textSize.atSmallest}
								title={menu['menu.textSmaller']}
								aria-label={menu['menu.textSmaller']}
							>
								<span class="smaller-a">A</span>
							</button>
							<span class="text-now">{i18n.t(`text.${textSize.mode}`)}</span>
							<button
								type="button"
								class="icon-only text-step"
								onclick={() => textSize.step(1)}
								disabled={textSize.atLargest}
								title={menu['menu.textLarger']}
								aria-label={menu['menu.textLarger']}
							>
								<span class="larger-a">A</span>
							</button>
						</div>
					</div>

					<div class="view-row">
						<span class="view-label">{i18n.t('theme.label')}</span>
						<div class="view-controls">
							{#each ['auto', 'light', 'dark'] as const as mode (mode)}
								<button
									type="button"
									class="icon-only"
									class:active={theme.mode === mode}
									title={i18n.t(`theme.${mode}`)}
									aria-label={`${i18n.t('theme.label')}: ${i18n.t(`theme.${mode}`)}`}
									onclick={() => theme.set(mode as ThemeMode)}
								>
									<Icon name={mode === 'auto' ? 'contrast' : mode === 'light' ? 'sun' : 'moon'} />
								</button>
							{/each}
						</div>
					</div>

					<div class="view-row">
						<span class="view-label"><Icon name="globe" size={13} /></span>
						<select
							value={i18n.locale}
							onchange={(event) => i18n.set((event.currentTarget as HTMLSelectElement).value)}
							aria-label="Language"
						>
							{#each LOCALES as locale (locale.code)}
								<option value={locale.code}>{locale.native}</option>
							{/each}
						</select>
					</div>

				</div>
			{/if}
		</div>

		<!-- **合言葉ごと写せる形にする**（D23）。 -->
		<button
			type="button"
			class="mcp"
			class:failed={mcpFailure !== null}
			onclick={copyMcpCommand}
			disabled={!mcp}
			title={mcpFailure
				? i18n.t('mcp.failed.help', { port: String(mcpFailure.port), detail: mcpFailure.detail })
				: mcp
					? i18n.t('mcp.token.help')
					: 'MCP'}
		>
			<Icon name={mcpFailure ? 'warning' : mcpCopied ? 'check' : 'copy'} size={12} />
			{#if mcpFailure}{i18n.t('mcp.failed', { port: String(mcpFailure.port) })}{:else if !mcp}{i18n.t(
					'mcp.starting'
				)}{:else if mcpCopied}{i18n.t('mcp.copied')}{:else}{i18n.t('mcp.copy')}{/if}
		</button>

		<!--
			窓の操作。**Windows は右上、macOS も同じ並びに揃えます**（house style・D17）。
			`decorations: false` にすると macOS の信号機ボタンも消えるため、自前で出します。

			**Windows 11 の Snap Layouts は落ちます**（D17 で了承済み）。
			Tauri 2 に Electron の `titleBarOverlay` 相当が無く、`WM_NCHITTEST` を
			書くコストが今の段階では高すぎるため。**製品間で挙動が揃うこと自体は利点。**
		-->
		<div class="window-controls">
			<button
				type="button"
				class="wc"
				onclick={() => titlebar.minimize()}
				title={i18n.t('win.minimize')}
				aria-label={i18n.t('win.minimize')}
			>
				─
			</button>
			<button
				type="button"
				class="wc"
				onclick={() => titlebar.toggleMaximize()}
				title={titlebar.isMaximized ? i18n.t('win.restore') : i18n.t('win.maximize')}
				aria-label={titlebar.isMaximized ? i18n.t('win.restore') : i18n.t('win.maximize')}
			>
				{titlebar.maximizeGlyph}
			</button>
			<button
				type="button"
				class="wc close"
				onclick={() => titlebar.close()}
				title={i18n.t('win.close')}
				aria-label={i18n.t('win.close')}
			>
				✕
			</button>
		</div>
	</header>

	<!--
		更新の知らせ（D34）。**黙って入れ替えません。**押すのは人。
		「何も無い」「調べている」は出しません — **静かなときは静かでいる。**

		**本文の流れに埋めない。**細い帯として並べていたら、
		**そこにあると気づかれませんでした**（実機・2026-09-03）。
		画面の隅へ浮かせ、影を付けて、他と違う面として見せます。
		ただし**画面を塞ぐ窓にはしません** — 作業中に手を止めさせる話ではないので。
	-->
	{#if updater.state.kind !== 'idle' && updater.state.kind !== 'checking' && updater.state.kind !== 'none'}
		<aside class="update-toast" role="status" aria-live="polite">
			{#if updater.state.kind === 'found'}
				<span class="toast-icon"><Icon name="download" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.found', { version: updater.state.version })}</strong>
					<div class="toast-actions">
						<button type="button" class="cta" onclick={() => updater.install()}>
							{i18n.t('update.install')}
						</button>
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
					</div>
				</div>
			{:else if updater.state.kind === 'downloading'}
				<span class="toast-icon"><Icon name="download" size={16} /></span>
				<div class="toast-body">
					<strong>
						{i18n.t('update.downloading', { version: updater.state.version })}
						{#if updater.state.percent !== null}&nbsp;{updater.state.percent}%{/if}
					</strong>
					{#if updater.state.percent !== null}
						<!-- **進み具合を出す。**落としている間、止まって見えないように。 -->
						<div
							class="toast-bar"
							role="progressbar"
							aria-valuenow={updater.state.percent}
							aria-valuemin="0"
							aria-valuemax="100"
						>
							<span style:width="{updater.state.percent}%"></span>
						</div>
					{/if}
				</div>
			{:else if updater.state.kind === 'ready'}
				<span class="toast-icon"><Icon name="check" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.ready', { version: updater.state.version })}</strong>
					<div class="toast-actions">
						<button type="button" class="cta" onclick={() => updater.restart()}>
							{i18n.t('update.restart')}
						</button>
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
					</div>
				</div>
			{:else if updater.state.kind === 'failed'}
				<!-- **黙らない。**繋がらないのか署名が合わないのかで、人の次の一手が違う。 -->
				<span class="toast-icon warn"><Icon name="warning" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.failed', { detail: updater.state.detail })}</strong>
					<div class="toast-actions">
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
					</div>
				</div>
			{/if}
		</aside>
	{/if}

	{#if aboutOpen}
		<AboutDialog onClose={() => (aboutOpen = false)} />
	{/if}

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}

	{#if view === 'console'}
		<section class="console" aria-label={i18n.t('tab.console')}>
			<p class="what">{i18n.t('console.what')}</p>
			<!--
				**端末にも接続先を選べるように**（実機の指摘・2026-09-03）。
				ファイルの面には接続タブがあるのに端末に無いのは、筋が通りません。
				**選ぶ先はファイルの面と同じ 1 つ**です（`session.focus`）。
			-->
			{#if session.all.length > 1}
				<div class="console-conns" role="tablist" aria-label={i18n.t('tab.connections')}>
					{#each session.all as held (held.id)}
						<button
							type="button"
							role="tab"
							class="console-conn"
							class:active={held.id === session.activeId}
							aria-selected={held.id === session.activeId}
							onclick={() => session.focus(held.id)}
						>
							<span data-secret>{held.name}</span>
							{#if held.tag}<span class="tag" data-secret>{held.tag}</span>{/if}
						</button>
					{/each}
				</div>
			{/if}

			<div class="console-head">
				<!-- **誰が握っているかを、常に出す。**見えない所で AI が打っている、を作らない。 -->
				<!-- **どの接続の端末かを常に出す**（D25）。
				     タブを移しても端末は付いてこないので、書いていないと迷子になる。 -->
				{#if consoleOn}
					<span class="on" data-secret>{consoleOn}</span>
				{/if}
				<span class="holder" class:ai={holder === 'ai'} class:mine={iHold}>
					<Icon name={holder ? 'lock' : 'terminal'} size={12} />
					{holder === 'ai'
						? i18n.t('console.held.ai')
						: iHold
							? i18n.t('console.held.me')
							: i18n.t('console.held.none')}
				</span>
				{#if !holder}
					<button type="button" class="primary" onclick={openConsole}>
						<Icon name="terminal" />
						{i18n.t('console.open')}
					</button>
				{:else if !iHold}
					<!-- **人はいつでも取り返せる**（D29）。 -->
					<button type="button" class="primary" onclick={takeConsole}>
						<Icon name="lock" />
						{i18n.t('console.take')}
					</button>
				{/if}
				{#if holder}
					<!-- **止まらない停止ボタンは、無い方がまし。**必ず効く。 -->
					<button type="button" class="danger" onclick={stopConsole}>
						<Icon name="stop" />
						{i18n.t('console.stop')}
					</button>
				{/if}
				<!-- **押せる所にも置く。**ショートカットだけだと、知らない人には無いのと同じ。 -->
				<button type="button" onclick={() => openSearch('console')}>
					{i18n.t('search.label')}
				</button>
			</div>
			{@render searchBar('console')}
			<div class="terminal shell" class:locked={holder === 'ai'}>
				<div class="core terminal-core" bind:this={consoleHost}></div>
			</div>
		</section>
	{:else if view === 'files'}
		<FileBrowser />
	{:else if view === 'connections'}
		<ConnectionManager />
	{:else if view === 'diag'}
		<!-- **何が起きたか。**MCP の `diagnostics` と同じ 1 つを見ている。 -->
		<section class="diag" aria-label={i18n.t('tab.diag')}>
			<div class="diag-head">
				<span class="label">{i18n.t('diag.label')}</span>
				<span class="scaffold">{i18n.t('diag.help')}</span>
				<button type="button" onclick={loadDiagnostics}>
					<Icon name="refresh" size={13} />
					{i18n.t('diag.refresh')}
				</button>
			</div>
			{#if diag.length === 0}
				<p class="empty">{i18n.t('diag.empty')}</p>
			{:else}
				<ol class="diag-list">
					{#each diag as event (event.seq)}
						<li class={event.level}>
							<span class="at">{(event.atMs / 1000).toFixed(1)}s</span>
							<span class="stage">{event.stage}</span>
							{#if event.connection}<span class="who">{event.connection}</span>{/if}
							<span class="what">
								{event.message}
								{#if event.hint}<em class="hint">→ {event.hint}</em>{/if}
							</span>
						</li>
					{/each}
				</ol>
			{/if}
		</section>
	{:else}
	<section class="stream" aria-label={i18n.t('stream.label')}>
		<!-- **この面が何をする所かを、その場に書く。**
		     読んで分からない面は、無いのと同じ（実際に分からなかった）。 -->
		<p class="what">{i18n.t('stream.what')}</p>
		<div class="stream-head">
			<span class="label">{i18n.t('stream.label')}</span>
			<input
				class="path"
				bind:value={followPath}
				onkeydown={(event) => event.key === 'Enter' && follow()}
				placeholder={i18n.t('stream.path')}
				aria-label={i18n.t('stream.path')}
				spellcheck="false"
				data-secret
			/>
			<button type="button" onclick={follow} disabled={streaming}>
				{i18n.t('stream.start')}
			</button>
			<button type="button" onclick={stopStream}>{i18n.t('stream.stop')}</button>
			<button type="button" onclick={() => openSearch('output')}>
				{i18n.t('search.label')}
			</button>
		</div>
		{@render searchBar('output')}
		<div class="terminal shell">
			<div class="core terminal-core" bind:this={terminalHost}></div>
		</div>
	</section>

	<section class="band" aria-label={i18n.t('band.label')}>
		<!-- **帯が何なのかを書く。**「ここに何が出るのか」が分からないと、
		     空のときに壊れているのか、何も起きていないのかが区別できない。 -->
		<p class="what">{i18n.t('band.what')}</p>
		{#if lines.length === 0}
			<p class="empty">{i18n.t('band.empty')}</p>
		{:else}
			<ol>
				{#each lines as line (line.seq)}
					<li class:ai={line.tag === '[AI]'}>{line.rendered}</li>
				{/each}
			</ol>
		{/if}
	</section>
	{/if}
</main>

<style>
	/* 色は tokens.css の変数だけ。**ここに 16 進数を書かない。** */
	:global(body) {
		margin: 0;
		background: var(--ground);
		color: var(--fg);
		font-family: var(--font-ui);
		font-feature-settings: 'palt' 1;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
		padding: 0.9rem 1rem 1rem;
		box-sizing: border-box;
		gap: 0.7rem;
	}

	/* **上の帯は 1 行。**折り返しても 2 行までに収まる高さにする。 */
	/* **折り返さない。**折り返すと帯が 2 段になり、本文の面積が減る。
	   入りきらないときは、説明 → MCP の順に縮める。 */
	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: nowrap;
		min-width: 0;
		/*
		 * **この帯が OS のタイトルバー**（D17）。
		 *
		 * 掴める高さを確保します。34px では**掴める隙間がほとんど無く、
		 * 窓を動かせない**と言われました（実機）。
		 */
		min-height: 40px;
		/* 掴む所に文字を選ばせない。**掴んだつもりで選択が始まると、窓が動かない。** */
		user-select: none;
	}

	/* --- 更新の知らせ（D34） --- */

	.update-toast {
		position: fixed;
		right: 1rem;
		bottom: 1rem;
		z-index: 60;
		display: flex;
		gap: 0.6rem;
		max-width: min(24rem, calc(100vw - 2rem));
		padding: 0.75rem 0.85rem;
		border: 1px solid var(--hairline-strong);
		border-radius: var(--r-shell);
		background: var(--surface);
		/* **他と違う面に見せる。**本文と同じ平面に置くと、見落とされます。 */
		box-shadow: var(--lift-3);
	}

	.toast-icon {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border-radius: 999px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.toast-icon.warn {
		background: var(--warning-soft);
		color: var(--warning);
	}

	.toast-body {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		min-width: 0;
	}

	.toast-body strong {
		font-size: 0.8rem;
		font-weight: 600;
		line-height: 1.45;
	}

	.toast-actions {
		display: flex;
		gap: 0.4rem;
	}

	.toast-actions button {
		font-size: 0.75rem;
		padding: 0.25rem 0.6rem;
	}

	.toast-bar {
		height: 4px;
		border-radius: 999px;
		background: var(--surface-2);
		overflow: hidden;
	}

	.toast-bar span {
		display: block;
		height: 100%;
		background: var(--accent);
	}

	/* --- 「表示」メニュー（自前タイトルバーで OS のメニューを失った分） --- */

	.console-conns {
		display: flex;
		gap: 0.3rem;
		flex-wrap: wrap;
		padding-bottom: 0.35rem;
	}

	.console-conn {
		font-size: 0.75rem;
		padding: 0.2rem 0.55rem;
		border: 1px solid var(--hairline);
		border-radius: var(--r-control);
		background: transparent;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
	}

	.console-conn.active {
		border-color: var(--accent);
		color: var(--accent);
	}

	.about-open {
		flex: 0 0 auto;
		width: 24px;
		height: 24px;
		padding: 0;
		border-radius: 999px;
		background: transparent;
		border: 1px solid var(--hairline);
		color: var(--fg-muted);
		font-size: 0.75rem;
		line-height: 1;
		margin-left: auto;
	}

	.about-open:hover {
		color: var(--fg);
		border-color: var(--hairline-strong);
	}

	.view-menu {
		position: relative;
		display: flex;
		align-items: center;
	}

	.view-trigger {
		font-size: 0.75rem;
		padding: 0.2rem 0.55rem;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 6px;
		color: var(--fg);
		white-space: nowrap;
	}

	.view-trigger:hover,
	.view-trigger[aria-expanded='true'] {
		border-color: var(--hairline);
	}

	/* **外を押したら閉じる。**帯より下は全部当たり判定にする。 */
	.view-backdrop {
		position: fixed;
		inset: 0;
		z-index: 40;
	}

	.view-panel {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 41;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.6rem;
		min-width: 220px;
		border: 1px solid var(--hairline);
		border-radius: 8px;
		background: var(--surface);
		box-shadow: 0 8px 24px rgb(0 0 0 / 25%);
	}

	.view-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
	}

	.view-label {
		font-size: 0.75rem;
		color: var(--fg-muted);
		white-space: nowrap;
	}

	.view-controls {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	/* --- 窓の操作（自前タイトルバー・D17） --- */

	.window-controls {
		display: flex;
		align-items: stretch;
		gap: 0;
		margin-left: 0.25rem;
		/* 帯の端まで届かせる。**Windows の人は右上の角を押しに行く。** */
		margin-right: calc(-1 * var(--shell-pad, 0.6rem));
		align-self: stretch;
	}

	.wc {
		min-width: 40px;
		border: 0;
		background: transparent;
		color: var(--fg-muted);
		font-size: 0.8rem;
		line-height: 1;
		cursor: default;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0 0.5rem;
	}

	.wc:hover {
		background: var(--surface-2, rgba(127, 127, 127, 0.18));
		color: var(--fg);
	}

	/* **閉じるだけは赤。**押し間違いの結果が他と違う。 */
	.wc.close:hover {
		background: #c42b1c;
		color: #fff;
	}

	.wc:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	/* 長い説明は常時出さない。**ここに置くと本文の面積が消える。**
	   全文は title 属性と、帯が空のときの案内に出す。 */
	.phase {
		font-size: 0.7rem;
		color: var(--fg-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		flex: 1 1 auto;
	}

	/* **合言葉ごと写せるボタン。**起動ごとに変わる値を人に書き写させない（D23）。 */
	/* 文字サイズ。**アイコンではなく「A」の大小そのもの**で示す方が伝わる。 */
	.text-step {
		line-height: 1;
		font-weight: 600;
	}

	.smaller-a {
		font-size: 0.62rem;
	}

	.larger-a {
		font-size: 0.86rem;
	}

	/* いまどの段か。**出さないと、押した結果が分からない。** */
	.text-now {
		font-size: 0.62rem;
		color: var(--fg-muted);
		white-space: nowrap;
		padding: 0 0.1rem;
		min-width: 2.2rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}

	/* 診断。**帯とは別。**帯は「誰が何をしたか」、ここは「なぜそうなったか」。 */
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
		border-radius: 999px;
	}

	.what {
		flex: 1 1 auto;
		min-width: 0;
	}

	/* 次の一手。**失敗には必ず付く。** */
	.hint {
		display: block;
		font-style: normal;
		color: var(--fg-muted);
		font-size: 0.66rem;
	}

	.mcp {
		display: inline-flex;
		align-items: center;
		gap: 0.28rem;
		font-family: var(--font-ui);
		font-size: 0.66rem;
		color: var(--fg-muted);
		white-space: nowrap;
		flex: none;
		padding: 0.2rem 0.5rem;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		background: var(--shell);
		cursor: pointer;
		transition: background var(--fast) var(--ease);
	}

	.mcp:hover:not(:disabled) {
		background: var(--surface-2);
	}

	.mcp:disabled {
		opacity: 0.5;
		cursor: default;
	}

	/* 設定と切替は、浮いた 1 本のピルにまとめる。 */
	.view-panel,
	.tabs {
		display: flex;
		align-items: center;
		gap: 0.2rem;
		flex-wrap: nowrap;
		flex: none;
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 3px;
	}

	.view-panel-icon {
		display: inline-flex;
		align-items: center;
		padding: 0 0.15rem 0 0.4rem;
		color: var(--fg-faint);
	}

	/* **アイコンと文字を横に並べる。**これが無いと、SVG が block なので
	   文字の上に乗ってしまう（実際に乗った）。 */
	.view-panel button,
	.tabs button {
		display: inline-flex;
		align-items: center;
		gap: 0.32rem;
		font: inherit;
		font-size: 0.74rem;
		color: var(--fg-muted);
		background: transparent;
		border: 1px solid transparent;
		border-radius: 999px;
		padding: 0.2rem 0.7rem;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease),
			transform var(--fast) var(--ease);
	}

	.view-panel button.icon-only {
		padding: 0.26rem 0.4rem;
	}

	.view-panel button:active,
	.tabs button:active {
		transform: scale(0.97);
	}

	.view-panel button.active,
	.tabs button.active {
		color: var(--fg);
		background: var(--surface);
		box-shadow: var(--inner-highlight), var(--lift-1);
	}

	.view-panel button:focus-visible,
	.tabs button:focus-visible,
	.view-panel select:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.view-panel select {
		font: inherit;
		font-size: 0.74rem;
		color: var(--fg);
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 0.16rem 0.5rem;
		margin-left: 0.15rem;
		box-shadow: var(--inner-highlight);
	}

	.failure {
		margin: 0;
		padding: 0.55rem 0.8rem;
		border-radius: var(--r-control);
		background: var(--danger-soft);
		color: var(--danger);
		font-size: 0.78rem;
	}

	.stream {
		display: flex;
		flex-direction: column;
		flex: 2;
		min-height: 0;
		gap: 0.45rem;
	}

	.stream-head {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.stream-head .label {
		flex: 1;
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	/* 端末の検索バー。**帯やログの並びに合わせただけ**の仮置き（DESIGN.md）。 */
	.search-bar {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.search-bar input {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: 0.72rem;
	}

	/* **無かったことを、その場に出す。**色に頼らず文字で言う。 */
	.search-bar .missed {
		color: var(--fg-faint);
	}

	/* 端末（D29）。**誰が握っているかを常に出す。** */
	.console {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-height: 0;
		flex: 1 1 auto;
	}

	/* **説明文を伸ばさない。**他のルールを拾って 340px に伸び、
	   端末の上に大きな空白ができていた（実際にそうなった）。 */
	.console .what {
		flex: 0 0 auto;
	}

	.console-head {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		flex: 0 0 auto;
	}

	/* **この面のボタンは自前で整える。**素のままだと OS 既定の灰色になり、
	   アイコンの下に文字が回り込んで潰れる（実際に潰れた）。 */
	.console-head button {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		white-space: nowrap;
		padding: 0.35rem 0.7rem;
		border: 1px solid var(--hairline);
		border-radius: var(--r-control);
		background: var(--surface);
		color: var(--fg);
		font-size: 0.76rem;
		cursor: pointer;
	}

	.console-head button.primary {
		border-color: var(--accent);
		color: var(--accent);
	}

	.console-head button.danger {
		border-color: var(--danger);
		color: var(--danger);
	}

	.console-head .on {
		font-family: var(--font-mono);
		font-size: 0.7rem;
		padding: 0.15rem 0.4rem;
		border: 1px solid var(--hairline);
		border-radius: var(--r-control);
		color: var(--fg-muted);
	}

	.holder {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		margin-right: auto;
	}

	.holder.mine {
		color: var(--ok);
	}

	/* **AI が握っている間は、見て分かるようにする。**締まっていることが伝わらないと、
	   打てないのを不具合だと思われる。 */
	.holder.ai {
		color: var(--warning);
	}

	.terminal.locked {
		outline: 1.5px solid var(--warning);
		outline-offset: 2px;
	}

	/* **この面が何をする所か**を書く 1 行。読んで分からない面は、無いのと同じ。 */
	.what {
		margin: 0 0 0.35rem;
		font-size: 0.72rem;
		line-height: 1.6;
		color: var(--fg-muted);
	}

	/* 追うログのパス。**入力欄が伸びて、ボタンが右へ寄る。** */
	.stream-head .path {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: 0.72rem;
	}

	.stream-head button {
		font: inherit;
		font-size: 0.72rem;
		color: var(--fg);
		background: var(--surface-2);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 0.22rem 0.75rem;
		cursor: pointer;
		box-shadow: var(--inner-highlight), var(--lift-1);
		transition: transform var(--fast) var(--ease);
	}

	.stream-head button:active {
		transform: scale(0.97);
	}

	.stream-head button:disabled {
		opacity: 0.45;
		cursor: default;
		box-shadow: none;
	}

	/* 端末は暗いまま。ANSI の既定色は暗い背景に載る前提で作られている。 */
	.terminal {
		flex: 1;
		min-height: 0;
	}

	.terminal-core {
		background: var(--terminal-bg);
		overflow: hidden;
		padding: 6px 8px;
	}

	.band {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		padding: 0.6rem 0.75rem;
	}

	.band ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.band li {
		white-space: pre;
		font-family: var(--font-mono);
		font-size: 0.76rem;
		line-height: 1.75;
		color: var(--fg-muted);
	}

	.band li.ai {
		color: var(--accent);
	}

	.empty {
		margin: 0;
		font-size: 0.76rem;
		color: var(--fg-faint);
	}
</style>
