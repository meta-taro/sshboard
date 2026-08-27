<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';
	import { createTerminal, writeChunk } from '$lib/terminal.svelte';
	import '@xterm/xterm/css/xterm.css';
	import type { Terminal } from '@xterm/xterm';

	let lines = $state<BandLine[]>([]);
	let mcpUrl = $state<string | null>(null);
	let failure = $state<string | null>(null);
	let streaming = $state(false);

	let terminalHost: HTMLDivElement | undefined = $state();
	let terminal: Terminal | undefined;

	async function startDemoStream() {
		try {
			await invoke('start_demo_stream');
			streaming = true;
		} catch (error: unknown) {
			failure = `出力を流せませんでした: ${String(error)}`;
		}
	}

	async function stopStream() {
		try {
			await invoke('stop_stream');
			streaming = false;
		} catch (error: unknown) {
			failure = `出力を止められませんでした: ${String(error)}`;
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
			failure = `帯の受け取りを返せませんでした: ${String(error)}`;
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
				failure = `出力を購読できませんでした: ${String(error)}`;
			});

		listen<BandLine>('band://line', async (event) => {
			lines = appendLine(lines, event.payload);
			await tick();
			ackRendered(event.payload.seq);
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = `帯を購読できませんでした: ${String(error)}`;
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
				failure = `MCP の状態を取得できませんでした: ${String(error)}`;
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
		<p class="phase">
			Phase 0 — 帯だけ。<strong>SSH には繋いでいません。</strong>
		</p>
		<p class="phase">
			本来はここを人が操作しません。外の AI エージェントが MCP を呼び、その操作が帯に流れます。
		</p>
		<p class="mcp">
			MCP:
			{#if mcpUrl}
				<code>{mcpUrl}</code>
			{:else}
				<span class="waiting">立ち上げ中…</span>
			{/if}
		</p>
	</header>

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}

	<section class="stream" aria-label="追尾している出力">
		<div class="stream-head">
			<span class="label">出力（GUI は色付き / MCP はプレーン）</span>
			<span class="scaffold">足場 — 002 が通ったら消えます</span>
			<button type="button" onclick={startDemoStream} disabled={streaming}>
				合成の出力を流す
			</button>
			<button type="button" onclick={stopStream}>止める</button>
		</div>
		<div class="terminal" bind:this={terminalHost}></div>
	</section>

	<section class="band" aria-label="操作の帯">
		{#if lines.length === 0}
			<p class="empty">まだ何も流れていません。MCP から <code>ping</code> を呼ぶと 1 行増えます。</p>
		{:else}
			<ol>
				{#each lines as line (line.seq)}
					<li class:ai={line.tag === '[AI]'}>{line.rendered}</li>
				{/each}
			</ol>
		{/if}
	</section>
</main>

<style>
	/*
	 * 見た目は決まっていません（DESIGN.md）。ここは Phase 0 で帯が出ることを
	 * 確かめるための最低限で、配色・字送りの方向性は人が決めます。
	 */
	:global(body) {
		margin: 0;
		background: #16181d;
		color: #d7dae0;
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
		color: #8b929e;
	}

	.waiting {
		color: #8b929e;
	}

	.failure {
		margin: 0;
		padding: 0.5rem 0.75rem;
		background: #3a1d1d;
		color: #ffb4b4;
		font-size: 0.8rem;
	}

	.stream {
		display: flex;
		flex-direction: column;
		flex: 2;
		min-height: 0;
		border-top: 1px solid #262a31;
		padding-top: 0.75rem;
		gap: 0.5rem;
	}

	.stream-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #8b929e;
	}

	.stream-head .label {
		flex: 1;
	}

	/* このボタンは製品の操作ではない。**002 が通ったら消える足場。** */
	.stream-head .scaffold {
		padding: 0.1rem 0.4rem;
		border: 1px dashed #4a3f2a;
		border-radius: 3px;
		color: #b9a06a;
	}

	.stream-head button {
		font: inherit;
		color: #d7dae0;
		background: #22262e;
		border: 1px solid #333944;
		border-radius: 3px;
		padding: 0.2rem 0.6rem;
		cursor: pointer;
	}

	.stream-head button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.terminal {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.band {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		border-top: 1px solid #262a31;
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
		color: #7fd1b9;
	}

	.empty {
		margin: 0;
		font-size: 0.8rem;
		color: #6f7681;
	}
</style>
