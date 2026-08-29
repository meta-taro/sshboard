<script lang="ts">
	/**
	 * ファイルの面。**副ユーザー（SFTP / Tera Term の使い手）の既定の画面**（PRD §1）。
	 *
	 * 左が手元、右がサーバー。**AI を一度も使わなくても、ここだけで仕事になること。**
	 * 上げるのは人の操作なので、書き込みの囲い（D22）はかかりません。
	 */
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWebview } from '@tauri-apps/api/webview';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { onMount } from 'svelte';

	import Icon from './Icon.svelte';
	import type { Connection } from '$lib/connections';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import {
		baseName,
		humanSize,
		joinPath,
		parentOf,
		session,
		type Listed
	} from '$lib/session.svelte';

	/** 初見のホスト鍵。**人が確かめて登録するまで、ここで止める。** */
	type Untrusted = {
		algorithm: string;
		fingerprint: string;
		/** 登録済みの指紋。**あるのに食い違うなら、すり替えの疑い。** */
		expected: string | null;
	};

	type ConnectFailure =
		| ({ kind: 'untrusted' } & Untrusted)
		| { kind: 'passphraseNeeded' }
		| { kind: 'other'; message: string };

	let registered = $state<Connection[]>([]);
	let chosenId = $state('');
	let passphrase = $state('');
	let needsPassphrase = $state(false);
	let untrusted = $state<Untrusted | null>(null);

	let remotePath = $state('.');
	let entries = $state<Listed[]>([]);
	let loading = $state(false);

	/** 上げる待ちの手元のファイル。**パスだけを持つ**（中身は Rust 側で読む）。 */
	let staged = $state<string[]>([]);
	let dropping = $state(false);
	let newDirName = $state('');
	let notice = $state<string | null>(null);
	let failure = $state<string | null>(null);

	const connected = $derived(session.open !== null);

	async function loadRegistered() {
		try {
			registered = await invoke<Connection[]>('connections_list');
			if (!chosenId && registered.length > 0) chosenId = registered[0].id;
		} catch (error: unknown) {
			failure = i18n.t('err.list', { detail: String(error) });
		}
	}

	async function connect() {
		if (!chosenId || session.busy) return;
		failure = null;
		notice = null;
		untrusted = null;
		session.busy = true;
		try {
			await invoke('session_connect', {
				id: chosenId,
				passphrase: passphrase || null
			});
			// **入れてもらった秘密は、その場で捨てる**（D14）。
			passphrase = '';
			needsPassphrase = false;
			remotePath = '.';
			await refresh();
		} catch (error: unknown) {
			const why = error as ConnectFailure;
			if (why?.kind === 'untrusted') {
				// **行き止まりにしない。**指紋を見せて、人が確かめて登録できるようにする。
				untrusted = why;
			} else if (why?.kind === 'passphraseNeeded') {
				needsPassphrase = true;
				failure = i18n.t('files.passphrase.needed');
			} else {
				failure = why?.kind === 'other' ? why.message : String(error);
			}
		} finally {
			session.busy = false;
		}
	}

	/**
	 * 見えた指紋を、この接続の正解として登録する。**人が確かめたという記録。**
	 *
	 * 以後この接続は、**同じ指紋の相手としか繋がらない**（すり替えを検出できる）。
	 */
	async function trustFingerprint() {
		if (!untrusted) return;
		const entry = registered.find((held) => held.id === chosenId);
		if (!entry) return;

		try {
			await invoke('connection_save', {
				entry: { ...entry, fingerprint: untrusted.fingerprint }
			});
			untrusted = null;
			await loadRegistered();
			await connect();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	async function disconnect() {
		try {
			await invoke('session_disconnect');
			entries = [];
			staged = [];
			notice = null;
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	async function refresh() {
		if (!connected) return;
		loading = true;
		failure = null;
		try {
			entries = await invoke<Listed[]>('remote_list_dir', { path: remotePath });
		} catch (error: unknown) {
			failure = String(error);
		} finally {
			loading = false;
		}
	}

	async function enter(name: string) {
		remotePath = remotePath === '.' ? name : joinPath(remotePath, name);
		await refresh();
	}

	async function goUp() {
		remotePath = parentOf(remotePath);
		await refresh();
	}

	async function pickFiles() {
		try {
			const picked = await openDialog({ multiple: true, directory: false });
			if (!picked) return;
			stage(Array.isArray(picked) ? picked : [picked]);
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 同じものを二度並べない。**押すたびに増えるのは事故のもと。** */
	function stage(paths: string[]) {
		const merged = [...staged];
		for (const path of paths) if (!merged.includes(path)) merged.push(path);
		staged = merged;
	}

	function unstage(path: string) {
		staged = staged.filter((held) => held !== path);
	}

	async function upload() {
		if (staged.length === 0 || !connected) return;
		failure = null;
		notice = null;
		loading = true;
		try {
			const done = await invoke<Array<{ name: string; bytes: number }>>('remote_upload', {
				localPaths: staged,
				remoteDir: remotePath
			});
			const total = done.reduce((sum, one) => sum + one.bytes, 0);
			notice = i18n.t('files.uploaded', {
				count: String(done.length),
				size: humanSize(total)
			});
			staged = [];
			await refresh();
		} catch (error: unknown) {
			failure = String(error);
		} finally {
			loading = false;
		}
	}

	async function makeDir() {
		const name = newDirName.trim();
		if (!name || !connected) return;
		failure = null;
		try {
			await invoke('remote_ensure_dir', { path: joinPath(remotePath, name) });
			newDirName = '';
			await refresh();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	onMount(() => {
		const stops: Array<() => void> = [];
		loadRegistered();
		session
			.watch()
			.then((stop) => stops.push(stop))
			.catch((error: unknown) => {
				failure = String(error);
			});

		// **ウィンドウへ放り込むのが、この層の慣れた操作**（WinSCP / FFFTP）。
		// ここで取れるのは実体のパスで、ブラウザの `File` とは違う。
		getCurrentWebview()
			.onDragDropEvent((event) => {
				if (event.payload.type === 'over') dropping = true;
				else if (event.payload.type === 'drop') {
					dropping = false;
					stage(event.payload.paths);
				} else dropping = false;
			})
			.then((stop) => stops.push(stop))
			.catch(() => {
				/* 放り込めないだけ。**選ぶボタンは動く。** */
			});

		return () => stops.forEach((stop) => stop());
	});
</script>

<section class="files" class:dropping>
	<!-- どこへ繋がっているかを、**常に一番上に**出す。 -->
	<div class="bar shell">
		<div class="core">
			{#if session.open}
				<span class="live" title={session.open.fingerprint}>
					<Icon name="plug" />
					<strong>{session.open.name}</strong>
					{#if session.open.tag}<span class="tag">{session.open.tag}</span>{/if}
				</span>
				<code class="fp">{session.open.fingerprint}</code>
				<span class="scope" title={i18n.t('files.scope.help')}>
					{#if session.open.write.aiRoots.length > 0}
						{i18n.t('files.scope.some', { roots: session.open.write.aiRoots.join(' , ') })}
					{:else}
						{i18n.t('files.scope.none')}
					{/if}
				</span>
				<button type="button" onclick={disconnect}>
					<Icon name="unplug" />
					{i18n.t('files.disconnect')}
				</button>
			{:else}
				<!-- **プルダウンにしない。**印（色とタグ）が畳まれて見えなくなる。
				     どのサーバーかを間違えないための印が、選ぶ瞬間に消えては意味がない。 -->
				<span class="title">{i18n.t('files.choose')}</span>
				{#if needsPassphrase}
					<input
						type="password"
						bind:value={passphrase}
						placeholder={i18n.t('files.passphrase')}
						aria-label={i18n.t('files.passphrase')}
					/>
				{/if}
				{#if registered.length === 0}
					<span class="hint">{i18n.t('files.none')}</span>
				{/if}
			{/if}
		</div>
	</div>

	<!-- 繋いでいないときは、印つきの一覧から選ぶ。**色とタグが見えたまま選べる。** -->
	{#if !session.open && registered.length > 0}
		<div class="picker shell">
			<ul class="core">
				{#each registered as entry (entry.id)}
					<li>
						<button
							type="button"
							class="pick"
							class:chosen={chosenId === entry.id}
							style:--mark={entry.color ? `var(--mark-${entry.color})` : 'transparent'}
							onclick={() => (chosenId = entry.id)}
							ondblclick={() => {
								chosenId = entry.id;
								connect();
							}}
						>
							<span class="mark-bar" aria-hidden="true"></span>
							<span class="who">{entry.name || entry.id}</span>
							{#if entry.tag}<span class="tag">{entry.tag}</span>{/if}
							{#if entry.fingerprint}
								<span class="known" title={i18n.t('files.known')}>
									<Icon name="check" size={11} />
								</span>
							{/if}
						</button>
					</li>
				{/each}
			</ul>
			<div class="picker-foot">
				<button
					type="button"
					class="primary"
					onclick={connect}
					disabled={!chosenId || session.busy}
				>
					<Icon name="plug" />
					{session.busy ? i18n.t('files.connecting') : i18n.t('files.connect')}
				</button>
				<span class="hint">{i18n.t('files.pick.help')}</span>
			</div>

			<!-- **初動で迷わせない。**繋がるまでの 3 手順をその場に置く。 -->
			<ol class="steps">
				<li>{i18n.t('files.step1')}</li>
				<li>{i18n.t('files.step2')}</li>
				<li>{i18n.t('files.step3')}</li>
			</ol>
		</div>
	{/if}

	<!-- **初見の指紋は、行き止まりにしない。**確かめて登録する道をここに置く。 -->
	{#if untrusted}
		<div class="trust shell" class:danger={untrusted.expected}>
			<div class="core">
				{#if untrusted.expected}
					<p class="head">
						<Icon name="warning" size={14} />
						<strong>{i18n.t('files.trust.mismatch')}</strong>
					</p>
					<p class="body">{i18n.t('files.trust.mismatch.body')}</p>
					<dl>
						<dt>{i18n.t('files.trust.seen')}</dt>
						<dd><code>{untrusted.fingerprint}</code></dd>
						<dt>{i18n.t('files.trust.expected')}</dt>
						<dd><code>{untrusted.expected}</code></dd>
					</dl>
					<!-- **一押しで受け入れる道を置かない。**すり替えかもしれないため。 -->
					<p class="body">{i18n.t('files.trust.mismatch.how')}</p>
				{:else}
					<p class="head">
						<Icon name="key" size={14} />
						<strong>{i18n.t('files.trust.first')}</strong>
					</p>
					<p class="body">{i18n.t('files.trust.first.body')}</p>
					<dl>
						<dt>{untrusted.algorithm}</dt>
						<dd><code>{untrusted.fingerprint}</code></dd>
					</dl>
					<div class="trust-actions">
						<button type="button" class="primary" onclick={trustFingerprint}>
							<Icon name="check" />
							{i18n.t('files.trust.accept')}
						</button>
						<button type="button" onclick={() => (untrusted = null)}>
							{i18n.t('files.trust.cancel')}
						</button>
					</div>
				{/if}
			</div>
		</div>
	{/if}

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}
	{#if notice}
		<p class="notice" role="status">{notice}</p>
	{/if}

	<div class="panes">
		<!-- 手元 -->
		<div class="pane shell">
			<div class="core">
				<header>
					<Icon name="file" />
					<span class="title">{i18n.t('files.local')}</span>
					<button type="button" onclick={pickFiles}>
						<Icon name="plus" />
						{i18n.t('files.pick')}
					</button>
				</header>

				{#if staged.length === 0}
					<p class="empty">{i18n.t('files.drop')}</p>
				{:else}
					<ul class="list">
						{#each staged as path (path)}
							<li>
								<Icon name="file" size={12} />
								<span class="name" title={path}>{baseName(path)}</span>
								<button
									type="button"
									class="ghost"
									onclick={() => unstage(path)}
									aria-label={i18n.t('files.remove')}
								>
									<Icon name="trash" size={12} />
								</button>
							</li>
						{/each}
					</ul>
				{/if}

				<footer>
					<button
						type="button"
						class="primary"
						onclick={upload}
						disabled={!connected || staged.length === 0 || loading}
					>
						<Icon name="upload" />
						{i18n.t('files.upload', { count: String(staged.length) })}
					</button>
				</footer>
			</div>
		</div>

		<!-- サーバー -->
		<div class="pane shell">
			<div class="core">
				<header>
					<Icon name="server" />
					<button
						type="button"
						class="ghost"
						onclick={goUp}
						disabled={!connected}
						aria-label={i18n.t('files.up')}
						title={i18n.t('files.up')}
					>
						<Icon name="arrow-up" size={13} />
					</button>
					<input
						class="path"
						bind:value={remotePath}
						onkeydown={(event) => event.key === 'Enter' && refresh()}
						disabled={!connected}
						aria-label={i18n.t('files.remote')}
						spellcheck="false"
					/>
					<button
						type="button"
						class="ghost"
						onclick={refresh}
						disabled={!connected || loading}
						aria-label={i18n.t('files.refresh')}
						title={i18n.t('files.refresh')}
					>
						<Icon name="refresh" size={13} />
					</button>
				</header>

				{#if !connected}
					<p class="empty">{i18n.t('files.notconnected')}</p>
				{:else if entries.length === 0}
					<p class="empty">{loading ? i18n.t('files.loading') : i18n.t('files.emptydir')}</p>
				{:else}
					<ul class="list">
						{#each entries as entry (entry.name)}
							<li>
								<Icon name={entry.isDir ? 'folder' : 'file'} size={12} />
								{#if entry.isDir}
									<button type="button" class="link" onclick={() => enter(entry.name)}>
										{entry.name}
									</button>
								{:else}
									<span class="name">{entry.name}</span>
								{/if}
								<span class="size">{entry.isDir ? '' : humanSize(entry.size)}</span>
							</li>
						{/each}
					</ul>
				{/if}

				<footer>
					<input
						bind:value={newDirName}
						onkeydown={(event) => event.key === 'Enter' && makeDir()}
						placeholder={i18n.t('files.newdir')}
						aria-label={i18n.t('files.newdir')}
						disabled={!connected}
						spellcheck="false"
					/>
					<button type="button" onclick={makeDir} disabled={!connected || !newDirName.trim()}>
						<Icon name="folder" />
						{i18n.t('files.mkdir')}
					</button>
				</footer>
			</div>
		</div>
	</div>
</section>

<style>
	/* 色は tokens.css の変数だけ。**ここに 16 進数を書かない。** */
	.files {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-height: 0;
		flex: 1 1 auto;
	}

	/* 放り込める最中だけ、受け皿であることを見せる。 */
	.files.dropping .panes {
		outline: 1.5px dashed var(--accent);
		outline-offset: 3px;
		border-radius: var(--r-shell);
	}

	/* 二重縁。外の受け皿と、中身の面（DESIGN.md）。 */
	.shell {
		background: var(--shell);
		border: 1px solid var(--hairline);
		border-radius: var(--r-shell);
		padding: var(--shell-pad);
		min-height: 0;
	}

	.core {
		background: var(--surface);
		border-radius: var(--r-core);
		box-shadow: var(--inner-highlight);
		min-height: 0;
	}

	.bar .core {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		padding: 0.4rem 0.6rem;
		flex-wrap: nowrap;
		overflow: hidden;
	}

	.live {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		white-space: nowrap;
	}

	.tag {
		font-size: 0.62rem;
		padding: 0.05rem 0.32rem;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		color: var(--fg-muted);
	}

	.fp {
		font-family: var(--font-mono);
		font-size: 0.6rem;
		color: var(--fg-faint);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		flex: 0 1 auto;
	}

	.scope,
	.hint {
		font-size: 0.66rem;
		color: var(--fg-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		flex: 1 1 auto;
	}

	/* 接続を選ぶところ。**印（色とタグ）が見えたまま選べる。** */
	.picker .core {
		list-style: none;
		margin: 0;
		padding: 0.2rem;
		max-height: 8.5rem;
		overflow: auto;
	}

	.picker li {
		list-style: none;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		width: 100%;
		/* **左寄せ。**一覧として読むもの。 */
		text-align: left;
		border: none;
		background: none;
		border-radius: var(--r-control);
		padding: 0.2rem 0.4rem;
	}

	.pick:hover {
		background: var(--surface-2);
	}

	.pick.chosen {
		/* **選択の枠も印の色に合わせる。**緑の固定色だと印が意味を失う。 */
		background: var(--surface-2);
		box-shadow: inset 0 0 0 1.5px var(--mark, var(--accent));
	}

	/* 印の色。**タグと二重に出す**（色が見えない人にも効くように）。
	   **`.bar` と名付けない。**上の接続バーと衝突して、そちらが幅 3px に潰れた
	   （実際に潰れた）。 */
	.mark-bar {
		flex: none;
		width: 3px;
		height: 0.95rem;
		border-radius: 2px;
		background: var(--mark);
	}

	.who {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.known {
		flex: none;
		color: var(--ok);
		display: inline-flex;
	}

	/* 3 手順。**繋がるまでのあいだだけ出す。**繋がったら消える。 */
	.steps {
		margin: 0.15rem 0 0;
		padding: 0 0 0 1.1rem;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		font-size: 0.66rem;
		color: var(--fg-faint);
		line-height: 1.5;
	}

	.picker-foot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.35rem 0.2rem 0.05rem;
	}

	/* 初見の指紋。**行き止まりにしない。** */
	.trust .core {
		padding: 0.6rem 0.7rem;
	}

	.trust.danger .core {
		background: var(--danger-soft);
	}

	.trust .head {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		margin: 0 0 0.3rem;
		font-size: 0.78rem;
	}

	.trust.danger .head {
		color: var(--danger);
	}

	.trust .body {
		margin: 0 0 0.4rem;
		font-size: 0.72rem;
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.trust dl {
		margin: 0 0 0.5rem;
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.15rem 0.5rem;
		align-items: baseline;
	}

	.trust dt {
		font-size: 0.66rem;
		color: var(--fg-faint);
		white-space: nowrap;
	}

	.trust dd {
		margin: 0;
		min-width: 0;
	}

	.trust code {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		word-break: break-all;
	}

	.trust-actions {
		display: flex;
		gap: 0.4rem;
	}

	.panes {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		min-height: 0;
		flex: 1 1 auto;
	}

	/* 窓が狭いときは上下に積む。**横に潰して両方読めなくしない。** */
	@media (max-width: 720px) {
		.panes {
			grid-template-columns: 1fr;
		}
	}

	.pane {
		display: flex;
		min-height: 0;
	}

	.pane .core {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		overflow: hidden;
	}

	.pane header,
	.pane footer {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.35rem 0.5rem;
		flex: none;
	}

	.pane header {
		border-bottom: 1px solid var(--hairline);
	}

	.pane footer {
		border-top: 1px solid var(--hairline);
	}

	.title {
		font-size: 0.72rem;
		color: var(--fg-muted);
		flex: 1 1 auto;
	}

	.path {
		flex: 1 1 auto;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: 0.68rem;
	}

	.list {
		list-style: none;
		margin: 0;
		padding: 0.2rem 0;
		overflow: auto;
		flex: 1 1 auto;
		min-height: 0;
	}

	.list li {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.14rem 0.5rem;
		font-size: 0.72rem;
		/* **左寄せ。**中央寄せにすると一覧として読めない。 */
		text-align: left;
	}

	.list li:hover {
		background: var(--surface-2);
	}

	.name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.size {
		font-family: var(--font-mono);
		font-size: 0.64rem;
		color: var(--fg-faint);
		font-variant-numeric: tabular-nums;
		flex: none;
	}

	.link {
		background: none;
		border: none;
		padding: 0;
		font: inherit;
		color: var(--accent);
		cursor: pointer;
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		text-align: left;
	}

	.empty {
		margin: 0;
		padding: 1.2rem 0.8rem;
		font-size: 0.72rem;
		color: var(--fg-faint);
		flex: 1 1 auto;
	}

	.failure,
	.notice {
		margin: 0;
		font-size: 0.72rem;
		padding: 0.3rem 0.5rem;
		border-radius: var(--r-core);
	}

	.failure {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.notice {
		color: var(--fg-muted);
		background: var(--shell);
	}

	button {
		display: inline-flex;
		align-items: center;
		gap: 0.28rem;
		font: inherit;
		font-size: 0.7rem;
		padding: 0.22rem 0.5rem;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		background: var(--shell);
		color: var(--fg);
		cursor: pointer;
		flex: none;
		transition: background 220ms var(--ease);
	}

	button:hover:not(:disabled) {
		background: var(--surface-2);
	}

	button:disabled {
		opacity: 0.45;
		cursor: default;
	}

	button.primary {
		border-color: transparent;
		background: var(--accent);
		color: var(--accent-fg);
	}

	button.ghost {
		border-color: transparent;
		background: none;
		padding: 0.18rem;
	}

	input {
		font: inherit;
		font-size: 0.7rem;
		padding: 0.2rem 0.4rem;
		border-radius: var(--r-core);
		border: 1px solid var(--hairline);
		background: var(--surface-input);
		color: var(--fg);
	}
</style>
