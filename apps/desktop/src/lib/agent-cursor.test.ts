// @vitest-environment jsdom
/**
 * エージェント専用カーソルの、画面を開かずに確かめられる所（2026-11-12）。
 *
 * - **伏せる所にある接続先は、探しても見つからない**（見つかる／見つからないで当てさせない）
 * - **押してよい印の無い物は、押してよい物にならない**
 * - 吹き出しは窓の四隅でも窓の中に収まる
 */
import { describe, expect, test } from 'vitest';

import { agentView, bubblePlacement, findTarget, showMs } from './agent-cursor';

/** jsdom は寸法を持たないので、**見えている**と見なして探す。 */
const everyone = () => true;

function page(html: string): Document {
	document.body.innerHTML = html;
	return document;
}

describe('findTarget', () => {
	test('finds a tab by its visible text', () => {
		// Arrange
		const doc = page('<nav><button data-agent-view="files">Files</button></nav>');

		// Act
		const found = findTarget(doc, 'Files', everyone);

		// Assert
		expect(found?.textContent).toBe('Files');
	});

	test('never matches a connection detail hidden behind data-secret', () => {
		// Arrange —— 端末のタブは接続名を伏せる印の中に持つ。
		const doc = page(
			'<button role="tab"><span class="num">#1</span><span data-secret>host-secret-01</span></button>'
		);

		// Act
		const found = findTarget(doc, 'host-secret-01', everyone);

		// Assert
		expect(found).toBeUndefined();
	});

	test('does not read the label of something that holds a secret', () => {
		// Arrange —— 読み上げ名に接続先が入っていても、それで当てさせない。
		const doc = page('<button aria-label="192.0.2.7"><span data-secret>192.0.2.7</span></button>');

		// Act
		const found = findTarget(doc, '192.0.2.7', everyone);

		// Assert
		expect(found).toBeUndefined();
	});

	test('does not search inside the terminal or an input', () => {
		// Arrange
		const doc = page(
			'<div class="xterm"><button>demo@host ~</button></div><label><input value="secret-host" /></label>'
		);

		// Act + Assert
		expect(findTarget(doc, 'demo@host', everyone)).toBeUndefined();
		expect(findTarget(doc, 'secret-host', everyone)).toBeUndefined();
	});

	test('prefers an exact name over one that merely contains it', () => {
		// Arrange
		const doc = page('<button>Files and more</button><button>Files</button>');

		// Act
		const found = findTarget(doc, 'Files', everyone);

		// Assert
		expect(found?.textContent).toBe('Files');
	});
});

describe('agentView', () => {
	test('names the view only for things marked for agents', () => {
		// Arrange
		const doc = page('<button data-agent-view="console">Console</button><button>Allow</button>');
		const [tab, allow] = [...doc.querySelectorAll('button')];

		// Act + Assert —— 承認の釦のような印の無い物は、押してよい物にならない。
		expect(agentView(tab)).toBe('console');
		expect(agentView(allow)).toBeUndefined();
	});
});

describe('bubblePlacement', () => {
	const view = { width: 800, height: 600 };
	const size = { width: 280, height: 60 };
	const corners = [
		{ name: 'top-left', box: { left: 0, top: 0, right: 30, bottom: 20 } },
		{ name: 'top-right', box: { left: 770, top: 0, right: 800, bottom: 20 } },
		{ name: 'bottom-left', box: { left: 0, top: 580, right: 30, bottom: 600 } },
		{ name: 'bottom-right', box: { left: 770, top: 580, right: 800, bottom: 600 } }
	];

	for (const { name, box } of corners) {
		test(`stays inside the window at the ${name} corner`, () => {
			// Arrange
			const anchor = { x: (box.left + box.right) / 2, y: (box.top + box.bottom) / 2 };

			// Act
			const place = bubblePlacement(box, anchor, size, view);

			// Assert —— 窓の端から 8px 内側。
			expect(place.left).toBeGreaterThanOrEqual(8);
			expect(place.top).toBeGreaterThanOrEqual(8);
			expect(place.left + size.width).toBeLessThanOrEqual(view.width - 8);
			expect(place.top + size.height).toBeLessThanOrEqual(view.height - 8);
		});
	}
});

describe('showMs', () => {
	test('uses the seconds asked for, else the length of the message', () => {
		expect(showMs('hi', 3)).toBe(3000);
		expect(showMs('')).toBe(2600);
		expect(showMs('a'.repeat(10))).toBe(6000);
		expect(showMs('a'.repeat(1000))).toBe(20000);
	});
});

