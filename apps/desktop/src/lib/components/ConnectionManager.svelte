<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { onMount } from 'svelte';

	import Icon from '$lib/components/Icon.svelte';
	import { i18n } from '$lib/i18n/i18n.svelte';

	import {
		clampListWidth,
		DEFAULT_LIST_WIDTH,
		loadListWidth,
		MIN_LIST_WIDTH,
		saveListWidth
	} from '$lib/splitter.svelte';

	import {
		CONNECTION_COLORS,
		CONNECTION_TAG_MAX_CHARS,
		emptyConnection,
		keyNotice,
		whyNotSavable,
		type Connection,
		type KeyReport
	} from '$lib/connections';

	let items = $state<Connection[]>([]);
	let draft = $state<Connection>(emptyConnection());
	let selectedId = $state<string | null>(null);
	let storePath = $state('');
	let notice = $state<string | null>(null);
	let failure = $state<string | null>(null);

	let listWidth = $state(DEFAULT_LIST_WIDTH);
	let manager: HTMLElement | undefined = $state();
	let dragging = $state(false);
	/** **いきなり消さない**（product-baseline §13）。押してから、もう一度確かめる。 */
	let confirmingDelete = $state(false);

	/* --- 書き出し／取り込み（D18） --- */

	/** Rust 側（`sshboard-bundle`）と同じ下限。**強さの物差しではなく、空を弾くため。** */
	const MIN_PASSPHRASE = 8;

	/** 書き出しに選んだ接続。**既定は空** — 押した覚えのないものを渡さない。 */
	let ticked = $state<string[]>([]);
	/** 'export' / 'import' / null。**同時に開かない。** */
	let transfer = $state<'export' | 'import' | null>(null);
	let passphrase = $state('');
	let transferBusy = $state(false);
	let transferNote = $state('');

	function toggleTicked(id: string) {
		ticked = ticked.includes(id) ? ticked.filter((held) => held !== id) : [...ticked, id];
	}

	function openTransfer(kind: 'export' | 'import') {
		transfer = kind;
		passphrase = '';
		transferNote = '';
	}

	function closeTransfer() {
		transfer = null;
		// **パスフレーズを画面に残さない。**閉じた時点で捨てる。
		passphrase = '';
	}

	async function runExport() {
		if (ticked.length === 0 || passphrase.length < MIN_PASSPHRASE) return;
		transferBusy = true;
		transferNote = '';
		try {
			const destination = await saveDialog({
				defaultPath: 'sshboard.sshbx',
				filters: [{ name: 'sshboard', extensions: ['sshbx'] }]
			});
			if (!destination) return;
			const count = await invoke<number>('bundle_export', {
				ids: ticked,
				passphrase,
				destination
			});
			transferNote = i18n.t('bundle.exported', { count: String(count) });
			closeTransfer();
			ticked = [];
		} catch (error: unknown) {
			transferNote = String(error);
		} finally {
			transferBusy = false;
		}
	}

	async function runImport() {
		if (passphrase.length === 0) return;
		transferBusy = true;
		transferNote = '';
		try {
			const source = await openDialog({
				multiple: false,
				directory: false,
				filters: [{ name: 'sshboard', extensions: ['sshbx'] }]
			});
			if (!source || Array.isArray(source)) return;
			const count = await invoke<number>('bundle_import', { source, passphrase });
			transferNote = i18n.t('bundle.imported', { count: String(count) });
			closeTransfer();
			reload();
		} catch (error: unknown) {
			transferNote = String(error);
		} finally {
			transferBusy = false;
		}
	}

	function containerWidth(): number {
		return manager?.getBoundingClientRect().width ?? 0;
	}

	function applyWidth(next: number) {
		listWidth = clampListWidth(next, containerWidth());
		saveListWidth(listWidth);
	}

	function startDrag(event: PointerEvent) {
		dragging = true;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function onDrag(event: PointerEvent) {
		if (!dragging || !manager) return;
		applyWidth(event.clientX - manager.getBoundingClientRect().left);
	}

	function endDrag(event: PointerEvent) {
		dragging = false;
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
	}

	/** **ダブルクリックで定位置へ戻す。** */
	function resetWidth() {
		applyWidth(DEFAULT_LIST_WIDTH);
	}

	/** 掴めない人のために、矢印キーでも動かせるようにする。 */
	function onSplitterKey(event: KeyboardEvent) {
		const step = event.shiftKey ? 40 : 10;
		if (event.key === 'ArrowLeft') applyWidth(listWidth - step);
		else if (event.key === 'ArrowRight') applyWidth(listWidth + step);
		else if (event.key === 'Home' || event.key === 'Enter') resetWidth();
		else return;
		event.preventDefault();
	}

	/** 編集中なら、自分自身の識別子は重複扱いにしない。 */
	const takenIds = $derived(items.map((item) => item.id).filter((id) => id !== selectedId));
	const blocker = $derived(whyNotSavable(draft, takenIds));
	const blockerText = $derived(
		blocker ? i18n.t(blocker.key, { ...blocker }) : ''
	);
	/**
	 * 指した鍵が何なのか。**判定は Rust が中身を見て返します**（D28）。
	 *
	 * ここで拡張子を見ないこと。`*.tera.ppk` の中身が OpenSSH 秘密鍵だった、が
	 * 実際に在り、**要らない変換作業へ人を送っていました。**
	 */
	let keyReport = $state<KeyReport | null>(null);
	const keyLine = $derived(keyNotice(draft.key_path, keyReport));

	/** 鍵のパスが変わるたびに見に行く。**人が「判定」を押す必要は無い。** */
	$effect(() => {
		const path = draft.key_path?.trim() ?? '';
		if (!path) {
			keyReport = null;
			return;
		}
		let alive = true;
		invoke<KeyReport>('inspect_key_file', { path })
			.then((report) => {
				// 打っている途中の古い結果で上書きしない。
				if (alive) keyReport = report;
			})
			.catch(() => {
				// 見に行けないだけ。**登録は妨げない**（繋ぐときに正直に失敗する）。
				if (alive) keyReport = null;
			});
		return () => {
			alive = false;
		};
	});

	/** 鍵をファイル選択で選ぶ。**パスを手で打たせない。** */
	async function pickKey() {
		try {
			const picked = await openDialog({ multiple: false, directory: false });
			if (typeof picked === 'string') draft = { ...draft, key_path: picked };
		} catch (error: unknown) {
			failure = String(error);
		}
	}

	async function reload() {
		try {
			items = await invoke<Connection[]>('connections_list');
		} catch (error: unknown) {
			failure = i18n.t('err.list', { detail: String(error) });
		}
	}

	function startNew() {
		draft = emptyConnection();
		selectedId = null;
		notice = null;
		confirmingDelete = false;
	}

	function edit(item: Connection) {
		draft = { ...item };
		selectedId = item.id;
		notice = null;
		confirmingDelete = false;
	}

	async function save() {
		failure = null;
		try {
			await invoke('connection_save', { entry: draft });
			notice = i18n.t('conn.saved', { id: draft.id });
			selectedId = draft.id;
			await reload();
		} catch (error: unknown) {
			failure = i18n.t('err.save', { detail: String(error) });
		}
	}

	async function remove(id: string) {
		failure = null;
		confirmingDelete = false;
		try {
			await invoke('connection_delete', { id });
			notice = i18n.t('conn.removed', { id });
			if (selectedId === id) startNew();
			await reload();
		} catch (error: unknown) {
			failure = i18n.t('err.delete', { detail: String(error) });
		}
	}

	onMount(() => {
		const stops: Array<() => void> = [];

		listWidth = clampListWidth(loadListWidth(), containerWidth());

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
				failure = i18n.t('err.subscribe.list', { detail: String(error) });
			});

		return () => stops.forEach((stop) => stop());
	});
</script>

<section
	class="manager"
	class:dragging
	style:--list-width="{listWidth}px"
	bind:this={manager}
	aria-label="接続の登録"
>
	<aside class="list shell">
		<div class="core list-core">
		<div class="list-head">
			<span>{i18n.t('conn.heading')}</span>
			<button type="button" class="new" onclick={startNew}>
				<Icon name="plus" size={13} />
				{i18n.t('conn.new')}
			</button>
		</div>

		<div class="transfer-bar">
			<button
				type="button"
				class="ghost tiny"
				onclick={() => openTransfer('export')}
				disabled={ticked.length === 0}
				title={ticked.length === 0 ? i18n.t('bundle.export.none') : ''}
			>
				<Icon name="download" size={11} />
				{i18n.t('bundle.export', { count: String(ticked.length) })}
			</button>
			<button type="button" class="ghost tiny" onclick={() => openTransfer('import')}>
				<Icon name="upload" size={11} />
				{i18n.t('bundle.import')}
			</button>
		</div>

		{#if transfer !== null}
			<div class="transfer-panel">
				<p class="what">
					{transfer === 'export' ? i18n.t('bundle.export.what') : i18n.t('bundle.import.what')}
				</p>
				<!-- **同じ経路で送らない**（D18）。ここに書いておかないと、
				     ファイルとパスフレーズを同じメールに付けられます。 -->
				<p class="warn">{i18n.t('bundle.channel')}</p>
				<input
					type="password"
					bind:value={passphrase}
					placeholder={i18n.t('bundle.passphrase')}
					aria-label={i18n.t('bundle.passphrase')}
					autocomplete="off"
				/>
				{#if transfer === 'export' && passphrase.length > 0 && passphrase.length < MIN_PASSPHRASE}
					<p class="warn">{i18n.t('bundle.tooshort', { min: String(MIN_PASSPHRASE) })}</p>
				{/if}
				<div class="transfer-actions">
					<button
						type="button"
						class="cta"
						disabled={transferBusy ||
							passphrase.length === 0 ||
							(transfer === 'export' && passphrase.length < MIN_PASSPHRASE)}
						onclick={() => (transfer === 'export' ? runExport() : runImport())}
					>
						{transferBusy
							? i18n.t('bundle.working')
							: transfer === 'export'
								? i18n.t('bundle.export.go')
								: i18n.t('bundle.import.go')}
					</button>
					<button type="button" onclick={closeTransfer}>{i18n.t('conn.delete.no')}</button>
				</div>
			</div>
		{/if}

		{#if transferNote}
			<p class="transfer-note" role="status">{transferNote}</p>
		{/if}

		{#if items.length === 0}
			<p class="empty">{i18n.t('conn.empty')}</p>
		{:else}
			<ul>
				{#each items as item (item.id)}
					<li class="row-line">
						<!--
							**書き出しに選ぶ印。**行そのものは「開く」ままにします
							（既存の操作を変えない）。チェックは別の当たり判定にして、
							**押し間違いで接続先が 1 つ余計に渡ることを防ぎます。**
						-->
						<input
							type="checkbox"
							class="tick"
							checked={ticked.includes(item.id)}
							onchange={() => toggleTicked(item.id)}
							aria-label={i18n.t('bundle.tick', { name: item.name || item.id })}
						/>
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
								<span class="row-name" data-secret>{item.name || item.id}</span>
								{#if item.tag}
									<span
										class="row-tag"
										style:color={item.color ? `var(--mark-${item.color})` : '#8b929e'}
										data-secret
									>
										{item.tag}
									</span>
								{/if}
							</span>
							<span class="row-id" data-secret>{item.id}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		{#if storePath}
			<p class="path" title={storePath} data-secret>{storePath}</p>
		{/if}
		</div>
	</aside>

	<!--
		WAI-ARIA の「window splitter」は、**焦点を当てられる separator** がそのまま定義です
		（矢印キーで動かせる必要がある）。lint は separator を非対話とみなすので、
		ここだけ黙らせます。**キーボードで動かせる状態を保つこと。**
	-->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="splitter"
		role="separator"
		aria-orientation="vertical"
		aria-label={i18n.t('conn.splitter')}
		aria-valuenow={listWidth}
		aria-valuemin={MIN_LIST_WIDTH}
		tabindex="0"
		onpointerdown={startDrag}
		onpointermove={onDrag}
		onpointerup={endDrag}
		onpointercancel={endDrag}
		ondblclick={resetWidth}
		onkeydown={onSplitterKey}
	></div>

	<div class="form shell">
		<div class="core form-core">
		{#if failure}
			<p class="failure" role="alert">{failure}</p>
		{:else if notice}
			<p class="notice">{notice}</p>
		{/if}

		<label>
			<span>{i18n.t('conn.id')}</span>
			<input bind:value={draft.id} placeholder="web-prod" disabled={selectedId !== null} />
			<small>{i18n.t('conn.id.help')}</small>
		</label>

		<label>
			<span>{i18n.t('conn.name')}</span>
			<input bind:value={draft.name} placeholder="Web (prod)" />
		</label>

		<div class="pair">
			<label>
				<span><Icon name="server" size={12} />{i18n.t('conn.host')}</span>
				<input bind:value={draft.host} spellcheck="false" />
			</label>
			<label>
				<span>{i18n.t('conn.port')}</span>
				<input type="number" bind:value={draft.port} min="1" max="65535" />
			</label>
		</div>

		<label>
			<span><Icon name="user" size={12} />{i18n.t('conn.user')}</span>
			<input bind:value={draft.user} spellcheck="false" />
		</label>

		<label>
			<span><Icon name="key" size={12} />{i18n.t('conn.key')}</span>
			<div class="key-row">
				<input
					bind:value={draft.key_path}
					placeholder={i18n.t('conn.key.placeholder')}
					spellcheck="false"
				/>
				<button type="button" onclick={pickKey}>{i18n.t('conn.key.pick')}</button>
			</div>
			<!-- **形式は製品が見分けます**（D28）。人が拡張子を気にする必要はありません。 -->
			{#if keyLine.tone !== 'none'}
				<small class="key-note" class:bad={keyLine.tone === 'error'}>
					<Icon name={keyLine.tone === 'error' ? 'warning' : 'check'} size={11} />
					{i18n.t(keyLine.key, { format: keyLine.format })}
				</small>
			{:else}
				<small>{i18n.t('conn.key.help')}</small>
			{/if}
		</label>

		<div class="mark-row">
			<label class="tag">
				<span><Icon name="tag" size={12} />{i18n.t('conn.tag', { max: CONNECTION_TAG_MAX_CHARS })}</span>
				<input
					bind:value={draft.tag}
					maxlength={CONNECTION_TAG_MAX_CHARS}
					placeholder="prod / dev2"
				/>
				<small>{i18n.t('conn.tag.help')}</small>
			</label>
			<label>
				<span><Icon name="palette" size={12} />{i18n.t('conn.color')}</span>
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
					{i18n.t('conn.color.none')}
				</button>
			</label>
		</div>

		<!-- **囲いを人が決める**（D22）。空のままなら AI は 1 バイトも書けない。 -->
		<label>
			<span><Icon name="upload" size={12} />{i18n.t('conn.write')}</span>
			<textarea
				rows="2"
				value={(draft.write_roots ?? []).join('\n')}
				placeholder={i18n.t('conn.write.placeholder')}
				spellcheck="false"
				oninput={(event) => {
					draft.write_roots = (event.currentTarget as HTMLTextAreaElement).value
						.split('\n')
						.map((line) => line.trim())
						.filter((line) => line.length > 0);
				}}
			></textarea>
			<small>{i18n.t('conn.write.help')}</small>
		</label>

		<div class="actions">
			<button type="button" class="cta" onclick={save} disabled={blocker !== null}>
				<span>{i18n.t('conn.save')}</span>
				<span class="cta-icon"><Icon name="check" size={13} /></span>
			</button>
			{#if selectedId && !confirmingDelete}
				<button type="button" class="danger" onclick={() => (confirmingDelete = true)}>
					<Icon name="trash" size={13} />
					{i18n.t('conn.delete')}
				</button>
			{/if}
			{#if blockerText}
				<span class="blocker">{blockerText}</span>
			{/if}
		</div>

		{#if selectedId && confirmingDelete}
			<p class="confirm" role="alert">
				<strong>{i18n.t('conn.delete.confirm', { id: selectedId })}</strong>
				{i18n.t('conn.delete.scope')}
				<span class="confirm-actions">
					<button type="button" class="danger" onclick={() => remove(selectedId ?? '')}>
						{i18n.t('conn.delete.yes')}
					</button>
					<button type="button" onclick={() => (confirmingDelete = false)}>
						{i18n.t('conn.delete.no')}
					</button>
				</span>
			</p>
		{/if}
		</div>
	</div>
</section>

<style>
	/* 面は入れ子（外殻 → 芯）。**背景に直接置かない。**
	   色は tokens.css の変数だけ。**ここに 16 進数を書かない。** */
	.manager {
		display: grid;
		grid-template-columns: var(--list-width) 10px 1fr;
		gap: 0;
		flex: 1;
		min-height: 0;
	}

	.list,
	.form {
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	.list-core,
	.form-core {
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow: hidden;
	}

	.list-core {
		padding: 0.7rem 0.55rem 0.5rem;
		gap: 0.4rem;
	}

	.form-core {
		padding: 1rem 1.1rem;
		gap: 0.65rem;
		overflow-y: auto;
	}

	@media (max-width: 720px) {
		.manager {
			grid-template-columns: 1fr;
			grid-template-rows: minmax(130px, 32%) 1fr;
			gap: 0.5rem;
		}

		/* --- 書き出し／取り込み（D18） --- */

	.transfer-bar {
		display: flex;
		gap: 0.4rem;
		padding: 0 0.6rem 0.4rem;
		flex-wrap: wrap;
	}

	.transfer-panel {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		margin: 0 0.6rem 0.5rem;
		padding: 0.5rem;
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.transfer-panel .what {
		margin: 0;
		font-size: 0.75rem;
	}

	/* **同じ経路で送らない**を目立たせる（D18）。 */
	.transfer-panel .warn {
		margin: 0;
		font-size: 0.7rem;
		color: var(--fg-muted);
	}

	.transfer-actions {
		display: flex;
		gap: 0.4rem;
	}

	.transfer-note {
		margin: 0 0.6rem 0.5rem;
		font-size: 0.75rem;
	}

	.row-line {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	/* **当たり判定を分ける。**行は「開く」、印は「渡す」。
	   同じ所に重ねると、開いたつもりで渡す物が増えます。 */
	.tick {
		flex: 0 0 auto;
		margin-left: 0.5rem;
		cursor: pointer;
	}

	.row-line .row {
		flex: 1 1 auto;
		min-width: 0;
	}

	.splitter {
			display: none;
		}
	}

	.list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.4rem;
		padding: 0 0.3rem 0.15rem;
	}

	.list-head span {
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	.list ul {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.row {
		display: flex;
		flex-direction: column;
		/* **中央寄せにしない。**あとから足した `button { align-items: center }` が
		   ここにも当たり、列方向の flex なので全部が中央へ寄っていた。 */
		align-items: stretch;
		gap: 1px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--r-control);
		padding: 0.38rem 0.5rem;
		color: var(--fg);
		font: inherit;
		font-size: 0.84rem;
		cursor: pointer;
		transition:
			background var(--fast) var(--ease),
			border-color var(--fast) var(--ease),
			transform var(--fast) var(--ease);
	}

	.row:hover {
		background: var(--surface-2);
	}

	.row:active {
		transform: scale(0.99);
	}

	/* **選択の枠はその接続の印の色。** */
	.row.selected {
		background: var(--surface-2);
		border-color: color-mix(in srgb, var(--row-mark) 55%, transparent);
		box-shadow: var(--lift-1);
	}

	.row-top {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.chip {
		width: 7px;
		height: 7px;
		border-radius: 2px;
		border: 1px solid;
		flex: none;
	}

	.row-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
	}

	.row-tag {
		font-family: var(--font-mono);
		font-size: 9px;
		letter-spacing: 0.06em;
		border: 1px solid currentColor;
		border-radius: 999px;
		padding: 0 0.4rem;
		flex: none;
		opacity: 0.9;
	}

	.row-id {
		font-family: var(--font-mono);
		font-size: 0.66rem;
		color: var(--fg-faint);
		padding-left: 0.68rem;
	}

	/* 仕切り。掴めることが見た目で分かるように、中央に細い印を出す。 */
	.splitter {
		position: relative;
		cursor: col-resize;
		background: transparent;
		border: none;
		padding: 0;
		align-self: stretch;
	}

	.splitter::after {
		content: '';
		position: absolute;
		inset: 25% 50% 25% 50%;
		width: 2px;
		margin-left: -1px;
		border-radius: 999px;
		background: var(--hairline-strong);
		transition: background var(--fast) var(--ease);
	}

	.splitter:hover::after,
	.splitter:focus-visible::after,
	.manager.dragging .splitter::after {
		background: var(--accent);
		inset: 8% 50% 8% 50%;
	}

	.splitter:focus-visible {
		outline: none;
	}

	.manager.dragging {
		user-select: none;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.22rem;
	}

	/* **大文字化しない。**日本語ラベルの中のラテン文字まで大文字になり、
	   「空なら ssh-agent」が「空なら SSH-AGENT」になってしまう。 */
	label > span {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	input {
		font: inherit;
		font-size: 0.85rem;
		color: var(--fg);
		background: var(--surface-input);
		border: 1px solid var(--hairline);
		border-radius: var(--r-control);
		padding: 0.42rem 0.6rem;
		box-shadow: var(--inner-highlight);
		transition:
			border-color var(--fast) var(--ease),
			box-shadow var(--fast) var(--ease);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
		box-shadow: var(--inner-highlight), 0 0 0 3px var(--accent-soft);
	}

	input:disabled {
		opacity: 0.55;
	}

	small {
		color: var(--fg-faint);
		font-size: 0.7rem;
		line-height: 1.65;
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 96px;
		gap: 0.65rem;
	}

	.mark-row {
		display: flex;
		align-items: flex-start;
		gap: 1.1rem;
		flex-wrap: wrap;
	}

	.mark-row .tag input {
		width: 11rem;
	}

	/* **2 行 × 8。**flex の中で潰れないよう幅を固定する。 */
	.swatches {
		display: grid;
		grid-template-columns: repeat(8, 18px);
		gap: 0.28rem;
		width: max-content;
		flex: none;
	}

	.swatch {
		width: 18px;
		height: 18px;
		padding: 0;
		border-radius: 5px;
		border: 1px solid var(--hairline-strong);
		cursor: pointer;
		box-shadow: var(--lift-1);
		transition: transform var(--fast) var(--ease);
	}

	.swatch:hover {
		transform: scale(1.12);
	}

	.swatch.picked {
		outline: 2px solid var(--fg);
		outline-offset: 2px;
	}

	.clear-mark {
		align-self: flex-start;
		margin-top: 0.3rem;
		font-size: 0.7rem;
		background: none;
		border: none;
		color: var(--fg-faint);
		text-decoration: underline;
		text-underline-offset: 3px;
		padding: 0;
		cursor: pointer;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.35rem;
		flex-wrap: wrap;
	}

	button {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font: inherit;
		font-size: 0.78rem;
		color: var(--fg);
		background: var(--surface-2);
		border: 1px solid var(--hairline);
		border-radius: 999px;
		padding: 0.3rem 0.85rem;
		cursor: pointer;
		box-shadow: var(--inner-highlight), var(--lift-1);
		transition:
			transform var(--fast) var(--ease),
			background var(--fast) var(--ease);
	}

	button:hover {
		background: var(--surface);
	}

	button:active {
		transform: scale(0.97);
	}

	button:disabled {
		opacity: 0.45;
		cursor: default;
		box-shadow: none;
	}

	button:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	/* 主ボタン。**矢印を裸で置かず、丸の中に入れる。** */
	.cta {
		display: inline-flex;
		align-items: center;
		gap: 0.55rem;
		background: var(--accent);
		color: var(--accent-fg);
		border-color: transparent;
		padding: 0.32rem 0.35rem 0.32rem 0.95rem;
		font-weight: 700;
		box-shadow: var(--lift-2);
	}

	.cta:hover {
		background: var(--accent);
	}

	.cta-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--accent-fg) 18%, transparent);
		font-size: 0.72rem;
		transition: transform var(--fast) var(--ease);
	}

	.cta:hover .cta-icon {
		transform: translateX(2px) scale(1.06);
	}

	.danger {
		color: var(--danger);
		border-color: color-mix(in srgb, var(--danger) 40%, transparent);
	}

	.blocker,
	.empty,
	.path {
		font-size: 0.72rem;
		color: var(--fg-faint);
	}

	.path {
		font-family: var(--font-mono);
		font-size: 0.62rem;
		word-break: break-all;
		margin: 0;
		padding: 0.3rem 0.3rem 0;
		border-top: 1px solid var(--hairline);
	}

	.empty {
		padding: 0.5rem 0.35rem;
	}

	.notice {
		margin: 0;
		font-size: 0.78rem;
		color: var(--ok);
	}

	.failure,
	.confirm {
		margin: 0;
		padding: 0.55rem 0.7rem;
		border-radius: var(--r-control);
		font-size: 0.78rem;
		line-height: 1.7;
	}

	.failure,
	.confirm {
		background: var(--danger-soft);
		color: var(--danger);
	}

	/* 鍵の判定を出す 1 行（D28）。**普段は情報、駄目なときだけ強く。** */
	.key-row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.key-row input {
		flex: 1 1 auto;
		min-width: 0;
	}

	.key-note {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		color: var(--fg-muted);
	}

	.key-note.bad {
		color: var(--danger);
	}

	.confirm-actions {
		display: inline-flex;
		gap: 0.4rem;
		margin-left: 0.5rem;
	}
</style>
