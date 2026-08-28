<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount } from 'svelte';

	import {
		CONNECTION_COLORS,
		CONNECTION_TAG_MAX_CHARS,
		emptyConnection,
		isPuttyKey,
		whyNotSavable,
		type Connection
	} from '$lib/connections';

	let items = $state<Connection[]>([]);
	let draft = $state<Connection>(emptyConnection());
	let selectedId = $state<string | null>(null);
	let storePath = $state('');
	let notice = $state<string | null>(null);
	let failure = $state<string | null>(null);

	/** 編集中なら、自分自身の識別子は重複扱いにしない。 */
	const takenIds = $derived(items.map((item) => item.id).filter((id) => id !== selectedId));
	const blocker = $derived(whyNotSavable(draft, takenIds));
	const puttyWarning = $derived(isPuttyKey(draft.key_path));

	async function reload() {
		try {
			items = await invoke<Connection[]>('connections_list');
		} catch (error: unknown) {
			failure = `一覧を読めません: ${String(error)}`;
		}
	}

	function startNew() {
		draft = emptyConnection();
		selectedId = null;
		notice = null;
	}

	function edit(item: Connection) {
		draft = { ...item };
		selectedId = item.id;
		notice = null;
	}

	async function save() {
		failure = null;
		try {
			await invoke('connection_save', { entry: draft });
			notice = `${draft.id} を保存しました`;
			selectedId = draft.id;
			await reload();
		} catch (error: unknown) {
			failure = `保存できません: ${String(error)}`;
		}
	}

	async function remove(id: string) {
		failure = null;
		try {
			await invoke('connection_delete', { id });
			notice = `${id} を消しました`;
			if (selectedId === id) startNew();
			await reload();
		} catch (error: unknown) {
			failure = `消せません: ${String(error)}`;
		}
	}

	onMount(() => {
		const stops: Array<() => void> = [];

		reload();
		invoke<string>('connections_path')
			.then((path) => (storePath = path))
			.catch(() => {
				/* 置き場所が分からなくても一覧は出す */
			});

		// **人と AI の両方が一覧を書き換える。**
		// 開いたときに 1 回読むだけだと、AI が足した接続を人が知らないままになる
		// （PRD §4-2「AI の操作が人の画面にその場で流れる」）。
		listen('connections://changed', () => {
			reload();
		})
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = `一覧の更新を購読できません: ${String(error)}`;
			});

		return () => stops.forEach((stop) => stop());
	});
</script>

<section class="manager" aria-label="接続の登録">
	<aside class="list">
		<div class="list-head">
			<span>接続</span>
			<button type="button" onclick={startNew}>＋ 新規</button>
		</div>

		{#if items.length === 0}
			<p class="empty">まだ 1 件も登録されていません。</p>
		{:else}
			<ul>
				{#each items as item (item.id)}
					<li>
						<button
							type="button"
							class="row"
							class:selected={item.id === selectedId}
							style:--row-mark={item.color ? `var(--mark-${item.color})` : 'var(--accent)'}
							onclick={() => edit(item)}
						>
							<span class="row-top">
								<span
									class="chip"
									style:background={item.color ? `var(--mark-${item.color})` : 'transparent'}
									style:border-color={item.color ? `var(--mark-${item.color})` : '#3a4049'}
									aria-hidden="true"
								></span>
								<span class="row-name">{item.name || item.id}</span>
								{#if item.tag}
									<span class="row-tag" style:color={item.color ? `var(--mark-${item.color})` : '#8b929e'}>
										{item.tag}
									</span>
								{/if}
							</span>
							<span class="row-id">{item.id}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		{#if storePath}
			<p class="path" title={storePath}>{storePath}</p>
		{/if}
	</aside>

	<div class="form">
		{#if failure}
			<p class="failure" role="alert">{failure}</p>
		{:else if notice}
			<p class="notice">{notice}</p>
		{/if}

		<label>
			<span>識別子</span>
			<input bind:value={draft.id} placeholder="web-prod" disabled={selectedId !== null} />
			<small>AI に見えるのは、これと名前だけです。ホスト名は渡りません。</small>
		</label>

		<label>
			<span>名前</span>
			<input bind:value={draft.name} placeholder="Web (prod)" />
		</label>

		<div class="pair">
			<label>
				<span>ホスト</span>
				<input bind:value={draft.host} spellcheck="false" />
			</label>
			<label>
				<span>ポート</span>
				<input type="number" bind:value={draft.port} min="1" max="65535" />
			</label>
		</div>

		<label>
			<span>ログイン名</span>
			<input bind:value={draft.user} spellcheck="false" />
		</label>

		<label>
			<span>秘密鍵のパス（空なら ssh-agent）</span>
			<input bind:value={draft.key_path} placeholder="空のまま = ssh-agent を使う" spellcheck="false" />
			<small>
				<strong>空のままを推奨します。</strong>ssh-agent なら、パスフレーズを sshboard
				が一度も受け取りません。
			</small>
		</label>

		<div class="mark-row">
			<label class="tag">
				<span>タグ（{CONNECTION_TAG_MAX_CHARS} 文字まで）</span>
				<input
					bind:value={draft.tag}
					maxlength={CONNECTION_TAG_MAX_CHARS}
					placeholder="本番 / 開発2"
				/>
				<small>色が見えなくても効く方の印です。白黒の画面写真でも読めます。</small>
			</label>
			<label>
				<span>色</span>
				<div class="swatches">
					{#each CONNECTION_COLORS as color (color)}
						<button
							type="button"
							class="swatch"
							class:picked={draft.color === color}
							style:background="var(--mark-{color})"
							title={color}
							aria-label={color}
							onclick={() => (draft.color = color)}
						></button>
					{/each}
				</div>
				<button type="button" class="clear-mark" onclick={() => (draft.color = null)}>
					印なし
				</button>
			</label>
		</div>

		{#if puttyWarning}
			<p class="warning" role="alert">
				<strong>これは PuTTY 形式（.ppk）です。</strong>OpenSSH 系は読めません。先に変換してください。
				<code>puttygen &lt;鍵&gt;.ppk -O private-openssh -o &lt;鍵&gt;</code>
				変換して <code>ssh-add</code> しておけば、このパスは空で構いません。
			</p>
		{/if}

		<div class="actions">
			<button type="button" onclick={save} disabled={blocker !== ''}>保存</button>
			{#if selectedId}
				<button type="button" class="danger" onclick={() => remove(selectedId ?? '')}>削除</button>
			{/if}
			{#if blocker}
				<span class="blocker">{blocker}</span>
			{/if}
		</div>
	</div>
</section>

<style>
	/* 色は tokens.css の変数だけを使う。**ここに 16 進数を書かない。**
	   書くと、テーマを切り替えたときにそこだけ取り残される。 */
	.manager {
		display: grid;
		grid-template-columns: minmax(180px, 22%) 1fr;
		gap: 1rem;
		flex: 1;
		min-height: 0;
	}

	/* 窓が狭いときは 2 面をやめて縦に積む。**小さく使う人を締め出さない。** */
	@media (max-width: 720px) {
		.manager {
			grid-template-columns: 1fr;
			grid-template-rows: minmax(120px, 30%) 1fr;
		}

		.list {
			border-right: none;
			border-bottom: 1px solid var(--border);
			padding-right: 0;
			padding-bottom: 0.6rem;
		}
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-right: 1px solid var(--border);
		padding-right: 0.75rem;
		min-height: 0;
	}

	.list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 0.8rem;
		color: var(--fg-muted);
	}

	.list ul {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow-y: auto;
		flex: 1;
	}

	.row {
		display: flex;
		flex-direction: column;
		width: 100%;
		text-align: left;
		background: none;
		border: 1px solid transparent;
		border-radius: 3px;
		padding: 0.35rem 0.5rem;
		color: var(--fg);
		font: inherit;
		cursor: pointer;
	}

	.row:hover {
		background: var(--bg-raised);
	}

	/* **選択の枠はその接続の印の色。**印が無いときだけ既定色にする。 */
	.row.selected {
		border-color: var(--row-mark);
		background: var(--bg-raised);
	}

	.row-top {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.chip {
		width: 8px;
		height: 8px;
		border-radius: 2px;
		border: 1px solid;
		flex: none;
	}

	.row-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.row-tag {
		font-size: 0.68rem;
		border: 1px solid currentColor;
		border-radius: 2px;
		padding: 0 0.25rem;
		flex: none;
	}

	.row-id {
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		overflow-y: auto;
		padding-right: 0.25rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.78rem;
		color: var(--fg-muted);
	}

	input {
		font: inherit;
		font-size: 0.85rem;
		color: var(--fg);
		background: var(--bg-input);
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		padding: 0.3rem 0.45rem;
	}

	input:disabled {
		opacity: 0.6;
	}

	small {
		color: var(--fg-faint);
		font-size: 0.7rem;
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 90px;
		gap: 0.6rem;
	}

	/* タグは 12 文字までなので、入力欄も 12 文字分でよい。
	   **色見本は 2 行 × 4。**3 行にすると縦が伸びる。 */
	.mark-row {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.mark-row .tag input {
		width: 11rem;
	}

	.swatches {
		display: grid;
		grid-template-columns: repeat(4, 20px);
		gap: 0.25rem;
	}

	.swatch {
		width: 20px;
		height: 20px;
		padding: 0;
		border-radius: 3px;
		border: 1px solid var(--border-strong);
		cursor: pointer;
	}

	.swatch.picked {
		outline: 2px solid var(--fg);
		outline-offset: 1px;
	}

	.clear-mark {
		align-self: flex-start;
		margin-top: 0.3rem;
		font-size: 0.7rem;
		background: none;
		border: none;
		color: var(--fg-faint);
		text-decoration: underline;
		padding: 0;
		cursor: pointer;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.3rem;
		flex-wrap: wrap;
	}

	button {
		font: inherit;
		font-size: 0.78rem;
		color: var(--fg);
		background: var(--bg-raised);
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		padding: 0.25rem 0.7rem;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.danger {
		color: var(--danger-fg);
		border-color: var(--danger-fg);
	}

	.blocker,
	.empty,
	.path {
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	.path {
		word-break: break-all;
		margin: 0;
	}

	.notice {
		margin: 0;
		font-size: 0.78rem;
		color: var(--ok-fg);
	}

	.failure {
		margin: 0;
		padding: 0.4rem 0.6rem;
		background: var(--danger-bg);
		color: var(--danger-fg);
		font-size: 0.78rem;
	}

	.warning {
		margin: 0;
		padding: 0.5rem 0.6rem;
		background: var(--warning-bg);
		color: var(--warning-fg);
		font-size: 0.75rem;
		line-height: 1.6;
	}

	.warning code {
		display: block;
		margin-top: 0.3rem;
		color: var(--fg);
	}
</style>
