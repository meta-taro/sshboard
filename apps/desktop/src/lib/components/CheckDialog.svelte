<script lang="ts" module>
	/** 実行体の `Verdict` と対。 */
	export type CheckVerdict =
		| { kind: 'good' }
		| { kind: 'bad'; why: string }
		| { kind: 'unknown'; why: string };
	export type Judged = { verdict: CheckVerdict; value: string };
	export type StepReport = { readonly: string; title: string; judged: Judged; previous?: Judged };
	export type CheckReport = {
		id: string;
		description?: string;
		connection?: string;
		steps: StepReport[];
		good: number;
		bad: number;
		unknown: number;
	};
	export type CheckLine = {
		id: string;
		description?: string | null;
		steps: string[];
		last?: CheckReport | null;
	};
	export type ChecksOverview = { path: string; checks: CheckLine[] };
</script>

<script lang="ts">
	/**
	 * **点検**（D72・2026-11-05）。人が `checks.toml` に書いた点検を並べ、［回す］で回します。
	 *
	 * > 一度作ったら、そのコマンドを順に実行して値をエージェントと人がチェックできる
	 *
	 * 回す道は AI と同じです。**AI が回した結果も、開いた時点で出ています。**
	 * 値はサーバーから来た字で、ホスト名が混ざりえます —— 伏せられる所に置きます（禁止事項 4）。
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';
	import type { MessageKey } from '$lib/i18n/messages';

	interface Props {
		overview: ChecksOverview;
		/** いま回している点検の id。 */
		running: string | null;
		onRun: (id: string) => void;
		onClose: () => void;
	}
	let { overview, running, onRun, onClose }: Props = $props();

	const VERDICT_KEY: Record<CheckVerdict['kind'], MessageKey> = {
		good: 'check.good',
		bad: 'check.bad',
		unknown: 'check.unknown'
	};

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
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('check.title')}>
		<header class="head">
			<h2>{i18n.t('check.title')}</h2>
		</header>

		{#if overview.checks.length === 0}
			<p class="note">{i18n.t('check.empty')}</p>
			<code class="path" data-secret>{overview.path}</code>
		{:else}
			<div class="list">
				{#each overview.checks as check (check.id)}
					<section class="check">
						<div class="check-head">
							<div class="names">
								<span class="id">{check.id}</span>
								{#if check.description}<span class="what">{check.description}</span>{/if}
							</div>
							<button
								type="button"
								class="cta"
								disabled={running !== null}
								onclick={() => onRun(check.id)}
							>
								{running === check.id ? i18n.t('check.running') : i18n.t('check.run')}
							</button>
						</div>

						{#if check.last}
							<p class="summary">
								{i18n.t('check.summary', {
									good: check.last.good,
									bad: check.last.bad,
									unknown: check.last.unknown
								})}
							</p>
							<table>
								<thead>
									<tr>
										<th>{i18n.t('check.col.step')}</th>
										<th>{i18n.t('check.col.value')}</th>
										<th>{i18n.t('check.col.previous')}</th>
										<th>{i18n.t('check.col.verdict')}</th>
									</tr>
								</thead>
								<tbody>
									{#each check.last.steps as step, index (index)}
										<tr>
											<td>{step.title}</td>
											<td class="value"><span data-secret>{step.judged.value}</span></td>
											<td class="value previous">
												<span data-secret>{step.previous?.value ?? ''}</span>
											</td>
											<td>
												<span class="verdict {step.judged.verdict.kind}">
													{i18n.t(VERDICT_KEY[step.judged.verdict.kind])}
												</span>
												{#if step.judged.verdict.kind !== 'good'}
													<span class="why" data-secret>{step.judged.verdict.why}</span>
												{/if}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{:else}
							<p class="note">{i18n.t('check.never')}</p>
						{/if}
					</section>
				{/each}
			</div>
		{/if}

		<p class="note">{i18n.t('check.help')}</p>

		<div class="actions">
			<button type="button" onclick={onClose}>{i18n.t('about.close')}</button>
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
		width: min(44rem, 94vw);
		max-height: 100%;
		overflow-y: auto;
		box-sizing: border-box;
		padding: 1.1rem;
		background: var(--surface);
		border-radius: var(--r-shell);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	/* **積むのは普通の流れで**（StandingDialog と同じ・WebKit で詰まらない）。 */
	.dialog > :global(* + *) {
		margin-top: 0.7rem;
	}

	.head h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}

	.note,
	.summary {
		margin: 0;
		font-size: var(--text-control);
		color: var(--fg-muted);
		line-height: 1.6;
	}

	.path {
		display: block;
		padding: 6px 10px;
		border-radius: var(--r-control);
		background: var(--surface-2);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		overflow-wrap: anywhere;
	}

	.check {
		padding: 10px 12px;
		border-radius: var(--r-control);
		background: var(--surface-2);
	}

	.check > :global(* + *) {
		margin-top: 0.5rem;
	}

	.check-head {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.names {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.2rem 0.6rem;
	}

	.id {
		font-family: var(--font-mono);
		font-size: var(--text-control);
		font-weight: 600;
		color: var(--fg);
	}

	.what {
		font-size: var(--text-control);
		color: var(--fg-muted);
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--text-control);
		table-layout: fixed;
	}

	th {
		text-align: left;
		font-weight: 500;
		color: var(--fg-muted);
		padding: 4px 6px;
	}

	td {
		padding: 4px 6px;
		vertical-align: top;
		border-top: 1px solid var(--hairline);
		overflow-wrap: anywhere;
	}

	.value {
		font-family: var(--font-mono);
	}

	.previous {
		color: var(--fg-muted);
	}

	.verdict {
		font-weight: 600;
		margin-right: 0.4rem;
	}

	.verdict.good {
		color: var(--ok);
	}

	.verdict.bad {
		color: var(--danger);
	}

	.verdict.unknown {
		color: var(--warning);
	}

	.why {
		color: var(--fg-muted);
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

	button:disabled {
		opacity: 0.55;
		cursor: default;
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
	}
</style>
