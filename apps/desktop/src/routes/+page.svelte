<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import ConnectionManager from '$lib/components/ConnectionManager.svelte';
	import Icon from '$lib/components/Icon.svelte';
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
	<!-- **上の帯は 1 行に収める。**6 段積むと、道具として使う面積が消える。 -->
	<header>
		<span class="phase" title={i18n.t('app.driven')}>{i18n.t('app.phase0')}</span>

		<nav class="tabs">
			<button
				type="button"
				class:active={view === 'connections'}
				onclick={() => (view = 'connections')}
			>
				<Icon name="server" />
				{i18n.t('tab.connections')}
			</button>
			<button type="button" class:active={view === 'band'} onclick={() => (view = 'band')}>
				<Icon name="activity" />
				{i18n.t('tab.band')}
			</button>
		</nav>

		<div class="settings">
			<span class="settings-icon"><Icon name="globe" size={13} /></span>
			{#each ['auto', 'light', 'dark'] as const as mode (mode)}
				<button
					type="button"
					class="icon-only"
					class:active={theme.mode === mode}
					title={`${i18n.t('theme.label')}: ${i18n.t(`theme.${mode}`)}`}
					aria-label={`${i18n.t('theme.label')}: ${i18n.t(`theme.${mode}`)}`}
					onclick={() => theme.set(mode as ThemeMode)}
				>
					<Icon name={mode === 'auto' ? 'contrast' : mode === 'light' ? 'sun' : 'moon'} />
				</button>
			{/each}
			<select
				value={i18n.locale}
				aria-label={i18n.t('lang.label')}
				onchange={(event) => i18n.set((event.currentTarget as HTMLSelectElement).value)}
			>
				{#each LOCALES as locale (locale.code)}
					<option value={locale.code}>{locale.native}</option>
				{/each}
			</select>
		</div>

		<code class="mcp" title="MCP"><Icon name="link" size={12} />
			{#if mcpUrl}{mcpUrl}{:else}{i18n.t('mcp.starting')}{/if}
		</code>
	</header>

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}

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
		<div class="terminal shell">
			<div class="core terminal-core" bind:this={terminalHost}></div>
		</div>
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
	/* 色は tokens.css の変数だけ。**ここに 16 進数を書かない。** */
	:global(body) {
		margin: 0;
		background: var(--ground);
		color: var(--fg);
		font-family: var(--font-ui);
		font-feature-settings: 'palt' 1;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
		padding: 0.9rem 1rem 1rem;
		box-sizing: border-box;
		gap: 0.7rem;
	}

	/* **上の帯は 1 行。**折り返しても 2 行までに収まる高さにする。 */
	/* **折り返さない。**折り返すと帯が 2 段になり、本文の面積が減る。
	   入りきらないときは、説明 → MCP の順に縮める。 */
	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: nowrap;
		min-width: 0;
	}

	/* 長い説明は常時出さない。**ここに置くと本文の面積が消える。**
	   全文は title 属性と、帯が空のときの案内に出す。 */
	.phase {
		font-size: 0.7rem;
		color: var(--fg-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		flex: 1 1 auto;
	}

	.mcp {
		display: inline-flex;
		align-items: center;
		gap: 0.28rem;
		font-family: var(--font-mono);
		font-size: 0.62rem;
		color: var(--fg-faint);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		flex: 0 1 auto;
	}

	/* 設定と切替は、浮いた 1 本のピルにまとめる。 */
	.settings,
	.tabs {
		display: flex;
		align-items: center;
		gap: 0.2rem;
		flex-wrap: nowrap;
		flex: none;
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 3px;
	}

	.settings-icon {
		display: inline-flex;
		align-items: center;
		padding: 0 0.15rem 0 0.4rem;
		color: var(--fg-faint);
	}

	/* **アイコンと文字を横に並べる。**これが無いと、SVG が block なので
	   文字の上に乗ってしまう（実際に乗った）。 */
	.settings button,
	.tabs button {
		display: inline-flex;
		align-items: center;
		gap: 0.32rem;
		font: inherit;
		font-size: 0.74rem;
		color: var(--fg-muted);
		background: transparent;
		border: 1px solid transparent;
		border-radius: 999px;
		padding: 0.2rem 0.7rem;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			color var(--fast) var(--ease),
			transform var(--fast) var(--ease);
	}

	.settings button.icon-only {
		padding: 0.26rem 0.4rem;
	}

	.settings button:active,
	.tabs button:active {
		transform: scale(0.97);
	}

	.settings button.active,
	.tabs button.active {
		color: var(--fg);
		background: var(--surface);
		box-shadow: var(--inner-highlight), var(--lift-1);
	}

	.settings button:focus-visible,
	.tabs button:focus-visible,
	.settings select:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.settings select {
		font: inherit;
		font-size: 0.74rem;
		color: var(--fg);
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 0.16rem 0.5rem;
		margin-left: 0.15rem;
		box-shadow: var(--inner-highlight);
	}

	.failure {
		margin: 0;
		padding: 0.55rem 0.8rem;
		border-radius: var(--r-control);
		background: var(--danger-soft);
		color: var(--danger);
		font-size: 0.78rem;
	}

	.stream {
		display: flex;
		flex-direction: column;
		flex: 2;
		min-height: 0;
		gap: 0.45rem;
	}

	.stream-head {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		flex-wrap: wrap;
	}

	.stream-head .label {
		flex: 1;
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	/* このボタンは製品の操作ではない。**002 が通ったら消える足場。** */
	.stream-head .scaffold {
		font-family: var(--font-mono);
		font-size: 9px;
		letter-spacing: 0.1em;
		padding: 0.1rem 0.5rem;
		border: 1px dashed var(--warning);
		border-radius: 999px;
		color: var(--warning);
	}

	.stream-head button {
		font: inherit;
		font-size: 0.72rem;
		color: var(--fg);
		background: var(--surface-2);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 0.22rem 0.75rem;
		cursor: pointer;
		box-shadow: var(--inner-highlight), var(--lift-1);
		transition: transform var(--fast) var(--ease);
	}

	.stream-head button:active {
		transform: scale(0.97);
	}

	.stream-head button:disabled {
		opacity: 0.45;
		cursor: default;
		box-shadow: none;
	}

	/* 端末は暗いまま。ANSI の既定色は暗い背景に載る前提で作られている。 */
	.terminal {
		flex: 1;
		min-height: 0;
	}

	.terminal-core {
		background: var(--terminal-bg);
		overflow: hidden;
		padding: 6px 8px;
	}

	.band {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		padding: 0.6rem 0.75rem;
	}

	.band ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.band li {
		white-space: pre;
		font-family: var(--font-mono);
		font-size: 0.76rem;
		line-height: 1.75;
		color: var(--fg-muted);
	}

	.band li.ai {
		color: var(--accent);
	}

	.empty {
		margin: 0;
		font-size: 0.76rem;
		color: var(--fg-faint);
	}
</style>
