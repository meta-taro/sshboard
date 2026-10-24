<script lang="ts" module>
	/** 許可された 1 本。Rust 側の `AllowedLine` と対。 */
	export type AllowedLine = {
		/** AI が渡せる唯一の値。 */
		id: string;
		/** 実際に走る文字列。**人が書いたものが、そのまま走ります。** */
		run: string;
		/** 何をするものか。 */
		why: string | null;
		/** 1 時間あたりの上限。読むだけの側にはありません。 */
		maxPerHour: number | null;
	};

	/** 一覧 1 つ分。 */
	export type AllowedList = {
		/** 置き場。**本番で見つけられなかったのがここです。** */
		path: string;
		lines: AllowedLine[];
		/**
		 * 読めなかった理由。
		 *
		 * **`null` と「0 本」は別です。**書き間違いを 0 本と出すと、
		 * 人は「書いたのに断られる」所で止まります。
		 */
		trouble: string | null;
	};
</script>

<script lang="ts">
	/**
	 * **AI が走らせられることの一覧**（2026-10-23）。
	 *
	 * ## なぜ要るか
	 *
	 * **人だけが見られませんでした。**実行体は両方のファイルを読めて、
	 * AI も `list_readonly_commands` / `list_operations` で読めるのに、
	 * **画面へ出す口が 1 つもありませんでした。**
	 *
	 * 本番（2026-10-23）で詰まったのはここです ——
	 * 人は書く場所を探して見つけられず、**操作は sshboard の外で行われました。**
	 * 置き場を書いても、**入ったかを確かめる所が無ければ同じです。**
	 *
	 * ## 走らせません
	 *
	 * **ここは読むだけの面です。**釦を置くと、
	 * 「人が画面で押したから走った」と「AI が頼んだから走った」が混ざります。
	 * 走らせるのは承認の面（`OperationRequestDialog`）です。
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	let {
		reading,
		changing,
		onclose
	}: { reading: AllowedList; changing: AllowedList; onclose: () => void } = $props();
</script>

<!-- **閉じられること。**`Escape`・外側・釦の 3 つ（`KeyHelpDialog` と同じ形）。 -->
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
		aria-label={i18n.t('allow.title')}
		onclick={(event) => event.stopPropagation()}
	>
		<!--
			**題字と閉じる釦を同じ行の flex に入れません**（2026-09-18 / 09-21 に
			3 度踏んだ形）—— WebKit では flex 容器の高さが中身に追いつかず、
			題字が折れたときに本文へ重なります。
		-->
		<header>
			<h2>{i18n.t('allow.title')}</h2>
		</header>

		<p class="note">{i18n.t('allow.note')}</p>

		{#each [{ list: changing, label: i18n.t('allow.changing') }, { list: reading, label: i18n.t('allow.reading') }] as group (group.label)}
			<section>
				<h3>{group.label}</h3>
				<!-- **置き場を出す。**本番で見つけられなかったのがここです。 -->
				<p class="where">
					<span class="where-label">{i18n.t('allow.where')}</span>
					<!-- 手元の利用者名が入るので、伏せられる印を付けます。 -->
					<code data-secret>{group.list.path}</code>
				</p>

				{#if group.list.trouble}
					<!--
						**「空」と混ぜません。**書き間違いを 0 本と出すと、
						人は「書いたのに断られる」所で止まります。
					-->
					<p class="trouble">{group.list.trouble}</p>
				{:else if group.list.lines.length === 0}
					<!-- 「0 本」では足りません。**だから 1 本も走らない**まで書く。 -->
					<p class="empty">{i18n.t('allow.empty')}</p>
				{:else}
					<ul>
						{#each group.list.lines as line (line.id)}
							<li>
								<span class="id">{line.id}</span>
								<!--
									**サーバー側の道とドメインが入ります**（禁止事項 4）——
									`-d example.com` や `/var/www/html`。伏せられる印を付けます。
								-->
								<code class="run" data-secret>{line.run}</code>
								{#if line.why}<span class="why">{line.why}</span>{/if}
								{#if line.maxPerHour !== null}
									<!-- **上限を出す。**暴走しても、ここで止まることが見えるように。 -->
									<span class="ceiling">
										{i18n.t('allow.ceiling', { count: String(line.maxPerHour) })}
									</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		{/each}

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
		width: min(34rem, 100%);
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
		margin: 0 0 0.7rem;
		font-size: var(--text-sm);
		color: var(--fg-muted);
		line-height: 1.6;
	}

	section {
		margin-bottom: 0.9rem;
	}

	h3 {
		margin: 0 0 0.25rem;
		font-size: var(--text-control);
		font-weight: 700;
		color: var(--fg);
	}

	.where {
		display: flex;
		align-items: baseline;
		gap: 0.35rem;
		margin: 0 0 0.35rem;
		min-width: 0;
	}

	.where-label {
		font-size: var(--text-xs);
		color: var(--fg-dim);
		flex: none;
	}

	.where code {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--fg-muted);
		overflow-wrap: anywhere;
		min-width: 0;
	}

	/* **読めない。**空より目立たせる —— 書いた人が直せる所です。 */
	.trouble {
		margin: 0;
		padding: 0.3rem 0.4rem;
		font-size: var(--text-sm);
		color: var(--danger);
		background: var(--danger-soft);
		border-radius: var(--r-control);
		line-height: 1.5;
		overflow-wrap: anywhere;
	}

	.empty {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--fg-faint);
		line-height: 1.5;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	li {
		display: grid;
		grid-template-columns: minmax(0, auto) minmax(0, 1fr);
		gap: 0.1rem 0.45rem;
		padding: 0.28rem 0.2rem;
		border-bottom: 1px solid var(--hairline);
		min-width: 0;
	}

	.id {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--fg);
	}

	.run {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--fg-muted);
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.why {
		grid-column: 2;
		font-size: var(--text-xs);
		color: var(--fg-dim);
		line-height: 1.5;
	}

	.ceiling {
		grid-column: 2;
		font-size: var(--text-xs);
		color: var(--fg-faint);
	}

	.actions {
		margin-top: 0.2rem;
		text-align: right;
	}

	/* **既存の `KeyHelpDialog` と同じ釦。**新しい見た目を増やしません（§11）。 */
	.close {
		font: inherit;
		font-size: var(--text-sm);
		color: var(--fg-control);
		background: var(--shell-bg);
		border: 0;
		border-radius: var(--r-pill);
		padding: 0 12px;
		cursor: pointer;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
	}

	.close:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
</style>
