<script lang="ts">
	/**
	 * 鍵のパスフレーズを、繋ぐときに正面から聞く（Issue #7 の提案 2）。
	 *
	 * **保存はしません**（D11 / D14）。繋ぐたびに人が入れます。
	 * 変えたのは**聞き方だけ**です。
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
		onSubmit: (passphrase: string) => void;
		onCancel: () => void;
	}
	let { id, busy, onSubmit, onCancel }: Props = $props();

	let value = $state('');
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
		onSubmit(value);
		// **画面から即座に捨てる。**渡した先が使い終わるのを待たない。
		value = '';
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget && !busy) onCancel();
	}}
>
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
