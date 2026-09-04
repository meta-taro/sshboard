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
		onregister = undefined
	}: {
		onconnected?: () => void;
		onregister?: () => void;
	} = $props();

	let registered = $state<Connection[]>([]);
	let chosenId = $state('');
	let passphrase = $state('');
	let needsPassphrase = $state(false);
	let untrusted = $state<Untrusted | null>(null);
	let failure = $state<string | null>(null);

	async function loadRegistered() {
		try {
			registered = await invoke<Connection[]>('connections_list');
			if (!chosenId && registered.length > 0) chosenId = registered[0].id;
		} catch (error: unknown) {
			failure = i18n.t('err.list', { detail: String(error) });
		}
	}

	onMount(loadRegistered);

	async function connect() {
		if (!chosenId || session.busy) return;
		failure = null;
		untrusted = null;
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
				untrusted = why;
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

	/**
	 * 見えた指紋を、この接続の正解として登録する。**人が確かめたという記録。**
	 *
	 * 以後この接続は、**同じ指紋の相手としか繋がらない**（すり替えを検出できる）。
	 */
	async function trustFingerprint() {
		if (!untrusted) return;
		const entry = registered.find((held) => held.id === chosenId);
		if (!entry) return;

		try {
			await invoke('connection_save', {
				entry: { ...entry, fingerprint: untrusted.fingerprint }
			});
			untrusted = null;
			await loadRegistered();
			await connect();
		} catch (error: unknown) {
			failure = String(error);
		}
	}
</script>

{#if registered.length === 0}
	<!--
		**1 件も登録が無いときに、空の一覧だけ出さない。**
		前は何も出ず、人は「押す所が無い」に遭いました。
	-->
	<div class="picker shell">
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
{#if untrusted}
	<div class="trust shell" class:danger={untrusted.expected}>
		<div class="core">
			{#if untrusted.expected}
				<p class="head">
					<Icon name="warning" size={14} />
					<strong>{i18n.t('files.trust.mismatch')}</strong>
				</p>
				<p class="body">{i18n.t('files.trust.mismatch.body')}</p>
				<dl>
					<dt>{i18n.t('files.trust.seen')}</dt>
					<dd><code data-secret>{untrusted.fingerprint}</code></dd>
					<dt>{i18n.t('files.trust.expected')}</dt>
					<dd><code data-secret>{untrusted.expected}</code></dd>
				</dl>
				<!-- **一押しで受け入れる道を置かない。**すり替えかもしれないため。 -->
				<p class="body">{i18n.t('files.trust.mismatch.how')}</p>
			{:else}
				<p class="head">
					<Icon name="key" size={14} />
					<strong>{i18n.t('files.trust.first')}</strong>
				</p>
				<p class="body">{i18n.t('files.trust.first.body')}</p>
				<dl>
					<dt>{untrusted.algorithm}</dt>
					<dd><code data-secret>{untrusted.fingerprint}</code></dd>
				</dl>
				<div class="trust-actions">
					<button type="button" class="primary" onclick={trustFingerprint}>
						<Icon name="check" />
						{i18n.t('files.trust.accept')}
					</button>
					<button type="button" onclick={() => (untrusted = null)}>
						{i18n.t('files.trust.cancel')}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

{#if failure}
	<p class="failure" role="alert">{failure}</p>
{/if}

{#if needsPassphrase && chosenId}
	<PassphraseDialog
		id={chosenId}
		busy={session.busy}
		onSubmit={(value) => {
			passphrase = value;
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
	 * **`FileBrowser` から動かしただけ**（書き直していません）。
	 * 見た目を変えずに置き場所だけ移すのが目的なので、
	 * **規則はそのまま**運んでいます。
	 *
	 * 色は tokens.css の変数だけ。**ここに 16 進数を書かない。**
	 */

	/* 枠。**`.shell` はこの部品の外にあった**ので、ここで持ち直します。 */
	.picker,
	.trust {
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
	}

	.picker .core,
	.trust .core {
		background: var(--surface);
		border-radius: var(--r-shell);
	}

	/* 1 件も登録が無いとき。**空の枠だけを出さない。** */
	.empty-core {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
		padding: 0.7rem;
	}

	.none {
		margin: 0;
		font-size: 0.75rem;
		color: var(--fg-muted);
	}

	.hint {
		font-size: 0.7rem;
		color: var(--fg-faint);
	}

	.tag {
		flex: none;
		font-size: 0.66rem;
		color: var(--fg-faint);
	}

	.failure {
		margin: 0.4rem 0 0;
		font-size: 0.74rem;
		color: var(--danger);
	}

	/* 接続を選ぶところ。**印（色とタグ）が見えたまま選べる。** */
	.picker .core {
		list-style: none;
		margin: 0;
		padding: 0.2rem;
		max-height: 8.5rem;
		overflow: auto;
	}

	.picker li {
		list-style: none;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		/* **左寄せ。**一覧として読むもの。 */
		text-align: left;
		border: none;
		background: none;
		border-radius: var(--r-control);
		padding: 0.2rem 0.4rem;
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

	/* 3 手順。**繋がるまでのあいだだけ出す。**繋がったら消える。 */
	.steps {
		margin: 0.15rem 0 0;
		padding: 0 0 0 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		font-size: 0.66rem;
		color: var(--fg-faint);
		line-height: 1.5;
	}

	.picker-foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.35rem 0.2rem 0.05rem;
	}

	/* 初見の指紋。**行き止まりにしない。** */
	.trust .core {
		padding: 0.6rem 0.7rem;
	}

	.trust.danger .core {
		background: var(--danger-soft);
	}

	.trust .head {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		margin: 0 0 0.3rem;
		font-size: 0.78rem;
	}

	.trust.danger .head {
		color: var(--danger);
	}

	.trust .body {
		margin: 0 0 0.4rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.trust dl {
		margin: 0 0 0.5rem;
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.15rem 0.5rem;
		align-items: baseline;
	}

	.trust dt {
		font-size: 0.66rem;
		color: var(--fg-faint);
		white-space: nowrap;
	}

	.trust dd {
		margin: 0;
		min-width: 0;
	}

	.trust code {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		word-break: break-all;
	}

	.trust-actions {
		display: flex;
		gap: 0.4rem;
	}

	/*
	 * **縮むのは中の一覧だけ。**枠ごと縮むと、中身がはみ出して
	 * 下の要素の上に重なって出ます（`.shell` は `min-height: 0` を持っていて、
	 * **縮んでも中身を隠しません**）。実際に接続ボタンと重なりました。
	 *
	 * かといって縮ませないと、窓が低いときに**下のファイル 2 ペインが潰れます。**
	 * 縦に並べ直して、**足りない分は一覧が巻き取る**形にします。
	 */
	.picker {
		display: flex;
		flex-direction: column;
		flex: 0 1 auto;
		/*
		 * **`.shell` の `min-height: 0` をここだけ打ち消す。**
		 *
		 * 0 のままだと、場所が足りないときに枠が中身より小さくなり、
		 * **はみ出した中身が下の要素の上に重なって出ます**（実際に 2 回起きました）。
		 * `min-content` なら「これ以上は縮めない」を中身が決めます。
		 * 一覧は下で 1 行分まで縮むので、**最小はかなり小さい**まま保てます。
		 */
		min-height: min-content;
	}

	.picker .core {
		flex: 0 1 auto;
		/* 1 行だけでも残す。**0 まで潰れると、何を選ぶ所か分からなくなる。** */
		min-height: 2.4rem;
	}

	.picker-foot,
	.steps {
		flex: 0 0 auto;
	}
</style>
