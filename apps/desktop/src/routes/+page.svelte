<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import ConnectionManager from '$lib/components/ConnectionManager.svelte';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import { LOCALES } from '$lib/i18n/locales';
	import { textSize } from '$lib/text-size/text-size.svelte';
	import { theme, type ThemeMode } from '$lib/theme/theme.svelte';
	import { createTerminal, writeChunk } from '$lib/terminal.svelte';
	import '@xterm/xterm/css/xterm.css';
	import type { Terminal } from '@xterm/xterm';

	type McpAccess = { url: string; token: string };

	/** 何が起きたかの記録。Rust 側の `Event` と対。**接続先は入っていない。** */
	type DiagEvent = {
		seq: number;
		atMs: number;
		level: 'info' | 'warn' | 'error';
		stage: string;
		connection?: string;
		message: string;
		hint?: string;
	};

	let lines = $state<BandLine[]>([]);
	let mcp = $state<McpAccess | null>(null);
	let mcpCopied = $state(false);
	let failure = $state<string | null>(null);
	let streaming = $state(false);
	let view = $state<'files' | 'connections' | 'band' | 'diag'>('files');
	let diag = $state<DiagEvent[]>([]);

	/** 記録を取り直す。**押したときと、診断を開いたときに読む。** */
	async function loadDiagnostics() {
		try {
			diag = await invoke<DiagEvent[]>('diagnostics_recent', { limit: 200 });
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	// 診断を開いたら、その場で読む。**古い記録を見せない。**
	$effect(() => {
		if (view === 'diag') loadDiagnostics();
	});

	/**
	 * エージェントへ貼る 1 行。
	 *
	 * **合言葉は起動ごとに変わる**（D23）ので、覚えて書き写す形にしない。
	 * ここを写して貼るだけで済むようにする。
	 */
	const mcpCommand = $derived(
		mcp
			? `claude mcp add --transport http sshboard ${mcp.url} --header "Authorization: Bearer ${mcp.token}"`
			: ''
	);

	async function copyMcpCommand() {
		if (!mcpCommand) return;
		try {
			await navigator.clipboard.writeText(mcpCommand);
			mcpCopied = true;
			setTimeout(() => (mcpCopied = false), 2000);
		} catch (error: unknown) {
			// 写せないことを黙らない。**押したのに何も起きない**が一番困る。
			failure = String(error);
		}
	}

	let terminalHost: HTMLDivElement | undefined = $state();
	let terminal: Terminal | undefined;

	// **端末の字も一緒に変える。**xterm.js は自前で描くので `rem` が効かない。
	// 画面だけ大きくなって端末が小さいままだと、同じ 1 つの道具に見えない。
	$effect(() => {
		const px = textSize.terminalPx;
		if (terminal) terminal.options.fontSize = px;
	});

	/**
	 * `⌘ +` / `⌘ −` / `⌘ 0`。**この操作は覚えなくても手が知っている。**
	 *
	 * WebView 側の既定の拡大は効かない（アプリなので）。
	 * 押しても何も起きないより、**期待どおりに動く方**へ寄せる。
	 */
	function onKeydown(event: KeyboardEvent) {
		if (!(event.metaKey || event.ctrlKey)) return;
		// `+` は Shift 付きだったり `=` だったりする。**両方受ける。**
		if (event.key === '+' || event.key === '=' || event.key === ';') {
			textSize.step(1);
		} else if (event.key === '-' || event.key === '_') {
			textSize.step(-1);
		} else if (event.key === '0') {
			textSize.set('normal');
		} else {
			return;
		}
		event.preventDefault();
	}

	/** 追いたいログのパス。**サーバー側**（右のペインで見ているのと同じ場所）。 */
	let followPath = $state('/var/log/messages');

	/**
	 * サーバーのログを追う（`tail -f`）。
	 *
	 * **GUI へは色付き・MCP へは素**で流れます（Issue 005）。
	 * コマンドは Rust 側が組み立てるので、**任意の文字列はシェルへ渡りません**（D3）。
	 */
	async function follow() {
		const path = followPath.trim();
		if (!path) return;
		try {
			await invoke('stream_follow', { path, lines: null });
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
			terminal = createTerminal(terminalHost, textSize.terminalPx);
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

		// OS のメニュー（表示 > 文字を大きく）から。**大きさを持っているのは画面側。**
		listen<'larger' | 'smaller' | 'reset'>('menu://text-size', (event) => {
			if (event.payload === 'reset') textSize.set('normal');
			else textSize.step(event.payload === 'larger' ? 1 : -1);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = i18n.t('err.subscribe', { detail: String(error) });
			});

		listen<McpAccess>('mcp://ready', (event) => {
			mcp = event.payload;
		})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 下の取り直しで拾う */
			});

		// 起動が速いと 'mcp://ready' を購読より先に取りこぼす。取り直す。
		invoke<McpAccess | null>('mcp_url')
			.then((access) => {
				if (access) mcp = access;
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

<svelte:window onkeydown={onKeydown} />

<main>
	<!-- **上の帯は 1 行に収める。**6 段積むと、道具として使う面積が消える。 -->
	<header>
		<span class="phase" title={i18n.t('app.driven')}>{i18n.t('app.phase0')}</span>

		<nav class="tabs">
			<!-- **既定はファイル**（PRD §1）。副ユーザーに端末を覚えさせない。 -->
			<button type="button" class:active={view === 'files'} onclick={() => (view = 'files')}>
				<Icon name="folder" />
				{i18n.t('tab.files')}
			</button>
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
			<button type="button" class:active={view === 'diag'} onclick={() => (view = 'diag')}>
				<Icon name="warning" />
				{i18n.t('tab.diag')}
			</button>
		</nav>

		<div class="settings">
			<!-- 文字サイズ。**この道具は小さい字が画面いっぱいに並ぶ。**読めなければ始まらない。
			     いまどの段かを間に出す。**出さないと、押した結果が分からない。**
			     OS のメニュー（表示）にも同じものを置いてある。 -->
			<button
				type="button"
				class="icon-only text-step"
				onclick={() => textSize.step(-1)}
				disabled={textSize.atSmallest}
				title={`${i18n.t('text.label')}: ${i18n.t('text.smaller')}`}
				aria-label={`${i18n.t('text.label')}: ${i18n.t('text.smaller')}`}
			>
				<span class="smaller-a">A</span>
			</button>
			<span class="text-now" aria-hidden="true">{i18n.t(`text.${textSize.mode}`)}</span>
			<button
				type="button"
				class="icon-only text-step"
				onclick={() => textSize.step(1)}
				disabled={textSize.atLargest}
				title={`${i18n.t('text.label')}: ${i18n.t('text.larger')}`}
				aria-label={`${i18n.t('text.label')}: ${i18n.t('text.larger')}`}
			>
				<span class="larger-a">A</span>
			</button>

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

		<!-- **合言葉ごと写せる形にする**（D23）。起動ごとに変わるので、書き写させない。 -->
		<button
			type="button"
			class="mcp"
			onclick={copyMcpCommand}
			disabled={!mcp}
			title={mcp ? i18n.t('mcp.token.help') : 'MCP'}
		>
			<Icon name={mcpCopied ? 'check' : 'copy'} size={12} />
			{#if !mcp}{i18n.t('mcp.starting')}{:else if mcpCopied}{i18n.t('mcp.copied')}{:else}{i18n.t(
					'mcp.copy'
				)}{/if}
		</button>
	</header>

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}

	{#if view === 'files'}
		<FileBrowser />
	{:else if view === 'connections'}
		<ConnectionManager />
	{:else if view === 'diag'}
		<!-- **何が起きたか。**MCP の `diagnostics` と同じ 1 つを見ている。 -->
		<section class="diag" aria-label={i18n.t('tab.diag')}>
			<div class="diag-head">
				<span class="label">{i18n.t('diag.label')}</span>
				<span class="scaffold">{i18n.t('diag.help')}</span>
				<button type="button" onclick={loadDiagnostics}>
					<Icon name="refresh" size={13} />
					{i18n.t('diag.refresh')}
				</button>
			</div>
			{#if diag.length === 0}
				<p class="empty">{i18n.t('diag.empty')}</p>
			{:else}
				<ol class="diag-list">
					{#each diag as event (event.seq)}
						<li class={event.level}>
							<span class="at">{(event.atMs / 1000).toFixed(1)}s</span>
							<span class="stage">{event.stage}</span>
							{#if event.connection}<span class="who">{event.connection}</span>{/if}
							<span class="what">
								{event.message}
								{#if event.hint}<em class="hint">→ {event.hint}</em>{/if}
							</span>
						</li>
					{/each}
				</ol>
			{/if}
		</section>
	{:else}
	<section class="stream" aria-label={i18n.t('stream.label')}>
		<!-- **この面が何をする所かを、その場に書く。**
		     読んで分からない面は、無いのと同じ（実際に分からなかった）。 -->
		<p class="what">{i18n.t('stream.what')}</p>
		<div class="stream-head">
			<span class="label">{i18n.t('stream.label')}</span>
			<input
				class="path"
				bind:value={followPath}
				onkeydown={(event) => event.key === 'Enter' && follow()}
				placeholder={i18n.t('stream.path')}
				aria-label={i18n.t('stream.path')}
				spellcheck="false"
				data-secret
			/>
			<button type="button" onclick={follow} disabled={streaming}>
				{i18n.t('stream.start')}
			</button>
			<button type="button" onclick={stopStream}>{i18n.t('stream.stop')}</button>
		</div>
		<div class="terminal shell">
			<div class="core terminal-core" bind:this={terminalHost}></div>
		</div>
	</section>

	<section class="band" aria-label={i18n.t('band.label')}>
		<!-- **帯が何なのかを書く。**「ここに何が出るのか」が分からないと、
		     空のときに壊れているのか、何も起きていないのかが区別できない。 -->
		<p class="what">{i18n.t('band.what')}</p>
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

	/* **合言葉ごと写せるボタン。**起動ごとに変わる値を人に書き写させない（D23）。 */
	/* 文字サイズ。**アイコンではなく「A」の大小そのもの**で示す方が伝わる。 */
	.text-step {
		line-height: 1;
		font-weight: 600;
	}

	.smaller-a {
		font-size: 0.62rem;
	}

	.larger-a {
		font-size: 0.86rem;
	}

	/* いまどの段か。**出さないと、押した結果が分からない。** */
	.text-now {
		font-size: 0.62rem;
		color: var(--fg-muted);
		white-space: nowrap;
		padding: 0 0.1rem;
		min-width: 2.2rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}

	/* 診断。**帯とは別。**帯は「誰が何をしたか」、ここは「なぜそうなったか」。 */
	.diag {
		display: flex;
		flex-direction: column;
		min-height: 0;
		flex: 1 1 auto;
		gap: 0.4rem;
	}

	.diag-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.diag-list {
		list-style: none;
		margin: 0;
		padding: 0.3rem;
		overflow: auto;
		flex: 1 1 auto;
		min-height: 0;
		background: var(--surface);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		font-size: 0.7rem;
		line-height: 1.6;
	}

	.diag-list li {
		display: flex;
		gap: 0.45rem;
		padding: 0.12rem 0.4rem;
		align-items: baseline;
	}

	.diag-list li.error {
		color: var(--danger);
	}

	.diag-list li.warn {
		color: var(--warning);
	}

	.at {
		font-family: var(--font-mono);
		font-size: 0.62rem;
		color: var(--fg-faint);
		font-variant-numeric: tabular-nums;
		flex: none;
		min-width: 3.2rem;
		text-align: right;
	}

	.stage,
	.who {
		font-size: 0.62rem;
		color: var(--fg-muted);
		flex: none;
		white-space: nowrap;
	}

	.who {
		padding: 0 0.3rem;
		border: 1px solid var(--hairline);
		border-radius: 999px;
	}

	.what {
		flex: 1 1 auto;
		min-width: 0;
	}

	/* 次の一手。**失敗には必ず付く。** */
	.hint {
		display: block;
		font-style: normal;
		color: var(--fg-muted);
		font-size: 0.66rem;
	}

	.mcp {
		display: inline-flex;
		align-items: center;
		gap: 0.28rem;
		font-family: var(--font-ui);
		font-size: 0.66rem;
		color: var(--fg-muted);
		white-space: nowrap;
		flex: none;
		padding: 0.2rem 0.5rem;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		background: var(--shell);
		cursor: pointer;
		transition: background var(--fast) var(--ease);
	}

	.mcp:hover:not(:disabled) {
		background: var(--surface-2);
	}

	.mcp:disabled {
		opacity: 0.5;
		cursor: default;
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

	/* **この面が何をする所か**を書く 1 行。読んで分からない面は、無いのと同じ。 */
	.what {
		margin: 0 0 0.35rem;
		font-size: 0.72rem;
		line-height: 1.6;
		color: var(--fg-muted);
	}

	/* 追うログのパス。**入力欄が伸びて、ボタンが右へ寄る。** */
	.stream-head .path {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: 0.72rem;
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
