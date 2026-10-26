<script lang="ts" module>
	/** 実行体の `StandingAllowance` と対。**操作 id × 接続**。 */
	export type Standing = { operation: string; connection: string };
</script>

<script lang="ts">
	/**
	 * **許可を続けている操作**（D47 改・2026-10-26）。
	 *
	 * > その恒久許可をあとから確認・解除できる設定の枠がほしい。
	 *
	 * 「外すまで許可する」を押した操作を、**どの接続で**許しているかと一緒に並べ、
	 * 1 件ずつ外せるようにします。外すと、次からまた尋ねます。
	 * アプリを終えるか［AI を止める］を押すと、全部外れます（保存しません）。
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		allowances: readonly Standing[];
		onRevoke: (allowance: Standing) => void;
		onClose: () => void;
	}
	let { allowances, onRevoke, onClose }: Props = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onClose();
	}}
>
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('standing.title')}>
		<header class="head">
			<h2>{i18n.t('standing.title')}</h2>
		</header>

		{#if allowances.length === 0}
			<p class="empty">{i18n.t('standing.empty')}</p>
		{:else}
			<ul class="list">
				{#each allowances as held (held.operation + '\u0000' + held.connection)}
					<li>
						<span class="what">{held.operation}</span>
						<!-- **接続の名前は伏せられる所**（禁止事項 4）。 -->
						<span class="where" data-secret>{held.connection}</span>
						<button type="button" onclick={() => onRevoke(held)}>
							{i18n.t('standing.revoke')}
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		<p class="note">{i18n.t('standing.help')}</p>

		<div class="actions">
			<button type="button" class="cta" onclick={onClose}>{i18n.t('about.close')}</button>
		</div>
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
		width: min(32rem, 92vw);
		padding: 1.1rem;
		background: var(--surface);
		border: 0;
		border-radius: var(--r-shell);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	/* **積むのは普通の流れで**（flex で積むと、WebKit で本文の高さが追いつかず詰まる）。 */
	.dialog > :global(* + *) {
		margin-top: 0.7rem;
	}

	.head h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.empty,
	.note {
		margin: 0;
		font-size: var(--text-control);
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-height: 18rem;
		overflow-y: auto;
	}

	.list li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 6px 10px;
		border-radius: var(--r-control);
		background: var(--surface-2);
	}

	.what {
		font-family: var(--font-mono);
		font-size: var(--text-control);
		color: var(--fg);
	}

	.where {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--fg-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	button {
		display: inline-flex;
		align-items: center;
		height: var(--control-h);
		padding: 0 12px;
		border: 0;
		border-radius: var(--r-pill);
		background: var(--shell-bg);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		color: var(--fg);
		font: inherit;
		font-size: var(--text-control);
		white-space: nowrap;
		cursor: pointer;
	}

	button.cta {
		background: var(--accent-soft);
		color: var(--accent);
		box-shadow: inset 0 0 0 1px var(--accent);
		font-weight: 600;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
