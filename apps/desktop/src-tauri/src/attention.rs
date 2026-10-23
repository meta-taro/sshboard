//! **問いが出たことに、人が気づけるようにする**（Issue #30）。
//!
//! ## なぜ要るか
//!
//! 窓が他のウィンドウの後ろに居ると、**問いが出ても見えません。**
//!
//! > パス入れるときに、アプリを前面に出せます？
//! > **私の場合、裏で聞かれていて気づかないですね**
//!
//! 報告者の環境では、**1 日 4 回**パスフレーズを聞かれ、
//! **その半分以上で「押しましたか」「まだですか」の往復**が起きています。
//!
//! `await_answer`（#29）で **AI は待てるようになりました。**
//! **AI がどれだけ上手に待っても、人が気づかなければ待ち時間は減りません。**
//!
//! ## 前面強制を既定にしません
//!
//! 報告者自身がこう書いています ——
//!
//! > **人によっては邪魔といいますが**、私の場合、裏で聞かれていて気づかない
//!
//! **既定は点滅**（Windows のタスクバー／macOS の Dock）。
//! 前面へ出すのは**選んだ人だけ**です。
//!
//! ## 壊れていたら、黙りません
//!
//! 読めない設定を「off」と読むと、**気づかせる仕掛けが黙って消えます。**
//! 読めなければ**既定（点滅）**に倒します —— 黙るより、鳴る方が直せます。

use std::path::Path;

/// どう気づかせるか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    /// 何もしない。**選んだ人だけ**がここへ来ます。
    Off,
    /// 点滅（Windows のタスクバー／macOS の Dock）。**既定。**
    ///
    /// **前面へは出しません。**「そこに何かある」だけを伝えます。
    #[default]
    Flash,
    /// 前面へ出す。**割り込みます。**選んだ人だけ。
    Front,
}

impl How {
    /// 設定ファイルの名前。**`readonly.toml` などと同じ並び**に置きます。
    pub const FILE: &'static str = "attention.toml";

    fn from_word(word: &str) -> Option<Self> {
        match word
            .trim()
            .trim_matches(['"', '\''])
            .to_ascii_lowercase()
            .as_str()
        {
            "off" => Some(How::Off),
            "flash" => Some(How::Flash),
            "front" => Some(How::Front),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            How::Off => "off",
            How::Flash => "flash",
            How::Front => "front",
        }
    }
}

/// 読む。**既定は点滅。**
///
/// **置いていない・壊れている・知らない言葉**は、どれも既定に倒します。
/// ここで黙ると、**気づかせる仕掛けが静かに消えます。**
pub fn parse(text: &str) -> How {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "how" {
            continue;
        }
        // **知らない言葉も既定へ。**打ち間違いで黙らせない。
        return How::from_word(value).unwrap_or_default();
    }
    How::default()
}

/// ファイルから読む。**無ければ既定。**
pub fn read(path: &Path) -> How {
    std::fs::read_to_string(path)
        .map(|text| parse(&text))
        .unwrap_or_default()
}

/// 書く。**人が後から読んで直せる形**にします。
pub fn write(path: &Path, how: How) -> std::io::Result<()> {
    std::fs::write(
        path,
        format!(
            "# 問いが出たときに、どう気づかせるか（Issue #30）。\n\
             #\n\
             # flash … 点滅だけ（既定）。Windows はタスクバー、macOS は Dock。\n\
             #         **前面へは出しません。**\n\
             # front … 前面へ出す。**割り込みます。**\n\
             # off   … 何もしない。\n\
             #\n\
             # **読めない値は flash に倒します。**打ち間違いで黙らせないためです。\n\
             how = \"{}\"\n",
            how.word()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{parse, How};

    #[test]
    fn nothing_written_means_flashing() {
        // **既定は点滅。**前面強制は、選んだ人だけ。
        assert_eq!(parse(""), How::Flash);
    }

    #[test]
    fn a_person_can_ask_for_the_window_to_come_forward() {
        assert_eq!(parse("how = \"front\"\n"), How::Front);
    }

    #[test]
    fn a_person_can_turn_it_off() {
        assert_eq!(parse("how = \"off\"\n"), How::Off);
    }

    #[test]
    fn a_typo_does_not_silence_it() {
        // **ここが一番大事。**読めない値を off と読むと、
        // 気づかせる仕掛けが**黙って消えます。**
        assert_eq!(parse("how = \"flsah\"\n"), How::Flash);
        assert_eq!(parse("how = \"\"\n"), How::Flash);
        assert_eq!(parse("нечто = x\n"), How::Flash);
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        assert_eq!(parse("# front にしたい\n\nhow = \"off\"\n"), How::Off);
    }

    #[test]
    fn the_first_setting_wins() {
        // 2 回書かれていたら上を採ります。**黙って下を採ると、
        // 人は直したつもりの行を見て混乱します。**
        assert_eq!(parse("how = \"front\"\nhow = \"off\"\n"), How::Front);
    }

    #[test]
    fn it_accepts_a_bare_word_too() {
        // 引用符を忘れても読みます。**設定で人を弾きません。**
        assert_eq!(parse("how = front\n"), How::Front);
    }
}
