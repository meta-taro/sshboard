<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, tick } from 'svelte';

	import { appendLine, type BandLine } from '$lib/band';

	let lines = $state<BandLine[]>([]);
	let mcpUrl = $state<string | null>(null);
	let failure = $state<string | null>(null);

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

		return () => stops.forEach((stop) => stop());
	});
</script>

<main>
	<header>
		<h1>sshboard</h1>
		<p class="phase">Phase 0 — 帯だけ。SSH には繋いでいません。</p>
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

	.band {
		flex: 1;
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
