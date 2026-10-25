<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import type { DiagEvent } from '$lib/diagnostics';
	import ConnectionManager from '$lib/components/ConnectionManager.svelte';
	import ConsoleRequestDialog from '$lib/components/ConsoleRequestDialog.svelte';
	import HostKeyDialog from '$lib/components/HostKeyDialog.svelte';
	import OperationRequestDialog from '$lib/components/OperationRequestDialog.svelte';
	import PassphraseDialog from '$lib/components/PassphraseDialog.svelte';
	import ConnectPanel from '$lib/components/ConnectPanel.svelte';
	import ConsolePaneView from '$lib/components/ConsolePane.svelte';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { session } from '$lib/session.svelte';
	import { menuLabels } from '$lib/i18n/messages-menu';
	import { titlebar } from '$lib/window/titlebar.svelte';
	import AboutDialog from '$lib/components/AboutDialog.svelte';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';

	import BandExportDialog from '$lib/components/BandExportDialog.svelte';
	import BandPanel from '$lib/components/BandPanel.svelte';
	import ConnectionRail from '$lib/components/ConnectionRail.svelte';
	import DiagPanel from '$lib/components/DiagPanel.svelte';
	import AllowlistDialog, {
		type AllowedList
	} from '$lib/components/AllowlistDialog.svelte';
	import KeyHelpDialog from '$lib/components/KeyHelpDialog.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import TransferList from '$lib/components/TransferList.svelte';
	import type { TransferLine } from '$lib/components/TransferList.svelte';
	import OperationProposalDialog from '$lib/components/OperationProposalDialog.svelte';
	import type { OperationProposal } from '$lib/components/OperationProposalDialog.svelte';
	import { summarise } from '$lib/update/notes';
	import { cutMessageKey, cutSummary } from '$lib/update/restart-cost';
	import { forget as forgetReopen, take as takeReopen, type Reopen } from '$lib/update/reopen';
	import { updater } from '$lib/update/updater.svelte';
	import { listenForPageShot } from '$lib/capture/page-shot';
	import { LOCALES } from '$lib/i18n/locales';
	import { textSize } from '$lib/text-size/text-size.svelte';
	import { theme, type ThemeMode } from '$lib/theme/theme.svelte';
	import { attachFit, attachSearch, createTerminal, writeChunk } from '$lib/terminal.svelte';
	import { paneLayout, type ConsolePane } from '$lib/console-panes';
	import {
		BEFORE_CONNECTING,
		emptyBacklogs,
		keyFor,
		rememberFor,
		replayFor,
		type Backlogs
	} from '$lib/stream-backlog';
	import { applyEdit, editIntent, type EditableField } from '$lib/edit-keys';
	import { appIntent } from '$lib/app-keys';
	import { redaction } from '$lib/redaction/redaction.svelte';
	import { canFollow, isView, VIEWS, type View, viewAfterAnswered} from '$lib/view-request'
	import { asConnectFailure, readableFailure } from '$lib/connect-failure';
	import { attachClipboard, browserClipboard, detectPlatform } from '$lib/terminal-clipboard';
	import { isFindShortcut, type TerminalSearch } from '$lib/terminal-search';
	import '@xterm/xterm/css/xterm.css';
	import type { Terminal } from '@xterm/xterm';

	type McpAccess = { url: string; token: string };
	type McpFailure = { port: number; detail: string };

	/** 何が起きたかの記録。Rust 側の `Event` と対。**接続先は入っていない。** */

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

	/**
	 * **人が自分でタブを切り替えた時刻**（D44）。
	 *
	 * AI に画面を奪わせないための材料です。**選んだ端から戻されるのは、
	 * 奪われたのと同じ**なので、しばらくは頼まれても動きません。
	 */
	let humanSwitchedAt = $state(0);

	/** 人がタブを押した。**AI の頼みより、こちらが優先。** */
	function humanShows(next: typeof view) {
		humanSwitchedAt = Date.now();
		view = next;
	}

	/**
	 * **画面を伏せる**（UI 案・2026-10-21／D26）。
	 *
	 * 配線はもともと在りました —— `[data-redact='on'] [data-secret]` が
	 * 中身を塗り潰し（**ぼかしでないので拡大しても読めない**）、印は 9 ファイルに
	 * 付いています。**足りなかったのは人が押す口だけ**で、
	 * それまでは写真撮り用のイベントでしか立ちませんでした。
	 *
	 * **この製品は画面そのものに接続先が写り込みます**（CLAUDE.md 禁止事項 4）。
	 * 画面共有や撮影の前に、人が 1 回押せることが要ります。
	 */
	/**
	 * **帯を書き出す**（案・4 枚目「AI の操作記録（帯）を書き出す」）。
	 *
	 * **囲い（D22）は落とし先を守りません。**だから押す前に
	 * 何が入るかを並べて見せます（`BandExportDialog`）。
	 *
	 * **この口は画面にしかありません。**MCP には出しません ——
	 * D27 で「手元へ落とす口は AI に作らない」と決めたのと同じ性質です。
	 */
	let bandExportOpen = $state(false);

	/**
	 * **問いが出たことに気づけるように**（Issue #30）。
	 *
	 * > パス入れるときに、アプリを前面に出せます？
	 * > **私の場合、裏で聞かれていて気づかないですね**
	 *
	 * **既定は点滅だけ。**前面へ出すのは選んだ人だけです ——
	 * 報告者自身が「**人によっては邪魔**」と書いています。
	 */
	type AttentionHow = 'off' | 'flash' | 'front';
	let attention = $state<AttentionHow>('flash');

	async function setAttention(how: AttentionHow) {
		try {
			await invoke('attention_set', { how });
			attention = how;
		} catch (error: unknown) {
			// **握り潰さない。**押したのに変わっていない、を作らない。
			failure = String(error);
		}
	}

	async function saveBand(payload: { text: string; name: string }) {
		bandExportOpen = false;
		try {
			const destination = await saveDialog({
				// **日時を入れる。**同じ名前だと 2 回目から上書きになります。
				defaultPath: payload.name,
				filters: [{ name: 'sshboard', extensions: ['txt'] }]
			});
			if (!destination) return;
			await invoke<number>('band_export', { destination, text: payload.text });
		} catch (error: unknown) {
			// **握り潰さない。**書けなかったのに書けたように見せない。
			failure = String(error);
		}
	}

	function toggleMask() {
		void redaction.set(!redaction.on).catch((error: unknown) => {
			// **握り潰さない。**押したのに伏せられていない、が一番危ない。
			failure = String(error);
		});
	}

	/**
	 * **AI を止める**（UI 案・2026-10-21）。
	 *
	 * 止めるのは**いま走っているもの**です ——
	 * 流れている出力（`stop_stream`）と、AI が握っている端末（`console_stop`）。
	 *
	 * **MCP そのものを閉じる口はまだありません。**
	 * 「AI を二度と繋げない」ではなく「**いまの手を止める**」です。
	 * 文言もそこに合わせています（案の `AI を止める`）。
	 */
	async function stopAi() {
		failure = null;
		try {
			// **出力を先に止める。**端末を先に閉じると、残りが流れ続けます。
			await invoke('stop_stream');
			if (holder === 'ai') await invoke('console_stop');
			// **口も閉じる**（2026-10-23）。
			//
			// それまでは**走っているものしか止めていませんでした** ——
			// 口が空いたままなので、AI は次の呼びかけで戻ってきます。
			// 「止めた」と言って止まっていないのが、いちばん悪い形です。
			//
			// **戻すにはアプリを立ち上げ直します。**開き直す口は作りません ——
			// 「止めたのに勝手に戻る」を作らないためです。
			await invoke('mcp_stop');
			mcp = null;
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** **止める前に 1 回聞く。**口を閉じると、戻すには立ち上げ直しが要ります。 */
	let stopAiAsking = $state(false);

	/**
	 * **立ち上げ直したあと、開いていた接続へ繋ぎ直すかを聞く**（UI 案・5 枚目）。
	 *
	 * **勝手には繋ぎません** —— 立ち上げ直した瞬間に繋ぎ直すと、
	 * **人が見ていない所で SSH が開きます**（禁止事項 3）。
	 * **AI の分も繋ぎ直しません。**
	 */
	let reopenAsk = $state<Reopen | null>(null);

	/**
	 * **転送の列**（UI 案・2 枚目）。**誰が動かしたかが行ごとに残ります。**
	 *
	 * 帯には 1 行の文字列として出ていましたが、**構造が無かった**ので
	 * 向き・誰が・何を・何バイト・成否を並べて読めませんでした。
	 * とくに**柵の外へ書こうとして断られたこと**が、帯の 1 行に埋もれていました。
	 */
	let transfers = $state<TransferLine[]>([]);

	/** 控えにあった接続へ、1 本ずつ繋ぎ直す。**失敗は握り潰さない。** */
	async function reopenThem(ids: readonly string[]) {
		reopenAsk = null;
		for (const id of ids) {
			try {
				await invoke('session_connect', { id, passphrase: null });
			} catch (error: unknown) {
				// **1 本落ちても残りは試す。**そのうえで理由は出します。
				failure = String(error);
			}
		}
		await session.refresh();
	}



	/**
	 * **キー操作の一覧が開いているか**（UI 案・2026-10-21）。
	 *
	 * `Ctrl Shift H` を入れても、**どこにも書いていなければ無いのと同じ**です。
	 * `F1` と、ヘッダーの［?］で開きます。
	 */
	let keyHelpOpen = $state(false);
	/**
	 * **AI が走らせられることの一覧**（2026-10-23）。
	 *
	 * **人だけが見られませんでした** —— 実行体は両方のファイルを読めて、
	 * AI も `list_operations` で読めるのに、画面へ出す口がありませんでした。
	 * 本番で詰まったのはここで、**人は書く場所を探して見つけられませんでした。**
	 */
	type AllowlistOverview = { reading: AllowedList; changing: AllowedList };
	let allowlist = $state<AllowlistOverview | null>(null);
	let allowlistOpen = $state(false);

	async function openAllowlist() {
		viewMenuOpen = false;
		// **開くたびに読み直します。**人が外でファイルを書き換えた直後に
		// 開くのが、まさにこの面の使い所です（D59 と同じ考え）。
		try {
			allowlist = await invoke<AllowlistOverview>('allowlist_overview');
			allowlistOpen = true;
		} catch (error: unknown) {
			// **握り潰さない。**開かない理由が分からないと、人は製品の外で直します。
			failure = String(error);
		}
	}


	/** 画面ぜんたいのキー操作を実行する（判定は `app-keys.ts`）。 */
	function runApp(intent: NonNullable<ReturnType<typeof appIntent>>) {
		if (intent === 'mask') {
			toggleMask();
			return;
		}
		if (intent === 'stopAi') {
			// **何もしていないときは何もしない。**空振りを「止めた」と見せない。
			// 口が立っているなら、走っていなくても閉じる意味があります。
			if (aiIsDoing || mcp !== null) stopAiAsking = true;
			return;
		}
		if (intent === 'menu') {
			viewMenuOpen = true;
			return;
		}
		if (intent === 'help') {
			keyHelpOpen = true;
			return;
		}
		if (intent === 'newConnection') {
			humanShows('connections');
			return;
		}
		// `view:1` 〜 `view:5`。**並びは `VIEWS` が正本**
		// （案は 端末 / ファイル / 接続 の順ですが、実物は **接続が 1 番目**です ——
		//  「繋がないとファイルも端末も空」という実機の指摘でそうなっています）。
		const at = Number(intent.slice('view:'.length)) - 1;
		const next = VIEWS[at];
		if (next) humanShows(next);
	}

	// --- 端末（D29）------------------------------------------------------------
	// **同時に触れるのは 1 人。**AI が握っている間、人の入力は締まる。
	/**
	 * **開いている端末ぜんぶ**（分割・2026-10-06）。
	 *
	 * 実行体が押し出します（`console://list`）。**握っている側だけでは、
	 * 画面は「何枚描けばよいか」が分かりません。**
	 */
	let consolesOpen = $state<ConsolePane[]>([]);
	/** 人が選んだ面の番号。**閉じたら忘れます**（`paneLayout` が面倒を見ます）。 */
	let chosenConsole = $state<number | null>(null);
	// **`$state` にする。**そうしないと、文字サイズの効果が
	// 「端末ができたこと」を追えず、片方だけ取り残される（実際に取り残された）。

	/**
	 * **直近の出力**（Issue #14 / #11）。
	 *
	 * 端末の面は、そのタブを見ている間しか存在しません。別のタブに居る間に
	 * 届いた分は**どこへも書かれず**、見に行くと**まっさら**でした。
	 * 承認のダイアログは「打った内容は画面にそのまま出ます」と約束しているのに、
	 * **出ていませんでした。**面ができた時点で、ここから書き戻します。
	 */
	// **接続ごとに覚えます**（D60）。1 本で持つと、タブを戻したときに
	// **別のサーバーの出力が出ます。**
	//
	// **わざと反応しない入れ物に入れています**（`$state` を付けないこと）。
	// ここは「面ができた時点で書き戻すための控え」で、**画面を描き直す元ではありません。**
	// 生きている面への書き込みは `consoleWriters`（`onready` で受けた口）が運びます。
	//
	// `$state` を付けると、`ConsolePane` へ渡す `backlog` が**出力 1 片ごとに変わり**、
	// 面の `$effect` が毎回走ります（面は既にあるので何も書かずに戻るだけの空回り）。
	// 控えを反応させても**打鍵が画面に出るようにはなりません** ——
	// 0.1.21 の回帰はそこを間違えて、控えに溜めるだけで面へ書いていませんでした。
	const backlogs: { all: Backlogs } = { all: emptyBacklogs() };
	/** 誰が握っているか。`null` は誰も握っていない。 */
	let holder = $state<'human' | 'ai' | null>(null);
	/** **どの接続の端末か。**タブを移しても端末は付いてこない（D25）。 */
	let consoleOn = $state<string | null>(null);
	/**
	 * **AI が端末を使いたいと言っている**（D42）。`'ai'` なら問いを出します。
	 *
	 * 実機の指摘（2026-09-06）— 「AI からのアクションが分からない」。
	 * 気づかせる口がここです。
	 */
	let consoleAsk = $state<string | null>(null);

	/**
	 * **AI が繋ごうとして、パスフレーズで止まった接続**（Issue #13）。
	 *
	 * 実機の指摘 —— AI から繋ぐと「画面で人が入れてください」と返るのに、
	 * **問いがどこにも出ませんでした。**問いは人の経路にしか無かったためです。
	 * 案内された側は詰みます。**存在しない操作を指していました。**
	 */
	/**
	 * **AI が走らせたい操作**（D45 / D47 / D48）。**何が走るのかも一緒に持ちます** ——
	 * id だけでは、人は何を許したのか分かりません。
	 *
	 * `needsSecret` は **その場でパスワードを聞く必要があるか**（Issue #19）。
	 */
	type OperationAsk = { id: string; runs: string; needsSecret: boolean };
	let operationAsk = $state<OperationAsk | null>(null);

	/**
	 * **AI が「この操作を足してよいか」と出した提案**（実運用の報告・2026-10-23）。
	 *
	 * **これは実行の要求ではありません。**登録と実行は別の行為で、
	 * 登録しても走りません（走るときは上の `operationAsk` を通ります）。
	 * **1 回の押下にまとめたら、それは手順を増やした `run_command` です**（D3）。
	 */
	let operationProposal = $state<OperationProposal | null>(null);

	/** **人が登録する。**書くのは画面で、AI にこの口はありません（D22 / D27）。 */
	async function registerProposal(proposal: OperationProposal) {
		failure = null;
		try {
			await invoke('operation_register', { proposal });
			operationProposal = null;
		} catch (error: unknown) {
			// **握り潰さない。**押したのに入っていない、を作らない。
			failure = String(error);
		}
	}

	async function dismissProposal() {
		try {
			await invoke('operation_dismiss');
		} catch (error: unknown) {
			failure = String(error);
		}
		operationProposal = null;
	}

	/**
	 * `secret` は **人がその場で入れたものだけ**（D48）。
	 *
	 * **どこにも残しません。**ここでは変数にすら置かず、そのまま渡します ——
	 * 画面側に持つと、**次の問いまで居座ります。**
	 */
	async function answerOperation(allow: boolean, secret?: string) {
		// **先に画面から消す。**残っていると二重に押します。
		operationAsk = null;
		try {
			await invoke('operation_answer', { allow, secret: secret ?? null });
		} catch (error: unknown) {
			// **失敗の文言に入れたものは載りません**（engine 側は識別子しか返しません）。
			failure = String(error);
		}
	}

	let passphraseFor = $state<string | null>(null);
	/**
	 * **ホスト鍵の問い**（Issue #26）。
	 *
	 * #24 で型は作ったのに、**ここが無かった** —— `pending_status` は返るので
	 * **AI からは待ちが見えるのに、人の画面には何も出ず**、
	 * 赤帯が `[object Object]` になっていました。
	 */
	let hostKeyAsk = $state<{
		id: string;
		algorithm: string;
		fingerprint: string;
		expected: string | null;
	} | null>(null);
	let hostKeyBusy = $state(false);

	async function answerHostKey(trust: boolean) {
		const asked = hostKeyAsk;
		if (!asked || hostKeyBusy) return;
		hostKeyBusy = true;
		try {
			await invoke(trust ? 'host_key_trust' : 'host_key_refuse', { id: asked.id });
			hostKeyAsk = null;
			// **承認したら、そのまま繋ぎ直す**（Issue #26 の後始末）。
			//
			// 以前は `ConnectPanel` が自分で繋ぎ直していました。問いを画面側へ
			// 移したので、**繋ぎ直しも一緒に移します** ——
			// 置いてこないと、**人は承認したのに何も起きない**と読みます。
			if (trust) await invoke('session_connect', { id: asked.id, passphrase: null });
		} catch (error: unknown) {
			// **黙って閉じない。**閉じると、人は答えたつもりで答えていない状態になります。
			//
			// **そして、構造を持った失敗を帯へ潰さない**（実運用の指摘・2026-09-28）——
			// ここで `String(error)` を使っていたため、**上部の帯に `[object Object]`**
			// だけが出ていました。人には何も伝わらず、**こちらにも何が起きたか
			// 分かりませんでした**（この字だけで 2 日使っています）。
			const why = asConnectFailure(error);
			if (why?.kind === 'passphraseNeeded') {
				// **承認したあと、鍵にパスフレーズが要るだけ**の場合。
				// **帯に落とすと、人は承認したのに先へ進めません。**箱を開きます。
				passphraseFor = asked.id;
			} else if (why?.kind === 'untrusted') {
				// 実行体が問いを立て直しています。**橋が画面へ運びます。**
				// ここで帯を出すと、**問いと帯が二重に出ます。**
			} else {
				failure = readableFailure(error);
			}
		} finally {
			hostKeyBusy = false;
		}
	}
	let passphraseBusy = $state(false);

	async function answerPassphrase(passphrase: string, remember: boolean) {
		const id = passphraseFor;
		if (!id) return;
		passphraseBusy = true;
		try {
			// **預けるのは、押した人が決めた場合だけ**（D11・実運用の要望）。
			//
			// **繋ぐ前に預けます。**繋いだあとにすると、**繋ぎ損ねた回だけ
			// 預からない**という分かりにくい形になります。
			//
			// **失敗しても繋ぐのは止めません** —— 預けられないことと、
			// 繋げないことは別です。**預けたかっただけの人が繋げなくなります。**
			if (remember) {
				try {
					await invoke('connection_passphrase_save', { id, passphrase });
				} catch (error: unknown) {
					// **黙らない。**預かったつもりで預かっていないのが、いちばん悪い形です。
					failure = String(error);
				}
			}
			// **人の経路をそのまま通す。**AI 側へ秘密を渡す道は作りません（D14）。
			await invoke('session_connect', { id, passphrase });
			passphraseFor = null;
			await session.refresh();
		} catch (error: unknown) {
			// **構造を持った失敗を `String()` で潰さない**（Issue #33・2026-10-03）。
			//
			// > その直後に赤帯が `[object Object]` になり、接続が開かない
			//
			// `session_connect` は**構造を持った失敗**を返します
			// （ホスト鍵・パスフレーズ・その他）。`String(error)` だと
			// **`[object Object]` になり、人にも私たちにも何も伝わりません。**
			// ホスト鍵の経路（`answerHostKey`）だけが直っていて、ここは残っていました。
			failure = readableFailure(error);
		} finally {
			passphraseBusy = false;
		}
	}

	async function dismissPassphrase() {
		passphraseFor = null;
		try {
			await invoke('passphrase_dismiss');
		} catch {
			/* 畳めなくても画面からは消えている */
		}
	}

	async function answerConsole(allow: boolean) {
		// **先に画面から消す。**押したのに残っていると、二重に押します。
		consoleAsk = null;
		try {
			await invoke('console_answer', { allow });
			// **許可したら、端末の面へ連れて行く**（実運用の指摘・2026-09-28）。
			//
			// > 端末をにぎりたいとアラートがでて、**通った時点で、一緒にタブも切り替わる**
			//
			// **`show_view` を当てにしません。**AI が呼び忘れたら人は見ないままで、
			// 呼んでも `canFollow` で止まりうる（問いに答えた直後は `askOpen` が
			// 落ちているとはいえ、入力欄に焦点があれば止まります）。
			// **許可したのは端末なので、AI の裁量に委ねる場面ではありません。**
			//
			// `humanShows` を通します —— **これは人が押した結果**なので、
			// 奪ったことになりません（そのあと 8 秒は AI に動かさせない、も込み）。
			const next = viewAfterAnswered('console', allow);
			if (next) humanShows(next);
		} catch (error: unknown) {
			failure = String(error);
		}
	}
	const iHold = $derived(holder === 'human');
	/**
	 * **AI がいま何かしているか。**「AI を止める」を出していいかの判断。
	 *
	 * 握っていないし何も走っていないときに押せるボタンを出すと、
	 * **押しても何も起きません。**そこは「止まった」と読まれるので、
	 * **出さない**より悪いです。
	 */
	const aiIsDoing = $derived(holder === 'ai' || session.busy);

	/**
	 * **いま面に映っている接続**（D60・2026-10-02）。
	 *
	 * 出力は接続ごとに分かれているので、**どの接続の分を映すのか**を
	 * 1 か所で決めます。繋がっていない間は、繋ぐ前の分（接続の失敗など）。
	 */
	const shownConnection = $derived(session.open?.id ?? BEFORE_CONNECTING);

	/** 出力の塊 1 つ。**どこのものか**が添えてあります（D60 / 分割）。 */
	type StreamChunk = { connection: string | null; console: number | null; chunk: number[] };

	/**
	 * 端末ごとの控えの鍵。**接続の識別子とぶつからない形**にします ——
	 * ぶつかると、端末の出力が *出力* の面へ混ざります。
	 */
	function consoleKey(id: number): string {
		return `console:${id}`;
	}

	/**
	 * **いま何枚、どれを描くか**（分割・2026-10-06）。
	 *
	 * 規則は `console-panes.ts` に在ります（画面を描かずに確かめられる形）。
	 */
	const layout = $derived(
		paneLayout(consolesOpen, session.open?.id ?? null, chosenConsole)
	);

	/*
	 * **ここに在った「端末はいまここに無い」の帯は、2026-10-02 に外しました**（D60）。
	 *
	 * 帯が言っていたのは「端末は全体で 1 本なので、タブを移しても付いてこない」で、
	 * **端末を接続ごとに持てるようにしたので、その状態は起こりません。**
	 * `consoleOn` は宛先の端末を指すので、見ている接続と食い違いません。
	 *
	 * **まだ出していないもの**: AI が別の接続の端末を握っているとき、
	 * どこを握っているか。`console_list` は実行体に在りますが、画面へは出していません
	 * （D60 の次の一手 ——「人がどの面を見ているか」と一緒にやります）。
	 */

	/**
	 * **端末の面から、別の接続を開く**（実機の指摘・2026-10-04）。
	 *
	 * > **端末画面に、別の接続を開くがないです。**
	 *
	 * ファイルの面には在って、端末の面には無かった。端末の面に並ぶのは
	 * **既に繋がっているもの**だけで、1 本繋いだ時点で選ぶ所が消えます ——
	 * **2 台目へ繋ぐには、いちど別の面へ移るしかありませんでした。**
	 *
	 * Issue #9 と同じ壊れ方です。**「できるようになった後にだけ現れる案内」**は、
	 * 探している人には見えません。
	 */
	let consoleAddingAnother = $state(false);
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
	let detachOutput: Array<() => void> = [];

	// --- 検索 --------------------------------------------------------------------
	// `⌘F` / `Ctrl+Shift+F`。**素の `Ctrl+F` は端末の「1 文字進む」なので取りません。**
	type SearchPane = 'console' | 'output';
	let searchOpen = $state<SearchPane | null>(null);
	let searchTerm = $state('');
	/** 打った語が無かったか。**「押したのに何も起きない」を作らないため。** */
	let searchMissed = $state(false);
	let searchInput: HTMLInputElement | undefined = $state();
	/**
	 * **面ごとの検索の取っ手**（分割）。
	 *
	 * 検索の窓は 1 つで、面は 2 枚あります。**探すのは、いま打鍵が行く面**です ——
	 * 見ていない面を探し始めると、人は「何も見つからない」と読みます。
	 */
	let consoleSearches = $state(new Map<number, TerminalSearch>());
	/**
	 * **面ごとの「書く口」**（2026-10-07 の回帰）。
	 *
	 * **これが無いと、開いたままの面には何も出ません。**控えに入れるだけでは、
	 * 面は**作られたときの 1 回**しか描かず、**そのあと流れてくる分が落ちます。**
	 * タブを切り替えると出るのは、作り直しで控えを書き戻しているからです。
	 *
	 * 実運用でこう踏みました ——
	 *
	 * > リアルタイムでコマンドを打てたのが分からず、画面を切り替えないと反映されない
	 *
	 * **AI が `rm` を打っても人の画面は静かなまま**で、
	 * 「人が常に見ている」（D29）が崩れていました。
	 */
	let consoleWriters = new Map<number, (chunk: number[]) => void>();
	let outputSearch: TerminalSearch | undefined;

	function searchFor(pane: SearchPane): TerminalSearch | undefined {
		if (pane === 'output') return outputSearch;
		return layout.focused === null ? undefined : consoleSearches.get(layout.focused);
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
	/**
	 * **ここに在った 1 枚分の配線は、`ConsolePane.svelte` へ移しました**
	 * （分割・2026-10-06）。
	 *
	 * xterm を作る・書き戻す・打鍵を送る・窓に追従させる・検索とクリップボードを
	 * 付ける —— 全部ここに在ったので、**2 枚にするには写すしかありませんでした。**
	 * 写した日に、片方だけ直る日が来ます（D39）。
	 *
	 * 部品にしたので、**端末の面を描いて確かめるテストが初めて書けました**
	 * （`ConsolePane.svelte.test.ts`・10 本）。
	 */

	$effect(() => {
		const host = terminalHost;
		if (!host) return;
		if (!terminal) {
			terminal = createTerminal(host, textSize.terminalPx);
			// **こちらも書き戻す。**同じ 1 本の出力を見ている面なので、
			// 片方だけ戻ると「どちらが本当か」が分からなくなります。
			writeChunk(terminal, replayFor(backlogs.all, shownConnection));
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

	/**
	 * **端末の面を開いたら、端末も開く**（実機の指摘・2026-09-06）。
	 *
	 * > 接続のあと［端末を開く］を押さんといけないのはなぜ？
	 *
	 * **押させる必然性は D29 のどこにも書いていませんでした。**
	 * 「開く → 握る → 打つ」と作って、最初の一歩を誰も疑わなかっただけです。
	 *
	 * **AI が握っているときは奪いません**（そこは［取り返す］のまま）。
	 * 一度失敗したら繰り返しません — **押せない釦を自動で押し続けない。**
	 */
	let autoOpenTried = $state(false);
	$effect(() => {
		if (view !== 'console') return;
		if (session.open === null) return;
		if (holder !== null || autoOpenTried || session.busy) return;
		autoOpenTried = true;
		void openConsole();
	});

	// **繋ぎ直したら、また自動で開ける。**1 回きりにすると、
	// 切って繋いだ人が押す所を探すことになります。
	$effect(() => {
		if (session.open === null) autoOpenTried = false;
	});

	/**
	 * **いま何が見えているかを、実行体へ言う**（PRD §4-0・2026-10-07）。
	 *
	 * 言わないと、AI は**宛先の接続から推測するしかありません。**
	 * 推測は、同じサーバに 2 枚開いた日に必ず外れます。
	 *
	 * **端末の面を離れたら空を言います。**人がファイルの面を見ているなら、
	 * 端末は 1 枚も見えていません。**それを AI が知れることが「同じ視点」です。**
	 */
	$effect(() => {
		const shown = view === 'console' ? layout.panes.map((pane) => pane.id) : [];
		const typingInto = view === 'console' ? layout.focused : null;
		invoke('console_on_screen', { shown, typingInto }).catch(() => {
			/* 言えないだけ。**画面は止めません。** */
		});
	});

	/**
	 * **宛先が変わったら、面を描き直す**（D60・2026-10-02）。
	 *
	 * 描き直さないと、**前の接続の出力の下に次の接続の出力が続いて**出ます。
	 * 人は境目が分からず、**どの台で打ったのかを取り違えます** ——
	 * `rm` を打つ相手を取り違えるのが、この製品で一番怖い事故です。
	 *
	 * 消してから、その接続の控えを書き戻します。**消すだけにしません** ——
	 * 戻ってきたタブが白紙だと、人は「落ちた」と読みます。
	 */
	$effect(() => {
		const on = shownConnection;
		const back = replayFor(backlogs.all, on);
		// **端末の面は部品が持ちます**（面ごとに控えが違うので、ここでは触りません）。
		if (!terminal) return;
		terminal.reset();
		writeChunk(terminal, back);
	});

	$effect(() => {
		const px = textSize.terminalPx;
		if (terminal) terminal.options.fontSize = px;
		// **端末の面は部品が持ちます**（`fontPx` を渡しています）。
	});

	/**
	 * `⌘ +` / `⌘ −` / `⌘ 0`。**この操作は覚えなくても手が知っている。**
	 *
	 * WebView 側の既定の拡大は効かない（アプリなので）。
	 * 押しても何も起きないより、**期待どおりに動く方**へ寄せる。
	 */
	function onKeydown(event: KeyboardEvent) {
		const target = event.target;
		const typing = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

		/*
		 * **画面ぜんたいのキー操作**（UI 案・2026-10-21）。
		 *
		 * **ここは飾りの検査より前に置きます。**`F1` は Ctrl も ⌘ も押さないので、
		 * 下の早期 return を通ると**永久に届きません。**
		 *
		 * 判定そのものは `app-keys.ts` です。**端末の担当と取り合っていないこと**を
		 * そこで数えています（`Ctrl Shift C` / `V` / `F` は端末のもの）。
		 */
		const app = appIntent(
			{
				key: event.key,
				ctrlKey: event.ctrlKey,
				metaKey: event.metaKey,
				shiftKey: event.shiftKey,
				altKey: event.altKey,
				inField: typing
			},
			platform
		);
		if (app) {
			event.preventDefault();
			runApp(app);
			return;
		}

		if (!(event.metaKey || event.ctrlKey)) return;

		/*
		 * **入力欄の切り取り・コピー・貼り付け・全選択を、自前で受ける**（実機の指摘）。
		 *
		 * > パスを win で Ctrl+V で打ててなかったように思う
		 *
		 * 自前タイトルバー（D17）で `decorations: false` にすると、
		 * **Windows ではメニューバーごと消えます。**メニューに載せていた
		 * 貼り付けの割り当ても一緒に消えるので、**Ctrl+V が効きませんでした。**
		 * D17 の記録には「WebView が標準で持っている」と書きましたが、
		 * **実機で確かめずに書いた一文**でした。
		 *
		 * **パスフレーズが入れられない ＝ 繋げない**ので、ここは一番手前の詰まりです。
		 * **端末の上では手を出しません**（素の Ctrl+C は「走っているものを止める」）。
		 */
		const inField = typing;
		const intent = editIntent(
			{
				key: event.key,
				ctrlKey: event.ctrlKey,
				metaKey: event.metaKey,
				shiftKey: event.shiftKey,
				inField
			},
			platform
		);
		if (intent && inField) {
			event.preventDefault();
			void applyEdit(intent, target as unknown as EditableField, clipboard).catch(
				(error: unknown) => {
					// **握り潰さない。**押したのに何も起きない、を作らない。
					failure = String(error);
				}
			);
			return;
		}

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
		try {
			// **大きさは部品が窓に追従して直します**（`attachFit`）。目安だけ渡します。
			await invoke('console_open', { cols: 80, rows: 24 });
			holder = 'human';
			await refreshConsoles();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/**
	 * **もう 1 枚、同じサーバに開く**（分割・2026-10-06）。
	 *
	 * > 同じサーバに２画面入る場合もある
	 *
	 * `console_open` は「在るなら握り直す」です（Issue #21 —— 握りを渡すたびに
	 * PTY を立て直すと、人の `su -` が消えます）。**こちらは毎回新しいシェル。**
	 */
	async function openAnotherConsole() {
		const on = session.open?.id;
		if (!on) return;
		try {
			const id = await invoke<number>('console_open_another', {
				connectionId: on,
				cols: 80,
				rows: 24
			});
			// **開いた面を、そのまま見せる。**開いたのに見えないのは、押せない釦と同じ。
			chosenConsole = id;
			await refreshConsoles();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 開いている端末を読み直す。**押し出しを取りこぼしたときの保険。** */
	async function refreshConsoles() {
		try {
			consolesOpen = await invoke<ConsolePane[]>('console_list');
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 握りを取り返す。**人は常に勝ちます**（D29）。 */
	/**
	 * **人の側から AI へ渡す**（案・1 枚目「AI に渡す」）。
	 *
	 * それまで握りが AI へ移る道は 1 本だけでした —— AI が頼み、人が許す。
	 * 実機の指摘（2026-09-06）——
	 * 「AI に渡すときに［止める］を押さないといけません」。
	 *
	 * **問い直しません。**押したことが許可です（実行体も `ask_first` を通しません）。
	 */
	async function giveConsole() {
		failure = null;
		try {
			await invoke('console_give');
		} catch (error: unknown) {
			// **握り潰さない。**渡せなかったのに渡ったように見せない。
			failure = String(error);
		}
	}

	async function takeConsole() {
		try {
			await invoke('console_take');
			holder = 'human';
			await refreshConsoles();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 止める。**失敗しません**（D29 の停止ボタン）。**番号で止めます。** */
	async function stopConsole() {
		// **いま見ている面を止める。**肩の［止める］は面の番号を直接渡します。
		if (layout.focused !== null) await stopConsoleOf(layout.focused);
	}

	/**
	 * **その面を止める**（UI 案・2026-10-21 で面の肩に置いた）。
	 *
	 * **番号で止める。**接続の名前で止めると、同じサーバに 2 枚ある日に
	 * **どちらが止まるか言えません。**
	 */
	async function stopConsoleOf(id: number) {
		try {
			await invoke('console_stop_of', { consoleId: id });
			holder = null;
			await refreshConsoles();
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

	// --- 更新の札（D34 の追記 3）------------------------------------------------

	/**
	 * 変更内容の全文を開いている版。**版そのものを持ちます。**
	 *
	 * 真偽値だと、次の版の札が出たときに**前の版で開いた状態が残ります。**
	 */
	let notesOpenFor = $state('');

	/** 札に出す変更内容。**先頭 3 行と、残りの件数。** */
	const notes = $derived(updater.state.kind === 'found' ? summarise(updater.state.notes) : null);

	const notesOpen = $derived(
		updater.state.kind === 'found' && notesOpenFor === updater.state.version
	);

	/**
	 * 再起動で切れるもの。**繋がっている本数と、端末が開いているか。**
	 *
	 * エディタや DB クライアントの再起動と違い、ここで落ちるのは
	 * 生きている SSH と端末、そして **MCP が共有している同じ 1 本**です。
	 * **止めはしません。**止めるかどうかは人が決めることで、
	 * こちらの仕事は「知らないまま押させない」ことです。
	 */
	const cost = $derived(
		cutSummary({
			sessions: session.all.length,
			terminalOpen: consoleOn !== null,
			// **AI の目も切れる**（UI 案・2026-10-21／禁止事項 3）。
			// GUI と MCP は同じ 1 本を共有しているので、立ち上げ直すと
			// AI が見ているものも落ちます。**口が立っているかで判断します** ——
			// 失敗していなければ立っている（`mcpFailure === null`）。
			mcpOpen: mcpFailure === null
		})
	);

	/** 札を出すか。**静かなときは静かでいる。** */
	const showUpdate = $derived.by(() => {
		if (updater.folded) return false;
		const kind = updater.state.kind;
		if (kind === 'idle') return false;
		// 「調べています」と「最新です」は、**人が押したときだけ**出します。
		if (kind === 'checking' || kind === 'none') return updater.manual;
		return true;
	});

	$effect(() => {
		// **入れ終わった時点で数え直す。**古い数のまま「3 本が切れます」と言わない
		// （Issue #8 で session が実態からずれた実績があります）。
		if (updater.state.kind !== 'ready') return;
		session.refresh().catch(() => {
			/* 取り直せなくても、いま持っている数で言う。黙るよりまし。 */
		});
	});

	function toggleNotes(version: string): void {
		notesOpenFor = notesOpenFor === version ? '' : version;
	}

	onMount(() => {
		// **立ち上げ直したあと、1 回だけ聞く**（UI 案・5 枚目）。
		// `take` は読んだら消すので、起動のたびに同じことを聞かれません。
		reopenAsk = takeReopen();
		// **起動直後の取りこぼし対策。**購読だけだと、前に動いた分が出ません。
		invoke<TransferLine[]>('transfers_recent')
			.then((lines) => {
				transfers = lines;
			})
			.catch(() => {});

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
		// **AI からの頼みを受ける**（D42）。**起動直後の取りこぼしも拾う。**
		invoke<string | null>('console_request')
			.then((asked) => {
				consoleAsk = asked;
			})
			.catch(() => {
				/* 取れなくても画面は出す */
			});
		// **走らせてよいかの問い**（D45 / D47）。起動直後の取りこぼしも拾う。
		invoke<OperationAsk | null>('operation_request')
			.then((asked) => {
				operationAsk = asked;
			})
			.catch(() => {
				/* 取れなくても画面は出す */
			});
		invoke<OperationProposal | null>('operation_proposal')
			.then((asked) => {
				operationProposal = asked;
			})
			.catch(() => {});
		listen<TransferLine>('transfer://line', (event) => {
			// **上限は Rust 側が持っています**（200 件）。ここでは足すだけ。
			transfers = [...transfers, event.payload];
		}).then((off) => stops.push(off));
		listen<OperationProposal | null>('operation://proposal', (event) => {
			operationProposal = event.payload;
		}).then((off) => stops.push(off));
		listen<OperationAsk | null>('operation://request', (event) => {
			operationAsk = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		// **パスフレーズの頼み**（Issue #13）。起動直後の取りこぼしも拾う。
		// **いまの選び方を読む**（Issue #30）。読めなければ既定のまま。
		invoke<AttentionHow>('attention_how')
			.then((how) => (attention = how))
			.catch(() => {
				/* 読めないだけ。**既定（点滅）のまま出します。** */
			});

		invoke<string | null>('passphrase_request')
			.then((waiting) => {
				passphraseFor = waiting;
			})
			.catch(() => {
				/* 取れなくても画面は出す */
			});
		// **ホスト鍵の頼み**（Issue #26）。起動直後の取りこぼしも拾う。
		invoke<typeof hostKeyAsk>('host_key_request')
			.then((waiting) => {
				hostKeyAsk = waiting;
			})
			.catch(() => {
				/* 取れなくても画面は出す */
			});
		listen<typeof hostKeyAsk>('host-key://request', (event) => {
			hostKeyAsk = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		// **AI が「こちらを見てください」と言ってきた**（D44 / Issue #15）。
		// **奪わせません。**人が何かしている最中は、頼まれても動きません。
		listen<string>('view://show', (event) => {
			const wanted = event.payload;
			if (!isView(wanted)) return;
			// **問いが出ている間は動かさない。**動かすと、人は何に答えたのか分からなくなります。
			const askOpen =
				passphraseFor !== null ||
				consoleAsk === 'ai' ||
				operationAsk !== null ||
				aboutOpen ||
				showUpdate;
			const active = document.activeElement;
			const focusInField =
				active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement;
			if (!canFollow({ askOpen, focusInField, sinceHumanSwitchMs: Date.now() - humanSwitchedAt }))
				return;
			view = wanted as View;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		listen<string | null>('passphrase://request', (event) => {
			passphraseFor = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		listen<string | null>('console://request', (event) => {
			consoleAsk = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		// **何枚描けばよいか**を受け取る（分割）。
		// 押し出しを取りこぼしても、`refreshConsoles` が保険になります。
		listen<ConsolePane[]>('console://list', (event) => {
			consolesOpen = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});

		listen<ConsoleState>('console://holder', (event) => {
			holder = event.payload.holder;
			consoleOn = event.payload.connection;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 購読できないだけ。**画面は出す。** */
			});


		// **ANSI を落とさずに渡す。**色は人の側にだけ残す（Issue 005）。
		listen<StreamChunk>('stream://raw', (event) => {
			// **端末の面へも書く**（Issue #10）。
			//
			// ここは長らく `terminal`（*出力* の面）にしか書いていませんでした。
			// **端末の面は別の xterm（`consoleTerm`）で、書く行がどこにもなく、
			// 入って以来 1 バイトも表示していません**（`e8a485c` / 2026-09-01）。
			// しかも `terminal` は *出力* の面が開かれるまで作られないので、
			// 端末タブだけを開いている間は `if (terminal)` が毎回素通りします。
			//
			// **出力は MCP と共有するもの**なので（PRD §4-1）、`tail -f` を
			// 走らせている間はその出力も端末の面に混ざります。**まず映すこと**を
			// 採りました — 混ざるより、何も出ない方が悪い。
			// 出どころで振り分ける案は `decisions.md` に出してあります。
			//
			// **接続ごとに分かれています**（D60）。どの接続の出力かが添えてあり、
			// **いま見ている接続の分だけ**を面へ書きます。
			// 書き分けないと、2 台の出力が 1 つの面で混ざります。
			// **端末のものは、その面へだけ**（分割・2026-10-06）。
			// 接続ごとの口には 2 枚分が混ざって流れるので、
			// **番号が入っているものは番号で分けます。**
			const from =
				event.payload.console === null
					? keyFor(event.payload.connection)
					: consoleKey(event.payload.console);
			// **面が無くても覚えておく**（Issue #14）。
			// ここを通らないと、別のタブに居る間の分が丸ごと消えます。
			backlogs.all = rememberFor(backlogs.all, from, event.payload.chunk);
			// ***出力* の面は、接続ごとのものだけ**を映します（端末の分は端末の面へ）。
			if (event.payload.console === null) {
				if (from === shownConnection && terminal) {
					writeChunk(terminal, event.payload.chunk);
				}
				return;
			}
			// **端末の分は、その面へその場で書きます。**
			// **控えに入れるだけにしません** —— 開いたままの面に何も出なくなります
			// （2026-10-07 の回帰。`rm` を打っても人の画面が静かなままでした）。
			consoleWriters.get(event.payload.console)?.(event.payload.chunk);
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
		<!--
			**ロゴがメニューの口**（UI 案・2026-10-21）。
			案はここに全部まとめました —— 表示（文字の大きさ・テーマ・言語）、
			ヘルプ、診断、このアプリについて。`Ctrl ,` でも開きます。

			印は端末の枠に、打っている矢印と行。**この道具が何なのかを 22px で言う。**
			飾りなので読み上げからは外します（隣に製品名が文字で在る）。
		-->
		<div class="view-menu">
			<button
				type="button"
				class="brand"
				aria-haspopup="true"
				aria-expanded={viewMenuOpen}
				onclick={() => (viewMenuOpen = !viewMenuOpen)}
				title={`${menu['menu.view']}（${platform === 'mac' ? '⌘' : 'Ctrl'} ,）`}
			>
				<svg
					class="brand-mark"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.8"
					stroke-linecap="round"
					aria-hidden="true"
				>
					<rect x="3" y="4" width="18" height="16" rx="4"></rect>
					<path d="M7 10l3 2-3 2M12 15h5"></path>
				</svg>
				<span class="brand-name">sshboard</span>
				<svg
					class="brand-chevron"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					aria-hidden="true"
				>
					<path d="M6 9l6 6 6-6"></path>
				</svg>
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

					<!--
						**問いに気づけるように**（Issue #30）。
						既定は点滅だけ。**前面強制は選んだ人だけ** ——
						報告者自身が「人によっては邪魔」と書いています。
					-->
					<div class="view-row">
						<span class="view-label">{i18n.t('attention.label')}</span>
						<div class="view-controls">
							{#each ['flash', 'front', 'off'] as const as how (how)}
								<button
									type="button"
									class:active={attention === how}
									title={i18n.t(`attention.${how}.what`)}
									onclick={() => void setAttention(how as AttentionHow)}
								>
									{i18n.t(`attention.${how}`)}
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

					<!--
						**ここから下は、ヘッダーから移した分**（UI 案・2026-10-21）。
						案はロゴの下に「このアプリ」の群を置きました ——
						ヘルプ・診断・このアプリについて。

						**［?］はヘッダーに残します。**案もそうしており、
						**押し方を探している人がメニューを開かなくていい**ためです
						（Issue #9 —— 触れる前の人には、隠れているものは無いのと同じ）。
					-->
					<div class="view-row">
						<!--
							**見出しと項目が同じ言葉にならないように**（2026-10-23）。
							`menu.about` は OS のメニューの「sshboard について」で、
							**群の見出しに使うと、中の項目と二重に出ます。**
							案は群を「このアプリ」と呼んでいます。
						-->
						<span class="view-label">{i18n.t('app.group')}</span>
						<div class="view-controls">
							<button type="button" onclick={() => { viewMenuOpen = false; keyHelpOpen = true; }}>
								{i18n.t('help.open')}
							</button>
							<button type="button" onclick={() => { viewMenuOpen = false; humanShows('diag'); }}>
								{i18n.t('tab.diag')}
							</button>
							<!--
								**AI が何を走らせられるか**（2026-10-23）。
								本番で、人は書く場所を探して見つけられませんでした ——
								**置き場と中身が見える所**をここに置きます。
							-->
							<button type="button" onclick={() => void openAllowlist()}>
								{i18n.t('allow.title')}
							</button>
							<!--
								**帯を書き出す**（案・4 枚目）。
								押す前に何が入るかを見せます ——
								**囲いは落とし先を守りません。**
							-->
							<button
								type="button"
								onclick={() => {
									viewMenuOpen = false;
									bandExportOpen = true;
								}}
							>
								{i18n.t('band.export.title')}
							</button>
							<button type="button" onclick={() => { viewMenuOpen = false; aboutOpen = true; }}>
								{i18n.t('about.title')}
							</button>
						</div>
					</div>
				</div>
			{/if}
		</div>

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
				onclick={() => humanShows('connections')}
			>
				<Icon name="server" />
				{i18n.t('tab.connections')}
			</button>
			<button type="button" class:active={view === 'files'} onclick={() => humanShows('files')}>
				<Icon name="folder" />
				{i18n.t('tab.files')}
			</button>
			<button type="button" class:active={view === 'console'} onclick={() => humanShows('console')}>
				<Icon name="terminal" />
				{i18n.t('tab.console')}
			</button>
			<button type="button" class:active={view === 'band'} onclick={() => humanShows('band')}>
				<Icon name="activity" />
				{i18n.t('tab.band')}
			</button>
			<button type="button" class:active={view === 'diag'} onclick={() => humanShows('diag')}>
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
		<!--
			**画面を伏せる**（UI 案・2026-10-21／D26）。
			**この製品は画面そのものに接続先が写り込みます**（禁止事項 4）。
			画面共有や撮影の前に、人が 1 回押せることが要ります。
			伏せている間は**地と字を入れ替えて**、押したままなのが分かるようにします。
		-->
		<button
			type="button"
			class="mask-toggle"
			class:on={redaction.on}
			onclick={toggleMask}
			title={`${i18n.t('mask.hide')}（${platform === 'mac' ? '⌘' : 'Ctrl'} Shift H）`}
			aria-pressed={redaction.on}
		>
			<Icon name={redaction.on ? 'eye-off' : 'eye'} size={12} />
			<!-- 狭い窓（既定の 1100px・英語）では短い言い方に。意味は同じ。 -->
			<span class="label-long">{redaction.on ? i18n.t('mask.on') : i18n.t('mask.hide')}</span>
			<span class="label-short">{redaction.on ? i18n.t('mask.on.short') : i18n.t('mask.hide.short')}</span>
		</button>

		<!--
			**AI を止める**（UI 案・2026-10-21）。
			止めるのは**いま走っているもの**（流れている出力と、AI が握っている端末）。
			**何もしていないときは押せません** —— 空振りを「止めた」と見せない。
		-->
		<button
			type="button"
			class="stop-ai"
			onclick={() => (stopAiAsking = true)}
			disabled={!aiIsDoing && mcp === null}
			title={aiIsDoing
				? `${i18n.t('ai.stop')}（${platform === 'mac' ? '⌘' : 'Ctrl'} Shift .）`
				: i18n.t('ai.stop.none')}
		>
			<span class="stop-ai-mark" aria-hidden="true"></span>
			<span class="label-long">{i18n.t('ai.stop')}</span>
			<span class="label-short">{i18n.t('ai.stop.short')}</span>
		</button>

		<!--
			**キー操作はここからも開ける**（UI 案・2026-10-21）。
			**ショートカットだけだと、知らない人には無いのと同じ** ——
			Issue #9 で一度やりました。`F1` も効きます。
		-->
		<button
			type="button"
			class="about-open"
			onclick={() => (keyHelpOpen = true)}
			title={`${i18n.t('help.keys')}（F1）`}
			aria-label={i18n.t('help.keys')}
		>
			?
		</button>



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
			<span class="mcp-text">{#if mcpFailure}{i18n.t('mcp.failed', { port: String(mcpFailure.port) })}{:else if !mcp}{i18n.t('mcp.label')} {i18n.t(
					'mcp.starting'
				)}{:else if mcpCopied}{i18n.t('mcp.label')} {i18n.t('mcp.copied')}{:else}{i18n.t('mcp.copy')}{/if}</span>
			<!-- 狭い窓での言い方。**「MCP」は固有名なので訳さない。** -->
			<span class="mcp-short">MCP</span>
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
		更新の知らせ（D34 ＋ 追記 3）。**黙って入れ替えません。**押すのは人。

		**本文の流れに埋めない。**細い帯として並べていたら、
		**そこにあると気づかれませんでした**（実機・2026-09-03）。
		画面の隅へ浮かせ、影を付けて、他と違う面として見せます。
		ただし**画面を塞ぐ窓にはしません** — 作業中に手を止めさせる話ではないので。

		「調べている」「最新です」は、**人が『更新を確認』を押したときだけ**出します。
		起動時の 1 回は、何も無ければ静かなまま — **静かなときは静かでいる。**
		押されたのに何も返さない、も作りません（`showUpdate`）。
	-->
	{#if showUpdate}
		<aside class="update-toast" role="status" aria-live="polite">
			{#if updater.state.kind === 'checking'}
				<span class="toast-icon"><Icon name="download" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.checking')}</strong>
				</div>
			{:else if updater.state.kind === 'none'}
				<!-- **押したのに何も起きない、を作らない。**最新なら最新と返す。 -->
				<span class="toast-icon"><Icon name="check" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.uptodate', { version: updater.state.version })}</strong>
					<div class="toast-actions">
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('about.close')}
						</button>
					</div>
				</div>
			{:else if updater.state.kind === 'found'}
				<span class="toast-icon"><Icon name="download" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.found', { version: updater.state.version })}</strong>
					{#if notes && notes.head.length > 0}
						<!--
							**押す材料を渡す。**版番号だけを見せて「はい」を押させるのは判断ではない。
							記号は `notes.ts` で剥がし、長さも行数もそこで切っています。
							**既定の 3 行はスクロールしません。**中で切れると、切れたことに気づけない。
						-->
						<ul class="toast-notes" class:open={notesOpen}>
							{#each notesOpen ? notes.lines : notes.head as line}
								<li>{line}</li>
							{/each}
						</ul>
						{#if notes.rest > 0}
							<div class="toast-actions">
								<button
									type="button"
									class="quiet"
									aria-expanded={notesOpen}
									onclick={() => toggleNotes(updater.state.kind === 'found' ? updater.state.version : '')}
								>
									{notesOpen
										? i18n.t('update.notes.less')
										: i18n.t('update.notes.more', { count: String(notes.rest) })}
								</button>
							</div>
						{/if}
					{/if}
					<div class="toast-actions">
						<button type="button" class="cta" onclick={() => updater.install()}>
							{i18n.t('update.install')}
						</button>
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
						<!-- **同じ札を毎日出さない。**読まれない警告になる（D30 で毎日上げる）。 -->
						<button type="button" onclick={() => updater.skip()}>
							{i18n.t('update.skip')}
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
					<!--
						**逃げ道を残す。**ここにボタンが 1 つも無いと、止まったときに手が無い。
						「中止」とは名乗りません — 中断できるか確かめていないので、
						**押しても止まらないボタンを作らない。**裏では落ち続け、
						入れ終われば自分から出直します。
					-->
					<div class="toast-actions">
						<button type="button" onclick={() => updater.fold()}>
							{i18n.t('update.fold')}
						</button>
					</div>
				</div>
			{:else if updater.state.kind === 'ready'}
				<span class="toast-icon"><Icon name="check" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t('update.ready', { version: updater.state.version })}</strong>
					{#if cost}
						<!--
							**この製品の再起動は、他のアプリの再起動と重さが違う。**
							生きている SSH・開いている端末・転送中のファイル、
							そして MCP が共有している同じ 1 本（CLAUDE.md 禁止事項 3）が落ちます。
							**止めはしません。知らないまま押させないだけです。**
						-->
						<p class="toast-cut">
							{i18n.t(cutMessageKey(cost), { count: String(cost.sessions) })}
							{#if cost.mcp}
								<!--
									**AI は自動で繋ぎ直しません**（UI 案・2026-10-21）。
									黙っていると「勝手に戻る」と思われ、戻っていないことに
									気づかないまま作業が進みます。
									**裏で見えない SSH を張らない**（禁止事項 3）ので、
									繋ぎ直すのは AI からもう一度呼ばれたときです。
								-->
								<span class="ai-note">{i18n.t('update.ai.note')}</span>
							{/if}
						</p>
					{/if}
					<div class="toast-actions">
						<button
							type="button"
							class="cta"
							onclick={() => {
								// **立ち上げ直す前に控える**（UI 案・5 枚目）。
								// 控えるのは識別子だけで、**勝手には繋ぎ直しません。**
								updater.rememberOpen(session.all.map((held) => held.id));
								void updater.restart();
							}}
						>
							{i18n.t('update.restart')}
						</button>
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
					</div>
				</div>
			{:else if updater.state.kind === 'failed'}
				<!--
					**黙らない。**繋がらないのか署名が合わないのかで、人の次の一手が違う。
					**生の文字列は出しません**（URL やパスが入りうる・D26）。
					中身は開発者コンソールへ落としています。
				-->
				<span class="toast-icon warn"><Icon name="warning" size={16} /></span>
				<div class="toast-body">
					<strong>{i18n.t(updater.state.messageKey)}</strong>
					<div class="toast-actions">
						<button type="button" class="cta" onclick={() => updater.check(true)}>
							{i18n.t('update.retry')}
						</button>
						<button type="button" onclick={() => updater.dismiss()}>
							{i18n.t('update.later')}
						</button>
					</div>
				</div>
			{/if}
		</aside>
	{/if}

	<!--
		**AI が端末を使いたいと言っている**（D42）。
		人が答えるまで、AI は握れません。**部品を作って繋がない、をやらない**
		（Issue #10 がまさにそれでした）。
	-->
	<!--
		**AI が繋ごうとして、パスフレーズで止まった**（Issue #13）。
		問いは `ConnectPanel` の中にしか無く、**MCP の経路から出す口が
		ありませんでした。**部品はそのまま使います —— 問いを 2 か所に作ると、
		片方だけ直る日が来ます（D39 と同じ理由）。
	-->
	<!--
		**AI が状態を変える操作を走らせたい**（D45 / D47）。
		**何が走るのかを見せてから聞きます** —— id だけでは答えられません。
	-->
	{#if operationAsk}
		<OperationRequestDialog
			id={operationAsk.id}
			runs={operationAsk.runs}
			needsSecret={operationAsk.needsSecret}
			onAllow={(secret) => answerOperation(true, secret)}
			onDeny={() => answerOperation(false)}
		/>
	{/if}

	{#if passphraseFor}
		<PassphraseDialog
			id={passphraseFor}
			busy={passphraseBusy}
			onSubmit={answerPassphrase}
			onCancel={dismissPassphrase}
		/>
	{/if}

	{#if hostKeyAsk}
		<HostKeyDialog
			id={hostKeyAsk.id}
			algorithm={hostKeyAsk.algorithm}
			fingerprint={hostKeyAsk.fingerprint}
			expected={hostKeyAsk.expected}
			busy={hostKeyBusy}
			onTrust={() => answerHostKey(true)}
			onRefuse={() => answerHostKey(false)}
		/>
	{/if}

	{#if consoleAsk === 'ai'}
		<ConsoleRequestDialog
			onAllow={() => answerConsole(true)}
			onDeny={() => answerConsole(false)}
		/>
	{/if}

	{#if aboutOpen}
		<AboutDialog onClose={() => (aboutOpen = false)} />
	{/if}

	<!--
		**キー操作の一覧**（UI 案・2026-10-21）。`F1` で開きます。
		表は `app-keys.ts` が持っており、**実物とずれていないことを試験が数えます**
		（表からキーを組み立てて `appIntent` に通す）。
	-->
	{#if keyHelpOpen}
		<KeyHelpDialog {platform} onclose={() => (keyHelpOpen = false)} />
	{/if}

	{#if bandExportOpen}
		<BandExportDialog
			{lines}
			masked={redaction.on}
			onclose={() => (bandExportOpen = false)}
			onsave={(payload) => void saveBand(payload)}
		/>
	{/if}

	{#if allowlistOpen && allowlist !== null}
		<AllowlistDialog
			reading={allowlist.reading}
			changing={allowlist.changing}
			onclose={() => (allowlistOpen = false)}
		/>
	{/if}

	<!--
		**AI が「この操作を足してよいか」と出した提案**（実運用の報告・2026-10-23）。
		**コマンドを 1 文字も省略せずに出します** —— 読めない同意は、同意ではありません。
	-->
	<!--
		**止める前に 1 回聞く**（2026-10-23）。
		口を閉じると、**戻すにはアプリを立ち上げ直します** ——
		押し間違いの結果が他と違うので、確かめます（product-baseline §13）。
	-->
	<!--
		**立ち上げ直したあとの繋ぎ直し**（UI 案・5 枚目）。
		**勝手には繋ぎません** —— 人が見ていない所で SSH が開くのを作らない（禁止事項 3）。
		**AI の分も繋ぎ直しません**（AI のほうから呼ばれたときに繋がります）。
	-->
	{#if reopenAsk}
		<ConfirmDialog
			title={i18n.t('reopen.title')}
			body={i18n.t('reopen.what', { ids: reopenAsk.ids.join(' , ') })}
			yes={i18n.t('reopen.yes')}
			no={i18n.t('reopen.no')}
			onyes={() => void reopenThem(reopenAsk!.ids)}
			onno={() => {
				forgetReopen();
				reopenAsk = null;
			}}
		/>
	{/if}

	{#if stopAiAsking}
		<ConfirmDialog
			title={i18n.t('ai.stop.title')}
			body={i18n.t('ai.stop.what')}
			yes={i18n.t('ai.stop.yes')}
			no={i18n.t('propose.no')}
			onyes={() => {
				stopAiAsking = false;
				void stopAi();
			}}
			onno={() => (stopAiAsking = false)}
		/>
	{/if}

	{#if operationProposal}
		<OperationProposalDialog
			proposal={operationProposal}
			onregister={() => void registerProposal(operationProposal!)}
			ondismiss={() => void dismissProposal()}
		/>
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
			<!--
				**繋いでいないなら、繋ぐ所をここに出す**（実機の指摘・2026-09-04）。
				前は押せない［端末を開く］と、いちばん小さい字の「先にサーバーへ繋ぐ →」が
				並んでいて、しかもその行き先は *接続*（登録の画面）でした。
				**そこに繋ぐボタンはありません。**「どのように接続しますか？」と
				聞かれたのは、この行き止まりです。

				`ConnectPanel` はファイルの面と**同じ 1 つの部品**です。
			-->
			{#if session.open === null}
				<ConnectPanel onregister={() => (view = 'connections')} />
				<!--
					**1 本のときも出す**（Issue #9）。以前は `> 1` で、
					**2 本目が開いて初めて現れました。**
					つまり「2 台目に繋げるのか」を探している人には絶対に見えず、
					**できるようになった後にだけ現れる案内**になっていました。
					1 本でも出せば、どのサーバーの端末なのかが常に見えます。
				-->
			{/if}
			<!--
				**3 つの列**（UI 案・2026-10-21）。
				左に接続と柵、中央に端末の面、右に帯。**帯を常に見えるようにする**のが要点で、
				PRD §4-1「見えないことが最大の危険」に直接効きます ——
				端末を打っている人が、**AI が何をしたかを見るために画面を移らなくていい。**

				**狭いときは折り返して縦に積みます**（`flex-wrap`）。
				列を固定幅で並べると、窓を狭めた人の端末が消えます。
			-->
			<div class="three">
				<!--
					**1 本のときも出す**（Issue #9）。以前は `> 1` で、
					**2 本目が開いて初めて現れました** ——
					「2 台目に繋げるのか」を探している人には絶対に見えなかった。
				-->
				{#if session.all.length > 0}
					<ConnectionRail
						layout="rail"
						items={session.all}
						activeId={session.activeId}
						roots={session.open?.write.aiRoots ?? []}
						onpick={(id) => session.focus(id)}
						onadd={() => (consoleAddingAnother = !consoleAddingAnother)}
					/>
				{/if}
				<div class="middle">
					<!--
						**繋ぐ所は 1 つの部品**（`ConnectPanel`）。
						写して 2 か所に置くと、**片方だけ直る日**が来ます（D39）。
					-->
					{#if consoleAddingAnother}
						<ConnectPanel
							onconnected={() => (consoleAddingAnother = false)}
							onregister={() => {
								consoleAddingAnother = false;
								view = 'connections';
							}}
						/>
					{/if}

					<div class="console-head">
						<!-- **誰が握っているかを、常に出す。**見えない所で AI が打っている、を作らない。 -->
						<!-- **どの接続の端末かを常に出す**（D25）。
						     タブを移しても端末は付いてこないので、書いていないと迷子になる。 -->
						{#if consoleOn}
							<span class="on" data-secret>{consoleOn}</span>
						{/if}
						<!--
							**いま誰が打てるか**（UI 案・2026-10-21）。
							案の「いま打てるのは [Human · あなた]」。
							**色だけに頼りません** —— 人（青）と AI（橙）は明るさがほぼ同じ
							（差 9.5 / 255）なので、色が見えない人には見分けられません。
							**札の中に `Human` / `AI` と文字で書きます。**
							長い説明文は読み上げへ回します（画面では札で足りる）。
						-->
						<span class="who" class:ai={holder === 'ai'} class:mine={iHold}>
							<span class="who-label">{i18n.t('console.who')}</span>
							<span class="who-badge">
								{holder === 'ai'
									? i18n.t('console.who.ai')
									: iHold
										? i18n.t('console.who.me')
										: i18n.t('console.who.none')}
							</span>
						</span>
						<span class="holder visually-hidden" class:ai={holder === 'ai'} class:mine={iHold}>
							<Icon name={holder ? 'lock' : 'terminal'} size={12} />
							{holder === 'ai'
								? i18n.t('console.held.ai')
								: iHold
									? i18n.t('console.held.me')
									: i18n.t('console.held.none')}
						</span>
						{#if !holder && session.open !== null}
							<!--
								**繋いでいないときは、そもそも出しません。**
								押せない釦を置いても、何をすればいいかは伝わりません
								（実機で「意味が分からない」と言われた並びです）。
								繋ぐ所は上の `ConnectPanel` に出ています。
							-->
							<button type="button" class="primary" onclick={openConsole}>
								<Icon name="terminal" />
								{i18n.t('console.open')}
							</button>
							<!-- **「何本開けるか」を、押す前に言う。**使う人が機能の有無を判断できるように。 -->
							<span class="onlyone">{i18n.t('console.onlyone')}</span>
						{:else if holder && !iHold}
							<!-- **人はいつでも取り返せる**（D29）。 -->
							<button type="button" class="primary" onclick={takeConsole}>
								<Icon name="lock" />
								{i18n.t('console.take')}
							</button>
						{:else if holder && iHold}
							<!--
								**人の側から渡せる**（案・1 枚目）。
								握っているときだけ出します —— 握っていないのに
								「渡す」は意味を成しません。
								**問いは出ません。**押したことが許可です。
							-->
							<button type="button" onclick={giveConsole} title={i18n.t('console.give.what')}>
								<Icon name="arrow-right" size={13} />
								{i18n.t('console.give')}
							</button>
						{/if}
						{#if holder}
							<!-- **止まらない停止ボタンは、無い方がまし。**必ず効く。 -->
							<button type="button" class="danger" onclick={stopConsole}>
								<Icon name="stop" />
								{i18n.t('console.stop')}
							</button>
						{/if}
						<!--
							**押せる所にも置く。**ショートカットだけだと、知らない人には無いのと同じ。
							ただし**端末が開いていないときは出しません** — 繋ぎ方を探している人の前に、
							無関係な釦を同じ重さで並べていました（実機の指摘）。
						-->
						{#if holder}
							<button type="button" onclick={() => openSearch('console')}>
								{i18n.t('search.label')}
							</button>
						{/if}
					</div>
					{@render searchBar('console')}
					<!--
						**端末のタブ**（分割・2026-10-06）。番号・接続・色を出します ——
						人が口で「②を見て」と言えて、AI も同じ番号で指せる（PRD §4-0）。
						**3 枚目以降はここで切り替えます**（半々は 2 枚まで・DESIGN.md）。
					-->
					{#if consolesOpen.length > 0}
						<div class="console-tablist" role="tablist" aria-label={i18n.t('tab.console')}>
							{#each consolesOpen as held (held.id)}
								<button
									type="button"
									role="tab"
									class="console-tab"
									class:active={layout.panes.some((pane) => pane.id === held.id)}
									aria-selected={layout.focused === held.id}
									onclick={() => (chosenConsole = held.id)}
								>
									<span class="num">#{held.id}</span>
									<span data-secret>{held.connection}</span>
								</button>
							{/each}
							{#if layout.canOpenAnother}
								<!-- **同じサーバにもう 1 枚。**押しても断られる釦は出しません。 -->
								<button
									type="button"
									class="ghost add"
									onclick={openAnotherConsole}
									title={i18n.t('console.another')}
								>
									<Icon name="plus" size={13} />
									{i18n.t('console.another')}
								</button>
							{/if}
						</div>
					{/if}
					<!--
						**半々に並べる**（人が決めた形）——

						> 見た目はデスクトップみたいに半々とかタブ切り替えかなぁ

						**等分から始めます。**掴んで動かす境目は、要ると分かってから足します。
					-->
					<div class="console-panes" class:locked={holder === 'ai'}>
						{#if layout.panes.length === 0}
							<!--
								**言葉の無い黒い面を描かないこと**（実機の報告・2026-10-23）。
								`doko005` を切ったあと「黒い面がそのまま残った」と言われました。
								**残っていたのではなく、空の面を無言で描いていました。**
								人からは「切ったのに何も起きていない」と読めます。

								**止まっている理由が届かないと、人は製品の外で直します。**
							-->
							<div class="terminal shell">
								<div class="core terminal-core empty">
									<p class="nothing">
										{session.open
											? i18n.t('console.none.connected')
											: i18n.t('console.none.disconnected')}
									</p>
								</div>
							</div>
						{:else}
							{#each layout.panes as pane (pane.id)}
								<ConsolePaneView
									{pane}
									{clipboard}
									{platform}
									backlog={replayFor(backlogs.all, consoleKey(pane.id))}
									mark={session.all.find((held) => held.id === pane.connection)?.color ?? null}
									fontPx={textSize.terminalPx}
									active={layout.focused === pane.id}
									iHold={pane.holder === 'human'}
									findIsOpen={findFrom('console')}
									onpick={(id) => (chosenConsole = id)}
									onstop={(id) => void stopConsoleOf(id)}
									onready={(id, write) => {
										// **地図は作り直しません。**面の数は 2 つなので、
										// 書き換えても描き直しは要りません（`$state` にしていません）。
										if (write) consoleWriters.set(id, write);
										else consoleWriters.delete(id);
									}}
									onsearch={(id, search) => {
										const next = new Map(consoleSearches);
										if (search) next.set(id, search);
										else next.delete(id);
										consoleSearches = next;
									}}
									onfailure={(message) => (failure = message)}
								/>
							{/each}
						{/if}
					</div>
				</div>
				<!-- **帯は右の列に常設。**同じ部品を帯の面でも使っています（写していません）。 -->
				<BandPanel {lines} />
			</div>
		</section>
	{:else if view === 'files'}
		<FileBrowser />
		<!--
			**転送の列**（UI 案・2 枚目）。案はファイルの面の下に置きました ——
			**動かした所のすぐ下**で読めることが要点です。
		-->
		<TransferList lines={transfers} />
	{:else if view === 'connections'}
		<ConnectionManager />
	{:else if view === 'diag'}
		<!-- **何が起きたか。**MCP の `diagnostics` と同じ 1 つを見ている。 -->
		<DiagPanel found={diag} onrefresh={loadDiagnostics} />
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

	<BandPanel {lines} />
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
		/* 案: gap 16px・左右 16px・高さ 52px。詰めると古く見えます。 */
		gap: 1rem;
		padding: 0 1rem;
		/*
		 * **折り返します**（2026-10-23 に実機で切れた）。
		 *
		 * `nowrap` だと、窓が狭いときに**ヘッダーの中身が窓の外へ出て、
		 * 一番外側に横スクロールバーが出ます** —— 左端（ロゴ・タブの先頭）が
		 * 切れて読めなくなりました。**最大化していない普通の幅で起きます。**
		 *
		 * 案も `flex-wrap: wrap` です。畳めば縦に伸びるだけで、**切れません。**
		 * **縦に伸びるのは許容します** —— 切れて読めないより、ずっとまし。
		 */
		flex-wrap: wrap;
		min-width: 0;
		/*
		 * **窓の操作（─ □ ×）は右上に釘付け**（2026-10-25・Windows 実機）。
		 * 折り返しの流れに乗せると、帯が折れた日に × が 2 段目の左端へ逃げます。
		 * その分の幅を右に空けて、他の札がその下へ潜らないようにします。
		 */
		position: relative;
		--window-controls-w: 120px;
		padding-right: var(--window-controls-w);
		/*
		 * **この帯が OS のタイトルバー**（D17）。
		 *
		 * 掴める高さを確保します。34px では**掴める隙間がほとんど無く、
		 * 窓を動かせない**と言われました（実機）。
		 */
		min-height: 52px;
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
		border: 0;
		border-radius: var(--r-shell);
		background: var(--surface);
		/* **他と違う面に見せる。**本文と同じ平面に置くと、見落とされます。 */
		box-shadow: var(--shell-ring), var(--lift-2);
	}

	.toast-icon {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border-radius: var(--r-pill);
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
		font-size: var(--text-control);
		font-weight: 600;
		line-height: 1.45;
	}

	.toast-actions {
		display: flex;
		gap: 0.4rem;
	}

	.toast-actions button {
		font-size: var(--text-control);
		padding: 0.25rem 0.6rem;
	}

	/* 「全文」のような、押せるが主役ではないもの。**主ボタンと見た目で分ける。** */
	.toast-actions button.quiet {
		border-color: transparent;
		background: none;
		padding: 0.1rem 0;
		color: var(--accent);
		text-decoration: underline;
	}

	/*
	 * 変更内容。**押す材料。**
	 *
	 * 閉じている 3 行は**スクロールしません** — 既定で中が切れると、
	 * 切れていること自体に気づけないからです（比べた 2 つがそうでした）。
	 * 開いたときだけ高さで止めます。**人が自分で開いた状態**なので、
	 * そこに続きがあることは分かっています。
	 */
	.toast-notes {
		margin: 0;
		padding-left: 1.05rem;
		font-size: var(--text-control);
		line-height: 1.65;
		color: var(--fg-muted);
	}

	.toast-notes.open {
		max-height: min(40vh, 16rem);
		overflow-y: auto;
	}

	/* 再起動で切れるもの。**危険側の色で、本文と区別する。** */
	.toast-cut {
		margin: 0;
		font-size: var(--text-control);
		line-height: 1.6;
		font-weight: 500;
		color: var(--warning);
	}

	.toast-bar {
		height: 4px;
		border-radius: var(--r-pill);
		background: var(--surface-2);
		overflow: hidden;
	}

	.toast-bar span {
		display: block;
		height: 100%;
		background: var(--accent);
	}

	/* --- 「表示」メニュー（自前タイトルバーで OS のメニューを失った分） --- */

	/*
	 * **3 つの列**（UI 案・2026-10-21）。左に接続と柵、中央に端末、右に帯。
	 *
	 * **帯を常に見えるようにする**のが要点です（PRD §4-1
	 * 「見えないことが最大の危険」）—— 端末を打っている人が、
	 * **AI が何をしたかを見るために画面を移らなくていい。**
	 *
	 * **固定幅で並べません。**窓を狭めた人の端末が消えます。
	 * 中央に `flex: 999` を与えて**余りを全部渡し**、両脇は上限だけ決めます
	 * （案も `flex: 999 1 560px` と `max-width` でそうしています）。
	 */
	.three {
		display: flex;
		flex-wrap: wrap;
		align-items: stretch;
		gap: 0.5rem;
		flex: 1 1 auto;
		min-height: 0;
		min-width: 0;
	}

	/*
	 * **中央が主役。**`flex: 999` は「余りは全部こちら」という意味で、
	 * 両脇が先に縮みます。`min-width: 0` が無いと、
	 * **端末の中身が列を押し広げて横スクロールが出ます。**
	 */
	.three .middle {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		flex: 999 1 22rem;
		min-width: 0;
		min-height: 0;
	}

	/*
	 * 両脇。**上限だけ決めて、下限は内容に任せます。**
	 * 狭いと `flex-wrap` で下へ落ち、縦に積まれます ——
	 * **消えるのではなく、下に来る。**
	 */
	.three > :global(nav.rail) {
		flex: 1 1 13rem;
		max-width: 16rem;
		min-width: 0;
		overflow-y: auto;
	}

	.three > :global(section.band) {
		flex: 1 1 16rem;
		max-width: 21rem;
		min-width: 0;
	}

	.console-panes {
		display: flex;
		gap: 0.4rem;
		flex: 1 1 auto;
		min-height: 0;
	}

	/* **AI が握っている間は、人の入力を締める**（D29）。 */
	.console-panes.locked {
		opacity: 0.92;
	}

	.console-tablist {
		display: flex;
		gap: 0.3rem;
		flex-wrap: wrap;
		padding-bottom: 0.35rem;
	}

	.console-tab {
		font-size: var(--text-control);
		padding: 0.2rem 0.55rem;
		border: 0;
		border-radius: var(--r-control);
		background: transparent;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		box-shadow: var(--shell-ring);
	}

	/* **並んで出ている面**（半々のどちらか）。 */
	.console-tab.active {
		border-color: var(--accent);
		color: var(--accent);
	}

	.console-tab .num {
		font-variant-numeric: tabular-nums;
		font-weight: 600;
	}

	.console-tablist .add {
		font-size: var(--text-control);
		padding: 0.2rem 0.55rem;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		flex: 0 0 auto;
	}






	/*
	 * **画面を伏せる**（UI 案・2026-10-21／D26）。
	 *
	 * 押している間は**地と字を入れ替えます** —— 選ばれているタブと同じ合図。
	 * 「伏せているのに伏せていないと思っている」が一番危ないので、
	 * **弱い合図にしません。**
	 */















	/*
	 * **AI は自動で繋ぎ直さない**という断り（UI 案・2026-10-21）。
	 *
	 * 橙（AI の色）で、立ち上げ直す前の帯に添えます。
	 * 黙っていると「勝手に戻る」と思われ、**戻っていないことに
	 * 気づかないまま作業が進みます。**
	 */
	.ai-note {
		display: block;
		margin-top: 0.3rem;
		font-size: var(--text-sm);
		line-height: 1.55;
		color: var(--warning);
	}

	.mask-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		flex: none;
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg-control);
		/* 案: 透明に近い塗り＋内側の輪。暗い画面で明るい塗りにしない。 */
		background: var(--shell-bg);
		border: 0;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		border-radius: var(--r-pill);
		height: var(--control-h-lg);
		padding: 0 var(--control-px);
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease);
		min-width: max-content;
	}

	.mask-toggle:hover {
		background: var(--surface-2);
	}

	.mask-toggle.on {
		color: var(--selected-fg);
		background: var(--selected);
		font-weight: 700;
		box-shadow: none;
	}

	.mask-toggle:focus-visible,
	.stop-ai:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	/*
	 * **AI を止める**（UI 案・2026-10-21）。
	 *
	 * **何もしていないときは押せません。**押しても何も起きないボタンは、
	 * 「止まった」と読まれるので**出さないより悪い**。
	 * 止まっているときは薄くして、**在ることだけ**は見せます
	 * （探して見つからない方が困る）。
	 */
	.stop-ai {
		display: inline-flex;
		align-items: center;
		gap: 0.36rem;
		flex: none;
		font: inherit;
		font-size: var(--text-control);
		font-weight: 700;
		color: var(--danger);
		background: var(--danger-soft);
		border: 0;
		box-shadow: inset 0 0 0 1px rgba(255, 120, 120, 0.35);
		border-radius: var(--r-pill);
		height: var(--control-h-lg);
		padding: 0 1rem;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			opacity var(--fast) var(--ease);
		min-width: max-content;
	}

	.stop-ai:hover:not(:disabled) {
		background: color-mix(in srgb, var(--danger) 22%, transparent);
	}

	.stop-ai:disabled {
		opacity: 0.4;
		cursor: default;
	}

	/* 止める印。**丸ではなく角。**「停止」は四角という慣習に乗る。 */
	.stop-ai-mark {
		width: 0.42rem;
		height: 0.42rem;
		border-radius: 2px;
		background: currentColor;
		flex: none;
	}

	.about-open {
		flex: 0 0 auto;
		width: 34px;
		height: 34px;
		padding: 0;
		border-radius: var(--r-pill);
		background: var(--shell-bg);
		border: 0;
		color: var(--fg-muted);
		font-size: var(--text-control);
		line-height: 1;
		margin-left: auto;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
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



	/* **外を押したら閉じる。**帯より下は全部当たり判定にする。 */
	.view-backdrop {
		position: fixed;
		inset: 0;
		z-index: 40;
	}

	.view-panel {
		position: absolute;
		top: calc(100% + 6px);
		/*
		 * **ロゴの左端で留めます。**ロゴは窓の左端に在るので、
		 * `right: 0` だと**パネルが左へ伸びて窓の外へ出ます**（実機で出た形）。
		 */
		left: 0;
		z-index: 41;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.6rem;
		min-width: 220px;
		/*
		 * **窓からはみ出させない。**中身が増えても、ここで止まります。
		 * `100vw - 24px` は左右の余白ぶん。
		 */
		max-width: min(26rem, calc(100vw - 24px));
		border: 0;
		border-radius: 8px;
		background: var(--surface);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	/*
	 * **行は折り返します。**折り返さないと、釦が増えた日に
	 * パネルごと窓の外へ出ます（2026-10-23 に出ました —— 釦 4 つで 497px）。
	 */
	.view-panel .view-row {
		flex-wrap: wrap;
	}

	.view-panel .view-controls {
		flex-wrap: wrap;
	}

	.view-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6rem;
	}

	.view-label {
		font-size: var(--text-control);
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
		/*
		 * **右上の角に釘付け。**Windows の人は右上の角を押しに行く。
		 * 流れに乗せると、狭い窓で帯が折れたとき 2 段目の左端へ落ちました（0.1.27）。
		 */
		position: absolute;
		top: 0;
		right: 0;
		height: 52px;
		width: var(--window-controls-w);
	}

	.wc {
		min-width: 40px;
		border: 0;
		background: transparent;
		color: var(--fg-muted);
		font-size: var(--text-control);
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

	/*
	 * **閉じるだけは赤。**押し間違いの結果が他と違う。
	 *
	 * **ここは 16 進数の直書きで正しい**（「色はトークンだけ」の例外）——
	 * `#c42b1c` は **Windows 自身が閉じるボタンに使う色**で、
	 * 配色の都合で変えるものではありません。**OS の見た目に合わせる**のが目的なので、
	 * テーマのトークンに繋ぐと**明るい配色のときに OS と違う赤**になります。
	 * 自前タイトルバー（D17）にした以上、ここは OS に寄せます。
	 */
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
	/*
	 * **ロゴの印と製品名**（UI 案・2026-10-21）。
	 *
	 * 印は人の色（青）で引きます。**この道具の主は人**で、
	 * AI は橙で別に名乗ります。色の役をここから一貫させます。
	 */
	/*
	 * **ロゴはメニューの口**（UI 案・2026-10-21）。
	 *
	 * **掴める所が減ります。**この帯は OS のタイトルバー（D17）で、
	 * 34px では**窓を動かせない**と実機から言われています。
	 * だからロゴは**必要な幅だけ**にして（`flex: 0 1 auto`）、
	 * 余りは `nav.tabs` の `data-tauri-drag-region` に残します。
	 */
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-width: max-content;
		/* **製品名は切らない。**掴む所は header 全体（data-tauri-drag-region）に在り、ロゴを縮める必要は無い。 */
		flex: none;
		overflow: visible;
		font: inherit;
		font-size: var(--text-control);
		font-weight: 700;
		color: inherit;
		/* 案: 青の薄い塗り＋輪で「ここがメニュー」と常に言う。高さ 36px。 */
		background: var(--human-soft);
		border: 0;
		box-shadow: inset 0 0 0 1px rgba(140, 194, 255, 0.35);
		border-radius: var(--r-pill);
		height: 36px;
		padding: 0 0.75rem 0 0.5rem;
		gap: 0.5rem;
		cursor: pointer;
	}

	.brand:hover {
		box-shadow: inset 0 0 0 1px var(--human);
	}

	/* **開いているあいだは、開いていると分かること。** */
	.brand[aria-expanded='true'] {
		box-shadow: inset 0 0 0 1px var(--human);
	}

	.brand:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	/* 落としの印。**開くものであることを、押す前に言う。** */
	.brand-chevron {
		width: 0.7rem;
		height: 0.7rem;
		flex: none;
		color: var(--fg-faint);
	}

	.brand-mark {
		width: 1.1rem;
		height: 1.1rem;
		flex: none;
		color: var(--human);
	}

	.brand-name {
		font-size: var(--text-control);
		font-weight: 700;
		letter-spacing: 0.02em;
		color: var(--fg);
		white-space: nowrap;
		overflow: visible;
		text-overflow: clip;
		min-width: auto;
	}

	/* **合言葉ごと写せるボタン。**起動ごとに変わる値を人に書き写させない（D23）。 */
	/* 文字サイズ。**アイコンではなく「A」の大小そのもの**で示す方が伝わる。 */
	.text-step {
		line-height: 1;
		font-weight: 600;
	}

	.smaller-a {
		font-size: var(--text-xs);
	}

	.larger-a {
		font-size: 0.86rem;
	}

	/* いまどの段か。**出さないと、押した結果が分からない。** */
	.text-now {
		font-size: var(--text-xs);
		color: var(--fg-muted);
		white-space: nowrap;
		padding: 0 0.1rem;
		min-width: 2.2rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}












	.mcp {
		display: inline-flex;
		align-items: center;
		gap: 0.28rem;
		font-family: var(--font-ui);
		font-size: var(--text-sm);
		color: var(--fg-muted);
		white-space: nowrap;
		/*
		 * **縮みます。**`flex: none` だと、狭いときに
		 * この釦だけが幅を譲らず、ヘッダーを押し出しました。
		 * 縮んだら文字は `…` になりますが、**押せる所は残ります。**
		 */
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		height: var(--control-h);
		padding: 0 0.75rem;
		border-radius: var(--r-pill);
		border: 0;
		background: var(--shell-bg);
		cursor: pointer;
		transition: background var(--fast) var(--ease);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
	}

	.mcp:hover:not(:disabled) {
		background: var(--surface-2);
	}

	.mcp:disabled {
		opacity: 0.5;
		cursor: default;
	}

	/*
	 * 切替は、浮いた 1 本のピルにまとめる。
	 *
	 * **`.view-panel` をここに入れてはいけません**（2026-10-23 に実機で壊れた）。
	 * フェーズ A の時点では、あれは**ヘッダーの横並びの帯**でした。
	 * `75fff70` で**ロゴの下のドロップダウン**に変えたのに、この規則が残り ——
	 *
	 * ```
	 * border-radius: 999px  → 丸い巨大な塊になる
	 * flex-wrap: nowrap     → 「このアプリ」の行が折り返さず 497px
	 * ```
	 *
	 * パネルが 505px になり、`right: 0` で留めてあるので **窓の左外（x = −379）**
	 * へ出ました。Windows の実機では「空の角丸と、外へはみ出した 1 項目」に見えます。
	 *
	 * **手元（macOS の Tauri ＝ WebKit）では出ません。**Chrome で同じ幅にして
	 * 初めて再現しました。**Windows は WebView2 ＝ Chromium です。**
	 */
	.tabs {
		display: flex;
		align-items: center;
		gap: 2px;
		flex-wrap: nowrap;
		flex: none;
		/* **半透明＋内側の輪**（案）。線ではなく濃淡。 */
		background: var(--shell-bg);
		box-shadow: var(--shell-ring);
		border-radius: var(--r-pill);
		padding: 4px;
	}


	/* **アイコンと文字を横に並べる。**これが無いと、SVG が block なので
	   文字の上に乗ってしまう（実際に乗った）。 */
	.view-panel button,
	.tabs button {
		display: inline-flex;
		align-items: center;
		gap: 0.32rem;
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg-control);
		background: transparent;
		border: 0;
		border-radius: var(--r-pill);
		height: var(--control-h);
		padding: 0 var(--control-px);
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease),
			transform var(--fast) var(--ease);
		flex: none;
		min-width: max-content;
	}

	.view-panel button.icon-only {
		padding: 0 0.6rem;
	}

	.view-panel button:active,
	.tabs button:active {
		transform: scale(0.97);
	}

	/*
	 * **選ばれているタブ。**
	 *
	 * 2026-09-24、`--surface` では暗い配色で帯との差が 6 / 255 しか無く、
	 * **どれが選ばれているか分かりませんでした**（実機の指摘）。
	 * そのときは `--surface-raised` で**明るさで浮かせて**直しました
	 * （影は暗い背景に出ないため）。
	 *
	 * **UI 案（2026-10-21）はもっと強い合図を使います** —— 地と字を入れ替える。
	 * 浮かせるのは「少し明るい」でしかなく、**白黒で撮ると消えます。**
	 * 反転なら残ります。上の指摘は直したままで、合図だけ強くしました。
	 */
	.view-panel button.active,
	.tabs button.active {
		color: var(--selected-fg);
		background: var(--selected);
		font-weight: 700;
	}

	.view-panel button:focus-visible,
	.tabs button:focus-visible,
	.view-panel select:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.view-panel select {
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg);
		background: var(--surface);
		border: 0;
		border-radius: var(--r-pill);
		padding: 0.16rem 0.5rem;
		margin-left: 0.15rem;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
	}

	.failure {
		margin: 0;
		padding: 0.55rem 0.8rem;
		border-radius: var(--r-control);
		background: var(--danger-soft);
		color: var(--danger);
		font-size: var(--text-control);
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
		font-size: var(--text-sm);
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.stream-head .label {
		flex: 0 0 auto;
		font-size: var(--text-sm);
		color: var(--fg-faint);
		white-space: nowrap;
	}

	/* 端末の検索バー。**帯やログの並びに合わせただけ**の仮置き（DESIGN.md）。 */
	.search-bar {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: var(--text-sm);
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.search-bar input {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
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

	/* **「何本開けるか」を言う文。**釦の隣に、控えめに置く（注意書きであって警告ではない）。 */
	.onlyone {
		font-size: var(--text-sm);
		color: var(--fg-faint);
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
		padding: 0 12px;
		border: 0;
		border-radius: var(--r-pill);
		background: var(--shell-bg);
		color: var(--fg);
		font-size: var(--text-control);
		cursor: pointer;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
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
		font-size: var(--text-sm);
		padding: 0.15rem 0.4rem;
		border: 0;
		border-radius: var(--r-control);
		color: var(--fg-muted);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		white-space: nowrap;
	}

	/*
	 * **いま誰が打てるか**（UI 案・2026-10-21）。
	 *
	 * 案は「いま打てるのは [Human · あなた]」という錠剤にしました。
	 * 札の地を人の色／AI の色で塗り、**字は地の色に反転させます** ——
	 * 浮かせるだけだと白黒で撮ったときに消えるからです。
	 *
	 * **色だけに頼りません。**案の 2 色は明るさがほぼ同じ（差 9.5 / 255）で、
	 * 色が見えない人には見分けられないので、**札の中に文字で書きます。**
	 */
	.who {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		margin-right: auto;
		/* 案: 4px 4px 4px 14px・高さ 32px */
		padding: 3px 3px 3px 14px;
		min-height: 32px;
		border-radius: var(--r-pill);
		background: var(--shell-bg);
		border: 0;
		box-shadow: var(--shell-ring);
		font-size: var(--text-control);
		white-space: nowrap;
		line-height: 1.2;
		min-width: max-content;
	}

	.who-label {
		color: var(--fg-control);
		flex: none;
		white-space: nowrap;
	}

	.who-badge {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 600;
		padding: 4px 10px;
		border-radius: var(--r-pill);
		/* 誰も握っていないときは、色で主張しない。 */
		background: var(--surface-2);
		color: var(--fg-muted);
		line-height: 1.2;
		flex: none;
		white-space: nowrap;
	}

	.who.mine {
		background: var(--human-soft);
		box-shadow: inset 0 0 0 1px rgba(140, 194, 255, 0.25);
	}

	.who.mine .who-badge {
		background: var(--human);
		color: var(--ground);
	}

	/* **AI が握っている間は、見て分かるようにする。**締まっていることが伝わらないと、
	   打てないのを不具合だと思われる。 */
	.who.ai {
		background: var(--ai-soft);
		box-shadow: inset 0 0 0 1px rgba(245, 166, 91, 0.3);
	}

	.who.ai .who-badge {
		background: var(--ai);
		color: var(--ground);
	}

	/*
	 * 長い説明文（「AI が握っています — あなたの入力は締まっています」）は
	 * **読み上げだけに回します。**画面では上の札で足りており、
	 * 2 つ並べると帯が溢れます。**消しません** —— 読み上げには要る文です。
	 */
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		padding: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}

	.holder {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: var(--text-sm);
		color: var(--fg-muted);
	}


	/* **この面が何をする所か**を書く 1 行。読んで分からない面は、無いのと同じ。 */
	/* 面の説明文。**伸びません**（Issue #22 —— 伸びて 250px の空白を作っていました）。 */
	.what {
		flex: 0 0 auto;
		margin: 0 0 0.35rem;
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--fg-muted);
	}

	/* 追うログのパス。**入力欄が伸びて、ボタンが右へ寄る。** */
	.stream-head .path {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		height: 32px;
		padding: 0 10px;
		box-sizing: border-box;
		border: 0;
		border-radius: var(--r-control);
		background: var(--surface-input);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		color: var(--fg);
	}

	.stream-head button {
		font: inherit;
		font-size: var(--text-sm);
		color: var(--fg);
		background: var(--shell-bg);
		border: 0;
		border-radius: var(--r-pill);
		padding: 0 12px;
		cursor: pointer;
		transition: transform var(--fast) var(--ease);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
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
		background: var(--terminal-bg, #0d1013);
		overflow: hidden;
		padding: 6px 8px;
	}





	.empty {
		margin: 0;
		font-size: var(--text-control);
		color: var(--fg-faint);
	}
	/* **空の面にも言葉を置く。**無言の黒は「壊れている」と読まれます。 */
	.nothing {
		margin: 0;
		padding: 1.2rem;
		font-family: var(--font-ui);
		font-size: var(--text-control);
		line-height: 1.8;
		color: var(--fg-faint);
		text-align: center;
		overflow-wrap: anywhere;
	}


	/*
	 * **既定の窓（1100×700）でヘッダーを 1 段に収める**（2026-10-24・写真で判明）。
	 * 土台を案の寸法（gap 16px・左右 16px）にしたら、1100px では
	 * **窓の操作釦が 2 段目に落ち、ロゴが「sshb…」に切れました。**
	 * 狭いときは隙間だけ詰めます。釦の高さや字は変えません。
	 */
	@media (max-width: 1240px) {
		header {
			gap: 0.5rem;
			/* 右は窓の操作の分を空けたまま（詰めると × の下へ札が潜る）。 */
			padding: 0 var(--window-controls-w) 0 0.6rem;
		}

		/* 錠剤の中の余白も少し詰める（高さと字は変えない）。1100px で計測して 1 段に入る値。 */
		.tabs button {
			padding: 0 9px;
		}

		/* 長い文言（英語）は短い言い方に切り替える。製品名とタブは切らない。 */
		.mask-toggle .label-long,
		.stop-ai .label-long {
			display: none;
		}

		.mask-toggle .label-short,
		.stop-ai .label-short {
			display: inline;
		}

		/* **MCP の釦は印だけ**（文言は title に残る）。実機（WebKit・1100px）では
		   Chrome より文字が広く、この釦で窓の操作が 2 段目に落ちました。 */
		.mcp {
			padding: 0 10px;
		}

		.mcp .mcp-text {
			display: none;
		}

		/* **印だけにはしない。**0.1.27 で印だけにしたら、Windows で「MCP が無い」と読まれた。 */
		.mcp .mcp-short {
			display: inline;
		}
	}
	/* 出力の面の道の欄。**白い線の枠が残っていました**（写真で判明）。他の入力と同じ輪に。 */
	.stream-head .path:focus {
		outline: none;
		box-shadow:
			inset 0 0 0 1px var(--accent),
			0 0 0 3px var(--accent-soft);
	}
	.label-short,
	.mcp-short {
		display: none;
	}
</style>
