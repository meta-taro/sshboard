/**
 * 端末が、いま見ている接続のものかどうか。
 *
 * **端末は全体で 1 本**です（D29）。接続のタブを移しても端末は付いてきません。
 * **そのことが画面に出ていないと、使う人は「開けないのか、やり方が分からないのか」を
 * 判断できません**（2026-10-02 に実機で踏みました）。
 */

/**
 * いま見ている接続と、端末が開いている接続が違うなら、**その相手の名前**を返す。
 * 同じ／どちらか無いなら `null`。
 */
export function consoleElsewhere(consoleOn: string | null, selected: string | null): string | null {
	if (!consoleOn || !selected) return null;
	return consoleOn === selected ? null : consoleOn;
}
