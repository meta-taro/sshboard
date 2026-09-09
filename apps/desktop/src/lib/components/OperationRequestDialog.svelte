<script lang="ts">
	/**
	 * **AI が状態を変える操作を走らせたい**（D45 / D47 / Issue #17）。
	 *
	 * `ConsoleRequestDialog`（端末の許可）と同じ形ですが、**違いが 1 つ**あります。
	 * **何が走るのかを、そのまま見せます。**
	 *
	 * > 「AI が doko001 で restart-httpd を実行したいと言っています
	 * >    **実行される内容: sudo systemctl restart httpd**」
	 *
	 * id だけでは答えられません。**`operations.toml` に書いた本人でも、
	 * 半年後には何を書いたか覚えていない**からです。
	 *
	 * ここで守っていること。
	 *
	 * - **既定は「断る」側。**Enter や Escape で勝手に許可されない
	 * - **1 回の許可で 1 回だけ。**釦の文言にもそう書きます
	 * - **接続先は出しません**（PRD §8）。出すのは操作の id と、打つ文字列だけ
	 * - **背景を押しても閉じません。**人が始めていない問いを、誤操作で消させない
	 *
	 * ## パスワード欄（D48 / Issue #19）
	 *
	 * `become = "ask"` の接続では、**ここが root へ届く唯一の道**です。
	 *
	 * > **AI 側から root 領域を読む道が、現時点でゼロです。**
	 *
	 * **保存しません。**この 1 回のために持ち、走った瞬間に捨てます。
	 * 画面から出て行くのは `operation_answer` の 1 回だけで、
	 * **記録にも帯にも端末の面にも流れません**（`Engine::answer_operation`）。
	 *
	 * **`needsSecret` が false のときは欄ごと出しません。**
	 * 要らないのに毎回パスワードを聞かれたら、人は使うのをやめます。
	 */
	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		/** 操作の識別子。**人が `operations.toml` に書いたもの。** */
		id: string;
		/** 実際にサーバーで走る文字列。 */
		runs: string;
		/**
		 * **この 1 回のためのパスワードが要るか**（D48）。
		 * `become = "ask"` の接続で `sudo` を打つときだけ true になります。
		 */
		needsSecret?: boolean;
		/** 入れたものは**そのまま engine へ渡り、走った瞬間に捨てられます。** */
		onAllow: (secret?: string) => void;
		onDeny: () => void;
	}
	let { id, runs, needsSecret = false, onAllow, onDeny }: Props = $props();

	let denyButton: HTMLButtonElement | undefined = $state();
	let secretField: HTMLInputElement | undefined = $state();
	let secret = $state('');

	// **焦点は「断る」に置く。**開いた勢いで Enter を打っても、許可になりません。
	//
	// **パスワードを聞くときも同じです。**入力欄へ先に飛ばすと、
	// 人は**何を許すのかを読む前に打ち始めます。**読ませてから打たせます。
	$effect(() => {
		denyButton?.focus();
	});

	function allow() {
		// **空欄で許可させない。**空のまま渡すと、サーバー側で
		// 「パスワードがありません」に落ちるだけで、**人には何が起きたか分かりません。**
		if (needsSecret && secret.length === 0) {
			secretField?.focus();
			return;
		}
		const typed = needsSecret ? secret : undefined;
		// **画面からも消す。**渡したあとに残しておく理由がありません。
		secret = '';
		onAllow(typed);
	}

	function onKeydown(event: KeyboardEvent) {
		// **Escape は「断る」。**黙って消さず、答えとして扱います。
		if (event.key === 'Escape') {
			event.preventDefault();
			secret = '';
			onDeny();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="backdrop" role="presentation">
	<div
		class="dialog"
		role="alertdialog"
		aria-modal="true"
		aria-label={i18n.t('operation.request.title')}
	>
		<header>
			<Icon name="warning" size={15} />
			<h2>{i18n.t('operation.request.title')}</h2>
		</header>

		<p class="what">{id}</p>
		<p class="lead">{i18n.t('operation.request.body')}</p>

		<!--
			**打つものを、そのまま出す。**丸めたり要約したりしません ——
			**要約した時点で、人は何を許したのか分からなくなります。**
		-->
		<pre class="runs">{runs}</pre>

		<p class="note">{i18n.t('operation.request.note')}</p>

		{#if needsSecret}
			<!--
				**`become = "ask"` のときだけ出します**（D48 / Issue #19）。
				**保存しません。**この 1 回のためだけに持ちます。
			-->
			<label class="secret">
				<span>{i18n.t('operation.request.secret')}</span>
				<input
					bind:this={secretField}
					bind:value={secret}
					type="password"
					autocomplete="off"
					spellcheck="false"
					onkeydown={(event) => {
						// **Enter で許可します。**打ち終えた指がそのまま進めるように。
						if (event.key === 'Enter') {
							event.preventDefault();
							allow();
						}
					}}
				/>
			</label>
			<p class="note">{i18n.t('operation.request.secret.note')}</p>
		{/if}

		<div class="actions">
			<button type="button" bind:this={denyButton} onclick={onDeny}>
				{i18n.t('operation.request.deny')}
			</button>
			<button type="button" class="cta" onclick={allow}>
				{i18n.t('operation.request.allow')}
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		/* **いちばん手前。**答えるまで、他の操作へ進ませない。 */
		z-index: 85;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 45%);
	}

	.dialog {
		width: min(30rem, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1.1rem;
		background: var(--surface);
		/* **色だけを手がかりにしない**（DESIGN.md）。枠でも分かるように。 */
		border: 1px solid var(--warning, var(--hairline-strong));
		border-radius: var(--r-shell);
		box-shadow: var(--lift-3);
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}

	header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.what {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.lead,
	.note {
		margin: 0;
		font-size: 0.78rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	/* **打つものは、折り返してでも全部見せる。**隠すと判断できません。 */
	.runs {
		margin: 0;
		padding: 0.6rem 0.7rem;
		background: var(--surface-2);
		border: 1px solid var(--hairline);
		border-radius: var(--r-item, 6px);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-all;
	}

	/* **欄は広く取る。**打ったものが見えない入力なので、狭いと不安になります。 */
	.secret {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.78rem;
	}

	.secret input {
		padding: 0.45rem 0.55rem;
		background: var(--surface-2);
		color: var(--fg);
		border: 1px solid var(--hairline-strong);
		border-radius: var(--r-item, 6px);
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
