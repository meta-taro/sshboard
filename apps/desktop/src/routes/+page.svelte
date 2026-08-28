<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import ConnectionManager from '$lib/components/ConnectionManager.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { LOCALES } from '$lib/i18n/locales';
	import { theme, type ThemeMode } from '$lib/theme/theme.svelte';
	import { createTerminal, writeChunk } from '$lib/terminal.svelte';
	import '@xterm/xterm/css/xterm.css';
	import type { Terminal } from '@xterm/xterm';

	let lines = $state<BandLine[]>([]);
	let mcpUrl = $state<string | null>(null);
	let failure = $state<string | null>(null);
	let streaming = $state(false);
	let view = $state<'connections' | 'band'>('connections');

	let terminalHost: HTMLDivElement | undefined = $state();
	let terminal: Terminal | undefined;

	async function startDemoStream() {
		try {
			await invoke('start_demo_stream');
			streaming = true;
		} catch (error: unknown) {
			failure = i18n.t('err.stream.start', { detail: String(error) });
		}
	}

	async function stopStream() {
		try {
			await invoke('stop_stream');
			streaming = false;
		} catch (error: unknown) {
			failure = i18n.t('err.stream.stop', { detail: String(error) });
		}
	}

	/**
	 * 「帯へ入れた」と返す。
	 *
	 * **描画（requestAnimationFrame）を待たないこと。**
	 * WKWebView はウィンドウが前面に無いと rAF を止める。この製品は
	 * 「人はエディタを見ていて、AI が裏で動く」使い方が普通なので、
	 * 描画を待つ作りにすると背面にした瞬間に MCP が全部失敗する。
	 *
	 * 待つのは DOM に入るところまで（`tick()`）。そこまで来ていれば、
	 * 人がウィンドウを見た瞬間にその行がある。**応答が先に返ることは無い。**
	 */
	function ackRendered(seq: number) {
		invoke('band_ack', { seq }).catch((error: unknown) => {
			failure = i18n.t('err.ack', { detail: String(error) });
		});
	}

	onMount(() => {
		const stops: Array<() => void> = [];

		if (terminalHost) {
			terminal = createTerminal(terminalHost);
		}

		// **ANSI を落とさずに渡す。**色は人の側にだけ残す（Issue 005）。
		listen<number[]>('stream://raw', (event) => {
			if (terminal) writeChunk(terminal, event.payload);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe.stream', { detail: String(error) });
			});

		listen<BandLine>('band://line', async (event) => {
			lines = appendLine(lines, event.payload);
			await tick();
			ackRendered(event.payload.seq);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe', { detail: String(error) });
			});

		listen<string>('mcp://ready', (event) => {
			mcpUrl = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 下の取り直しで拾う */
			});

		// 起動が速いと 'mcp://ready' を購読より先に取りこぼす。取り直す。
		invoke<string | null>('mcp_url')
			.then((url) => {
				if (url) mcpUrl = url;
			})
			.catch((error: unknown) => {
				failure = i18n.t('err.mcp', { detail: String(error) });
			});

		return () => {
			stops.forEach((stop) => stop());
			terminal?.dispose();
		};
	});
</script>

<main>
	<header>
		<h1>sshboard</h1>
		<p class="phase">{i18n.t('app.phase0')}</p>
		<p class="phase">{i18n.t('app.driven')}</p>
		<div class="settings">
			<span class="settings-label">{i18n.t('theme.label')}</span>
			{#each ['auto', 'light', 'dark'] as const as mode (mode)}
				<button
					type="button"
					class:active={theme.mode === mode}
					onclick={() => theme.set(mode as ThemeMode)}
				>
					{i18n.t(`theme.${mode}`)}
				</button>
			{/each}

			<span class="settings-label lang">{i18n.t('lang.label')}</span>
			<select
				value={i18n.locale}
				onchange={(event) => i18n.set((event.currentTarget as HTMLSelectElement).value)}
			>
				{#each LOCALES as locale (locale.code)}
					<option value={locale.code}>{locale.native}</option>
				{/each}
			</select>
		</div>

		<p class="mcp">
			{i18n.t('mcp.label')}:
			{#if mcpUrl}
				<code>{mcpUrl}</code>
			{:else}
				<span class="waiting">{i18n.t('mcp.starting')}</span>
			{/if}
		</p>
	</header>

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}

	<nav class="tabs">
		<button type="button" class:active={view === 'connections'} onclick={() => (view = 'connections')}>
			{i18n.t('tab.connections')}
		</button>
		<button type="button" class:active={view === 'band'} onclick={() => (view = 'band')}>
			{i18n.t('tab.band')}
		</button>
	</nav>

	{#if view === 'connections'}
		<ConnectionManager />
	{:else}
	<section class="stream" aria-label={i18n.t('stream.label')}>
		<div class="stream-head">
			<span class="label">{i18n.t('stream.label')}</span>
			<span class="scaffold">{i18n.t('stream.scaffold')}</span>
			<button type="button" onclick={startDemoStream} disabled={streaming}>
				{i18n.t('stream.start')}
			</button>
			<button type="button" onclick={stopStream}>{i18n.t('stream.stop')}</button>
		</div>
		<div class="terminal" bind:this={terminalHost}></div>
	</section>

	<section class="band" aria-label={i18n.t('band.label')}>
		{#if lines.length === 0}
			<p class="empty">{i18n.t('band.empty')}</p>
		{:else}
			<ol>
				{#each lines as line (line.seq)}
					<li class:ai={line.tag === '[AI]'}>{line.rendered}</li>
				{/each}
			</ol>
		{/if}
	</section>
	{/if}
</main>

<style>
	/* 色は tokens.css の変数だけを使う。**ここに 16 進数を書かない。** */
	:global(body) {
		margin: 0;
		background: var(--bg);
		color: var(--fg);
		font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
		padding: 1rem 1.25rem;
		box-sizing: border-box;
		gap: 0.75rem;
	}

	h1 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.phase,
	.mcp {
		margin: 0.25rem 0 0;
		font-size: 0.8rem;
		color: var(--fg-muted);
	}

	.waiting {
		color: var(--fg-muted);
	}

	.settings {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		margin-top: 0.4rem;
		font-size: 0.75rem;
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.settings-label {
		margin-right: 0.2rem;
	}

	.settings-label.lang {
		margin-left: 0.6rem;
	}

	.settings select {
		font: inherit;
		font-size: 0.75rem;
		color: var(--fg);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		padding: 0.1rem 0.3rem;
	}

	.settings button,
	.tabs button {
		font: inherit;
		font-size: 0.75rem;
		color: var(--fg-muted);
		background: none;
		border: 1px solid transparent;
		border-radius: 3px;
		padding: 0.15rem 0.55rem;
		cursor: pointer;
	}

	.settings button.active,
	.tabs button.active {
		color: var(--fg);
		border-color: var(--border-strong);
		background: var(--bg-raised);
	}

	.failure {
		margin: 0;
		padding: 0.5rem 0.75rem;
		background: var(--danger-bg);
		color: var(--danger-fg);
		font-size: 0.8rem;
	}

	.tabs {
		display: flex;
		gap: 0.35rem;
		border-top: 1px solid var(--border);
		padding-top: 0.6rem;
	}

	.stream {
		display: flex;
		flex-direction: column;
		flex: 2;
		min-height: 0;
		border-top: 1px solid var(--border);
		padding-top: 0.75rem;
		gap: 0.5rem;
	}

	.stream-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.stream-head .label {
		flex: 1;
	}

	/* このボタンは製品の操作ではない。**002 が通ったら消える足場。** */
	.stream-head .scaffold {
		padding: 0.1rem 0.4rem;
		border: 1px dashed var(--warning-fg);
		border-radius: 3px;
		color: var(--warning-fg);
	}

	.stream-head button {
		font: inherit;
		font-size: 0.75rem;
		color: var(--fg);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		padding: 0.2rem 0.6rem;
		cursor: pointer;
	}

	.stream-head button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	/* 端末は暗いままにする。ANSI の既定色は暗い背景に載る前提で作られている。 */
	.terminal {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		background: var(--terminal-bg);
	}

	.band {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		border-top: 1px solid var(--border);
		padding-top: 0.75rem;
	}

	.band ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.band li {
		white-space: pre;
		font-size: 0.85rem;
		line-height: 1.6;
	}

	.band li.ai {
		color: var(--accent);
	}

	.empty {
		margin: 0;
		font-size: 0.8rem;
		color: var(--fg-faint);
	}
</style>
