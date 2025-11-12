use allora::{patterns::filter::Filter, Exchange, Message};

fn ex(body: &str) -> Exchange {
    Exchange::new(Message::from_text(body))
}

#[test]
fn filter_apl_header_equals() {
    let mut e = ex("body irrelevant");
    e.in_msg.set_header("x-bot", "hello");
    let f = Filter::from_apl(r#"header("x-bot") == "hello""#).unwrap();
    assert!(f.accepts(&e));
    e.in_msg.set_header("x-bot", "bydse");
    assert!(!f.accepts(&e));
}

#[test]
fn filter_apl_exists_header() {
    let mut e = ex("payload");
    let f = Filter::from_apl(r#"exists(header("Trace-Id"))"#).unwrap();
    assert!(!f.accepts(&e));
    e.in_msg.set_header("Trace-Id", "abc");
    assert!(f.accepts(&e));
}

#[test]
fn filter_apl_body_contains() {
    let e = ex("This KEEP should match");
    let f = Filter::from_apl(r#"body.contains("KEEP")"#).unwrap();
    assert!(f.accepts(&e));
    let e2 = ex("No token here");
    assert!(!f.accepts(&e2));
}

#[test]
fn filter_apl_fallback_literal() {
    let e = ex("EXACT_LITERAL");
    let f = Filter::from_apl("EXACT_LITERAL").unwrap();
    assert!(f.accepts(&e));
    let e2 = ex("DIFFERENT");
    assert!(!f.accepts(&e2));
}

#[test]
fn filter_apl_and_short_circuit() {
    // First atom false should short-circuit && chain.
    let mut e = ex("will not check second");
    e.in_msg.set_header("one", "1");
    let f = Filter::from_apl(r#"header("missing") == "x" && body.contains("will")"#).unwrap();
    assert!(!f.accepts(&e));
}

#[test]
fn filter_apl_or_short_circuit() {
    // First atom true should short-circuit || chain.
    let mut e = ex("body KEEP");
    e.in_msg.set_header("h", "v");
    let f = Filter::from_apl(r#"body.contains("KEEP") || header("missing") == "x""#).unwrap();
    assert!(f.accepts(&e));
}

#[test]
fn filter_apl_invalid_leading_operator() {
    let err = Filter::from_apl(r#"&& body.contains("X")"#).unwrap_err();
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_invalid_trailing_operator() {
    let err = Filter::from_apl(r#"header("x") == "y" &&"#).unwrap_err();
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_invalid_consecutive_operators() {
    let err = Filter::from_apl(r#"header("x") == "y" && || body.contains("Y")"#).unwrap_err();
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("logical operator parse error")),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn filter_apl_chain_mixed() {
    let mut e = ex("KEEP token present");
    e.in_msg.set_header("x-bot", "hello");
    e.in_msg.set_header("trace", "t-1");
    let f = Filter::from_apl(
        r#"header("x-bot") == "hello" && body.contains("KEEP") || exists(header("trace"))"#,
    )
    .unwrap();
    // (true && true) || true -> true
    assert!(f.accepts(&e));
    e.in_msg.set_header("x-bot", "bye"); // first atom false, second true => (false && true)=false then || true => true
    assert!(f.accepts(&e));
    e.in_msg.headers.remove("trace"); // now (false && true)=false || false => false
    assert!(!f.accepts(&e));
}

#[test]
fn filter_apl_does_not_mutate_exchange() {
    let mut e = ex("KEEP body");
    e.in_msg.set_header("x", "1");
    let orig_headers = e.in_msg.headers.clone();
    let f = Filter::from_apl(r#"body.contains("KEEP") && header("x") == "1""#).unwrap();
    assert!(f.accepts(&e));
    assert_eq!(orig_headers, e.in_msg.headers);
}
