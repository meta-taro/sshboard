<script lang="ts">
	/**
	 * 済んだことを知らせる小さな面。**しばらくして自分で消えます。**
	 *
	 * 以前は保存の知らせをフォームの先頭に置いていました。
	 * **スクロール位置によっては見切れて、保存できたのか分からない**
	 * という指摘を実機で受けています（2026-09-03）。
	 *
	 * **画面の隅へ浮かせます。**更新の知らせ（D34）と同じ見せ方に揃えて、
	 * 「済んだこと」はいつも同じ場所に出るようにします。
	 *
	 * **失敗はここへ出しません。**消えてよいのは「済んだこと」だけで、
	 * 直すべきことは消えずに残るべきです。
	 */
	import Icon from '$lib/components/Icon.svelte';

	interface Props {
		text: string;
		/** 消えるまで。**押し直す間もなく消えない長さ**にしています。 */
		after?: number;
		onDone: () => void;
	}
	let { text, after = 4000, onDone }: Props = $props();

	$effect(() => {
		// `text` が変わったら測り直す。**続けて保存しても、最後の分だけ残る。**
		void text;
		const timer = setTimeout(onDone, after);
		return () => clearTimeout(timer);
	});
</script>

<!-- **読み上げにも届くように。**目で追っていない人にも「済んだ」が伝わります。 -->
<p class="toast" role="status" aria-live="polite">
	<Icon name="check" size={13} />
	{text}
</p>

<style>
	.toast {
		position: fixed;
		left: 50%;
		bottom: 1.1rem;
		transform: translateX(-50%);
		z-index: 65;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin: 0;
		padding: 0.5rem 0.9rem;
		font-size: 0.8rem;
		color: var(--fg);
		background: var(--surface);
		border: 1px solid var(--hairline-strong);
		border-radius: 999px;
		box-shadow: var(--lift-3);
		max-width: min(28rem, calc(100vw - 2rem));
	}

	@media (prefers-reduced-motion: no-preference) {
		.toast {
			animation: rise 160ms ease-out;
		}
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translate(-50%, 6px);
		}
		to {
			opacity: 1;
			transform: translate(-50%, 0);
		}
	}
</style>
