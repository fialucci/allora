use allora::build;

#[path = "common/temp.rs"]
mod temp;
use temp::temp_yaml;

#[test]
fn allora_spec_build_from_str_success() {
    let raw = r#"version: 1
channels:
  - kind: direct
    id: inbound.orders
  - kind: direct
    id: processed.orders
"#;
    let tf = temp_yaml(raw);
    let runtime = build(tf.path()).expect("build top-level allora");
    assert_eq!(runtime.channel_count(), 2);
    let ids: Vec<&str> = runtime.channels().map(|c| c.id()).collect();
    assert!(ids.contains(&"inbound.orders"));
    assert!(ids.contains(&"processed.orders"));
}

#[test]
fn allora_spec_build_from_file_success() {
    let runtime = build("tests/fixtures/allora.yml").expect("build from fixture");
    assert_eq!(runtime.channel_count(), 3);
}

#[test]
fn allora_spec_missing_channels_success() {
    let raw = "version: 1"; // no channels
    let tf = temp_yaml(raw);
    let runtime = build(tf.path()).expect("build top-level allora without channels");
    assert_eq!(runtime.channel_count(), 0, "no channels expected");
}

#[test]
fn allora_spec_outbound_only_success() {
    let raw = r#"version: 1
http-outbound-adapters:
  - id: http.outboundEcho
    url: http://127.0.0.1:18080/receiveGateway
    method: POST
"#;
    let tf = temp_yaml(raw);
    let runtime = build(tf.path()).expect("build outbound-only allora");
    assert_eq!(runtime.channel_count(), 0);
    assert_eq!(runtime.http_outbound_adapter_count(), 1);
    assert_eq!(runtime.http_outbound_adapters()[0].id(), "http.outboundEcho");
}

#[test]
fn allora_spec_wrong_version_error() {
    let raw = r#"version: 2
channels: []"#;
    let tf = temp_yaml(raw);
    let err = build(tf.path()).expect_err("expected version error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("unsupported version")),
        other => panic!("unexpected error: {other:?}"),
    }
}
