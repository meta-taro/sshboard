<script lang="ts">
	/**
	 * **帯を書き出す前に、何が入るかを見せる**（案・4 枚目）。
	 *
	 * 案の「書き出す」側は、**含める / 含めない**を並べて見せています。
	 * 同じ形にします —— **押してから知るのでは遅い**からです。
	 *
	 * **囲いは落とし先を守りません。**書き出した文を公開の場へ貼ると
	 * 接続先が出ます（禁止事項 4）。だから赤で言います。
	 */
	import { bandExportText, defaultBandName } from '$lib/band-export';
	import type { BandLine } from '$lib/band';
	import { i18n } from '$lib/i18n/i18n.svelte';

	let {
		lines,
		masked,
		onclose,
		onsave
	}: {
		lines: readonly BandLine[];
		/** 画面を伏せているか。**伏せているなら、書き出しも伏せます。** */
		masked: boolean;
		onclose: () => void;
		onsave: (payload: { text: string; name: string }) => void;
	} = $props();

	/** **押す前に見せる中身。**最初の数行をそのまま出します。 */
	const preview = $derived(
		bandExportText(lines.slice(-4), { masked }).split('\n').slice(-5).join('\n').trim()
	);

	function save() {
		onsave({ text: bandExportText(lines, { masked }), name: defaultBandName() });
	}
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && onclose()} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="veil" role="presentation" onclick={onclose}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="sheet"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		aria-label={i18n.t('band.export.title')}
		onclick={(event) => event.stopPropagation()}
	>
		<!-- **題字と閉じるを同じ flex に入れません**（WebKit の高さの件）。 -->
		<header>
			<h2>{i18n.t('band.export.title')}</h2>
		</header>

		<p class="note">{i18n.t('band.export.what', { count: String(lines.length) })}</p>

		<!-- **含める / 含めない。**案の書き出し側と同じ形。 -->
		<dl class="what">
			<div class="row"><dt>{i18n.t('band.export.in.who')}</dt><dd class="yes">{i18n.t('band.export.yes')}</dd></div>
			<div class="row"><dt>{i18n.t('band.export.in.what')}</dt><dd class="yes">{i18n.t('band.export.yes')}</dd></div>
			<div class="row"><dt>{i18n.t('band.export.in.where')}</dt><dd class="yes">{i18n.t('band.export.yes')}</dd></div>
			<div class="row"><dt>{i18n.t('band.export.in.secret')}</dt><dd class="no">{i18n.t('band.export.no')}</dd></div>
			<div class="row"><dt>{i18n.t('band.export.in.output')}</dt><dd class="no">{i18n.t('band.export.no')}</dd></div>
		</dl>

		<!-- **囲いは落とし先を守りません。**赤で言います。 -->
		<p class="warn">{i18n.t('band.export.warn')}</p>

		{#if masked}
			<!-- **画面と同じものが出ます。**読める形で要るなら、伏せを解いてから。 -->
			<p class="masked">{i18n.t('band.export.masked')}</p>
		{/if}

		<div class="preview">
			<span class="preview-label">{i18n.t('band.export.preview')}</span>
			<pre data-secret>{preview}</pre>
		</div>

		<div class="actions">
			<!-- **「足さない」を流用しません。**あれは提案を断る言葉で、意味が違います。 -->
			<button type="button" class="close" onclick={onclose}>{i18n.t('conn.delete.no')}</button>
			<button type="button" class="go" onclick={save} disabled={lines.length === 0}>
				{i18n.t('band.export.go')}
			</button>
		</div>
	</div>
</div>

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 80;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.5);
	}

	.sheet {
		width: min(30rem, 100%);
		max-height: 100%;
		overflow: auto;
		padding: 1rem 1.1rem;
		border: 0;
		border-radius: var(--r-shell);
		background: var(--surface);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	/* **ここに `display: flex` を置きません**（`component-wiring.test.ts` の決まり）。 */
	header {
		margin-bottom: 0.4rem;
	}

	h2 {
		margin: 0;
		font-size: 0.95rem;
		font-weight: 700;
		color: var(--fg);
	}

	.note {
		margin: 0 0 0.6rem;
		font-size: var(--text-sm);
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.what {
		margin: 0 0 0.6rem;
	}

	.row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.6rem;
		padding: 0.22rem 0;
		border-bottom: 1px solid var(--hairline);
		font-size: var(--text-sm);
	}

	dt {
		color: var(--fg-muted);
		min-width: 0;
	}

	dd {
		margin: 0;
		flex: none;
		font-weight: 700;
	}

	.yes {
		color: var(--ok);
	}

	.no {
		color: var(--danger);
	}

	/* **囲いは落とし先を守りません。** */
	.warn {
		margin: 0 0 0.5rem;
		padding: 0.4rem 0.5rem;
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--danger);
		background: var(--danger-soft);
		border-radius: var(--r-control);
	}

	.masked {
		margin: 0 0 0.5rem;
		font-size: var(--text-sm);
		color: var(--fg-dim);
		line-height: 1.6;
	}

	.preview {
		margin-bottom: 0.7rem;
	}

	.preview-label {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}

	pre {
		margin: 0.2rem 0 0;
		padding: 0.4rem 0.5rem;
		max-height: 7rem;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: 1.5;
		color: var(--fg-muted);
		background: var(--surface-input);
		border-radius: var(--r-control);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
	}

	.close,
	.go {
		font: inherit;
		font-size: var(--text-sm);
		border-radius: var(--r-pill);
		padding: 0.16rem 0.7rem;
		cursor: pointer;
	}

	.close {
		color: var(--fg-control);
		background: var(--shell-bg);
		border: 0;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
		padding: 0 12px;
		border-radius: var(--r-pill);
	}

	.go {
		color: var(--accent-fg);
		background: var(--accent);
		border: 1px solid transparent;
		font-weight: 700;
	}

	.go:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
