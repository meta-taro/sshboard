<script lang="ts">
	/**
	 * **過去の帯**（Phase 2「監査ログの永続化」・2026-11-25）。
	 *
	 * 帯はアプリを閉じると消えていました。実行体が帯をこの PC の中へ
	 * 1 日 1 本で控え（14 日で消します）、ここで日を選んで読み戻します。
	 *
	 * **何が残っているかを、中身より先に言います**（帯の書き出しと同じ考え）。
	 * 接続の名前とパスが入るので、**画面を伏せていれば一緒に伏せます**（`data-secret`）。
	 */
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	import { journalLine, type JournalEntry, type JournalOverview } from '$lib/band-journal';
	import { i18n } from '$lib/i18n/i18n.svelte';

	let { onclose }: { onclose: () => void } = $props();

	let overview = $state<JournalOverview | null>(null);
	let day = $state<string | null>(null);
	let entries = $state<JournalEntry[]>([]);
	let failure = $state<string | null>(null);

	async function pick(next: string) {
		day = next;
		failure = null;
		try {
			entries = await invoke<JournalEntry[]>('journal_read', { day: next });
		} catch (error: unknown) {
			entries = [];
			failure = String(error);
		}
	}

	onMount(async () => {
		try {
			overview = await invoke<JournalOverview>('journal_days');
			const newest = overview.days[0];
			if (newest !== undefined) await pick(newest);
		} catch (error: unknown) {
			failure = String(error);
		}
	});
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
		aria-label={i18n.t('journal.title')}
		onclick={(event) => event.stopPropagation()}
	>
		<!-- **題字と閉じるを同じ flex に入れません**（WebKit の高さの件）。 -->
		<header>
			<h2>{i18n.t('journal.title')}</h2>
		</header>

		<p class="note">
			{i18n.t('journal.note', { days: String(overview?.keepDays ?? 14) })}
		</p>

		{#if overview}
			<p class="where">
				<span>{i18n.t('journal.where')}</span>
				<code data-secret>{overview.dir}</code>
			</p>

			{#if overview.days.length === 0}
				<p class="empty">{i18n.t('journal.empty')}</p>
			{:else}
				<div class="days" role="tablist">
					{#each overview.days as each (each)}
						<button
							type="button"
							role="tab"
							aria-selected={each === day}
							class:on={each === day}
							onclick={() => void pick(each)}
						>
							{each}
						</button>
					{/each}
				</div>

				{#if day !== null && entries.length === 0 && failure === null}
					<p class="empty">{i18n.t('journal.day.empty')}</p>
				{:else if entries.length > 0}
					<pre data-secret>{entries.map(journalLine).join('\n')}</pre>
				{/if}
			{/if}
		{/if}

		{#if failure}
			<p class="failure">{failure}</p>
		{/if}

		<div class="actions">
			<button type="button" class="close" onclick={onclose}>{i18n.t('conn.delete.no')}</button>
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
		width: min(44rem, 100%);
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

	.note,
	.where,
	.empty,
	.failure {
		margin: 0 0 0.6rem;
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--fg-muted);
	}

	.where span {
		margin-right: 0.4rem;
		color: var(--fg-dim);
	}

	.where code {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		overflow-wrap: anywhere;
	}

	.failure {
		color: var(--danger);
	}

	.days {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin-bottom: 0.6rem;
	}

	.days button {
		font: inherit;
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		padding: 0.1rem 0.55rem;
		border-radius: var(--r-pill);
		border: 0;
		color: var(--fg-control);
		background: var(--shell-bg);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		cursor: pointer;
	}

	.days button.on {
		color: var(--accent-fg);
		background: var(--accent);
		box-shadow: none;
	}

	pre {
		margin: 0 0 0.7rem;
		padding: 0.4rem 0.5rem;
		max-height: 50vh;
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
	}

	.close {
		font: inherit;
		font-size: var(--text-sm);
		color: var(--fg-control);
		background: var(--shell-bg);
		border: 0;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
		padding: 0 12px;
		border-radius: var(--r-pill);
		cursor: pointer;
	}
</style>
