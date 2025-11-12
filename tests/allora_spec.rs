use allora::{build, Channel};

#[path = "common/temp.rs"]
mod temp;
use temp::temp_yaml;

#[test]
fn allora_spec_build_from_str_success() {
    let raw = r#"version: 1
channels:
  - kind: in_memory
    id: inbound.orders
  - kind: in_memory
    id: processed.orders
"#;
    let tf = temp_yaml(raw);
    let runtime = build(tf.path()).expect("build top-level allora");
    assert_eq!(runtime.channel_count(), 2);
    let ids: Vec<&str> = runtime.channels().iter().map(|c| c.id()).collect();
    assert!(ids.contains(&"inbound.orders"));
    assert!(ids.contains(&"processed.orders"));
}

#[test]
fn allora_spec_build_from_file_success() {
    let runtime = build("tests/fixtures/allora.yml").expect("build from fixture");
    assert_eq!(runtime.channel_count(), 3);
}

#[test]
fn allora_spec_missing_channels_error() {
    let raw = "version: 1"; // no channels
    let tf = temp_yaml(raw);
    let err = build(tf.path()).expect_err("expected error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'channels'")),
        other => panic!("unexpected error: {other:?}"),
    }
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
