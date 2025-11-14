use allora::{build_channel, Error};
use std::path::PathBuf;
#[path = "common/temp.rs"]
mod temp;
use temp::temp_yaml;

#[test]
fn spec_build_channel_success() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  id: success-chan
  kind: direct
"#,
    );
    let chan = build_channel(tmp.path()).expect("channel builds");
    assert_eq!(chan.id(), "success-chan");
    assert_eq!(chan.kind(), "direct");
}

#[test]
fn spec_build_missing_file_error() {
    let bogus = PathBuf::from(format!(
        "tests/fixtures/does_not_exist_{}.yml",
        uuid::Uuid::new_v4()
    ));
    let err = build_channel(&bogus).expect_err("expected missing file error");
    match err {
        Error::Other(msg) => assert!(msg.contains("read error")),
        _ => panic!("wrong error variant: {err:?}"),
    }
}

#[test]
fn spec_build_invalid_yaml_parse_error() {
    let tmp = temp_yaml("::not valid: [yaml");
    let err = build_channel(tmp.path()).expect_err("expected parse error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("yaml parse error")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_missing_version_error() {
    let tmp = temp_yaml(
        r#"channel:
  kind: direct
  id: no-version
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected missing version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("missing 'version'")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_unsupported_kind_error() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: kafka
  id: bad-kind
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected unsupported kind error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unsupported channel.kind")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_empty_id_error() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: direct
  id: ""
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected empty id error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.id must not be empty")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_channel_kind_in_memory() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: in_memory
  id: kind-check
"#,
    );
    let chan = build_channel(tmp.path()).expect("channel builds");
    assert_eq!(chan.id(), "kind-check");
    // Downcast not needed: ChannelInfo implemented for InMemoryChannel returned by build_channel
    assert_eq!(chan.kind(), "in_memory");
}

#[test]
fn spec_build_channel_kind_direct() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: direct
  id: kind-check
"#,
    );
    let chan = build_channel(tmp.path()).expect("channel builds");
    assert_eq!(chan.id(), "kind-check");
    // Downcast not needed; kind available via Channel trait
    assert_eq!(chan.kind(), "direct");
}

#[test]
fn spec_build_unknown_root_property_error() {
    let tmp = temp_yaml(
        r#"version: 1
extra: something
channel:
  kind: direct
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected unknown root property error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unknown root property 'extra'")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_unknown_channel_property_error() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: direct
  extra_field: ooops
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected unknown channel property error");
    match err {
        Error::Serialization(msg) => {
            assert!(msg.contains("unknown channel property 'extra_field'"))
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_channel_kind_omitted_defaults() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  id: omitted-kind
"#,
    );
    let chan = build_channel(tmp.path()).expect("channel builds");
    assert_eq!(chan.id(), "omitted-kind");
    assert_eq!(chan.kind(), "direct");
}

#[test]
fn spec_build_non_integer_version_error() {
    let tmp = temp_yaml(
        r#"version: "1"
channel:
  kind: direct
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected non-integer version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("'version' must be integer")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_non_string_kind_error() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  kind: 123
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected non-string kind error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.kind must be string")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_channel_not_mapping_error() {
    let tmp = temp_yaml(
        r#"version: 1
channel: 42
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected channel not mapping error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("'channel' must be a mapping")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_unsupported_version_error() {
    let tmp = temp_yaml(
        r#"version: 2
channel:
  kind: direct
"#,
    );
    let err = build_channel(tmp.path()).expect_err("expected unsupported version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unsupported version")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_channel_defaults_kind_direct() {
    let tmp = temp_yaml(
        r#"version: 1
channel:
  id: default-kind
"#,
    );
    let chan = build_channel(tmp.path()).expect("channel builds");
    assert_eq!(chan.id(), "default-kind");
    // Internally still 'in_memory' variant.
    assert_eq!(chan.kind(), "direct");
}
