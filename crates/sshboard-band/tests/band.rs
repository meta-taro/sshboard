//! 帯の受け入れテスト。
//!
//! ここで守っているのは Issue 001 の完了条件のうち 2 つ:
//!   - 行頭に `[AI]` が付き、人の操作と区別できる
//!   - 帯への反映がツール応答より先か同時（＝ ack を待てる）

use std::time::Duration;

use sshboard_band::{Actor, Band, BandLine, DeliveryOutcome};

#[test]
fn ai_lines_are_tagged_so_they_can_be_told_apart_from_human_lines() {
    // Arrange
    let line = BandLine::new(0, Actor::Ai, "df -h");

    // Act
    let rendered = line.render();

    // Assert
    assert!(rendered.starts_with("[AI]"), "実際の出力: {rendered:?}");
    assert!(rendered.contains("df -h"), "実際の出力: {rendered:?}");
}

#[test]
fn human_lines_are_tagged_too() {
    let rendered = BandLine::new(0, Actor::Human, "cd /var/www").render();

    assert!(rendered.starts_with("[Human]"), "実際の出力: {rendered:?}");
}

#[test]
fn both_tags_are_padded_to_the_same_width_so_the_band_lines_up() {
    // Arrange
    let ai = BandLine::new(0, Actor::Ai, "x").render();
    let human = BandLine::new(1, Actor::Human, "x").render();

    // Act
    let ai_text_starts_at = ai.find('x').expect("本文が無い");
    let human_text_starts_at = human.find('x').expect("本文が無い");

    // Assert
    assert_eq!(
        ai_text_starts_at, human_text_starts_at,
        "ai={ai:?} human={human:?}"
    );
}

#[tokio::test]
async fn seq_increases_by_one_per_recorded_line() {
    // Arrange
    let band = Band::new();

    // Act
    let first = band.record(Actor::Human, "one");
    let second = band.record(Actor::Ai, "two");

    // Assert
    assert_eq!(first.line().seq(), 0);
    assert_eq!(second.line().seq(), 1);
}

#[tokio::test]
async fn a_subscriber_receives_the_recorded_line() {
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();

    // Act
    band.record(Actor::Ai, "ping");
    let event = subscriber.recv().await.expect("購読者へ届いていない");

    // Assert
    assert_eq!(event.line().actor(), Actor::Ai);
    assert_eq!(event.line().text(), "ping");
}

#[tokio::test]
async fn a_line_is_not_delivered_until_the_subscriber_acks_it() {
    // これが 001 の本命。ack を待たずに応答を返すと、
    // 「AI が返答したあとに画面が追いつく」形になる。
    // Arrange
    let band = Band::new();
    let mut subscriber = band.subscribe();
    let delivery = band.record(Actor::Ai, "ping");

    // Act
    let before_ack = delivery.wait_acked(Duration::from_millis(50)).await;
    let event = subscriber.recv().await.expect("購読者へ届いていない");
    event.ack();
    let after_ack = delivery.wait_acked(Duration::from_millis(500)).await;

    // Assert
    assert_eq!(
        before_ack,
        DeliveryOutcome::TimedOut {
            acked: 0,
            expected: 1
        }
    );
    assert_eq!(after_ack, DeliveryOutcome::Delivered);
}

#[tokio::test]
async fn every_subscriber_must_ack_before_a_line_counts_as_delivered() {
    // Arrange
    let band = Band::new();
    let mut first = band.subscribe();
    let mut second = band.subscribe();
    let delivery = band.record(Actor::Ai, "ping");

    // Act
    first.recv().await.expect("1 人目へ届いていない").ack();
    let with_one_ack = delivery.wait_acked(Duration::from_millis(50)).await;
    second.recv().await.expect("2 人目へ届いていない").ack();
    let with_both_acks = delivery.wait_acked(Duration::from_millis(500)).await;

    // Assert
    assert_eq!(
        with_one_ack,
        DeliveryOutcome::TimedOut {
            acked: 1,
            expected: 2
        }
    );
    assert_eq!(with_both_acks, DeliveryOutcome::Delivered);
}

#[tokio::test]
async fn a_stuck_subscriber_times_out_instead_of_hanging_the_caller() {
    // 画面が固まったときに MCP 側が永遠に返らないと、原因が掴めなくなる。
    // Arrange
    let band = Band::new();
    let _subscriber = band.subscribe();

    // Act
    let outcome = band
        .record(Actor::Ai, "ping")
        .wait_acked(Duration::from_millis(50))
        .await;

    // Assert
    assert_eq!(
        outcome,
        DeliveryOutcome::TimedOut {
            acked: 0,
            expected: 1
        }
    );
}

#[tokio::test]
async fn a_line_with_no_subscribers_is_delivered_at_once() {
    // 画面をまだ開いていないときに MCP を叩かれても、待たされない。
    // Arrange
    let band = Band::new();

    // Act
    let outcome = band
        .record(Actor::Ai, "ping")
        .wait_acked(Duration::from_millis(50))
        .await;

    // Assert
    assert_eq!(outcome, DeliveryOutcome::Delivered);
    assert_eq!(band.subscriber_count(), 0);
}
