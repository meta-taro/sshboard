<script lang="ts" module>
	/** 実行体の `Step`（基準は平たく並ぶ）。 */
	export type ProposedStep = {
		readonly: string;
		label?: string;
		contains?: string;
		absent?: string;
		below?: number;
		above?: number;
	};
	/** 実行体の `Check`。手順は `step` に入る（`checks.toml` と同じ形）。 */
	export type ProposedCheck = { id: string; description?: string; step: ProposedStep[] };
</script>

<script lang="ts">
	/**
	 * **AI が出した点検の案**（D72 の次の段・2026-11-06）。
	 *
	 * AI はファイルを書きません。**保存するのは、この画面の人です**（D22 / D27）。
	 * 保存しても回りません —— 回すと各手順は帯に出てから走ります。
	 */
	import { i18n } from '$lib/i18n/i18n.svelte';

	interface Props {
		proposal: ProposedCheck;
		onSave: () => void;
		onDismiss: () => void;
	}
	let { proposal, onSave, onDismiss }: Props = $props();

	/** 基準を人の言葉に。**書いたものを全部並べる**（どれか 1 つだけ見せると、残りを見落とす）。 */
	function criterion(step: ProposedStep): string {
		const parts: string[] = [];
		if (step.contains !== undefined) parts.push(i18n.t('check.criterion.contains', { text: step.contains }));
		if (step.absent !== undefined) parts.push(i18n.t('check.criterion.absent', { text: step.absent }));
		if (step.below !== undefined) parts.push(i18n.t('check.criterion.below', { n: step.below }));
		if (step.above !== undefined) parts.push(i18n.t('check.criterion.above', { n: step.above }));
		return parts.length > 0 ? parts.join(' ・ ') : i18n.t('check.criterion.exit');
	}
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && onDismiss()} />

<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onDismiss();
	}}
>
	<div class="dialog" role="dialog" aria-modal="true" aria-label={i18n.t('checkProposal.title')}>
		<h2>{i18n.t('checkProposal.title')}</h2>
		<p class="note">{i18n.t('checkProposal.what')}</p>

		<div class="names">
			<span class="id">{proposal.id}</span>
			{#if proposal.description}<span class="what">{proposal.description}</span>{/if}
		</div>

		<table>
			<thead>
				<tr>
					<th>{i18n.t('check.col.step')}</th>
					<th>{i18n.t('check.col.criterion')}</th>
				</tr>
			</thead>
			<tbody>
				{#each proposal.step as step, index (index)}
					<tr>
						<td>
							<span class="title">{step.label ?? step.readonly}</span>
							{#if step.label}<span class="ref">{step.readonly}</span>{/if}
						</td>
						<td>{criterion(step)}</td>
					</tr>
				{/each}
			</tbody>
		</table>

		<p class="note">{i18n.t('checkProposal.note')}</p>

		<div class="actions">
			<button type="button" onclick={onDismiss}>{i18n.t('checkProposal.no')}</button>
			<button type="button" class="cta" onclick={onSave}>{i18n.t('checkProposal.yes')}</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 90;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.5rem;
		background: rgb(0 0 0 / 45%);
	}

	.dialog {
		width: min(36rem, 94vw);
		max-height: 100%;
		overflow-y: auto;
		box-sizing: border-box;
		padding: 1.1rem;
		background: var(--surface);
		border-radius: var(--r-shell);
		box-shadow: var(--shell-ring), var(--lift-3);
	}

	.dialog > :global(* + *) {
		margin-top: 0.7rem;
	}

	h2 {
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

	.names {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.2rem 0.6rem;
	}

	.id {
		font-family: var(--font-mono);
		font-size: var(--text-control);
		font-weight: 600;
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

	.ref {
		display: block;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--fg-muted);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
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
</style>
