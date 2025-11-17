use allora::{ensure_correlation, Exchange, Message};

#[test]
fn ensure_correlation_generates_uuid() {
    let mut exchange = Exchange::new(Message::from_text("ping"));
    assert!(exchange.in_msg.header("correlation_id").is_none());
    ensure_correlation(&mut exchange);
    let cid = exchange.in_msg.header("correlation_id").expect("cid added");
    assert_eq!(cid.len(), 36);
    assert_eq!(cid.chars().filter(|c| *c == '-').count(), 4);
}

#[test]
fn ensure_correlation_id_is_stable_on_subsequent_calls() {
    let mut exchange = Exchange::new(Message::from_text("ping"));
    ensure_correlation(&mut exchange);
    let first = exchange
        .in_msg
        .header("correlation_id")
        .unwrap()
        .to_string();
    // second call should not change
    ensure_correlation(&mut exchange);
    let second = exchange
        .in_msg
        .header("correlation_id")
        .unwrap()
        .to_string();
    assert_eq!(
        first, second,
        "Correlation id should be stable across calls"
    );
}
