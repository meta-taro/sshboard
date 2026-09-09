<script lang="ts">
	/**
	 * **AI が状態を変える操作を走らせたい**（D45 / D47 / Issue #17）。
	 *
	 * `ConsoleRequestDialog`（端末の許可）と同じ形ですが、**違いが 1 つ**あります。
	 * **何が走るのかを、そのまま見せます。**
	 *
	 * > 「AI が doko001 で restart-httpd を実行したいと言っています
	 * >    **実行される内容: sudo systemctl restart httpd**」
	 *
	 * id だけでは答えられません。**`operations.toml` に書いた本人でも、
	 * 半年後には何を書いたか覚えていない**からです。
	 *
	 * ここで守っていること。
	 *
	 * - **既定は「断る」側。**Enter や Escape で勝手に許可されない
	 * - **1 回の許可で 1 回だけ。**釦の文言にもそう書きます
	 * - **接続先は出しません**（PRD §8）。出すのは操作の id と、打つ文字列だけ
	 * - **背景を押しても閉じません。**人が始めていない問いを、誤操作で消させない
	 */
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		/** 操作の識別子。**人が `operations.toml` に書いたもの。** */
		id: string;
		/** 実際にサーバーで走る文字列。 */
		runs: string;
		onAllow: () => void;
		onDeny: () => void;
	}
	let { id, runs, onAllow, onDeny }: Props = $props();

	let denyButton: HTMLButtonElement | undefined = $state();

	// **焦点は「断る」に置く。**開いた勢いで Enter を打っても、許可になりません。
	$effect(() => {
		denyButton?.focus();
	});

	function onKeydown(event: KeyboardEvent) {
		// **Escape は「断る」。**黙って消さず、答えとして扱います。
		if (event.key === 'Escape') {
			event.preventDefault();
			onDeny();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="backdrop" role="presentation">
	<div
		class="dialog"
		role="alertdialog"
		aria-modal="true"
		aria-label={i18n.t('operation.request.title')}
	>
		<header>
			<Icon name="warning" size={15} />
			<h2>{i18n.t('operation.request.title')}</h2>
		</header>

		<p class="what">{id}</p>
		<p class="lead">{i18n.t('operation.request.body')}</p>

		<!--
			**打つものを、そのまま出す。**丸めたり要約したりしません ——
			**要約した時点で、人は何を許したのか分からなくなります。**
		-->
		<pre class="runs">{runs}</pre>

		<p class="note">{i18n.t('operation.request.note')}</p>

		<div class="actions">
			<button type="button" bind:this={denyButton} onclick={onDeny}>
				{i18n.t('operation.request.deny')}
			</button>
			<button type="button" class="cta" onclick={onAllow}>
				{i18n.t('operation.request.allow')}
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		/* **いちばん手前。**答えるまで、他の操作へ進ませない。 */
		z-index: 85;
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
		gap: 0.6rem;
		padding: 1.1rem;
		background: var(--surface);
		/* **色だけを手がかりにしない**（DESIGN.md）。枠でも分かるように。 */
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

	.what {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.lead,
	.note {
		margin: 0;
		font-size: 0.78rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	/* **打つものは、折り返してでも全部見せる。**隠すと判断できません。 */
	.runs {
		margin: 0;
		padding: 0.6rem 0.7rem;
		background: var(--surface-2);
		border: 1px solid var(--hairline);
		border-radius: var(--r-item, 6px);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
