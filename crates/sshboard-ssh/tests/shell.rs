//! 遠くのシェルへ渡す引数の囲い方。
//!
//! **`exec` は文字列をそのままシェルへ渡します。**引数を素で埋め込むと、
//! `nginx; rm -rf /` のような値が**そのまま実行されます。**
//!
//! `run_readonly` で引数のスロットを作らなかったのは、この面を作らないためでした（D3）。
//! 用途別ツールは引数を取るので、**ここで囲います。**

use sshboard_ssh::quote;

#[test]
fn a_plain_word_comes_back_quoted() {
    // **常に囲う。**「安全そうな文字だけ素通し」は、判定を間違えたら終わり。
    assert_eq!(quote("nginx"), "'nginx'");
}

#[test]
fn an_empty_argument_is_still_one_argument() {
    // 囲わないと**引数そのものが消えます。**位置がずれて別の意味になる。
    assert_eq!(quote(""), "''");
}

#[test]
fn a_command_separator_becomes_part_of_the_name() {
    // **これが本番。**囲えていなければ、2 つ目のコマンドとして走ります。
    assert_eq!(quote("nginx; rm -rf /"), "'nginx; rm -rf /'");
}

#[test]
fn a_single_quote_inside_does_not_end_the_quoting() {
    // ここを間違えると**囲いが途中で切れて、続きが素のシェルになります。**
    // POSIX に閉じ引用符の中へ引用符を入れる書き方は無いので、
    // 一度閉じて `\'` を置き、開き直す。
    assert_eq!(quote("it's"), "'it'\\''s'");
}

#[test]
fn a_closing_quote_followed_by_a_command_stays_inert() {
    // `'; rm -rf /; echo '` のような、囲いを閉じにくる値。
    let hostile = "'; rm -rf /; echo '";

    let quoted = quote(hostile);

    // **素の `;` が囲いの外に出ていないこと。**
    assert_eq!(quoted, "''\\''; rm -rf /; echo '\\'''");
    assert!(quoted.starts_with('\''));
    assert!(quoted.ends_with('\''));
}

#[test]
fn expansions_and_backticks_stay_literal() {
    // 単引用符の中では `$` も `` ` `` も展開されない。**囲うだけで足りる。**
    assert_eq!(quote("$(whoami)"), "'$(whoami)'");
    assert_eq!(quote("`id`"), "'`id`'");
    assert_eq!(quote("$HOME"), "'$HOME'");
}

#[test]
fn a_newline_survives_without_starting_a_new_command() {
    // 改行は**次のコマンドの始まり**になりうる。囲いの中なら、ただの改行。
    assert_eq!(quote("a\nb"), "'a\nb'");
}

#[test]
fn japanese_is_left_alone() {
    // 対象は国内のサーバー。**囲うだけで、中身は触らない。**
    assert_eq!(quote("設定ファイル"), "'設定ファイル'");
}
