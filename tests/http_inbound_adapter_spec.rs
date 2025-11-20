use allora_runtime::spec::HttpInboundAdapterSpec;

#[test]
fn http_inbound_adapter_spec_programmatic_new() {
    let spec = HttpInboundAdapterSpec::new(
        "127.0.0.1",
        8080,
        "/receive",
        vec!["POST".into()],
        "inbound.receive",
    );
    assert!(spec.id().is_none());
    assert_eq!(spec.host(), "127.0.0.1");
    assert_eq!(spec.port(), 8080);
    assert_eq!(spec.path(), "/receive");
    assert_eq!(spec.methods(), &["POST".to_string()]);
    assert_eq!(spec.request_channel(), "inbound.receive");
    assert_eq!(spec.reply_channel(), None);
}

#[test]
fn http_inbound_adapter_spec_programmatic_with_id_reply() {
    let spec = HttpInboundAdapterSpec::with_id_reply(
        "http.recv",
        "0.0.0.0",
        9090,
        "/recv",
        vec!["POST".into(), "GET".into()],
        "inbound.recv",
        "outbound.reply",
    );
    assert_eq!(spec.id(), Some("http.recv"));
    assert_eq!(spec.reply_channel(), Some("outbound.reply"));
    assert_eq!(spec.methods(), &["POST".into(), "GET".into()]);
}
