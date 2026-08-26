//! 人と AI の操作が流れる 1 本の帯。
//!
//! **GUI にも MCP にも SSH にも依存しない。**ここが依存を持った瞬間、
//! 帯は「画面の部品」になり、記録として使えなくなる（decisions D5）。

mod band;
mod line;

pub use band::{Band, BandEvent, Delivery, DeliveryOutcome, Subscriber};
pub use line::{Actor, BandLine};
