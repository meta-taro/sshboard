<script lang="ts">
	/**
	 * **AI が端末を使いたいと言っている**（D42）。正面から聞きます。
	 *
	 * 実機の指摘（2026-09-06）:
	 *
	 * > AI に端末を渡すときに「止める」を押さないといけません。
	 * > これだと **AI からのアクションが分からない**ので、
	 * > 「AI が操作をするために許可しますか」みたいなアラートで人に気づかせないと。
	 *
	 * **［止める］を押させる形をやめました。**AI から頼み、人が答えます。
	 *
	 * ここで守っていること。
	 *
	 * - **既定は「断る」側。**開いた勢いで Enter を打っても許可になりません
	 * - **接続先も、AI が打とうとしている中身も出しません**（PRD §8）。
	 *   出すのは「AI が使いたい」という事実だけ
	 * - **背景を押しても閉じません。**許可の問いを、誤操作で消させない
	 */
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		onAllow: () => void;
		onDeny: () => void;
	}
	let { onAllow, onDeny }: Props = $props();

	let denyButton: HTMLButtonElement | undefined = $state();

	// **焦点は「断る」に置く。**開いた勢いで Enter を打っても、許可になりません。
	$effect(() => {
		denyButton?.focus();
	});

	function onKeydown(event: KeyboardEvent) {
		// **Escape は「断る」。**問いを黙って消さない — 答えとして扱います。
		if (event.key === 'Escape') {
			event.preventDefault();
			onDeny();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!--
	**背景を押しても閉じません**（`PassphraseDialog` とはそこが違います）。
	あちらは人が自分で始めた操作の続きですが、こちらは**人が始めていない問い**です。
	誤って消すと、AI は待ち続け、人は問いがあったことすら知りません。
-->
<div class="backdrop" role="presentation">
	<div
		class="dialog"
		role="alertdialog"
		aria-modal="true"
		aria-label={i18n.t('console.request.title')}
	>
		<header>
			<Icon name="lock" size={15} />
			<h2>{i18n.t('console.request.title')}</h2>
		</header>

		<p class="note">{i18n.t('console.request.body')}</p>

		<div class="actions">
			<button type="button" bind:this={denyButton} onclick={onDeny}>
				{i18n.t('console.request.deny')}
			</button>
			<button type="button" class="cta" onclick={onAllow}>
				<Icon name="terminal" size={13} />
				{i18n.t('console.request.allow')}
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		/* **いちばん手前。**パスフレーズの問い（70）より上に置きます —
		   人が答えるまで、他の操作へ進ませない。 */
		z-index: 80;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 45%);
	}

	.dialog {
		width: min(26rem, 92vw);
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
		align-items: center;
		gap: 0.45rem;
	}

	header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.note {
		margin: 0;
		font-size: 0.78rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
