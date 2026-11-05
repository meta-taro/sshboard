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

/// 出力の中の最初の数（`-` と小数点を含む）。
fn first_number(out: &str) -> Option<f64> {
    let start = out.find(|c: char| c.is_ascii_digit())?;
    let signed = start > 0 && out.as_bytes()[start - 1] == b'-';
    let digits: String = out[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let number: f64 = digits.trim_end_matches('.').parse().ok()?;
    Some(if signed { -number } else { number })
}

fn shown(out: &str) -> String {
    let line = out
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    line.chars().take(VALUE_MAX_CHARS).collect()
}
