<script lang="ts">
	/**
	 * **繋ぐ、という 1 つの仕事。**
	 *
	 * もともと `FileBrowser` の中にしかありませんでした。そのため端末の面には
	 * 繋ぐ手段が無く、「先にサーバーへ繋ぐ →」が *接続* タブ（登録の画面）へ
	 * 飛んでいました。**そこに繋ぐボタンはありません。**
	 * 実機で「どのように接続しますか？」と聞かれたのは、この行き止まりです。
	 *
	 * 写すのではなく、**切り出して両方から使います。**
	 * 繋ぐ道が 2 つあると、片方だけ直る日が来ます。
	 */
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	import Icon from '$lib/components/Icon.svelte';
	import { readableFailure } from '$lib/connect-failure';
	import PassphraseDialog from '$lib/components/PassphraseDialog.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import type { Connection } from '$lib/connections';
	import { session } from '$lib/session.svelte';

	/** 初見のホスト鍵。**人が確かめて登録するまで、ここで止める。** */
	type Untrusted = {
		algorithm: string;
		fingerprint: string;
		/** 登録済みの指紋。**あるのに食い違うなら、すり替えの疑い。** */
		expected: string | null;
	};

	type ConnectFailure =
		| ({ kind: 'untrusted' } & Untrusted)
		| { kind: 'passphraseNeeded' }
		| { kind: 'other'; message: string };

	let {
		/** 繋がったあと、呼んだ側がやること（一覧の読み直しなど）。 */
		onconnected = () => {},
		/** 登録が 1 件も無いときに押させる逃げ道。 */
		onregister = undefined,
		/**
		 * **見出し。**箱の上端に入れます（2026-10-26）。
		 * 呼んだ側が別の箱に見出しを置くと、見出しだけが浮いて角がずれました。
		 */
		heading = undefined
	}: {
		onconnected?: () => void;
		onregister?: () => void;
		heading?: string;
	} = $props();

	let registered = $state<Connection[]>([]);
	let chosenId = $state('');
	let passphrase = $state('');
	let needsPassphrase = $state(false);
	let failure = $state<string | null>(null);

	/**
	 * **相手を選び直したら、前の失敗は消す**（Issue #26 のコメント・2026-09-24）。
	 *
	 * 帯を消していたのは `connect()` の頭と、パスフレーズを［やめる］したときの
	 * 2 か所だけでした。**選び直しても残ります。**
	 * 帯が指している相手は、もう選ばれていないのに。
	 *
	 * 実機でこうなりました ——
	 *
	 * > 赤帯の `[object Object]` が**前の失敗の残り**として画面上部に出続けており、
	 * > それをパスフレーズの問いだと取り違えました
	 *
	 * **古い帯は、人が「いま起きていること」として読みます。**
	 * そこから誤った起票が 1 本と、訂正の往復が 2 度生まれました。
	 */
	let lastChosen = '';
	$effect(() => {
		if (chosenId === lastChosen) return;
		lastChosen = chosenId;
		failure = null;
		needsPassphrase = false;
		// **入れてもらった秘密も、相手が変わったら捨てる**（D14）。
		passphrase = '';
	});

	async function loadRegistered() {
		try {
			registered = await invoke<Connection[]>('connections_list');
			if (!chosenId && registered.length > 0) chosenId = registered[0].id;
		} catch (error: unknown) {
			failure = i18n.t('err.list', { detail: String(error) });
		}
	}

	onMount(loadRegistered);

	/**
	 * **パスフレーズを OS の資格情報ストアへ預ける**（D11・実運用の要望）。
	 *
	 * **失敗しても繋ぐのは止めません。**預けられないことと、繋げないことは別です ——
	 * ここで止めると、**預けたかっただけの人が繋げなくなります。**
	 */
	async function keepPassphrase(id: string, value: string) {
		try {
			await invoke('connection_passphrase_save', { id, passphrase: value });
		} catch (error: unknown) {
			// **黙らない。**預かったつもりで預かっていないのが、いちばん悪い形です。
			// **構造を持った失敗を潰さない**（Issue #33）。
			// `[object Object]` だけが帯に出る形を、ここにも残していました。
			failure = readableFailure(error);
		}
	}

	async function connect() {
		if (!chosenId || session.busy) return;
		failure = null;
		session.busy = true;
		try {
			await invoke('session_connect', { id: chosenId, passphrase: passphrase || null });
			// **入れてもらった秘密は、その場で捨てる**（D14）。
			passphrase = '';
			needsPassphrase = false;
			await session.refresh();
			onconnected();
		} catch (error: unknown) {
			const why = error as ConnectFailure;
			if (why?.kind === 'untrusted') {
				// **行き止まりにしない。**指紋を見せて、人が確かめて登録できるようにする。
				// **ここでは出しません。**実行体が問いを立て、
				// 画面（`HostKeyDialog`）が出します（Issue #26）。
			} else if (why?.kind === 'passphraseNeeded') {
				needsPassphrase = true;
				failure = i18n.t('files.passphrase.needed');
			} else {
				failure = why?.kind === 'other' ? why.message : String(error);
			}
		} finally {
			session.busy = false;
		}
	}

</script>

{#if registered.length === 0}
	<!--
		**1 件も登録が無いときに、空の一覧だけ出さない。**
		前は何も出ず、人は「押す所が無い」に遭いました。
	-->
	<div class="picker shell">
		{#if heading}<span class="heading">{heading}</span>{/if}
		<div class="core empty-core">
			<p class="none">{i18n.t('files.none')}</p>
			{#if onregister}
				<button type="button" class="primary" onclick={onregister}>
					<Icon name="plus" />
					{i18n.t('conn.new')}
				</button>
			{/if}
		</div>
	</div>
{:else}
	<!-- 繋いでいないときは、印つきの一覧から選ぶ。**色とタグが見えたまま選べる。** -->
	<div class="picker shell">
		{#if heading}<span class="heading">{heading}</span>{/if}
		<ul class="core">
			{#each registered as entry (entry.id)}
				<li>
					<button
						type="button"
						class="pick"
						class:chosen={chosenId === entry.id}
						style:--mark={entry.color ? `var(--mark-${entry.color})` : 'transparent'}
						onclick={() => (chosenId = entry.id)}
						ondblclick={() => {
							chosenId = entry.id;
							connect();
						}}
					>
						<span class="mark-bar" aria-hidden="true"></span>
						<span class="who" data-secret>{entry.name || entry.id}</span>
						{#if entry.tag}<span class="tag" data-secret>{entry.tag}</span>{/if}
						{#if entry.fingerprint}
							<span class="known" title={i18n.t('files.known')}>
								<Icon name="check" size={11} />
							</span>
						{/if}
					</button>
				</li>
			{/each}
		</ul>
		<div class="picker-foot">
			<button type="button" class="primary" onclick={connect} disabled={!chosenId || session.busy}>
				<Icon name="plug" />
				{session.busy ? i18n.t('files.connecting') : i18n.t('files.connect')}
			</button>
			<span class="hint">{i18n.t('files.pick.help')}</span>
		</div>

		<!-- **初動で迷わせない。**繋がるまでの 3 手順をその場に置く。 -->
		<ol class="steps">
			<li>{i18n.t('files.step1')}</li>
			<li>{i18n.t('files.step2')}</li>
			<li>{i18n.t('files.step3')}</li>
		</ol>
	</div>
{/if}

<!-- **初見の指紋は、行き止まりにしない。**確かめて登録する道をここに置く。 -->
<!--
	**ホスト鍵の問いは、ここには置きません**（Issue #26 の後始末・2026-09-21）。

	以前はここに帯を出していました。**MCP から繋いだときは出ない**ので、
	#26 で画面側（`HostKeyDialog`）へ移しました。
	**両方に置いたままだと、人が［開く］で繋いだときに 2 つ出ます。**

	`+page.svelte` のコメントが、先に同じことを言っています ——
	**「問いを 2 か所に作ると、片方だけ直る日が来ます」**（D39）。
-->

{#if failure}
	<p class="failure" role="alert">{failure}</p>
{/if}

{#if needsPassphrase && chosenId}
	<PassphraseDialog
		id={chosenId}
		busy={session.busy}
		onSubmit={(value, remember) => {
			passphrase = value;
			// **預けるのは、押した人が決めた場合だけ**（D11）。
			// **繋ぐ前に預けます** —— 繋いだあとにすると、
			// 繋ぎ損ねた回だけ預からない、という分かりにくい形になります。
			if (remember) void keepPassphrase(chosenId, value);
			void connect();
		}}
		onCancel={() => {
			needsPassphrase = false;
			passphrase = '';
			failure = null;
		}}
	/>
{/if}

<style>
	/*
	 * **箱は 1 つ、中は上から縦に並べる**（2026-10-26・Windows 実機の崩れ）。
	 *
	 * 以前は `.picker` に「折り返して中央寄せ」と「縦並び」の 2 つの規則が重なり、
	 * **折り返す縦並び**になっていました。一覧・［接続］・3 手順が横の列へ散り、
	 * 一覧は余った高さで段を引き伸ばして、札の段の間が約 70px 空いていました。
	 *
	 * 色は tokens.css の変数だけ。**ここに 16 進数を書かない。**
	 */
	.picker {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 8px;
		padding: 12px 14px;
		flex: 0 1 auto;
		/*
		 * **`.shell` の `min-height: 0` をここだけ打ち消す。**
		 * 0 のままだと、場所が足りないときに枠が中身より小さくなり、
		 * **はみ出した中身が下の要素の上に重なって出ます**（実際に 2 回起きました）。
		 */
		min-height: min-content;
	}

	/* 見出し。**箱の上端に入れる**（別の箱に浮かせない）。 */
	.heading {
		font-size: var(--text-control);
		font-weight: 600;
		color: var(--fg);
	}

	/* 1 件も登録が無いとき。**空の枠だけを出さない。** */
	.empty-core {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
		padding: 10px 12px;
	}

	.none {
		margin: 0;
		font-size: var(--text-control);
		color: var(--fg-muted);
	}

	.hint {
		font-size: var(--text-sm);
		color: var(--fg-faint);
	}

	.tag {
		flex: none;
		font-size: var(--text-sm);
		color: var(--fg-faint);
	}

	.failure {
		margin: 0.4rem 0 0;
		font-size: var(--text-control);
		color: var(--danger);
	}

	/*
	 * 接続を選ぶところ。**印（色とタグ）が見えたまま選べる。**
	 *
	 * **段は上から詰める**（`align-content: flex-start`）。既定の stretch だと、
	 * 余った高さで段の間が広がります。高さは中身に合わせ、多いときだけ中で巻き取ります。
	 */
	.picker .core {
		display: flex;
		flex-wrap: wrap;
		align-content: flex-start;
		gap: 4px;
		list-style: none;
		margin: 0;
		padding: 6px;
		max-height: 8.5rem;
		/* 1 行だけでも残す。**0 まで潰れると、何を選ぶ所か分からなくなる。** */
		min-height: 2.4rem;
		overflow: auto;
		flex: 0 1 auto;
	}

	.picker li {
		list-style: none;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		/* **左寄せ。**一覧として読むもの。 */
		text-align: left;
		border: none;
		background: none;
		border-radius: var(--r-pill);
		height: var(--control-h);
		padding: 0 12px;
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg-control);
		cursor: pointer;
	}

	.pick:hover {
		background: var(--surface-2);
	}

	.pick.chosen {
		/* **選択の枠も印の色に合わせる。**緑の固定色だと印が意味を失う。 */
		background: var(--surface-2);
		box-shadow: inset 0 0 0 1.5px var(--mark, var(--accent));
	}

	/* 印の色。**タグと二重に出す**（色が見えない人にも効くように）。
	   **`.bar` と名付けない。**上の接続バーと衝突して、そちらが幅 3px に潰れた
	   （実際に潰れた）。 */
	.mark-bar {
		flex: none;
		width: 3px;
		height: 0.95rem;
		border-radius: 2px;
		background: var(--mark);
	}

	.who {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.known {
		flex: none;
		color: var(--ok);
		display: inline-flex;
	}

	.picker-foot {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem;
		flex: 0 0 auto;
	}

	/* ［接続］。他の面の釦と同じ錠剤の形に揃える。 */
	.picker-foot .primary,
	.empty-core .primary {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		height: var(--control-h);
		padding: 0 14px;
		border: 0;
		border-radius: var(--r-pill);
		background: var(--accent-soft);
		color: var(--accent);
		box-shadow: inset 0 0 0 1px var(--accent);
		font: inherit;
		font-size: var(--text-control);
		font-weight: 600;
		cursor: pointer;
	}

	.picker-foot .primary:disabled {
		opacity: 0.45;
		cursor: default;
	}

	/* 3 手順。**繋がるまでのあいだだけ出す。**繋がったら消える。 */
	.steps {
		margin: 0;
		padding: 0 0 0 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		font-size: var(--text-sm);
		color: var(--fg-faint);
		line-height: 1.5;
		flex: 0 0 auto;
	}
</style>
