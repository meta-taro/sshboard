/**
 * OS のメニューバーの文言。**訳はここ 1 箇所だけ。**
 *
 * Rust 側にも訳を置くと二重管理になるので、**画面側が訳した文字列を渡して**
 * メニューを組みます。
 */
export const MENU_KEYS = [
	'menu.about', 'menu.quit', 'menu.edit', 'menu.undo', 'menu.redo',
	'menu.cut', 'menu.copy', 'menu.paste', 'menu.selectAll',
	'menu.window', 'menu.minimize', 'menu.zoom', 'menu.close'
] as const;

export type MenuKey = (typeof MENU_KEYS)[number];

type MenuCatalog = Record<MenuKey, string>;

export const MENU: Record<string, MenuCatalog> = {
	en: { 'menu.about': 'About sshboard', 'menu.quit': 'Quit sshboard', 'menu.edit': 'Edit', 'menu.undo': 'Undo', 'menu.redo': 'Redo', 'menu.cut': 'Cut', 'menu.copy': 'Copy', 'menu.paste': 'Paste', 'menu.selectAll': 'Select All', 'menu.window': 'Window', 'menu.minimize': 'Minimize', 'menu.zoom': 'Zoom', 'menu.close': 'Close' },
	ja: { 'menu.about': 'sshboard について', 'menu.quit': 'sshboard を終了', 'menu.edit': '編集', 'menu.undo': '取り消す', 'menu.redo': 'やり直す', 'menu.cut': '切り取る', 'menu.copy': 'コピー', 'menu.paste': 'ペースト', 'menu.selectAll': 'すべてを選択', 'menu.window': 'ウインドウ', 'menu.minimize': 'しまう', 'menu.zoom': '拡大／縮小', 'menu.close': '閉じる' },
	ko: { 'menu.about': 'sshboard 정보', 'menu.quit': 'sshboard 종료', 'menu.edit': '편집', 'menu.undo': '실행 취소', 'menu.redo': '다시 실행', 'menu.cut': '오려두기', 'menu.copy': '복사하기', 'menu.paste': '붙여넣기', 'menu.selectAll': '모두 선택', 'menu.window': '윈도우', 'menu.minimize': '최소화', 'menu.zoom': '확대/축소', 'menu.close': '닫기' },
	'zh-CN': { 'menu.about': '关于 sshboard', 'menu.quit': '退出 sshboard', 'menu.edit': '编辑', 'menu.undo': '撤销', 'menu.redo': '重做', 'menu.cut': '剪切', 'menu.copy': '拷贝', 'menu.paste': '粘贴', 'menu.selectAll': '全选', 'menu.window': '窗口', 'menu.minimize': '最小化', 'menu.zoom': '缩放', 'menu.close': '关闭' },
	'zh-TW': { 'menu.about': '關於 sshboard', 'menu.quit': '結束 sshboard', 'menu.edit': '編輯', 'menu.undo': '還原', 'menu.redo': '重做', 'menu.cut': '剪下', 'menu.copy': '拷貝', 'menu.paste': '貼上', 'menu.selectAll': '全選', 'menu.window': '視窗', 'menu.minimize': '縮到最小', 'menu.zoom': '縮放', 'menu.close': '關閉' },
	de: { 'menu.about': 'Über sshboard', 'menu.quit': 'sshboard beenden', 'menu.edit': 'Bearbeiten', 'menu.undo': 'Widerrufen', 'menu.redo': 'Wiederholen', 'menu.cut': 'Ausschneiden', 'menu.copy': 'Kopieren', 'menu.paste': 'Einsetzen', 'menu.selectAll': 'Alles auswählen', 'menu.window': 'Fenster', 'menu.minimize': 'Im Dock ablegen', 'menu.zoom': 'Zoomen', 'menu.close': 'Schließen' },
	fr: { 'menu.about': 'À propos de sshboard', 'menu.quit': 'Quitter sshboard', 'menu.edit': 'Édition', 'menu.undo': 'Annuler', 'menu.redo': 'Rétablir', 'menu.cut': 'Couper', 'menu.copy': 'Copier', 'menu.paste': 'Coller', 'menu.selectAll': 'Tout sélectionner', 'menu.window': 'Fenêtre', 'menu.minimize': 'Réduire', 'menu.zoom': 'Zoom', 'menu.close': 'Fermer' },
	es: { 'menu.about': 'Acerca de sshboard', 'menu.quit': 'Salir de sshboard', 'menu.edit': 'Edición', 'menu.undo': 'Deshacer', 'menu.redo': 'Rehacer', 'menu.cut': 'Cortar', 'menu.copy': 'Copiar', 'menu.paste': 'Pegar', 'menu.selectAll': 'Seleccionar todo', 'menu.window': 'Ventana', 'menu.minimize': 'Minimizar', 'menu.zoom': 'Zoom', 'menu.close': 'Cerrar' },
	'pt-BR': { 'menu.about': 'Sobre o sshboard', 'menu.quit': 'Encerrar sshboard', 'menu.edit': 'Editar', 'menu.undo': 'Desfazer', 'menu.redo': 'Refazer', 'menu.cut': 'Recortar', 'menu.copy': 'Copiar', 'menu.paste': 'Colar', 'menu.selectAll': 'Selecionar tudo', 'menu.window': 'Janela', 'menu.minimize': 'Minimizar', 'menu.zoom': 'Zoom', 'menu.close': 'Fechar' },
	ru: { 'menu.about': 'О программе sshboard', 'menu.quit': 'Завершить sshboard', 'menu.edit': 'Правка', 'menu.undo': 'Отменить', 'menu.redo': 'Повторить', 'menu.cut': 'Вырезать', 'menu.copy': 'Скопировать', 'menu.paste': 'Вставить', 'menu.selectAll': 'Выбрать все', 'menu.window': 'Окно', 'menu.minimize': 'Убрать в Dock', 'menu.zoom': 'Масштаб', 'menu.close': 'Закрыть' },
	it: { 'menu.about': 'Informazioni su sshboard', 'menu.quit': 'Esci da sshboard', 'menu.edit': 'Modifica', 'menu.undo': 'Annulla', 'menu.redo': 'Ripristina', 'menu.cut': 'Taglia', 'menu.copy': 'Copia', 'menu.paste': 'Incolla', 'menu.selectAll': 'Seleziona tutto', 'menu.window': 'Finestra', 'menu.minimize': 'Riduci a icona', 'menu.zoom': 'Zoom', 'menu.close': 'Chiudi' }
};

/** その言語のメニュー文言。無ければ英語へ落とす。 */
export function menuLabels(locale: string): MenuCatalog {
	return MENU[locale] ?? MENU.en;
}
