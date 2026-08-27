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

		<div class="pair">
			<label>
				<span>タグ（{CONNECTION_TAG_MAX_CHARS} 文字まで）</span>
				<input bind:value={draft.tag} placeholder="本番 / 開発 / 開発2" />
				<small>色が見えなくても効く方の印です。白黒の画面写真でも読めます。</small>
			</label>
			<label>
				<span>色</span>
				<div class="swatches">
					<button
						type="button"
						class="swatch none"
						class:picked={!draft.color}
						title="印なし"
						aria-label="印なし"
						onclick={() => (draft.color = null)}
					></button>
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
	/* 見た目は決まっていません（DESIGN.md）。
	   左に一覧・右に入力の 2 面は HeidiSQL / WinSCP のサイトマネージャに合わせています。 */
	/* 印の色。**16 進数を設定ファイルに書かず、名前で持つ**（Rust 側の CONNECTION_COLORS）。
	   ここが「名前 → 実際の色」の対応表で、明暗のテーマを足すときはここだけ増やす。
	   **配色そのものは仮置きです**（DESIGN.md）。 */
	.manager {
		--mark-red: #d9534f;
		--mark-orange: #e08a3c;
		--mark-yellow: #d9b03c;
		--mark-green: #4fa85f;
		--mark-teal: #3a8f7a;
		--mark-blue: #4a86c8;
		--mark-purple: #8a6fc4;
		--mark-pink: #c46f9e;
	}

	.manager {
		display: grid;
		grid-template-columns: 220px 1fr;
		gap: 1rem;
		flex: 1;
		min-height: 0;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-right: 1px solid #262a31;
		padding-right: 0.75rem;
		min-height: 0;
	}

	.list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 0.8rem;
		color: #8b929e;
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
		color: #d7dae0;
		font: inherit;
		cursor: pointer;
	}

	.row:hover {
		background: #1d2129;
	}

	.row.selected {
		border-color: #3a8f7a;
		background: #1d2129;
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

	.swatches {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	.swatch {
		width: 18px;
		height: 18px;
		padding: 0;
		border-radius: 3px;
		border: 1px solid #333944;
		cursor: pointer;
	}

	.swatch.none {
		background: repeating-linear-gradient(45deg, #22262e, #22262e 3px, #2c313a 3px, #2c313a 6px);
	}

	.swatch.picked {
		outline: 2px solid #d7dae0;
		outline-offset: 1px;
	}

	.row-id {
		font-size: 0.72rem;
		color: #6f7681;
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
		color: #8b929e;
	}

	input {
		font: inherit;
		font-size: 0.85rem;
		color: #d7dae0;
		background: #1d2129;
		border: 1px solid #333944;
		border-radius: 3px;
		padding: 0.3rem 0.45rem;
	}

	input:disabled {
		opacity: 0.6;
	}

	small {
		color: #6f7681;
		font-size: 0.7rem;
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 90px;
		gap: 0.6rem;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.3rem;
	}

	button {
		font: inherit;
		font-size: 0.78rem;
		color: #d7dae0;
		background: #22262e;
		border: 1px solid #333944;
		border-radius: 3px;
		padding: 0.25rem 0.7rem;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.danger {
		color: #ffb4b4;
		border-color: #5a2d2d;
	}

	.blocker,
	.empty,
	.path {
		font-size: 0.72rem;
		color: #6f7681;
	}

	.path {
		word-break: break-all;
		margin: 0;
	}

	.notice {
		margin: 0;
		font-size: 0.78rem;
		color: #7fd1b9;
	}

	.failure {
		margin: 0;
		padding: 0.4rem 0.6rem;
		background: #3a1d1d;
		color: #ffb4b4;
		font-size: 0.78rem;
	}

	.warning {
		margin: 0;
		padding: 0.5rem 0.6rem;
		background: #3a331d;
		color: #ffe0a0;
		font-size: 0.75rem;
		line-height: 1.6;
	}

	.warning code {
		display: block;
		margin-top: 0.3rem;
		color: #d7dae0;
	}
</style>
