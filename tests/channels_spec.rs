use allora::dsl::component_builders::build_channels_from_spec;
use allora::spec::ChannelsSpecYamlParser;

#[test]
fn channels_spec_yaml_parse_and_build_success() {
    let raw = r#"version: 1
channels:
  - kind: direct
    id: inbound.orders
  - kind: direct
    id: processed.orders
  - kind: direct
    id: error.deadletter
"#;
    let spec = ChannelsSpecYamlParser::parse_str(raw).expect("parse channels spec");
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.channels().len(), 3);
    let built = build_channels_from_spec(spec).expect("build channels");
    assert_eq!(built.len(), 3);
    let ids: Vec<&str> = built.iter().map(|c| c.id()).collect();
    assert!(ids.contains(&"inbound.orders"));
    assert!(ids.contains(&"processed.orders"));
    assert!(ids.contains(&"error.deadletter"));
}

#[test]
fn channels_spec_yaml_missing_channels_error() {
    let raw = "version: 1"; // no channels
    let err = ChannelsSpecYamlParser::parse_str(raw).expect_err("expected missing channels error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'channels'")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn channels_spec_yaml_invalid_kind_error() {
    let raw = r#"version: 1
channels:
  - kind: kafka
    id: bad
"#;
    let err = ChannelsSpecYamlParser::parse_str(raw).expect_err("expected invalid kind error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("unsupported channel.kind")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn channels_spec_yaml_empty_id_error() {
    let raw = r#"version: 1
channels:
  - kind: direct
    id: ""
"#;
    let err = ChannelsSpecYamlParser::parse_str(raw).expect_err("expected empty id error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("channel.id must not be empty")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn channels_spec_duplicate_id_error() {
    let raw = r#"version: 1
channels:
  - kind: direct
    id: dup
  - kind: direct
    id: dup
"#;
    let spec = ChannelsSpecYamlParser::parse_str(raw).expect("parse spec even with dups");
    let err = build_channels_from_spec(spec).expect_err("expected duplicate id error during build");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("duplicate channel.id 'dup'")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}
