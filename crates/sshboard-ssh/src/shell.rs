//! 遠くのシェルへ渡す引数を囲う。
//!
//! **`exec` に渡した文字列は、向こうのシェルがそのまま解釈します。**
//! 引数を素で埋め込めば、`nginx; rm -rf /` はコマンド 2 本として走ります。
//!
//! `run_readonly` に引数のスロットを作らなかったのは、この面を作らないためでした（D3）。
//! 用途別ツール（`service_status` など）は引数を取るので、**必ずここを通します。**

/// 単引用符で囲う。**常に囲います。**
///
/// 「英数字だけなら素通し」という近道を作りません。**判定を 1 つ間違えたら終わり**で、
/// 間違いは「動いてしまった」形で現れます。囲うのは安く、外すのは高い。
///
/// 単引用符の中では `$` も `` ` `` も `;` も改行も、ただの文字です。
/// 唯一の例外が単引用符そのもので、これは**一度閉じて `\'` を置き、開き直します**
/// （POSIX に、囲いの中へ単引用符を入れる書き方はありません）。
pub fn quote(argument: &str) -> String {
    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('\'');
    for character in argument.chars() {
        if character == '\'' {
            // 閉じる → 引用符を素で 1 つ（`\'`）→ 開き直す。
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}
