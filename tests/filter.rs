use allora::{patterns::filter::Filter, Exchange, Message};

fn create_exchange(body: &str) -> Exchange {
    Exchange::new(Message::from_text(body))
}

#[test]
fn filter_apl_header_equals() {
    let mut exchange = create_exchange("body irrelevant");
    exchange.in_msg.set_header("x-bot", "hello");
    let filter = Filter::from_apl(r#"header("x-bot") == "hello""#).unwrap();
    assert!(filter.accepts(&exchange));
    exchange.in_msg.set_header("x-bot", "bydse");
    assert!(!filter.accepts(&exchange));
}

#[test]
fn filter_apl_exists_header() {
    let mut exchange = create_exchange("payload");
    let filter = Filter::from_apl(r#"exists(header("Trace-Id"))"#).unwrap();
    assert!(!filter.accepts(&exchange));
    exchange.in_msg.set_header("Trace-Id", "abc");
    assert!(filter.accepts(&exchange));
}

#[test]
fn filter_apl_body_contains() {
    let exchange = create_exchange("This KEEP should match");
    let filter = Filter::from_apl(r#"body.contains("KEEP")"#).unwrap();
    assert!(filter.accepts(&exchange));
    let exchange2 = create_exchange("No token here");
    assert!(!filter.accepts(&exchange2));
}

#[test]
fn filter_apl_fallback_literal() {
    let exchange = create_exchange("EXACT_LITERAL");
    let filter = Filter::from_apl("EXACT_LITERAL").unwrap();
    assert!(filter.accepts(&exchange));
    let exchange2 = create_exchange("DIFFERENT");
    assert!(!filter.accepts(&exchange2));
}

#[test]
fn filter_apl_and_short_circuit() {
    // First atom false should short-circuit && chain.
    let mut exchange = create_exchange("will not check second");
    exchange.in_msg.set_header("one", "1");
    let filter = Filter::from_apl(r#"header("missing") == "x" && body.contains("will")"#).unwrap();
    assert!(!filter.accepts(&exchange));
}

#[test]
fn filter_apl_or_short_circuit() {
    // First atom true should short-circuit || chain.
    let mut exchange = create_exchange("body KEEP");
    exchange.in_msg.set_header("h", "v");
    let filter = Filter::from_apl(r#"body.contains("KEEP") || header("missing") == "x""#).unwrap();
    assert!(filter.accepts(&exchange));
}

#[test]
fn filter_apl_invalid_leading_operator() {
    let filter_error = Filter::from_apl(r#"&& body.contains("X")"#).unwrap_err();
    match filter_error {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_invalid_trailing_operator() {
    let filter_error = Filter::from_apl(r#"header("x") == "y" &&"#).unwrap_err();
    match filter_error {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_invalid_consecutive_operators() {
    let filter_error =
        Filter::from_apl(r#"header("x") == "y" && || body.contains("Y")"#).unwrap_err();
    match filter_error {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_chain_mixed() {
    let mut exchange = create_exchange("KEEP token present");
    exchange.in_msg.set_header("x-bot", "hello");
    exchange.in_msg.set_header("trace", "t-1");
    let filter = Filter::from_apl(
        r#"header("x-bot") == "hello" && body.contains("KEEP") || exists(header("trace"))"#,
    )
    .unwrap();
    // (true && true) || true -> true
    assert!(filter.accepts(&exchange));
    exchange.in_msg.set_header("x-bot", "bye"); // first atom false, second true => (false && true)=false then || true => true
    assert!(filter.accepts(&exchange));
    exchange.in_msg.headers.remove("trace"); // now (false && true)=false || false => false
    assert!(!filter.accepts(&exchange));
}

#[test]
fn filter_apl_does_not_mutate_exchange() {
    let mut exchange = create_exchange("KEEP body");
    exchange.in_msg.set_header("x", "1");
    let orig_headers = exchange.in_msg.headers.clone();
    let filter = Filter::from_apl(r#"body.contains("KEEP") && header("x") == "1""#).unwrap();
    assert!(filter.accepts(&exchange));
    assert_eq!(orig_headers, exchange.in_msg.headers);
}
