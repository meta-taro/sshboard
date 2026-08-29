/**
 * 撮る前に伏せる（D26）。
 *
 * **この製品は画面そのものに接続先が写り込みます**（CLAUDE.md 禁止事項 4）。
 * 接続名・タグ・サーバー側のパス・指紋・ファイル一覧。
 * 撮った画像は AI の文脈に入り、記録に残ります。**伏せていない画像は、そこから漏れます。**
 *
 * ## なぜ画面側で伏せるのか
 *
 * 撮ってから画像を加工する形にすると、**元の画像が一瞬でも存在します。**
 * ここで先に伏せてしまえば、**伏せていない画素は最初から作られません。**
 *
 * ## 何を伏せるか
 *
 * 伏せるのは**中身**であって、**形ではありません。**
 * 大きさ・位置・重なり・はみ出しはそのまま残るので、**崩れは見つかります。**
 */
import { emit, listen } from '@tauri-apps/api/event';
import { tick } from 'svelte';

/** Rust 側が送ってくるイベント名。**片方だけ変えると伏せずに撮る。** */
export const REDACT_EVENT = 'capture://redact';

/** 画面が「伏せ終わった」と返す口。 */
export const REDACT_READY_EVENT = 'capture://ready';

class Redaction {
	/** いま伏せているか。**画面の見た目が変わるので `$state`。** */
	on = $state(false);

	/**
	 * 伏せる／戻す。
	 *
	 * **DOM に反映し終わるまで待って**から返します。
	 * 返した時点で伏せ終わっていないと、撮った画像に中身が残ります。
	 */
	async set(on: boolean): Promise<void> {
		this.on = on;
		if (typeof document !== 'undefined') {
			if (on) document.documentElement.setAttribute('data-redact', 'on');
			else document.documentElement.removeAttribute('data-redact');
		}
		await tick();
	}

	/** Rust からの指示を受け取り始める。返り値を呼ぶと購読を止める。 */
	async watch(): Promise<() => void> {
		return listen<boolean>(REDACT_EVENT, async (event) => {
			await this.set(event.payload);
			// **伏せ終わったことを返す。**返す前に撮られると意味が無い。
			await emit(REDACT_READY_EVENT, this.on);
		});
	}
}

export const redaction = new Redaction();
