<script lang="ts">
	/**
	 * 鍵のパスフレーズを、繋ぐときに正面から聞く（Issue #7 の提案 2）。
	 *
	 * **既定は保存しません。**繋ぐたびに人が入れます。
	 *
	 * **押した人だけが預けられます**（実運用の要望・2026-09-28）——
	 * 「パスフレーズを記憶する、の**チェックボックスつけられないですかね**」。
	 * 1 日に 4 回以上、同じものを打っていました。**人が［接続］を押す前提なら
	 * 1 日 1 回で済みますが、AI が呼ぶ運用だと回数が桁違いになります。**
	 *
	 * 預け先は **OS の資格情報ストア**（D11 が最初から指示している形）。
	 * **自前の鍵ストアは作りません。****AI はこの値を見ません**（口が無い）。
	 *
	 * 以前はファイル画面のバーの中に小さな入力欄が現れる形で、
	 * **［接続］を押して失敗するまで、その欄が存在しませんでした。**
	 * 接続タブから押した人には、そもそも見えません。
	 */
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		/** どの接続か。**識別子だけ**を出します（ホスト名は出さない・PRD §8）。 */
		id: string;
		busy: boolean;
		/** `remember` が真なら、**OS の資格情報ストアへ預けます**（D11）。 */
		onSubmit: (passphrase: string, remember: boolean) => void;
		onCancel: () => void;
	}
	let { id, busy, onSubmit, onCancel }: Props = $props();

	let value = $state('');
	/** **既定は預けない。**押した人だけが預けます。 */
	let remember = $state(false);
	let field: HTMLInputElement | undefined = $state();

	// **開いたら、そこへ焦点を置く。**押した流れのまま打てるように。
	$effect(() => {
		field?.focus();
	});

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !busy) onCancel();
	}

	function submit() {
		if (value.length === 0 || busy) return;
		onSubmit(value, remember);
		// **画面から即座に捨てる。**渡した先が使い終わるのを待たない。
		value = '';
		remember = false;
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!--
	**背景を押しても閉じません**（実運用の指摘・2026-09-28）。

	> パスワードきいてたじゃないですか？で、**ほかをくりっくするときえちゃう**んですが。

	前は**わざと**閉じていました。理由は「**人が自分で始めた操作の続きなので**」。
	**その前提が崩れています** —— AI が MCP から `connect` を呼んだときは、
	**人は何も始めていません。**箱が出ただけです。そこで背景を押すと ——

	```
	人   箱が消えた。**それだけが見える**
	AI   `waitingForPassphrase` のまま**待ち続ける**
	```

	**Issue #30 と同じ壊れ方**です（あれは「裏にあって気づけない」、これは
	「出ていたのに消えた」）。**どちらも、人が気づけない待ちが残ります。**

	**ここは「見て判断する箱」ではなく「打ち込む箱」**です。打とうとして
	どこかを押した拍子に落ちる余地を、残す理由がありません。
	**閉じる道は［やめる］と Escape だけ。**
-->
<div class="backdrop" role="presentation">
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('files.passphrase.title')}>
		<header>
			<h2><Icon name="key" size={15} /> {i18n.t('files.passphrase.title')}</h2>
		</header>

		<p class="what" data-secret>{id}</p>
		<p class="note">{i18n.t('files.passphrase.note')}</p>

		<form
			onsubmit={(event) => {
				event.preventDefault();
				submit();
			}}
		>
			<input
				bind:this={field}
				bind:value
				type="password"
				disabled={busy}
				placeholder={i18n.t('files.passphrase')}
				aria-label={i18n.t('files.passphrase')}
				autocomplete="off"
			/>
			<!--
				**預けるかどうかは、押した人が決めます。**既定は預けません。
				預け先は OS の資格情報ストア（D11）。**消す口は接続の編集画面**に在ります ——
				**預けっぱなしで消せないのが、いちばん悪い形**です。
			-->
			<label class="remember">
				<input type="checkbox" bind:checked={remember} disabled={busy} />
				<span>{i18n.t('files.passphrase.remember')}</span>
			</label>

			<div class="actions">
				<button type="submit" class="cta" disabled={busy || value.length === 0}>
					{busy ? i18n.t('files.connecting') : i18n.t('files.connect')}
				</button>
				<button type="button" onclick={onCancel} disabled={busy}>
					{i18n.t('conn.delete.no')}
				</button>
			</div>
		</form>
	</div>
</div>

<style>
	/* **預ける の行。**目立たせません —— 既定は預けない側です。 */
	.remember {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.78rem;
		color: var(--fg-muted);
		cursor: pointer;
	}

	.remember input {
		accent-color: var(--accent);
		cursor: pointer;
	}

	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 70;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 40%);
	}

	.dialog {
		width: min(26rem, 92vw);
		padding: 1.1rem;
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		box-shadow: var(--lift-3);
	}


	/*
	 * **flex を使いません**（2026-09-18 に実機で踏み、2026-09-21 に本当の原因が分かりました）。
	 *
	 * 題字が 2 行になると、**2 行目が本文へ重なっていました**（英語のみ）。
	 * `align-items: center` を `flex-start` に変えても直りませんでした ——
	 * **WebKit では、flex 容器の高さが中身に追いつかない場面がある**ためです
	 * （接続タブの名前が切れたのと同じ族）。
	 *
	 * **アイコンを題字の中へ入れて、普通の行の流れにします。**
	 * 行が増えれば高さも増える —— **ここは仕組みで保証されます。**
	 *
	 * **読めない同意は、同意ではありません。**
	 */
	header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		/* アイコンを文字と同じ行に置く（`Icon` は inline-block） */
		display: block;
	}

	header h2 :global(svg) {
		vertical-align: -0.125em;
		margin-right: 0.35rem;
	}

	.what {
		margin: 0.7rem 0 0;
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.note {
		margin: 0.7rem 0 0;
		font-size: 0.75rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

</style>
