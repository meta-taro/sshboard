<script lang="ts">
	/**
	 * **キー操作の一覧**（UI 案・2026-10-21 のヘルプ）。
	 *
	 * ## なぜ要るか
	 *
	 * `Ctrl Shift H`（画面を伏せる）を入れても、**どこにも書いていなければ
	 * 無いのと同じ**です。Issue #9 で一度やりました ——
	 * 「2 台目に繋げるのか」を探している人に、案内が見えていませんでした。
	 *
	 * **この製品は画面そのものに接続先が写り込みます**（禁止事項 4）。
	 * 画面共有の直前に伏せたい人が、**押し方を探せること**が要ります。
	 *
	 * ## 表は `app-keys.ts` が持っています
	 *
	 * ここは描くだけ。**ヘルプが実物とずれるのが一番たちの悪い壊れ方**
	 * （書いてあるのに効かない／効くのに書いていない）なので、
	 * `app-keys.test.ts` が**表からキーを組み立てて `appIntent` に通し、
	 * 本当にその動作になるか**を数えています。
	 */
	import { APP_SHORTCUTS, OTHER_SHORTCUTS, spellShortcut, type AppPlatform } from '$lib/app-keys';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { menuLabels } from '$lib/i18n/messages-menu';

	let { platform, onclose }: { platform: AppPlatform; onclose: () => void } = $props();

	const menu = $derived(menuLabels(i18n.locale));

	/**
	 * 文言を引く。
	 *
	 * **鍵は 2 か所に散っています** —— 画面の文言（`i18n`）と、
	 * OS のメニューから作り直した分（`messages-menu`）。
	 * 表には `menu.` で始まる鍵も並ぶので、そこだけ別の口から引きます。
	 */
	function label(key: string): string {
		if (key.startsWith('menu.')) return menu[key as keyof typeof menu] ?? key;
		return i18n.t(key as Parameters<typeof i18n.t>[0]);
	}
</script>

<!--
	**閉じられること。**`Escape` と、外側を押すのと、釦の 3 つ。
	どれか 1 つだけだと、その 1 つを知らない人は閉じられません。
-->
<svelte:window onkeydown={(event) => event.key === 'Escape' && onclose()} />

<!-- **外側を押しても閉じる。**`role="presentation"` は既存の
     `AboutDialog` と同じ形（lint がそれを対話要素と見ないため）。 -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="veil" role="presentation" onclick={onclose}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="sheet"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		aria-label={i18n.t('help.keys')}
		onclick={(event) => event.stopPropagation()}
	>
		<!--
			**題字と閉じる釦を同じ行の flex に入れません**（2026-09-18 / 09-21 に
			3 度踏んだ形）——**WebKit では flex 容器の高さが中身に追いつかず、
			題字が折れたときに本文へ重なります。**
			既存の `AboutDialog` と同じく、**題字だけの `header` と、
			下に閉じる釦**を置きます。
		-->
		<header>
			<h2>{i18n.t('help.keys')}</h2>
		</header>

		<p class="note">{i18n.t('help.keys.note')}</p>

		<dl>
			{#each APP_SHORTCUTS as row (row.intent)}
				<div class="row">
					<dt>{label(row.label)}</dt>
					<dd><kbd>{spellShortcut(row, platform)}</kbd></dd>
				</div>
			{/each}
			<!--
				**持ち主が違っても、押す人には同じ 1 枚の表。**
				端末の面やファイルの面が持っているものも並べます。
			-->
			{#each OTHER_SHORTCUTS as row (row.key + (row.alt ? 'alt' : ''))}
				<div class="row">
					<dt>{label(row.label)}</dt>
					<dd><kbd>{spellShortcut(row, platform)}</kbd></dd>
				</div>
			{/each}
		</dl>

		<div class="actions">
			<button type="button" class="close" onclick={onclose}>{i18n.t('about.close')}</button>
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
		width: min(28rem, 100%);
		max-height: 100%;
		overflow: auto;
		padding: 1rem 1.1rem;
		border: 1px solid var(--hairline-strong);
		border-radius: var(--r-shell);
		background: var(--surface);
		box-shadow: var(--lift-3);
	}

	/*
	 * **ここに `display: flex` を置きません。**
	 *
	 * `component-wiring.test.ts` が数えている決まりです ——
	 * **WebKit では flex 容器の高さが中身に追いつかない場面があり**、
	 * 題字が 2 行に折れたときに**本文へ重なりました**（2026-09-18 に実機）。
	 * 普通の行の流れなら、行が増えれば高さも増える ——**仕組みで保証されます。**
	 */
	header {
		margin-bottom: 0.3rem;
	}

	h2 {
		margin: 0;
		font-size: 0.95rem;
	}

	/* 閉じる釦は下に。**題字と同じ行に並べない**（上の理由）。 */
	.actions {
		margin-top: 0.7rem;
		text-align: right;
	}

	.close {
		font: inherit;
		font-size: 0.72rem;
		color: var(--fg-control);
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-pill);
		padding: 0.16rem 0.6rem;
		cursor: pointer;
	}

	.close:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.note {
		margin: 0 0 0.7rem;
		font-size: 0.7rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	dl {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	/* **名前とキーを 2 列に。**狭い窓でも折り返して読めるように。 */
	.row {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		padding: 0.22rem 0.1rem;
		border-bottom: 1px solid var(--hairline);
	}

	dt {
		flex: 1 1 auto;
		min-width: 0;
		font-size: 0.74rem;
		color: var(--fg);
	}

	dd {
		margin: 0;
		flex: none;
	}

	/* 等幅で、押すものに見せる。 */
	kbd {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		color: var(--fg-control);
		background: var(--surface-input);
		border: 1px solid var(--hairline-strong);
		border-radius: var(--r-control);
		padding: 0.1rem 0.4rem;
		white-space: nowrap;
	}
</style>
