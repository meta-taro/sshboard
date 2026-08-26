//! 人と AI の操作が流れる 1 本の帯。
//!
//! **GUI にも MCP にも SSH にも依存しない。**ここが依存を持った瞬間、
//! 帯は「画面の部品」になり、記録として使えなくなる（decisions D5）。

mod band;
mod line;

pub use band::{Band, BandEvent, Delivery, DeliveryOutcome, Subscriber};
pub use line::{Actor, BandLine};
/// 購読が追いつかなかった / 閉じたことを表す。
/// 帯の利用側が tokio を直接知らずに済むよう、ここから出す。
pub use tokio::sync::broadcast::error::RecvError;
