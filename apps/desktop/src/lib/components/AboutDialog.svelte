<script lang="ts">
	/**
	 * 「sshboard について」。
	 *
	 * **版を確かめる場所は、ここです。**
	 *
	 * 自前タイトルバー（D17）にした結果、Windows では OS のメニューごと消え、
	 * **版を確かめる場所がどこにも無くなっていました**（実機で指摘）。
	 * 「表示」や「診断」へ散らしたのは誤りで、
	 * **普通に探す場所は「〜について」／ヘルプ**です。
	 *
	 * dbboard も同じ形（タイトルバーの `?` → ダイアログ）なので、そこへ揃えます。
	 */
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	import { i18n } from '$lib/i18n/i18n.svelte';
	import { updater } from '$lib/update/updater.svelte';

	interface Props {
		onClose: () => void;
	}
	let { onClose }: Props = $props();

	const REPO_URL = 'https://github.com/meta-taro/sshboard';

	let version = $state('—');

	onMount(async () => {
		try {
			version = await invoke<string>('app_version');
		} catch {
			// Tauri の外では取れません。**「—」のままにして、誤りを出しません。**
		}
	});

	/**
	 * 手で更新を確かめる。
	 *
	 * 調べるのは起動時の 1 回だけ（D34）でした。**この道具は繋ぎっぱなしで使う**ので、
	 * 起動の間隔がいちばん長く、**何日も気づけません。**取りに来られる口を 1 つ置きます。
	 *
	 * **押したらこの窓は閉じます。**知らせは画面の隅に出るので、
	 * 開いたままだと自分で隠してしまいます。
	 */
	function checkForUpdate() {
		updater.check(true);
		onClose();
	}

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
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('about.title')}>
		<header class="head">
			<h2>{i18n.t('about.title')}</h2>
		</header>

		<dl class="meta">
			<dt>{i18n.t('about.version')}</dt>
			<dd class="mono">{version}</dd>
			<dt>{i18n.t('about.repo')}</dt>
			<dd class="mono">{REPO_URL}</dd>
		</dl>

		<p class="note">{i18n.t('about.alpha')}</p>

		<div class="actions">
			<button type="button" onclick={checkForUpdate}>{i18n.t('update.check')}</button>
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
		width: min(30rem, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
		padding: 1.1rem;
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		box-shadow: var(--lift-3);
	}

	.head h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.meta {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.3rem 0.9rem;
		margin: 0;
	}

	.meta dt {
		font-size: 0.68rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--fg-faint);
		align-self: center;
	}

	.meta dd {
		margin: 0;
		font-size: 0.82rem;
		overflow-wrap: anywhere;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.note {
		margin: 0;
		font-size: 0.76rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
	}
</style>
