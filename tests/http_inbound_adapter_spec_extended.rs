use allora_runtime::spec::HttpInboundAdapterSpec;

#[test]
fn http_inbound_adapter_spec_programmatic_with_id() {
    let spec = HttpInboundAdapterSpec::with_id(
        "adapter.id",
        "localhost",
        3000,
        "/ping",
        vec!["GET".into()],
        "inbound.ping",
    );
    assert_eq!(spec.id(), Some("adapter.id"));
    assert_eq!(spec.host(), "localhost");
    assert_eq!(spec.port(), 3000);
    assert_eq!(spec.path(), "/ping");
    assert_eq!(spec.methods(), &["GET".to_string()]);
    assert_eq!(spec.request_channel(), "inbound.ping");
    assert!(spec.reply_channel().is_none());
}

#[test]
fn http_inbound_adapter_spec_programmatic_with_reply() {
    let spec = HttpInboundAdapterSpec::with_reply(
        "0.0.0.0",
        8085,
        "/submit",
        vec!["POST".into(), "PUT".into()],
        "inbound.submit",
        "outbound.submit.reply",
    );
    assert!(spec.id().is_none());
    assert_eq!(spec.reply_channel(), Some("outbound.submit.reply"));
    assert_eq!(spec.methods(), &["POST".into(), "PUT".into()]);
}

#[test]
fn http_inbound_adapter_spec_programmatic_with_id_reply_and_set_id() {
    let mut spec = HttpInboundAdapterSpec::with_id_reply(
        "recv.initial",
        "127.0.0.1",
        9099,
        "/receive",
        vec!["POST".into()],
        "inbound.receive",
        "outbound.reply",
    );
    assert_eq!(spec.id(), Some("recv.initial"));
    assert_eq!(spec.reply_channel(), Some("outbound.reply"));
    spec.set_id("recv.updated".into());
    assert_eq!(spec.id(), Some("recv.updated"));
}
