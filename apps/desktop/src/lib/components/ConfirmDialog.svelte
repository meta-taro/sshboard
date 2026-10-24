<script lang="ts">
	/**
	 * **押し間違いの結果が他と違うとき、1 回聞く**（product-baseline §13）。
	 *
	 * 文言は呼ぶ側が渡します。**ここは言葉を持ちません** ——
	 * 持つと「この問いは何の問いか」が部品の中に隠れます。
	 *
	 * **閉じ方を 3 つ置きます**（`Escape`・外側・［いいえ］）——
	 * どれか 1 つだけだと、その 1 つを知らない人は閉じられません。
	 *
	 * **既定は「いいえ」側**です。`Escape` と外側はどちらも `onno` に行きます ——
	 * 迷って閉じた人に、やったことにしない。
	 */
	let {
		title,
		body,
		yes,
		no,
		onyes,
		onno
	}: {
		title: string;
		body: string;
		yes: string;
		no: string;
		onyes: () => void;
		onno: () => void;
	} = $props();
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && onno()} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="veil" role="presentation" onclick={onno}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="sheet"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		aria-label={title}
		onclick={(event) => event.stopPropagation()}
	>
		<!-- **題字に flex を置きません**（2026-09-18 / 09-21 に 3 度踏んだ形）——
		     WebKit では flex 容器の高さが中身に追いつかず、題字が折れると本文に重なる。 -->
		<header>
			<h2>{title}</h2>
		</header>
		<p class="body">{body}</p>
		<div class="actions">
			<button type="button" onclick={onno}>{no}</button>
			<button type="button" class="cta" onclick={onyes}>{yes}</button>
		</div>
	</div>
</div>

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 95;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.55);
	}

	.sheet {
		width: min(26rem, 100%);
		max-height: 100%;
		overflow: auto;
		padding: 1rem 1.1rem;
		border: 1px solid var(--danger);
		border-radius: var(--r-shell);
		background: var(--surface);
		box-shadow: var(--lift-3);
	}

	header {
		margin-bottom: 0.4rem;
	}

	h2 {
		margin: 0;
		font-size: 0.95rem;
		color: var(--danger);
	}

	.body {
		margin: 0;
		font-size: var(--text-control);
		line-height: 1.7;
		color: var(--fg);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.4rem;
		margin-top: 0.9rem;
	}

	button {
		font: inherit;
		font-size: var(--text-control);
		color: var(--fg-control);
		background: var(--shell-bg);
		border: 0;
		border-radius: var(--r-pill);
		padding: 0 12px;
		cursor: pointer;
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		height: var(--control-h);
	}

	button:hover {
		background: var(--surface-2);
	}

	/* **やる側は赤。**押し間違いの結果が他と違う。 */
	.cta {
		color: var(--ground);
		background: var(--danger);
		border-color: var(--danger);
		font-weight: 700;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}
</style>
