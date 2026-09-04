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
	import ConnectPanel from '$lib/components/ConnectPanel.svelte';
	import Toast from './Toast.svelte';
	import type { Connection } from '$lib/connections';
	import { i18n } from '$lib/i18n/i18n.svelte';
	import {
		back as histBack,
		canBack,
		canForward,
		createHistory,
		current as histCurrent,
		forward as histForward,
		visit,
		type History
	} from '$lib/history';
	import { mirrorMove, type Move } from '$lib/sync-browse';
	import {
		clampPaneRatio,
		DEFAULT_PANE_RATIO,
		loadPaneRatio,
		MIN_PANE_RATIO,
		savePaneRatio
	} from '$lib/splitter.svelte';
	import {
		baseName,
		humanSize,
		joinPath,
		localJoin,
		parentOf,
		session,
		type Listed
	} from '$lib/session.svelte';

	/** 印の色をタブへ出すためだけに持ちます。**繋ぐのは `ConnectPanel` の仕事。** */
	let registered = $state<Connection[]>([]);

	let remotePath = $state('.');
	let entries = $state<Listed[]>([]);
	let loading = $state(false);

	/** 手元の現在地と中身。**右と同じ形で持つ**（並べたときに揃うように）。 */
	let localPath = $state('');
	let localParent = $state<string | null>(null);
	let localEntries = $state<Listed[]>([]);

	/** 上げる待ちの手元のファイル。**パスだけを持つ**（中身は Rust 側で読む）。 */
	let staged = $state<string[]>([]);

	/**
	 * 落とす待ちのサーバー側のファイル。**名前だけ**を持ちます。
	 *
	 * 絶対パスにせず名前で持つのは、**いま見えている階層のものしか落とさない**ため。
	 * 階層を移ったら捨てます（別の場所の同じ名前を落としてしまわないように）。
	 */
	let pickedRemote = $state<string[]>([]);

	/**
	 * 手元にある同じ名前を上書きしてよいか。**既定は否**（product-baseline §13）。
	 *
	 * **1 回ごとに戻します。**一度承認された操作が、次も承認されているとは限らない。
	 */
	let overwrite = $state(false);
	let dropping = $state(false);
	let newDirName = $state('');
	let notice = $state<string | null>(null);
	let failure = $state<string | null>(null);

	const connected = $derived(session.open !== null);

	/** 接続を足す一覧を出すか。**開いていないときは常に出す。** */
	let addingAnother = $state(false);
	const showPicker = $derived(session.all.length === 0 || addingAnother);

	async function loadRegistered() {
		try {
			registered = await invoke<Connection[]>('connections_list');
		} catch (error: unknown) {
			// **握り潰さない。**色が出ないだけなので画面は止めませんが、理由は出します。
			failure = i18n.t('err.list', { detail: String(error) });
		}
	}

	/** その接続の印の色。**タブにも出す**（どれが本番かをタブで見分けるため）。 */
	function markOf(id: string): string {
		const entry = registered.find((held) => held.id === id);
		return entry?.color ? `var(--mark-${entry.color})` : 'transparent';
	}

	/* --- 戻る／進む・同期移動 --- */

	/**
	 * 左右それぞれの道のり。**別々に持ちます。**
	 * 片方で戻ったらもう片方も戻る、という作りは混乱の元でした（実装前に却下）。
	 */
	let localHist = $state<History>(createHistory(''));
	let remoteHist = $state<History>(createHistory('.'));

	/** 履歴で飛んでいる最中は、履歴へ積まない。**押すたびに増えていく事故を防ぐ。** */
	let navigating = false;

	/** 左右を連れて歩くか。**既定は切**（勝手に付いてくると驚く）。 */
	let syncBrowse = $state(false);

	/** 手元の区切り。Windows は `\\`。 */
	const localSeparator = $derived(localPath.includes('\\') ? '\\' : '/');

	/** もう片方を、同じ「動き」で連れて行く。 */
	async function mirror(from: 'local' | 'remote', move: Move) {
		if (!syncBrowse) return;
		if (from === 'local') {
			if (!connected) return;
			const next = mirrorMove(remotePath, move, '/');
			if (next === null || next === remotePath) return;
			remotePath = next;
			await refresh();
		} else {
			const next = mirrorMove(localPath, move, localSeparator);
			if (next === null || next === localPath) return;
			await loadLocal(next);
		}
	}

	async function localBack() {
		if (!canBack(localHist)) return;
		navigating = true;
		localHist = histBack(localHist);
		await loadLocal(histCurrent(localHist));
		navigating = false;
	}

	async function localForward() {
		if (!canForward(localHist)) return;
		navigating = true;
		localHist = histForward(localHist);
		await loadLocal(histCurrent(localHist));
		navigating = false;
	}

	async function remoteBack() {
		if (!canBack(remoteHist) || !connected) return;
		navigating = true;
		remoteHist = histBack(remoteHist);
		remotePath = histCurrent(remoteHist);
		await refresh();
		navigating = false;
	}

	async function remoteForward() {
		if (!canForward(remoteHist) || !connected) return;
		navigating = true;
		remoteHist = histForward(remoteHist);
		remotePath = histCurrent(remoteHist);
		await refresh();
		navigating = false;
	}

	/**
	 * マウスの「戻る／進む」。**押しても戻らない**と言われた所です。
	 *
	 * `button` の 3 が戻る、4 が進む。**`auxclick` ではなく `pointerdown`**
	 * で拾います（WebView によっては `auxclick` が来ない）。
	 * どちらのペインの上で押したかで、動かす側を変えます。
	 */
	function onSideButton(event: PointerEvent, side: 'local' | 'remote') {
		if (event.button !== 3 && event.button !== 4) return;
		event.preventDefault();
		const goBack = event.button === 3;
		if (side === 'local') {
			void (goBack ? localBack() : localForward());
		} else {
			void (goBack ? remoteBack() : remoteForward());
		}
	}

	/**
	 * キーボード。**OS の慣習どおり**にします（覚えなくても手が知っている）。
	 *
	 * - `Alt + ←` / `Alt + →` … 戻る／進む
	 * - `Alt + ↑` … 1 つ上へ
	 * - `F5` … 読み直す
	 *
	 * **どちらのペインを動かすかは、いま焦点がある側**で決めます。
	 * 焦点が無ければ、繋がっていればサーバー側、居なければ手元。
	 */
	function onKey(event: KeyboardEvent) {
		const inField =
			event.target instanceof HTMLElement &&
			(event.target.tagName === 'INPUT' || event.target.tagName === 'TEXTAREA');

		if (event.key === 'F5') {
			event.preventDefault();
			void (connected ? refresh() : loadLocal(localPath));
			return;
		}
		if (!event.altKey || inField) return;

		const side = focusedSide();
		if (event.key === 'ArrowLeft') {
			event.preventDefault();
			void (side === 'local' ? localBack() : remoteBack());
		} else if (event.key === 'ArrowRight') {
			event.preventDefault();
			void (side === 'local' ? localForward() : remoteForward());
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			void (side === 'local' ? localUp() : goUp());
		}
	}

	function focusedSide(): 'local' | 'remote' {
		const active = document.activeElement;
		if (active instanceof HTMLElement && active.closest('[data-side="remote"]')) return 'remote';
		if (active instanceof HTMLElement && active.closest('[data-side="local"]')) return 'local';
		return connected ? 'remote' : 'local';
	}

	/** 手元でフォルダへ入る。**サーバー側の `enter` と対になる。** */
	async function enterLocal(name: string) {
		await loadLocal(localJoin(localPath, name));
		await mirror('local', { kind: 'into', name });
	}

	/** 手元で 1 つ上へ。**サーバー側の `goUp` と対になる。** */
	async function localUp() {
		if (localParent === null) return;
		await loadLocal(localParent);
		await mirror('local', { kind: 'up' });
	}

	/** 手元を読む。**繋がっていなくても使える**（左は手元だけの話）。 */
	async function loadLocal(path?: string) {
		try {
			const listing = await invoke<{
				path: string;
				parent: string | null;
				entries: Listed[];
			}>('local_list_dir', { path: path ?? null });
			localPath = listing.path;
			localParent = listing.parent;
			localEntries = listing.entries;
			// **履歴で飛んでいる最中は積まない。**積むと戻るたびに増えます。
			if (!navigating) localHist = visit(localHist, listing.path);
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** 選んでいるか。**選択は絶対パスで持つ**（別の階層へ移っても消えない）。 */
	function isStaged(name: string): boolean {
		return staged.includes(localJoin(localPath, name));
	}

	function toggle(name: string) {
		const full = localJoin(localPath, name);
		staged = staged.includes(full) ? staged.filter((h) => h !== full) : [...staged, full];
	}

	async function disconnect(id?: string) {
		try {
			await invoke('session_disconnect', { id: id ?? null });
			await session.refresh();
			entries = [];
			notice = null;
			// **残っている接続があれば、そちらを読み直す。**画面が空のままにしない。
			if (session.open) await refresh();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	/** タブを押した。**宛先を変えて、その場所を読み直す。** */
	async function switchTo(id: string) {
		if (id === session.activeId) return;
		try {
			await session.focus(id);
			remotePath = '.';
			await refresh();
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	async function refresh() {
		// **繋がっていないなら、古い警告も古い一覧も残さない**（Issue #8）。
		//
		// 以前はここで素通りしており、**前に出た警告が消えないまま残りました。**
		// 「繋がっているのに『繋がっていません』と出続ける」の片側がこれです。
		if (!connected) {
			failure = null;
			entries = [];
			return;
		}
		loading = true;
		failure = null;
		// **一覧を読み直したら選択は捨てる。**名前で持っているので、
		// 階層が変われば同じ名前が別のファイルを指す。
		pickedRemote = [];
		try {
			entries = await invoke<Listed[]>('remote_list_dir', { path: remotePath });
			if (!navigating) remoteHist = visit(remoteHist, remotePath);
		} catch (error: unknown) {
			// **古い一覧を残さない**（Issue #8）。
			//
			// 残すと、**読めなかった場所に、前の場所の中身が出たまま**になります。
			// 「エラーは出ているのに一覧は正常に見える」という、
			// **画面が実態とずれる一番悪い形**でした。
			entries = [];
			failure = String(error);
			// **画面の思い込みではなく、本当の状態を取り直す。**
			// 繋がっていないと言われたなら、まず事実を確かめます
			// （AI 側が切っていた、という筋も含めて）。
			await session.refresh();
		} finally {
			loading = false;
		}
	}

	/** サーバー側のファイルを選ぶ／外す。**押した階層のものだけ。** */
	function toggleRemote(name: string) {
		pickedRemote = pickedRemote.includes(name)
			? pickedRemote.filter((held) => held !== name)
			: [...pickedRemote, name];
	}

	/**
	 * 選んだものを手元へ落とす。**いま左に見えているディレクトリへ入ります。**
	 *
	 * 同じ名前が手元にあるときは、上書きを選んでいない限り Rust 側が断ります。
	 */
	async function download() {
		if (pickedRemote.length === 0 || !connected) return;
		failure = null;
		notice = null;
		loading = true;
		try {
			const done = await invoke<Array<{ name: string; bytes: number }>>('remote_download', {
				names: pickedRemote,
				remoteDir: remotePath,
				localDir: localPath,
				overwrite
			});
			const total = done.reduce((sum, one) => sum + one.bytes, 0);
			notice = i18n.t('files.downloaded', {
				count: String(done.length),
				size: humanSize(total)
			});
			pickedRemote = [];
			await loadLocal(localPath);
		} catch (error: unknown) {
			failure = String(error);
		} finally {
			// **上書きの許しは 1 回きり。**押しっぱなしにさせない（§13）。
			overwrite = false;
			loading = false;
		}
	}

	async function enter(name: string) {
		remotePath = remotePath === '.' ? name : joinPath(remotePath, name);
		await refresh();
		await mirror('remote', { kind: 'into', name });
	}

	async function goUp() {
		remotePath = parentOf(remotePath);
		await refresh();
		await mirror('remote', { kind: 'up' });
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

	/* --- 左右の取り分（掴んで動かす・ダブルクリックで真ん中） --- */

	let panes: HTMLElement | undefined = $state();
	let paneRatio = $state(DEFAULT_PANE_RATIO);
	let draggingPanes = $state(false);

	function applyRatio(next: number) {
		paneRatio = clampPaneRatio(next);
		savePaneRatio(paneRatio);
	}

	function startPaneDrag(event: PointerEvent) {
		draggingPanes = true;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function onPaneDrag(event: PointerEvent) {
		if (!draggingPanes || !panes) return;
		const box = panes.getBoundingClientRect();
		if (box.width <= 0) return;
		applyRatio((event.clientX - box.left) / box.width);
	}

	function endPaneDrag(event: PointerEvent) {
		draggingPanes = false;
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
	}

	/** **ダブルクリックで真ん中へ戻す。** */
	function resetRatio() {
		applyRatio(DEFAULT_PANE_RATIO);
	}

	/** 掴めない人のために、矢印キーでも動かせるようにする（接続管理と同じ作り）。 */
	function onPaneSplitterKey(event: KeyboardEvent) {
		const step = event.shiftKey ? 0.05 : 0.02;
		if (event.key === 'ArrowLeft') applyRatio(paneRatio - step);
		else if (event.key === 'ArrowRight') applyRatio(paneRatio + step);
		else if (event.key === 'Home' || event.key === 'Enter') resetRatio();
		else return;
		event.preventDefault();
	}

	onMount(() => {
		paneRatio = loadPaneRatio();
		const stops: Array<() => void> = [];
		loadRegistered();
		loadLocal();
		// **接続状態の購読はここに置きません**（Issue #8）。
		//
		// この部品はファイルのタブでしか描かれないので、ここが持つと
		// **別のタブへ移った瞬間に更新が止まり、画面が実態とずれます。**
		// 持ち主は `+layout.svelte` — 窓が開いている間ずっと生きている所です。

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

<svelte:window onkeydown={onKey} />

<section class="files" class:dropping>
	<!-- 開いている接続。**1 本残らずここに出る**（D25）。裏に持つ場所は無い。 -->
	<div class="bar shell">
		<div class="core">
			{#if session.all.length > 0}
				<div class="conn-tabs" role="tablist">
					{#each session.all as held (held.id)}
						<div
							class="conn-tab"
							class:active={held.id === session.activeId}
							style:--mark={markOf(held.id)}
						>
							<button
								type="button"
								role="tab"
								aria-selected={held.id === session.activeId}
								onclick={() => switchTo(held.id)}
								title={held.fingerprint}
							>
								<span class="mark-bar" aria-hidden="true"></span>
								<span data-secret>{held.name}</span>
								{#if held.tag}<span class="tag" data-secret>{held.tag}</span>{/if}
							</button>
							<button
								type="button"
								class="ghost close"
								onclick={() => disconnect(held.id)}
								aria-label={i18n.t('files.disconnect')}
								title={i18n.t('files.disconnect')}
							>
								<Icon name="unplug" size={11} />
							</button>
						</div>
					{/each}
				</div>
				{#if session.open}
					<span class="scope" title={i18n.t('files.scope.help')}>
						{#if session.open.write.aiRoots.length > 0}
							<span data-secret
							>{i18n.t('files.scope.some', { roots: session.open.write.aiRoots.join(' , ') })}</span
						>
						{:else}
							{i18n.t('files.scope.none')}
						{/if}
					</span>
				{/if}
			{:else}
				<span class="title">{i18n.t('files.choose')}</span>
				{#if registered.length === 0}
					<span class="hint">{i18n.t('files.none')}</span>
				{/if}
			{/if}

			{#if session.all.length > 0}
				<!--
					**言葉を出す**（Issue #9）。以前はアイコンだけで、説明が
					`title` と `aria-label` にしかありませんでした。
					**ツールチップは、触れる前の人には存在しないのと同じ**です
					（「2 台目に繋げるのかどうか分からず手が止まった」）。
					文言は既にある鍵をそのまま使います（11 言語ぶん訳が揃っている）。
				-->
				<button
					type="button"
					class="ghost add"
					onclick={() => (addingAnother = !addingAnother)}
					title={i18n.t('files.another')}
				>
					<Icon name="plus" size={13} />
					{i18n.t('files.another')}
				</button>
			{/if}
		</div>
	</div>

	<!--
		**繋ぐ、は 1 つの部品**（`ConnectPanel`）。端末の面でも同じものを出します。
		写して 2 か所に置くと、**片方だけ直る日**が来ます。
	-->
	{#if showPicker}
		<ConnectPanel
			onconnected={() => {
				remotePath = '.';
				addingAnother = false;
				void refresh();
			}}
		/>
	{/if}

	{#if failure}
		<p class="failure" role="alert">{failure}</p>
	{/if}
	

	<!--
		**左右を連れて歩く**（同期移動）。既定は切 — 勝手に付いてくると驚きます。
		絶対パスを合わせるのではなく、**同じ動きをもう片方でもする**作りです
		（手元が `C:\Users\me`・相手が `/srv/app` でも成立させるため）。
	-->
	<label class="sync">
		<input type="checkbox" bind:checked={syncBrowse} />
		{i18n.t('files.sync')}
		<span class="hint">{i18n.t('files.sync.how')}</span>
	</label>

	<div class="panes" bind:this={panes} style:--pane-ratio={paneRatio}>
		<!-- 手元。**右と同じ形にする。**どの階層から上げるのかが見えないと、
		     「どこからどこへ」が分からない（実際に分からなかった）。 -->
		<div
			class="pane shell"
			data-side="local"
			role="group"
			aria-label={i18n.t('files.local')}
			onpointerdown={(event) => onSideButton(event, 'local')}
		>
			<div class="core">
				<header>
					<Icon name="file" />
					<!-- **戻る／進む。**マウスの側面ボタン（3 / 4）と Alt+←→ でも同じことをします。 -->
					<button
						type="button"
						class="ghost nav"
						onclick={localBack}
						disabled={!canBack(localHist)}
						aria-label={i18n.t('files.back')}
						title={i18n.t('files.back')}
					>‹</button>
					<button
						type="button"
						class="ghost nav"
						onclick={localForward}
						disabled={!canForward(localHist)}
						aria-label={i18n.t('files.forward')}
						title={i18n.t('files.forward')}
					>›</button>
					<button
						type="button"
						class="ghost"
						onclick={localUp}
						disabled={!localParent}
						aria-label={i18n.t('files.up')}
						title={i18n.t('files.up')}
					>
						<Icon name="arrow-up" size={13} />
					</button>
					<input
						class="path"
						bind:value={localPath}
						onkeydown={(event) => event.key === 'Enter' && loadLocal(localPath)}
						aria-label={i18n.t('files.local')}
						spellcheck="false"
					/>
				</header>

				{#if localEntries.length === 0}
					<p class="empty">{i18n.t('files.emptydir')}</p>
				{:else}
					<ul class="list">
						{#each localEntries as entry (entry.name)}
							<li class:picked={!entry.isDir && isStaged(entry.name)} data-secret>
								<Icon name={entry.isDir ? 'folder' : 'file'} size={12} />
								{#if entry.isDir}
									<button
										type="button"
										class="link"
										onclick={() => enterLocal(entry.name)}
									>
										{entry.name}
									</button>
								{:else}
									<!-- **ファイルは押すと選ぶ。**選んだものが右へ上がる。 -->
									<button type="button" class="link plain" onclick={() => toggle(entry.name)}>
										{entry.name}
									</button>
								{/if}
								<span class="size">{entry.isDir ? '' : humanSize(entry.size)}</span>
								{#if !entry.isDir && isStaged(entry.name)}
									<span class="chosen-mark"><Icon name="check" size={11} /></span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}

				<footer class="stack">
					<!-- **どこへ送るかを、送るボタンの隣に出す。** -->
					<div class="sending">
						<button
							type="button"
							class="primary"
							onclick={upload}
							disabled={!connected || staged.length === 0 || loading}
						>
							<Icon name="upload" />
							{i18n.t('files.upload', { count: String(staged.length) })}
						</button>
						{#if connected}
							<span class="to" title={remotePath} data-secret>
								<Icon name="arrow-up" size={11} />
								{remotePath}
							</span>
						{/if}
					</div>
					<div class="staging">
						<!--
							**押した結果が見える場所に置く。**
							以前はパス欄の隣に無地の「＋」があり、
							**何が起きるのか分からない**と言われました（α の実機から）。
							上げ待ちの件数のすぐ隣なら、足した結果がその場で数字に出ます。
						-->
						<button type="button" class="ghost tiny" onclick={pickFiles}>
							<Icon name="plus" size={11} />
							{i18n.t('files.pick')}
						</button>
						{#if staged.length > 0}
							<button type="button" class="ghost tiny" onclick={() => (staged = [])}>
								{i18n.t('files.clear')}
							</button>
						{:else}
							<!-- **選び方を、選んでいないときだけ出す。**選べている人には邪魔。 -->
							<span class="hint">{i18n.t('files.pick.how')}</span>
						{/if}
					</div>
				</footer>
			</div>
		</div>

		<!--
			左右の取り分。**掴んで動かし、ダブルクリックで真ん中に戻る。**
			接続管理の仕切りと同じ作りです（`$lib/splitter.svelte`）。
			WAI-ARIA の「window splitter」は焦点を当てられる separator がそのまま定義なので、
			lint をここだけ黙らせます。**キーボードで動かせる状態を保つこと。**
		-->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div
			class="pane-splitter"
			class:dragging={draggingPanes}
			role="separator"
			aria-orientation="vertical"
			aria-label={i18n.t('files.splitter')}
			aria-valuenow={Math.round(paneRatio * 100)}
			aria-valuemin={Math.round(MIN_PANE_RATIO * 100)}
			aria-valuemax={Math.round((1 - MIN_PANE_RATIO) * 100)}
			tabindex="0"
			onpointerdown={startPaneDrag}
			onpointermove={onPaneDrag}
			onpointerup={endPaneDrag}
			onpointercancel={endPaneDrag}
			ondblclick={resetRatio}
			onkeydown={onPaneSplitterKey}
		></div>

		<!-- サーバー -->
		<div
			class="pane shell"
			data-side="remote"
			role="group"
			aria-label={i18n.t('files.remote')}
			onpointerdown={(event) => onSideButton(event, 'remote')}
		>
			<div class="core">
				<header>
					<Icon name="server" />
					<button
						type="button"
						class="ghost nav"
						onclick={remoteBack}
						disabled={!connected || !canBack(remoteHist)}
						aria-label={i18n.t('files.back')}
						title={i18n.t('files.back')}
					>‹</button>
					<button
						type="button"
						class="ghost nav"
						onclick={remoteForward}
						disabled={!connected || !canForward(remoteHist)}
						aria-label={i18n.t('files.forward')}
						title={i18n.t('files.forward')}
					>›</button>
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
							<li class:picked={!entry.isDir && pickedRemote.includes(entry.name)} data-secret>
								<Icon name={entry.isDir ? 'folder' : 'file'} size={12} />
								{#if entry.isDir}
									<button type="button" class="link" onclick={() => enter(entry.name)}>
										{entry.name}
									</button>
								{:else}
									<!-- **左と同じ操作にする。**押すと選び、選んだものが手元へ落ちる。 -->
									<button type="button" class="link plain" onclick={() => toggleRemote(entry.name)}>
										{entry.name}
									</button>
								{/if}
								<span class="size">{entry.isDir ? '' : humanSize(entry.size)}</span>
								{#if !entry.isDir && pickedRemote.includes(entry.name)}
									<span class="chosen-mark"><Icon name="check" size={11} /></span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}

				<footer class="stack">
					<!-- **どこへ落ちるかを、落とすボタンの隣に出す**（上げる側と同じ）。 -->
					<div class="sending">
						<button
							type="button"
							class="primary"
							onclick={download}
							disabled={!connected || pickedRemote.length === 0 || loading}
						>
							<Icon name="download" />
							{i18n.t('files.download', { count: String(pickedRemote.length) })}
						</button>
						<span class="to" title={localPath} data-secret>
							<Icon name="arrow-down" size={11} />
							{localPath}
						</span>
					</div>
					{#if pickedRemote.length > 0}
						<!-- **既定は上書きしない。**落とす側が壊すのは人の手元のファイル。 -->
						<label class="overwrite">
							<input type="checkbox" bind:checked={overwrite} />
							{i18n.t('files.overwrite')}
						</label>
					{/if}
					<div class="sending">
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
					</div>
				</footer>
			</div>
		</div>
	</div>
</section>

{#if notice}
	<Toast text={notice} onDone={() => (notice = null)} />
{/if}

<!--
	**パスフレーズは正面から聞く**（Issue #7 の提案 2）。
	バーの中の小さな入力欄では、**［接続］を押して失敗するまで存在せず**、
	接続タブから押した人には見えませんでした。
	**保存しない方針は変えていません**（D11）。聞き方だけです。
-->
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

	/* 開いている接続のタブ。**1 本残らずここに出る**（D25）。 */
	.conn-tabs {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		flex: 1 1 auto;
		min-width: 0;
		overflow-x: auto;
	}

	.conn-tab {
		display: inline-flex;
		align-items: center;
		flex: none;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		background: var(--shell);
	}

	.conn-tab.active {
		background: var(--surface-2);
		/* **印の色で示す。**どれが本番かをタブで見分けられるように。 */
		box-shadow: inset 0 0 0 1.5px var(--mark, var(--accent));
	}

	.conn-tab button {
		border: none;
		background: none;
		border-radius: 999px;
	}

	.conn-tab .close {
		padding: 0.18rem 0.3rem 0.18rem 0.1rem;
		color: var(--fg-faint);
	}

	.conn-tab .close:hover {
		color: var(--danger);
	}

	.add {
		flex: none;
		color: var(--fg-muted);
	}


	.tag {
		font-size: 0.62rem;
		padding: 0.05rem 0.32rem;
		border-radius: 999px;
		border: 1px solid var(--hairline);
		color: var(--fg-muted);
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


	.panes {
		display: grid;
		/* **取り分は割合で持つ。**ピクセルで持つと、窓を広げたとき片側だけが伸びる。 */
		grid-template-columns: calc(var(--pane-ratio, 0.5) * 100%) auto 1fr;
		gap: 0.5rem;
		min-height: 0;
		flex: 1 1 auto;
	}

	/* 窓が狭いときは上下に積む。**横に潰して両方読めなくしない。** */
	@media (max-width: 720px) {
		.panes {
			grid-template-columns: 1fr;
		}

		/* 積んだら左右の仕切りは意味を失う。**残すと掴めない棒が挟まる。** */
		.pane-splitter {
			display: none;
		}
	}

	.staging {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.staging .hint {
		font-size: 0.7rem;
		color: var(--fg-muted);
	}

	/*
	 * **中身より小さくならないこと。**
	 *
	 * `.shell` は `min-height: 0` を持っており、**縮んでも中身を隠しません。**
	 * 縦の flex で場所が足りなくなると、はみ出した中身が
	 * 下の要素の上に重なって出ます（**接続ボタンとこの行が実際に重なりました**）。
	 * 縮んでよいのは、中で巻き取れるファイル一覧（`.panes`）だけです。
	 */
	.bar,
	.sync {
		flex: 0 0 auto;
	}


	.sync {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		padding: 0 0.1rem 0.35rem;
	}

	.sync .hint {
		color: var(--fg-muted);
		font-size: 0.7rem;
	}

	/* 戻る／進む。**細い記号なので、当たり判定は文字より広く。** */
	.nav {
		min-width: 22px;
		font-size: 1rem;
		line-height: 1;
	}

	.pane-splitter {
		width: 8px;
		margin: 0 -0.25rem;
		cursor: col-resize;
		border-radius: 999px;
		background: transparent;
		position: relative;
		align-self: stretch;
	}

	/* **掴む所は広く、見える線は細く。**細い線を狙わせると外す。 */
	.pane-splitter::after {
		content: '';
		position: absolute;
		inset-block: 0;
		left: 50%;
		width: 2px;
		transform: translateX(-50%);
		border-radius: 999px;
		background: var(--hairline-strong);
	}

	.pane-splitter:hover::after,
	.pane-splitter:focus-visible::after,
	.pane-splitter.dragging::after {
		background: var(--accent);
	}

	.pane-splitter:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
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

	.size {
		font-family: var(--font-mono);
		font-size: 0.64rem;
		color: var(--fg-faint);
		font-variant-numeric: tabular-nums;
		flex: none;
	}

	/* 選んだファイル。**色だけに頼らない。**右端に印も出す。 */
	.list li.picked {
		background: var(--accent-soft);
	}

	.chosen-mark {
		flex: none;
		color: var(--accent);
		display: inline-flex;
	}

	/* 送り先。**送るボタンの隣に出す。**「どこからどこへ」が見えないと動けない。 */
	footer.stack {
		flex-direction: column;
		align-items: stretch;
		gap: 0.25rem;
	}

	.sending {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
	}

	.to {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		font-family: var(--font-mono);
		font-size: 0.62rem;
		color: var(--fg-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	/* 上書きの許し。**押すたびに戻る**ので、見えていないと気づけない。 */
	.overwrite {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.66rem;
		color: var(--fg-muted);
		cursor: pointer;
	}

	.overwrite input {
		width: auto;
		margin: 0;
		accent-color: var(--accent);
	}

	button.tiny {
		font-size: 0.62rem;
		align-self: flex-start;
		color: var(--fg-muted);
	}

	/* ディレクトリは移動、ファイルは選択。**見た目で区別する。** */
	.link.plain {
		color: inherit;
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

	.failure {
		margin: 0;
		font-size: 0.72rem;
		padding: 0.3rem 0.5rem;
		border-radius: var(--r-core);
	}

	.failure {
		color: var(--danger);
		background: var(--danger-soft);
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

	/*
	 * **`.primary` を巻き込まないこと。**
	 *
	 * `button:hover:not(:disabled)` は `button.primary` より詳細度が高く、
	 * ホバーすると地だけが暗い面に置き換わり、**文字色は濃いまま**残ります。
	 * 暗いテーマでは `#06251C` の字が `#1C222A` の地に乗り、
	 * **ほぼ黒同士で読めなくなっていました**（実機で指摘・2026-09-03）。
	 */
	button:hover:not(:disabled):not(.primary) {
		background: var(--surface-2);
	}

	/* 主ボタンは**同じ色相のまま**明るくする。地と文字の関係を崩さない。 */
	button.primary:hover:not(:disabled) {
		filter: brightness(1.08);
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
