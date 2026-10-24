<script lang="ts" module>
	/** AI が出した提案。**実行の要求ではありません。** */
	export type OperationProposal = {
		id: string;
		/** **実際に打つ文字列。**1 文字も省略せずに出します。 */
		run: string;
		description: string;
		maxPerHour: number;
	};
</script>

<script lang="ts">
	/**
	 * **AI が「これを足してよいか」と出した提案**（実運用の報告・2026-10-23）。
	 *
	 * ## なぜ在るか
	 *
	 * 本番で、調べるところは sshboard で完結したのに直す段で止まりました ——
	 * `operations.toml` が空で、**人は書く場所を探して見つけられず、
	 * 操作は画面の外で行われ、記録も残りませんでした。**
	 *
	 * `run_command` を作らない決定（D3）が守ろうとしたもの
	 * 「**人の見ていない所で終わらせない**」が、まさに裏返った形です。
	 *
	 * ## これは `run_command` ではありません
	 *
	 * - **AI はファイルを書きません。**書くのは画面（D22 / D27）
	 * - **登録と実行は別の行為。**登録しても走らず、走るときは承認の問いを通る（D42）
	 * - **コマンドは 1 文字も省略せずに出します。**読めない同意は、同意ではありません
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	import Icon from './Icon.svelte';

	let {
		proposal,
		onregister,
		ondismiss
	}: {
		proposal: OperationProposal;
		onregister: () => void;
		ondismiss: () => void;
	} = $props();
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && ondismiss()} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="veil" role="presentation" onclick={ondismiss}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="sheet"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		aria-label={i18n.t('propose.title')}
		onclick={(event) => event.stopPropagation()}
	>
		<header>
			<h2><Icon name="warning" size={14} /> {i18n.t('propose.title')}</h2>
		</header>

		<p class="what">{i18n.t('propose.what')}</p>

		<dl>
			<dt>{i18n.t('propose.id')}</dt>
			<dd class="mono">{proposal.id}</dd>

			<dt>{i18n.t('propose.run')}</dt>
			<!--
				**1 文字も省略しません。**折り返して全部出します ——
				`…` で切ると、切った先に何が在るか人には分かりません。
				**読めない同意は、同意ではありません。**
			-->
			<dd class="mono run" data-secret>{proposal.run}</dd>

			<dt>{i18n.t('propose.why')}</dt>
			<dd>{proposal.description}</dd>

			<dt>{i18n.t('propose.ceiling')}</dt>
			<dd class="mono">{proposal.maxPerHour}</dd>
		</dl>

		<!--
			**登録しても走らない**ことを、押す前に言います。
			ここを書かないと、人は「許可した」と受け取ります。
		-->
		<p class="note">{i18n.t('propose.note')}</p>

		<div class="actions">
			<button type="button" onclick={ondismiss}>{i18n.t('propose.no')}</button>
			<button type="button" class="cta" onclick={onregister}>{i18n.t('propose.yes')}</button>
		</div>
	</div>
</div>

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 90;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.55);
	}

	.sheet {
		width: min(32rem, 100%);
		max-height: 100%;
		overflow: auto;
		padding: 1rem 1.1rem;
		/* **橙は AI の色。**AI から来た問いであることが色で読める。 */
		border: 1px solid var(--ai);
		border-radius: var(--r-shell);
		background: var(--surface);
		box-shadow: var(--lift-3);
	}

	/* **題字に flex を置きません**（2026-09-18 / 09-21 に 3 度踏んだ形）——
	   WebKit では flex 容器の高さが中身に追いつかず、題字が折れると本文に重なる。 */
	header {
		margin-bottom: 0.4rem;
	}

	h2 {
		margin: 0;
		font-size: 0.95rem;
		color: var(--warning);
	}

	.what {
		margin: 0 0 0.7rem;
		font-size: var(--text-control);
		line-height: 1.6;
		color: var(--fg-muted);
	}

	dl {
		margin: 0;
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.3rem 0.7rem;
		align-items: baseline;
	}

	dt {
		font-size: var(--text-sm);
		color: var(--fg-dim);
		white-space: nowrap;
	}

	dd {
		margin: 0;
		font-size: var(--text-control);
		min-width: 0;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}

	/* **全部出す。**折り返しても切らない。 */
	.run {
		padding: 0.4rem 0.5rem;
		border-radius: var(--r-control);
		background: var(--surface-input);
		border: 0;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
		box-shadow: inset 0 0 0 1px var(--hairline);
	}

	.note {
		margin: 0.8rem 0 0;
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--fg-muted);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
		margin-top: 0.9rem;
	}

	button {
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg-control);
		background: var(--shell-bg);
		border: 0;
		border-radius: var(--r-pill);
		padding: 0 12px;
		cursor: pointer;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
	}

	button:hover {
		background: var(--surface-2);
	}

	/* **登録する側。**AI の色で、これが AI から来たものだと分かるように。 */
	.cta {
		color: var(--ground);
		background: var(--ai);
		border-color: var(--ai);
		font-weight: 700;
	}

	.cta:hover {
		background: var(--warning);
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
</style>
