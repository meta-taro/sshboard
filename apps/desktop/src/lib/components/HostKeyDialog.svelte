<script lang="ts">
	/**
	 * **ホスト鍵を、人が承認する所**（Issue #26）。
	 *
	 * 実機の報告（0.1.13 / Windows）——
	 *
	 * > 赤帯に `[object Object]` とだけ表示され、［承認］［拒否］が描画されない
	 * > **アプリは人の答えを待ち続けるが、人には答える手段が無い**
	 *
	 * **#24 で「待ちを型で表せるように」したとき、画面へ繋いでいませんでした。**
	 * `pending_status` は返るので **AI からは見えるのに、人は答えられない**。
	 * **見えるのに答えられないのは、見えないより悪い**です ——
	 * 人は「反応しない」と読み、同じ操作を繰り返します。
	 *
	 * ## 出すもの
	 *
	 * **指紋と algorithm を、読める形で。**ここを畳むと承認の意味が消えます。
	 * **接続先は出しません**（PRD §8）。出るのは識別子だけ。
	 *
	 * ## 既定は［拒否］側
	 *
	 * 端末の許可（D42）と同じです。**開いた勢いの Enter で通さない。**
	 */
	import { onMount } from 'svelte';

	import Icon from './Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	let {
		id,
		algorithm,
		fingerprint,
		expected = null,
		busy = false,
		onTrust,
		onRefuse
	}: {
		id: string;
		algorithm: string;
		fingerprint: string;
		expected?: string | null;
		busy?: boolean;
		onTrust: () => void;
		onRefuse: () => void;
	} = $props();

	let refuseButton = $state<HTMLButtonElement | null>(null);

	/** **登録と食い違う。**初見ではなく、すり替えの疑い。 */
	const mismatch = $derived(expected !== null && expected !== fingerprint);

	onMount(() => {
		// **焦点は「断る」に置く。**開いた勢いで Enter を打っても、承認になりません。
		refuseButton?.focus();
	});
</script>

<div class="backdrop" role="presentation">
	<div class="dialog" role="alertdialog" aria-modal="true" aria-label={i18n.t('hostkey.title')}>
		<header>
			<Icon name={mismatch ? 'warning' : 'lock'} size={15} />
			<h2>{mismatch ? i18n.t('hostkey.title.mismatch') : i18n.t('hostkey.title')}</h2>
		</header>

		<p class="note">
			{mismatch ? i18n.t('hostkey.mismatch') : i18n.t('hostkey.body')}
		</p>

		<dl>
			<dt>{i18n.t('hostkey.connection')}</dt>
			<dd data-secret>{id}</dd>
			<dt>{i18n.t('hostkey.algorithm')}</dt>
			<dd>{algorithm}</dd>
			<dt>{i18n.t('hostkey.fingerprint')}</dt>
			<dd class="print">{fingerprint}</dd>
			{#if mismatch}
				<dt>{i18n.t('hostkey.registered')}</dt>
				<dd class="print was">{expected}</dd>
			{/if}
		</dl>

		<p class="how">{i18n.t('hostkey.how')}</p>

		<div class="actions">
			<button type="button" bind:this={refuseButton} onclick={onRefuse} disabled={busy}>
				{i18n.t('hostkey.refuse')}
			</button>
			<button type="button" class="cta" class:danger={mismatch} onclick={onTrust} disabled={busy}>
				{i18n.t('hostkey.trust')}
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		/* **パスフレーズ（70）より上、端末の許可（80）と同じ高さ。**
		   人が答えるまで、他の操作へ進ませない。 */
		z-index: 80;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 45%);
	}

	.dialog {
		width: min(30rem, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.7rem;
		padding: 1.1rem;
		background: var(--surface);
		/* **色だけを手がかりにしない**（DESIGN.md）。枠でも分かるようにします。 */
		border: 1px solid var(--warning, var(--hairline-strong));
		border-radius: var(--r-shell);
		box-shadow: var(--lift-3);
	}

	header {
		display: flex;
		/* **題字が 2 行になっても、下の文へ重ねない。**
		   英語は語が長いので、ここは実際に折れます（2026-09-18 に別の問いで踏んだ）。 */
		align-items: flex-start;
		gap: 0.45rem;
	}

	header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.note,
	.how {
		margin: 0;
		font-size: 0.78rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	dl {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.25rem 0.7rem;
		margin: 0;
		font-size: 0.78rem;
	}

	dt {
		color: var(--fg-faint);
		white-space: nowrap;
	}

	dd {
		margin: 0;
		color: var(--fg);
		word-break: break-all;
	}

	/* **指紋は等幅で。**1 文字違いを目で追える形にします。 */
	.print {
		font-family: var(--font-mono);
		font-size: 0.74rem;
	}

	.was {
		color: var(--fg-muted);
		text-decoration: line-through;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	button.cta.danger {
		/* **食い違いのときは、承認そのものが危ない。**色でもそう見せます。 */
		background: var(--danger);
		border-color: transparent;
		color: var(--accent-fg);
	}
</style>
