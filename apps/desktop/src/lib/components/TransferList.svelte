<script lang="ts" module>
	/** 画面へ届く 1 件。Rust 側の `TransferLine` と対。 */
	export type TransferLine = {
		seq: number;
		/** `human` か `ai`。**ここが列の値打ち。** */
		actor: 'human' | 'ai';
		direction: 'up' | 'down';
		/** 動かしたもの。**サーバー側の道が入ります**（伏せられる印を付けます）。 */
		what: string;
		bytes: number;
		outcome: 'done' | 'refused';
		/** 止めた理由。**「止めました」だけでは次に何をすればいいか分かりません。** */
		why?: string | null;
	};
</script>

<script lang="ts">
	/**
	 * **転送の列**（UI 案・2 枚目）。
	 *
	 * > 誰が動かしたかが行ごとに残ります
	 *
	 * ## なぜ要るか
	 *
	 * 帯には 1 行の文字列として出ていました。**構造が無かった**ので、
	 * 向き・誰が・何を・何バイト・成否を**並べて読む形**がありませんでした。
	 *
	 * そして**柵の外へ書こうとして断られたこと**は、帯の 1 行に埋もれていました。
	 * 案はそこを赤で「**止めました**」と出します ——
	 * **止まったことが見えないと、人は「動いた」と思って次へ進みます。**
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { humanSize } from '$lib/session.svelte';

	import Icon from './Icon.svelte';

	let { lines }: { lines: readonly TransferLine[] } = $props();
</script>

<section class="transfers" aria-label={i18n.t('transfer.label')}>
	<div class="head">
		<span class="label">{i18n.t('transfer.label')}</span>
		<span class="note">{i18n.t('transfer.what')}</span>
	</div>

	{#if lines.length === 0}
		<p class="empty">{i18n.t('transfer.empty')}</p>
	{:else}
		<ol>
			<!-- **新しいものを上に。**いま起きていることを探させない。 -->
			{#each [...lines].reverse() as line (line.seq)}
				<li class:refused={line.outcome === 'refused'}>
					<!-- 向き。**↑ が上げる、↓ が落とす。**読み上げには文字で渡します。 -->
					<span class="dir" aria-label={i18n.t(`transfer.${line.direction}`)}>
						<Icon name={line.direction === 'up' ? 'arrow-up' : 'arrow-down'} size={11} />
					</span>
					<!--
						**誰が動かしたか。**色だけに頼りません ——
						人（青）と AI（橙）は明るさがほぼ同じなので、**文字が本体**です。
					-->
					<span class="who" class:ai={line.actor === 'ai'}>
						{line.actor === 'ai' ? i18n.t('console.who.ai') : i18n.t('console.who.me')}
					</span>
					<!-- **サーバー側の道。**伏せられる印を付けます（禁止事項 4）。 -->
					<span class="what" data-secret>{line.what}</span>
					{#if line.outcome === 'refused'}
						<span class="said">
							{i18n.t('transfer.refused')}
							{#if line.why}<span class="why">{line.why}</span>{/if}
						</span>
					{:else}
						<span class="size">{humanSize(line.bytes)}</span>
					{/if}
				</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.transfers {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		flex: 0 1 auto;
		min-height: 0;
		min-width: 0;
		background: var(--surface);
		border-radius: 12px;
		padding: 12px 14px;
		box-shadow: var(--lift-core);
	}

	.head {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		min-width: 0;
	}

	.label {
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--fg);
		flex: none;
	}

	.note {
		font-size: var(--text-sm);
		color: var(--fg-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}

	.empty {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--fg-faint);
	}

	ol {
		margin: 0;
		padding: 0;
		list-style: none;
		overflow-y: auto;
		max-height: 9rem;
		min-height: 0;
	}

	li {
		display: flex;
		align-items: baseline;
		gap: 0.4rem;
		padding: 0.14rem 0.2rem;
		font-size: var(--text-sm);
		border-bottom: 1px solid var(--hairline);
		min-width: 0;
	}

	/* **止めた行は赤。**押し流されると「動いた」と読まれます。 */
	li.refused {
		background: var(--danger-soft);
	}

	.dir {
		display: inline-flex;
		flex: none;
		color: var(--fg-dim);
	}

	/* 人は青、AI は橙。**文字が本体で、色は添え物。** */
	.who {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 600;
		flex: none;
		color: var(--human);
	}

	.who.ai {
		color: var(--ai);
	}

	.what {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		overflow-wrap: anywhere;
	}

	.size {
		flex: none;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--fg-dim);
		font-variant-numeric: tabular-nums;
	}

	.said {
		flex: none;
		font-size: var(--text-xs);
		font-weight: 700;
		color: var(--danger);
	}

	/* 理由は細く。**でも出す** ——「止めました」だけでは次が分かりません。 */
	.why {
		display: block;
		font-weight: 400;
		color: var(--fg-muted);
		overflow-wrap: anywhere;
	}
</style>
