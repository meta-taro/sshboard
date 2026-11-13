/**
 * **エージェント専用カーソル**（2026-11-12・オーナーの依頼・tadori の仕様が正本）。
 *
 * エージェントが「どこを指して何を押したか」を、sshboard の画面の中だけで見せる。
 * **人の OS のマウスとキーボードは使いません**（奪わない）。描くのは画面の中の専用カーソルだけ。
 *
 * ## tadori より狭くしてある所（`crates/sshboard-mcp/src/agent_screen.rs` に理由）
 *
 * - **押せるのは `data-agent-view` の印が付いた物だけ**（いまは画面のタブ）。
 *   承認・端末を渡す・ホストを信頼する・パスフレーズは人が押すもの
 * - **打たない**（`type` は本体の側で断ります）
 * - **伏せ字の所（`data-secret` / `data-secret-area`）・端末・入力欄は探しません。**
 *   見つかる／見つからないで、AI が接続先を当てられるため（CLAUDE.md 禁止事項 4）
 * - 画面を切り替えるときは、人が何かしている最中なら動かない（`canFollow` と同じ判定）
 */

import type { OpenAct } from './files-request.svelte';

/** 画面へ届く指示（本体で検査済み）。 */
export type AgentAct =
	| { action: 'point' | 'click'; target: string; message?: string; seconds?: number }
	| { action: 'go'; view: string; message?: string; seconds?: number }
	| { action: 'say'; message: string; seconds?: number }
	| OpenAct;

/** 本体へ返す答え。`crates/sshboard-mcp` の `ActOutcome` と同じ名前。 */
export type ActOutcome = 'done' | 'not_found' | 'not_allowed' | 'busy';

/** 画面の側の口。**ページの事情（いま動いてよいか・画面の切り替え）はここから聞く。** */
export interface AgentPage {
	/** 人が問いに答えている・欄に打っている最中なら偽。 */
	mayTakeScreen(): boolean;
	/** 画面（タブ）を切り替える。**人が押した扱いにはしない。** */
	showView(view: string): void;
	/** ［ファイル］をその接続・そのフォルダで開く（`show_view files` から・2026-11-13）。 */
	openFiles(act: OpenAct): void;
}

/** カーソルが動く時間（`agent-cursor.css` の transition と合わせる）。 */
const MOVE_MS = 650;
/** 一言を出しておく時間：最低 5 秒＋1 文字 0.1 秒、上限 20 秒。 */
const SHOW_MIN_MS = 5000;
const SHOW_PER_CHAR_MS = 100;
const SHOW_MAX_MS = 20000;
/** 一言の無い強調の枠を出しておく時間。 */
const HIGHLIGHT_MS = 2600;
/** しばらく指示が無ければカーソルも隠す。 */
const CURSOR_IDLE_MS = 6000;
/** 吹き出しと窓の端・指した所とのすき間。しっぽを角から離す量。 */
const EDGE_PX = 8;
const GAP_PX = 10;
const TAIL_INSET_PX = 14;

/** 探す相手。tadori と同じ並び（押せる物・見出し・タブ）。 */
const TARGETS = 'button, a, [role="tab"], [role="menuitem"], h1, h2, h3, [data-agent-view]';

/**
 * **探さない所。**伏せる印の付いた所、端末（xterm）、入力欄。
 * 画面の写真で伏せている所と同じ印を使うので、2 つの一覧がずれない。
 */
export const NOT_SEARCHED = '[data-secret], [data-secret-area], .xterm, input, textarea, select';

/** 一言を出しておく時間（ミリ秒）。 */
export function showMs(message: string | undefined, seconds?: number): number {
	if (seconds) return seconds * 1000;
	if (!message) return HIGHLIGHT_MS;
	return Math.min(SHOW_MAX_MS, SHOW_MIN_MS + [...message].length * SHOW_PER_CHAR_MS);
}

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));

interface Box {
	left: number;
	top: number;
	right: number;
	bottom: number;
}
interface Size {
	width: number;
	height: number;
}

/** 吹き出しの置き場所。上 → 下 → 右 → 左の順に収まる所へ置き、窓の端から 8px 内側へ寄せる。 */
export function bubblePlacement(
	target: Box,
	anchor: { x: number; y: number },
	size: Size,
	view: Size
): { left: number; top: number; side: 'above' | 'below' | 'right' | 'left'; tail: number } {
	const fitsAbove = target.top - GAP_PX - size.height >= EDGE_PX;
	const fitsBelow = target.bottom + GAP_PX + size.height <= view.height - EDGE_PX;
	if (fitsAbove || fitsBelow) {
		const left = clamp(anchor.x - TAIL_INSET_PX, EDGE_PX, view.width - EDGE_PX - size.width);
		const top = fitsAbove ? target.top - GAP_PX - size.height : target.bottom + GAP_PX;
		const tail = clamp(anchor.x - left, TAIL_INSET_PX, size.width - TAIL_INSET_PX);
		return { left, top, side: fitsAbove ? 'above' : 'below', tail };
	}
	const fitsRight = target.right + GAP_PX + size.width <= view.width - EDGE_PX;
	const fitsLeft = target.left - GAP_PX - size.width >= EDGE_PX;
	const top = clamp(anchor.y - size.height / 2, EDGE_PX, view.height - EDGE_PX - size.height);
	const tail = clamp(anchor.y - top, TAIL_INSET_PX, size.height - TAIL_INSET_PX);
	if (fitsRight || !fitsLeft) {
		const left = clamp(target.right + GAP_PX, EDGE_PX, view.width - EDGE_PX - size.width);
		return { left, top, side: 'right', tail };
	}
	return { left: target.left - GAP_PX - size.width, top, side: 'left', tail };
}

/** 要素の中の、**探してよい字だけ**（伏せる所・端末の中の字は入れない）。 */
export function searchableText(el: Element): string {
	const parts: string[] = [];
	const walker = el.ownerDocument.createTreeWalker(el, NodeFilter.SHOW_TEXT);
	for (let node = walker.nextNode(); node; node = walker.nextNode()) {
		if (node.parentElement?.closest(NOT_SEARCHED)) continue;
		parts.push(node.textContent ?? '');
	}
	return parts.join(' ').replace(/\s+/g, ' ').trim();
}

/** 要素の呼び名。**伏せる所を中に持つ物は、読み上げ名・説明を使わない**（接続先が入りうる）。 */
function namesOf(el: Element): string[] {
	const names = [searchableText(el)];
	if (!el.querySelector('[data-secret], [data-secret-area]')) {
		for (const attr of ['aria-label', 'title']) {
			const value = el.getAttribute(attr)?.replace(/\s+/g, ' ').trim();
			if (value) names.push(value);
		}
	}
	return names.filter(Boolean);
}

/** 人に見えているか。**隠れた物は指さない・押さない。** */
function isVisible(el: Element): boolean {
	const r = el.getBoundingClientRect();
	return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
}

/**
 * 言葉で探す。ぴったり → 頭が同じ → 含む、の順に、画面の上から。
 * **探さない所の中の物は、候補に入れません。**
 */
export function findTarget(
	doc: Document,
	target: string,
	visible: (el: Element) => boolean = isVisible
): HTMLElement | undefined {
	const want = target.replace(/\s+/g, ' ').trim();
	if (!want) return undefined;
	const candidates = [...doc.querySelectorAll<HTMLElement>(TARGETS)].filter(
		(el) => !el.closest(NOT_SEARCHED) && visible(el)
	);
	const named = candidates.map((el) => ({ el, names: namesOf(el) }));
	return (
		named.find(({ names }) => names.some((n) => n === want))?.el ??
		named.find(({ names }) => names.some((n) => n.startsWith(want)))?.el ??
		named.find(({ names }) => names.some((n) => n.includes(want)))?.el
	);
}

/** **エージェントが押してよい物か。**印（`data-agent-view`）があれば、その画面の名前。 */
export function agentView(el: Element): string | undefined {
	return el.closest<HTMLElement>('[data-agent-view]')?.dataset.agentView;
}

/** 画面に描く部品。はじめて使うときに作る。 */
interface Parts {
	cursor: HTMLDivElement;
	box: HTMLDivElement;
	bubble: HTMLDivElement;
}

const ARROW =
	'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 2l15 9.5-6.6 1.4L16 20.5l-3 1.3-3.5-7.6L4 18z" stroke="#fff" stroke-width="1.4" stroke-linejoin="round"/></svg>';

/** カーソルを描き、指示を 1 つずつ実行する。 */
export class AgentCursor {
	private parts: Parts | undefined;
	private hideTimer: ReturnType<typeof setTimeout> | undefined;
	private idleTimer: ReturnType<typeof setTimeout> | undefined;
	private hovering = false;
	private queue: Promise<unknown> = Promise.resolve();

	constructor(
		private readonly doc: Document,
		private readonly page: AgentPage,
		private readonly label: string
	) {}

	/** 届いた順に 1 つずつ。**前の動きが終わってから次へ。** */
	run(act: AgentAct): Promise<ActOutcome> {
		const next = this.queue.then(() => this.runOne(act));
		this.queue = next.catch(() => undefined);
		return next;
	}

	private async runOne(act: AgentAct): Promise<ActOutcome> {
		if (act.action === 'say') {
			this.say(act.message, act.seconds);
			return 'done';
		}
		if (act.action === 'open') {
			// **人の作業中は動かない。**フォルダの有無は本体が確かめ済み。
			if (!this.page.mayTakeScreen()) return 'busy';
			this.page.openFiles(act);
			return 'done';
		}
		if (act.action === 'go') {
			if (!this.page.mayTakeScreen()) return 'busy';
			this.page.showView(act.view);
			if (act.message) this.say(act.message, act.seconds);
			return 'done';
		}
		const el = findTarget(this.doc, act.target);
		if (!el) return 'not_found';
		if (act.action === 'click') {
			const view = agentView(el);
			if (!view) {
				// **押さずに指すだけ。**どこにあるかは人に見せる。
				await this.moveTo(el, act.message);
				this.hideLater(showMs(act.message, act.seconds));
				return 'not_allowed';
			}
			if (!this.page.mayTakeScreen()) return 'busy';
			await this.moveTo(el, act.message);
			this.ring();
			// **人が押した扱いにはしない**（`humanShows` を通さない）。
			this.page.showView(view);
			this.hideLater(showMs(act.message, act.seconds));
			return 'done';
		}
		await this.moveTo(el, act.message);
		this.hideLater(showMs(act.message, act.seconds));
		return 'done';
	}

	private ensure(): Parts {
		if (this.parts) return this.parts;
		const cursor = this.doc.createElement('div');
		cursor.className = 'agent-cursor';
		cursor.hidden = true;
		cursor.setAttribute('aria-hidden', 'true');
		cursor.innerHTML = `${ARROW}<span class="tag"></span>`;
		this.setTag(cursor, this.label);
		const box = this.doc.createElement('div');
		box.className = 'agent-box';
		box.hidden = true;
		const bubble = this.doc.createElement('div');
		bubble.className = 'agent-bubble';
		bubble.hidden = true;
		bubble.setAttribute('role', 'status');
		bubble.innerHTML =
			'<span class="agent-bubble-text"></span><button type="button" class="agent-bubble-close" aria-label="×">×</button>';
		bubble.addEventListener('click', () => this.hideNow());
		bubble.addEventListener('mouseenter', () => {
			this.hovering = true;
			clearTimeout(this.hideTimer);
		});
		bubble.addEventListener('mouseleave', () => {
			this.hovering = false;
			this.hideLater(1500);
		});
		this.doc.body.append(cursor, box, bubble);
		this.parts = { cursor, box, bubble };
		return this.parts;
	}

	private setTag(cursor: HTMLElement, text: string) {
		const tag = cursor.querySelector('.tag');
		if (tag) tag.textContent = text;
	}

	private async moveTo(el: HTMLElement, message?: string) {
		const { cursor, box, bubble } = this.ensure();
		el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
		const r = el.getBoundingClientRect();
		const anchor = {
			x: Math.round(r.left + Math.min(r.width / 2, 40)),
			y: Math.round(r.top + r.height / 2)
		};
		cursor.hidden = false;
		cursor.style.transform = `translate(${anchor.x}px, ${anchor.y}px)`;
		box.hidden = false;
		Object.assign(box.style, {
			left: `${r.left - 4}px`,
			top: `${r.top - 4}px`,
			width: `${r.width + 8}px`,
			height: `${r.height + 8}px`
		});
		this.showBubble(bubble, r, anchor, message ?? '');
		await new Promise((done) => setTimeout(done, MOVE_MS));
	}

	private showBubble(bubble: HTMLElement, r: Box, anchor: { x: number; y: number }, message: string) {
		const text = bubble.querySelector('.agent-bubble-text');
		if (text) text.textContent = message;
		bubble.hidden = !message;
		if (!message) return;
		Object.assign(bubble.style, { left: '0px', top: '0px' });
		const size = bubble.getBoundingClientRect();
		const view = this.doc.defaultView;
		const place = bubblePlacement(r, anchor, size, {
			width: view?.innerWidth ?? 0,
			height: view?.innerHeight ?? 0
		});
		bubble.dataset.side = place.side;
		Object.assign(bubble.style, {
			left: `${Math.round(place.left)}px`,
			top: `${Math.round(place.top)}px`
		});
		bubble.style.setProperty('--tail', `${Math.round(place.tail)}px`);
	}

	private ring() {
		const { cursor } = this.ensure();
		cursor.classList.remove('click');
		void cursor.offsetWidth;
		cursor.classList.add('click');
	}

	private say(message: string, seconds?: number) {
		const { cursor } = this.ensure();
		cursor.hidden = false;
		this.setTag(cursor, message);
		this.hideLater(showMs(message, seconds));
	}

	private hideNow() {
		clearTimeout(this.hideTimer);
		this.hovering = false;
		if (!this.parts) return;
		this.parts.box.hidden = true;
		this.parts.bubble.hidden = true;
	}

	private hideLater(ms: number) {
		clearTimeout(this.hideTimer);
		clearTimeout(this.idleTimer);
		this.hideTimer = setTimeout(() => {
			if (!this.hovering) this.hideNow();
		}, ms);
		this.idleTimer = setTimeout(
			() => {
				if (!this.parts || this.hovering) return;
				this.parts.cursor.hidden = true;
				this.setTag(this.parts.cursor, this.label);
			},
			Math.max(CURSOR_IDLE_MS, ms + 1000)
		);
	}
}
