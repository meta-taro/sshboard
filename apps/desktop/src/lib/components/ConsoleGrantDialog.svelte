<script lang="ts" module>
	/** 実行体の許可（`console_grant_cmd::GrantView`）と対。 */
	export type ConsoleGrant = {
		scope: 'connection' | 'new';
		connection: string | null;
		remainingSecs: number;
	};
	export type GrantScopeChoice = ConsoleGrant['scope'];
</script>

<script lang="ts">
	/**
	 * **端末の期限付きの許可**（2026-11-16）。
	 *
	 * 時間で課金される測定用の機械へ AI が繋ぐたび、人が［許可］を押すまで
	 * 機械が止まったまま待っていました（6 分・30 分の実測）。
	 * **人が先に「いまから N 時間は聞かずに通す」を出しておく**枠です。
	 *
	 * - **既定は「これから増える接続だけ」**。出した時点で在った接続
	 *   （人がふだん使っている所）には効きません
	 * - 期限は 1・3・24 時間。**24 時間より長くは出せません**（実行体も断ります）
	 * - 人が打っている端末は取りません。アプリを終えるか［AI を止める］で外れます
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		/** いま開いている接続。無ければ「この接続だけ」は出しません。 */
		current: { id: string; name: string } | null;
		grant: ConsoleGrant | null;
		onGrant: (scope: GrantScopeChoice, connection: string | null, hours: number) => void;
		onRevoke: () => void;
		onClose: () => void;
	}
	let { current, grant, onGrant, onRevoke, onClose }: Props = $props();

	const HOURS = [1, 3, 24] as const;
	/** 測定の機械が 3 時間で消えるので、既定はそこに合わせます。 */
	const DEFAULT_HOURS = 3;
	const SECONDS_PER_MINUTE = 60;

	let scope = $state<GrantScopeChoice>('new');
	let hours = $state<number>(DEFAULT_HOURS);

	const minutesLeft = $derived(grant ? Math.ceil(grant.remainingSecs / SECONDS_PER_MINUTE) : 0);

	function give() {
		const connection = scope === 'connection' && current ? current.id : null;
		onGrant(connection ? 'connection' : 'new', connection, hours);
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onClose();
	}}
>
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('grant.title')}>
		<header class="head">
			<h2>{i18n.t('grant.title')}</h2>
		</header>

		{#if grant}
			<div class="active">
				<span class="what">
					{grant.scope === 'new' ? i18n.t('grant.scope.new') : i18n.t('grant.scope.connection')}
					{#if grant.connection}
						<!-- **接続の名前は伏せられる所**（禁止事項 4）。 -->
						<span class="where" data-secret>{grant.connection}</span>
					{/if}
				</span>
				<span class="left">{i18n.t('grant.left', { minutes: minutesLeft })}</span>
				<button type="button" onclick={onRevoke}>{i18n.t('grant.revoke')}</button>
			</div>
		{:else}
			<p class="note">{i18n.t('grant.none')}</p>
		{/if}

		<fieldset>
			<legend>{i18n.t('grant.scope.legend')}</legend>
			<label>
				<input type="radio" name="grant-scope" value="new" bind:group={scope} />
				{i18n.t('grant.scope.new')}
			</label>
			{#if current}
				<label>
					<input type="radio" name="grant-scope" value="connection" bind:group={scope} />
					{i18n.t('grant.scope.connection')}
					<span class="where" data-secret>{current.name}</span>
				</label>
			{/if}
		</fieldset>

		<fieldset>
			<legend>{i18n.t('grant.hours.legend')}</legend>
			{#each HOURS as choice (choice)}
				<label>
					<input type="radio" name="grant-hours" value={choice} bind:group={hours} />
					{i18n.t('grant.hours', { hours: choice })}
				</label>
			{/each}
		</fieldset>

		<p class="note">{i18n.t('grant.help')}</p>

		<div class="actions">
			<button type="button" onclick={onClose}>{i18n.t('about.close')}</button>
			<button type="button" class="cta" onclick={give}>{i18n.t('grant.give')}</button>
		</div>
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
		width: min(32rem, 92vw);
		padding: 1.1rem;
		background: var(--surface);
		border: 0;
		border-radius: var(--r-shell);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	/* **積むのは普通の流れで**（flex で積むと、WebKit で本文の高さが追いつかず詰まる）。 */
	.dialog > :global(* + *) {
		margin-top: 0.7rem;
	}

	.head h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.note {
		margin: 0;
		font-size: var(--text-control);
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.active {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 6px 10px;
		border-radius: var(--r-control);
		background: var(--surface-2);
		font-size: var(--text-control);
	}

	.what {
		flex: 1 1 auto;
		min-width: 0;
		color: var(--fg);
	}

	.left {
		color: var(--fg-muted);
		white-space: nowrap;
	}

	.where {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--fg-muted);
	}

	fieldset {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem 1rem;
		margin: 0;
		padding: 0;
		border: 0;
		font-size: var(--text-control);
	}

	legend {
		width: 100%;
		padding: 0 0 0.3rem;
		color: var(--fg-muted);
	}

	label {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		cursor: pointer;
	}

	button {
		display: inline-flex;
		align-items: center;
		height: var(--control-h);
		padding: 0 12px;
		border: 0;
		border-radius: var(--r-pill);
		background: var(--shell-bg);
		box-shadow: inset 0 0 0 1px var(--hairline-strong);
		color: var(--fg);
		font: inherit;
		font-size: var(--text-control);
		white-space: nowrap;
		cursor: pointer;
	}

	button.cta {
		background: var(--accent-soft);
		color: var(--accent);
		box-shadow: inset 0 0 0 1px var(--accent);
		font-weight: 600;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
