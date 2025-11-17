use allora::message::{Exchange, Message, Payload};

#[test]
fn payload_variants_and_accessors() {
    let text = Payload::Text("hello".to_string());
    assert_eq!(text.as_text(), Some("hello"));
    assert_eq!(text.as_bytes(), None);

    let bytes = Payload::Bytes(vec![1, 2, 3]);
    assert_eq!(bytes.as_bytes(), Some(&[1, 2, 3][..]));
    assert_eq!(bytes.as_text(), None);

    {
        let json = Payload::Json(serde_json::json!({"a": 1}));
        match json {
            Payload::Json(ref v) => assert_eq!(v["a"], 1),
            _ => panic!("Not JSON"),
        }
    }

    let empty = Payload::Empty;
    assert_eq!(empty.as_text(), None);
    assert_eq!(empty.as_bytes(), None);
    assert_eq!(Payload::default(), Payload::Empty);
}

#[test]
fn message_construction_and_headers() {
    let mut msg = Message::from_text("abc");
    assert_eq!(msg.body_text(), Some("abc"));
    assert_eq!(msg.header("foo"), None);
    msg.set_header("foo", "bar");
    assert_eq!(msg.header("foo"), Some("bar"));
    msg.set_header("foo", "baz");
    assert_eq!(msg.header("foo"), Some("baz"));
}

#[test]
fn exchange_construction_and_properties() {
    let mut msg = Message::from_text("payload");
    msg.set_header("h", "v");
    let mut exch = Exchange::new(msg);
    assert_eq!(exch.in_msg.body_text(), Some("payload"));
    assert_eq!(exch.in_msg.header("h"), Some("v"));
    assert!(exch.out_msg.is_none());
    assert_eq!(exch.property("p"), None);
    exch.set_property("p", "x");
    assert_eq!(exch.property("p"), Some("x"));
}

#[test]
fn exchange_out_message_flow() {
    let mut exch = Exchange::new(Message::from_text("in"));
    assert!(exch.out_msg.is_none());
    exch.out_msg = Some(Message::from_text("out"));
    assert_eq!(exch.out_msg.as_ref().unwrap().body_text(), Some("out"));
}

#[test]
fn message_and_exchange_defaults() {
    let msg = Message::default();
    assert_eq!(msg.body_text(), None);
    assert!(msg.headers.is_empty());
    let exch = Exchange::default();
    assert!(exch.in_msg.body_text().is_none());
    assert!(exch.out_msg.is_none());
    assert!(exch.properties.is_empty());
}

#[test]
fn serde_roundtrip_message_and_exchange() {
    let mut msg = Message::from_text("serde");
    msg.set_header("k", "v");
    let json = serde_json::to_string(&msg).unwrap();
    let msg2: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(msg2.body_text(), Some("serde"));
    assert_eq!(msg2.header("k"), Some("v"));

    let mut exch = Exchange::new(msg2);
    exch.set_property("p", "v2");
    let json = serde_json::to_string(&exch).unwrap();
    let exch2: Exchange = serde_json::from_str(&json).unwrap();
    assert_eq!(exch2.in_msg.body_text(), Some("serde"));
    assert_eq!(exch2.property("p"), Some("v2"));
}

#[test]
fn message_ids_and_correlation_ids() {
    let mut m1 = Message::from_text("x");
    let mid1 = m1.header("message_id").unwrap().to_string(); // clone to drop immutable borrow
    assert!(!mid1.is_empty());
    assert!(m1.header("correlation_id").is_none());
    let cid1 = m1.ensure_correlation_id().to_string();
    assert_eq!(m1.header("correlation_id"), Some(cid1.as_str()));
    let cid1b = m1.ensure_correlation_id();
    assert_eq!(cid1b, cid1.as_str());

    let m2 = Message::from_text("y");
    let mid2 = m2.header("message_id").unwrap();
    assert_ne!(mid1, mid2, "message ids should be unique");

    let mut exchange = Exchange::new(Message::from_text("exchange"));
    let cid_ex = exchange.correlation_id().to_string();
    assert_eq!(
        exchange.in_msg.header("correlation_id"),
        Some(cid_ex.as_str())
    );
}
