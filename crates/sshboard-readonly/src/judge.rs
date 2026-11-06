//! 手順 1 つの**良し悪し**（D72）。決まった形の基準だけを見ます。

use serde::{Deserialize, Serialize};

use crate::checks::Step;

/// 見せる値の長さの上限。**画面と帯を 1 行の値で埋めない。**
const VALUE_MAX_CHARS: usize = 200;

/// 基準。**書いたものだけを全部満たせば良し。**何も書かなければ終了コード 0 で良し。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Expect {
    /// この字が出ていれば良し。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    /// この字が出ていなければ良し。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent: Option<String>,
    /// 出力の最初の数が、これ**未満**なら良し。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub below: Option<f64>,
    /// 出力の最初の数が、これ**より大きければ**良し。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above: Option<f64>,
}

/// 手順を走らせた結果。**走らなかったことも結果です。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Ran { out: String, status: Option<u32> },
    DidNotRun { why: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    Good,
    Bad {
        why: String,
    },
    /// **測れなかった。良しにはしない。**
    Unknown {
        why: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Judged {
    pub verdict: Verdict,
    /// 人とエージェントに見せる値（出力の最初の空でない行）。
    pub value: String,
}

impl Step {
    /// 人に見せる名前。
    pub fn title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.readonly)
    }

    pub fn judge(&self, outcome: &StepOutcome) -> Judged {
        match outcome {
            StepOutcome::DidNotRun { why } => Judged {
                verdict: Verdict::Unknown { why: why.clone() },
                value: String::new(),
            },
            StepOutcome::Ran { out, status } => Judged {
                verdict: self.expect.verdict(out, *status),
                value: shown(out),
            },
        }
    }
}

impl Expect {
    fn verdict(&self, out: &str, status: Option<u32>) -> Verdict {
        match status {
            None => return unknown("終了コードが返ってきませんでした"),
            Some(code) if code != 0 => return bad(format!("終了コード {code}")),
            Some(_) => {}
        }
        if let Some(word) = &self.contains {
            if !out.contains(word.as_str()) {
                return bad(format!("「{word}」が出ていません"));
            }
        }
        if let Some(word) = &self.absent {
            if out.contains(word.as_str()) {
                return bad(format!("「{word}」が出ています"));
            }
        }
        self.number_verdict(out)
    }

    fn number_verdict(&self, out: &str) -> Verdict {
        if self.below.is_none() && self.above.is_none() {
            return Verdict::Good;
        }
        let Some(number) = first_number(out) else {
            return unknown("出力に数がありません");
        };
        if let Some(limit) = self.below {
            if number >= limit {
                return bad(format!("{number} は {limit} 未満ではありません"));
            }
        }
        if let Some(limit) = self.above {
            if number <= limit {
                return bad(format!("{number} は {limit} より大きくありません"));
            }
        }
        Verdict::Good
    }
}

fn bad(why: String) -> Verdict {
    Verdict::Bad { why }
}

fn unknown(why: &str) -> Verdict {
    Verdict::Unknown {
        why: why.to_owned(),
    }
}

/// 出力の中の**最初の数**。
///
/// **語の頭からだけ読みます**（コードレビュー M3・2026-11-06）—— 語の途中の数字
/// （`eth0`・`db01`）や、語を繋ぐ `-`（`web-1`）を数にすると、どんな値でも基準に通って
/// 「良し」になります。`-` は行頭か、空白・`=`・`:`・`(` のあとだけ符号です。
/// 3 桁ごとの `,` と指数（`1.5e-05`）は続けて読みます。日付（`2024-01-05`）は 2024 です。
fn first_number(out: &str) -> Option<f64> {
    let chars: Vec<char> = out.chars().collect();
    (0..chars.len()).find_map(|at| number_at(&chars, at))
}

/// `at` から数が始まっていれば読む。
fn number_at(chars: &[char], at: usize) -> Option<f64> {
    let before = at.checked_sub(1).map(|i| chars[i]);
    let signed = chars[at] == '-';
    let digits_from = if signed { at + 1 } else { at };
    if !chars.get(digits_from).is_some_and(char::is_ascii_digit) {
        return None;
    }
    let starts_a_word = match before {
        None => true,
        Some(c) if signed => c.is_whitespace() || "=:(,[".contains(c),
        Some(c) => !(c.is_alphanumeric() || c == '.' || c == '_' || c == '-'),
    };
    if !starts_a_word {
        return None;
    }
    let text: String = read_number(chars, digits_from);
    let number: f64 = text.parse().ok()?;
    Some(if signed { -number } else { number })
}

/// 数字・3 桁区切り・小数・指数を読み、**区切りを落とした**文字列を返す。
fn read_number(chars: &[char], from: usize) -> String {
    let mut text = String::new();
    let mut i = from;
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            text.push(c);
            i += 1;
        } else if c == ',' && (1..=3).all(|k| digit(i + k)) && !digit(i + 4) {
            i += 1;
        } else {
            break;
        }
    }
    if chars.get(i) == Some(&'.') && digit(i + 1) {
        text.push('.');
        i += 1;
        while digit(i) {
            text.push(chars[i]);
            i += 1;
        }
    }
    if matches!(chars.get(i), Some('e' | 'E')) {
        let sign = matches!(chars.get(i + 1), Some('+' | '-'));
        let first = if sign { i + 2 } else { i + 1 };
        if digit(first) {
            text.push('e');
            if sign {
                text.push(chars[i + 1]);
            }
            i = first;
            while digit(i) {
                text.push(chars[i]);
                i += 1;
            }
        }
    }
    text
}

fn shown(out: &str) -> String {
    let line = out
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    line.chars().take(VALUE_MAX_CHARS).collect()
}
