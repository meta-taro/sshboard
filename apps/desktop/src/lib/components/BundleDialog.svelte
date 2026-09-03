<script lang="ts">
	/**
	 * 接続情報の書き出し／取り込み（D18）。
	 *
	 * **狭い一覧の中に詰め込んでいたのを、ダイアログへ出しました。**
	 * 説明文・ファイル選び・パスフレーズ・ボタン 2 つを幅 250px の柱に入れており、
	 * **文字が折り返して読めない**と実機で言われました（Windows・2026-09-03）。
	 * dbboard もこの種の機能はダイアログにしています（`BackupDialog`）。
	 */
	import { invoke } from '@tauri-apps/api/core';
	import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';

	import Icon from '$lib/components/Icon.svelte';
	import { defaultBundleName } from '$lib/bundle-name';
	import { i18n } from '$lib/i18n/i18n.svelte';

	/** Rust 側（`sshboard-bundle`）と同じ下限。**強さの物差しではなく、空を弾くため。** */
	const MIN_PASSPHRASE = 8;

	interface Props {
		mode: 'export' | 'import';
		/** 書き出しに選ばれた接続。取り込みでは空。 */
		ids: string[];
		onClose: () => void;
		/** 取り込んだあと、一覧を読み直してもらう。 */
		onImported: () => void;
	}
	let { mode, ids, onClose, onImported }: Props = $props();

	let passphrase = $state('');
	let importFile = $state<string | null>(null);
	let busy = $state(false);
	let note = $state('');
	let done = $state(false);

	const tooShort = $derived(
		mode === 'export' && passphrase.length > 0 && passphrase.length < MIN_PASSPHRASE
	);
	const canGo = $derived(
		!busy &&
			passphrase.length > 0 &&
			(mode === 'export' ? passphrase.length >= MIN_PASSPHRASE : importFile !== null)
	);

	function close() {
		// **パスフレーズを画面に残さない。**閉じた時点で捨てる。
		passphrase = '';
		onClose();
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !busy) close();
	}

	/** 取り込むファイルを選ぶ。**パスフレーズより先。** */
	async function pickFile() {
		note = '';
		try {
			const source = await openDialog({
				multiple: false,
				directory: false,
				filters: [{ name: 'sshboard', extensions: ['sshbx'] }]
			});
			if (!source || Array.isArray(source)) return;
			importFile = source;
		} catch (error: unknown) {
			note = String(error);
		}
	}

	async function run() {
		if (!canGo) return;
		busy = true;
		note = '';
		try {
			if (mode === 'export') {
				const destination = await saveDialog({
					// **日時を入れる。**同じ名前だと 2 回目から上書きになります。
					defaultPath: defaultBundleName(),
					filters: [{ name: 'sshboard', extensions: ['sshbx'] }]
				});
				if (!destination) return;
				const count = await invoke<number>('bundle_export', {
					ids,
					passphrase,
					destination
				});
				note = i18n.t('bundle.exported', { count: String(count) });
			} else {
				const count = await invoke<number>('bundle_import', {
					source: importFile,
					passphrase
				});
				note = i18n.t('bundle.imported', { count: String(count) });
				onImported();
			}
			// **終わったことを見せてから閉じる。**黙って消えると、成功したのか分からない。
			done = true;
			passphrase = '';
		} catch (error: unknown) {
			note = String(error);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget && !busy) close();
	}}
>
	<div
		class="dialog"
		role="dialog"
		aria-modal="true"
		aria-label={mode === 'export' ? i18n.t('bundle.export.title') : i18n.t('bundle.import.title')}
	>
		<header>
			<Icon name={mode === 'export' ? 'download' : 'upload'} size={15} />
			<h2>{mode === 'export' ? i18n.t('bundle.export.title') : i18n.t('bundle.import.title')}</h2>
		</header>

		<p class="what">
			{mode === 'export' ? i18n.t('bundle.export.what') : i18n.t('bundle.import.what')}
		</p>

		{#if done}
			<p class="done" role="status"><Icon name="check" size={13} /> {note}</p>
			<div class="actions">
				<button type="button" class="cta" onclick={close}>{i18n.t('about.close')}</button>
			</div>
		{:else}
			{#if mode === 'export'}
				<p class="count">{i18n.t('bundle.export.count', { count: String(ids.length) })}</p>
			{:else}
				<!-- **先にファイルを選ぶ。**パスフレーズ待ちで押せなくしていたら、
				     「押しても何も起きない」と見えました。 -->
				<div class="field">
					<span class="label">{i18n.t('bundle.import.file')}</span>
					<div class="pick">
						<button type="button" onclick={pickFile} disabled={busy}>
							<Icon name="file" size={12} />
							{importFile === null
								? i18n.t('bundle.import.pick')
								: i18n.t('bundle.import.again')}
						</button>
						{#if importFile !== null}
							<span class="chosen" data-secret title={importFile}>{importFile}</span>
						{/if}
					</div>
				</div>
			{/if}

			<div class="field">
				<span class="label">{i18n.t('bundle.passphrase')}</span>
				<input
					type="password"
					bind:value={passphrase}
					disabled={busy || (mode === 'import' && importFile === null)}
					placeholder={mode === 'export'
						? i18n.t('bundle.tooshort', { min: String(MIN_PASSPHRASE) })
						: ''}
					autocomplete="off"
				/>
				<!-- **同じ経路で送らない**（D18）。書いておかないと、
				     ファイルと同じメールに付けられます。 -->
				<span class="hint">{i18n.t('bundle.channel')}</span>
				{#if tooShort}
					<span class="warn">{i18n.t('bundle.tooshort', { min: String(MIN_PASSPHRASE) })}</span>
				{/if}
			</div>

			{#if note}
				<p class="note" role="alert">{note}</p>
			{/if}

			<div class="actions">
				<button type="button" class="cta" disabled={!canGo} onclick={run}>
					{busy
						? i18n.t('bundle.working')
						: mode === 'export'
							? i18n.t('bundle.export.go')
							: i18n.t('bundle.import.go')}
				</button>
				<button type="button" onclick={close} disabled={busy}>{i18n.t('conn.delete.no')}</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 70;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 40%);
	}

	.dialog {
		width: min(34rem, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
		padding: 1.1rem;
		background: var(--surface);
		border: 1px solid var(--hairline);
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
		font-size: 0.82rem;
		line-height: 1.65;
		color: var(--fg-muted);
	}

	.count {
		margin: 0;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.label {
		font-size: 0.68rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--fg-faint);
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
	}

	.chosen {
		font-size: 0.72rem;
		color: var(--fg-muted);
		font-family: var(--font-mono);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		direction: rtl; /* 長いパスは**末尾（ファイル名）**を見せる */
		text-align: left;
	}

	.hint {
		font-size: 0.7rem;
		color: var(--fg-muted);
	}

	.warn {
		font-size: 0.7rem;
		color: var(--warning);
	}

	.note {
		margin: 0;
		font-size: 0.78rem;
		color: var(--danger);
		line-height: 1.6;
	}

	.done {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		margin: 0;
		font-size: 0.85rem;
		color: var(--ok);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
