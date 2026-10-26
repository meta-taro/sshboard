import type { Page } from '@playwright/test';

/**
 * **見栄えの検め**（オーナー・2026-10-26 ——「見栄え崩れは初めてではない」）。
 *
 * これまでの崩れを、**種類として**拾います。1 件ずつ後追いで試験を書くのではなく、
 * どの面にも同じ物差しを当てます。
 *
 * 1. **字が切れている・はみ出している** —— 製品名「sshb…」・太字の札（WebKit）・
 *    「別の接続を開く」が右端で切れた（0.1.27）
 * 2. **素のままの釦** —— OS 既定の灰色の釦が混ざる（［接続］が浮いていた）
 * 3. **箱から中身がはみ出す** —— 「接続先」の見出しが外枠に乗り上げた（0.1.30）
 * 4. **窓の外へ出る** —— 右上の × が 2 段目へ落ちた（0.1.27）・横スクロール
 * 5. **押せないほど小さい釦** —— 素の釦が 20px に潰れた
 * 6. **引き伸ばされたチェックボックス** —— 入力欄の高さが当たり、上下に細い線が出た（#28・0.1.31）
 *
 * 見つけたものを、**何がどこで**の文で返します（空なら崩れ無し）。
 */
export async function lookFindings(page: Page): Promise<string[]> {
	return page.evaluate(() => {
		const found: string[] = [];
		const SLACK = 1;
		const MIN_TARGET = 24;

		const shown = (el: Element): boolean => {
			const r = el.getBoundingClientRect();
			const cs = getComputedStyle(el);
			return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none';
		};
		const label = (el: Element): string => {
			const cls = typeof el.className === 'string' ? el.className : '';
			const classes = cls
				.split(/\s+/)
				.filter((c) => c && !c.startsWith('svelte-'))
				.join('.');
			const text = (el.textContent ?? '').trim().replace(/\s+/g, ' ').slice(0, 24);
			return `${el.tagName.toLowerCase()}${classes ? '.' + classes : ''}「${text}」`;
		};
		// 中で巻き取る箱の中は、見えていなくて正しい。
		const insideScroller = (el: Element): boolean => {
			for (let up = el.parentElement; up; up = up.parentElement) {
				const cs = getComputedStyle(up);
				if (/(auto|scroll)/.test(cs.overflowY + cs.overflowX)) return true;
			}
			return false;
		};
		// 伏せた画面の塗り潰しなどは測らない。
		const textBox = (el: Element): DOMRect | null => {
			const range = document.createRange();
			range.selectNodeContents(el);
			const r = range.getBoundingClientRect();
			return r.width > 0 ? r : null;
		};

		// 1. 字が切れている・はみ出している（釦・タブ・札）
		for (const el of document.querySelectorAll('button, [role="tab"], .pill, .tag, .who-badge')) {
			if (!shown(el)) continue;
			const cs = getComputedStyle(el);
			if (cs.textOverflow === 'ellipsis') continue;
			const text = textBox(el);
			if (!text) continue;
			const box = el.getBoundingClientRect();
			if (text.right > box.right + SLACK || text.left < box.left - SLACK) {
				found.push(`字がはみ出している: ${label(el)}`);
			} else if (el.scrollWidth > el.clientWidth + SLACK && cs.overflowX !== 'visible') {
				found.push(`字が切れている: ${label(el)}`);
			}
		}

		// 2. 素のままの釦（OS 既定の枠）。5. 押せないほど小さい釦
		for (const el of document.querySelectorAll('button')) {
			if (!shown(el)) continue;
			const cs = getComputedStyle(el);
			if (cs.borderTopStyle === 'outset' || cs.borderTopStyle === 'inset') {
				found.push(`素のままの釦: ${label(el)}`);
			}
			// **WCAG 2.2 の押せる大きさ（24px）。**一覧の名前のような、
			// 文の中の字の口（`.link`）は対象外（WCAG もそう定めています）。
			if (el.classList.contains('link')) continue;
			const h = el.getBoundingClientRect().height;
			if ((el.textContent ?? '').trim() && h < MIN_TARGET) {
				found.push(`釦が小さすぎる（${Math.round(h)}px）: ${label(el)}`);
			}
		}

		// 6. 引き伸ばされたチェックボックス・ラジオ（入力欄の高さが当たると、上下に線が出る）
		for (const el of document.querySelectorAll('input[type="checkbox"], input[type="radio"]')) {
			if (!shown(el)) continue;
			const r = el.getBoundingClientRect();
			if (r.height > 22 || Math.abs(r.height - r.width) > 4) {
				found.push(
					`チェックボックスが引き伸ばされている（${Math.round(r.width)}×${Math.round(r.height)}）: ${label(el.parentElement ?? el)}`
				);
			}
		}

		// 3. 箱から中身がはみ出す（外殻 .shell の子）
		for (const shell of document.querySelectorAll('.shell')) {
			if (!shown(shell)) continue;
			const frame = shell.getBoundingClientRect();
			for (const child of shell.children) {
				if (!shown(child)) continue;
				if (getComputedStyle(child).position === 'absolute') continue;
				const r = child.getBoundingClientRect();
				if (
					r.left < frame.left - SLACK ||
					r.right > frame.right + SLACK ||
					r.top < frame.top - SLACK ||
					r.bottom > frame.bottom + SLACK
				) {
					found.push(`箱からはみ出している: ${label(child)} ⊄ ${label(shell)}`);
				}
			}
		}

		// 4. 窓の外へ出る（中で巻き取る箱の中は除く）・窓ごとのスクロール
		const doc = document.documentElement;
		if (doc.scrollWidth > window.innerWidth + SLACK) found.push('窓に横スクロールが出ている');
		if (doc.scrollHeight > window.innerHeight + SLACK) found.push('窓に縦スクロールが出ている');
		for (const el of document.querySelectorAll('header *, button, [role="tab"]')) {
			if (!shown(el) || insideScroller(el)) continue;
			const r = el.getBoundingClientRect();
			if (r.right > window.innerWidth + SLACK || r.left < -SLACK) {
				found.push(`窓の外へ出ている: ${label(el)}`);
			}
		}

		return [...new Set(found)];
	});
}
