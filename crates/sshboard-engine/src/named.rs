//! **名指しした接続で動かす**（2026-11-11・オーナーが案 A を選んだ）。
//!
//! 宛先（`held.active`）は sshboard 全体で 1 つで、**人が画面でタブを押すと動きます。**
//! AI の道具がそれに頼ると、ある席の呼びが、画面で押された別の機械へ黙って届きます
//! （2026-11-11 に本番機へ届いた）。
//!
//! AI の道具は `Engine::on(id, …)` の中で動かします。その中では、
//! 宛先を引く所がすべて**名指しの接続**を使います。人の操作（画面）は今までどおり宛先です。
//!
//! 引数で 1 本ずつ通さずに task-local にしたのは、**宛先を引く所が実行体の奥に何か所もあり**
//! （SSH・上げ方・出力・操作の札）、1 か所でも通し忘れると、そこだけ画面の宛先へ戻るからです。

use std::future::Future;

tokio::task_local! {
    static NAMED: String;
}

/// `fut` を、名指しした接続 `id` で動かす。
pub(crate) async fn scoped<F: Future>(id: &str, fut: F) -> F::Output {
    NAMED.scope(id.to_owned(), fut).await
}

/// いま名指しされている接続。**名指しの外では `None`**（画面の宛先を使う）。
pub(crate) fn current() -> Option<String> {
    NAMED.try_with(Clone::clone).ok()
}
