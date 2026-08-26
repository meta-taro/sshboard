//! 生の出力から ANSI を落として、素のテキストにする。
//!
//! **ANSI を解釈しません。落とすだけです。**
//! 解釈は xterm.js の仕事で、そこは自前で書かない（D7）。ここがやるのは
//! 「AI へ渡す文字列にエスケープを混ぜない」ことだけ。
//!
//! **難しいのは境界です。**ネットワークから来るチャンクは、
//! エスケープの途中でも、UTF-8 の途中でも切れる。持ち越さないと壊れる。

/// ANSI を落としつつ、チャンクをまたぐ途中の並びを持ち越すフィルタ。
pub struct PlainFilter {
    state: State,
    /// まだ文字として完成していないバイト（UTF-8 の途中）。
    bytes: Vec<u8>,
    /// `\r` を見たが、次が `\n` かどうかまだ分からない。
    pending_carriage_return: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Text,
    /// `ESC` を見た。
    Escape,
    /// `ESC [` のあと。終端は 0x40〜0x7E。
    ControlSequence,
    /// `ESC (` のような中間バイトのあと。終端は 0x30〜0x7E。
    EscapeIntermediate,
    /// `ESC ]` などの文字列系。終端は BEL か `ESC \`。
    StringSequence,
    /// 文字列系の中で `ESC` を見た。次が `\` なら終わり。
    StringSequenceEscape,
}

impl PlainFilter {
    pub fn new() -> Self {
        Self {
            state: State::Text,
            bytes: Vec::new(),
            pending_carriage_return: false,
        }
    }

    /// チャンクを 1 つ食わせ、**文字として完成した分だけ**返す。
    pub fn push(&mut self, chunk: &[u8]) -> String {
        for byte in chunk {
            self.step(*byte);
        }
        self.take_completed()
    }

    /// 打ち切るときに、持ち越していた分を吐き出す。
    pub fn finish(&mut self) -> String {
        self.release_carriage_return();
        self.state = State::Text;
        self.take_completed()
    }

    fn step(&mut self, byte: u8) {
        match self.state {
            State::Text => self.step_text(byte),
            State::Escape => self.state = next_after_escape(byte),
            // 終端バイトが来るまで捨てる。
            State::ControlSequence => {
                if (0x40..=0x7E).contains(&byte) {
                    self.state = State::Text;
                }
            }
            State::EscapeIntermediate => {
                if (0x30..=0x7E).contains(&byte) {
                    self.state = State::Text;
                }
            }
            State::StringSequence => match byte {
                BEL => self.state = State::Text,
                ESC => self.state = State::StringSequenceEscape,
                _ => {}
            },
            State::StringSequenceEscape => match byte {
                b'\\' => self.state = State::Text,
                ESC => {}
                _ => self.state = State::StringSequence,
            },
        }
    }

    fn step_text(&mut self, byte: u8) {
        match byte {
            // `\r\n` は `\n` 1 つにする。pty 経由だと `\r\n` で来るので、
            // ここで揃えないと AI へ渡す文字列に `\r` が混ざる。
            b'\n' if self.pending_carriage_return => {
                self.pending_carriage_return = false;
                self.bytes.push(b'\n');
            }
            b'\r' => {
                // 直前の `\r` は `\n` を伴わなかった＝進捗表示。握り潰さない。
                self.release_carriage_return();
                self.pending_carriage_return = true;
            }
            ESC => {
                self.release_carriage_return();
                self.state = State::Escape;
            }
            _ => {
                self.release_carriage_return();
                self.bytes.push(byte);
            }
        }
    }

    /// 持ち越していた `\r` を確定させる。
    fn release_carriage_return(&mut self) {
        if self.pending_carriage_return {
            self.pending_carriage_return = false;
            self.bytes.push(b'\r');
        }
    }

    /// 文字として完成した分だけ取り出す。**途中で切れた UTF-8 は持ち越す。**
    fn take_completed(&mut self) -> String {
        let mut out = String::new();

        loop {
            match std::str::from_utf8(&self.bytes) {
                Ok(text) => {
                    out.push_str(text);
                    self.bytes.clear();
                    return out;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    // valid_up_to までは UTF-8 として妥当だと標準ライブラリが言っている。
                    out.push_str(std::str::from_utf8(&self.bytes[..valid]).unwrap_or_default());

                    match error.error_len() {
                        // 壊れたバイト。**黙って消さない。**
                        // ここでは符号化方式の変換をしない（それは 002 の結果で決める）。
                        Some(len) => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            self.bytes.drain(..valid + len);
                        }
                        // 途中で切れている。次のチャンクを待つ。
                        None => {
                            self.bytes.drain(..valid);
                            return out;
                        }
                    }
                }
            }
        }
    }
}

const ESC: u8 = 0x1B;
const BEL: u8 = 0x07;

/// `ESC` の次の 1 バイトで、どの並びが始まったかが決まる。
fn next_after_escape(byte: u8) -> State {
    match byte {
        b'[' => State::ControlSequence,
        // OSC / DCS / SOS / PM / APC。どれも終端は BEL か `ESC \`。
        b']' | b'P' | b'X' | b'^' | b'_' => State::StringSequence,
        // 中間バイト。`ESC ( B` のような文字集合の指定。
        0x20..=0x2F => State::EscapeIntermediate,
        // `ESC c` のような 2 バイトで終わる並び。
        _ => State::Text,
    }
}

impl Default for PlainFilter {
    fn default() -> Self {
        Self::new()
    }
}
