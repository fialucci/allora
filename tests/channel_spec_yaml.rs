use allora::channel::ChannelInfo;
use allora::{build_channel, Channel, Error};
use std::fs;
use std::path::PathBuf;

fn write_temp(contents: &str, name_hint: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("allora-tests");
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{}_{}.yml", name_hint, uuid::Uuid::new_v4()));
    fs::write(&file, contents).unwrap();
    file
}

#[test]
fn spec_build_channel_success() {
    let path = write_temp(
        r#"version: 1
channel:
  id: success-chan
  kind: in_memory
"#,
        "success",
    );
    let chan = build_channel(&path).expect("channel builds");
    assert_eq!(chan.id(), "success-chan");
    assert_eq!(chan.kind(), "in_memory");
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
    let path = write_temp("::not valid: [yaml", "invalid_parse");
    let err = build_channel(&path).expect_err("expected parse error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("yaml parse error")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_missing_version_error() {
    let path = write_temp(
        r#"channel:
  kind: in_memory
  id: no-version
"#,
        "no_version",
    );
    let err = build_channel(&path).expect_err("expected missing version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("missing 'version'")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_unsupported_kind_error() {
    let path = write_temp(
        r#"version: 1
channel:
  kind: kafka
  id: bad-kind
"#,
        "bad_kind",
    );
    let err = build_channel(&path).expect_err("expected unsupported kind error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unsupported channel.kind")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_empty_id_error() {
    let path = write_temp(
        r#"version: 1
channel:
  kind: in_memory
  id: ""
"#,
        "empty_id",
    );
    let err = build_channel(&path).expect_err("expected empty id error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.id must not be empty")),
        _ => panic!("unexpected error variant"),
    }
}

#[test]
fn spec_build_channel_kind_in_memory() {
    use allora::channel::ChannelInfo;
    let path = write_temp(
        r#"version: 1
channel:
  kind: in_memory
  id: kind-check
"#,
        "kind",
    );
    let chan = build_channel(&path).expect("channel builds");
    assert_eq!(chan.id(), "kind-check");
    // Downcast not needed: ChannelInfo implemented for InMemoryChannel returned by build_channel
    assert_eq!(chan.kind(), "in_memory");
}

#[test]
fn spec_build_unknown_root_property_error() {
    let path = write_temp(
        r#"version: 1
extra: something
channel:
  kind: in_memory
"#,
        "unknown_root",
    );
    let err = build_channel(&path).expect_err("expected unknown root property error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unknown root property 'extra'")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_unknown_channel_property_error() {
    let path = write_temp(
        r#"version: 1
channel:
  kind: in_memory
  extra_field: ooops
"#,
        "unknown_channel",
    );
    let err = build_channel(&path).expect_err("expected unknown channel property error");
    match err {
        Error::Serialization(msg) => {
            assert!(msg.contains("unknown channel property 'extra_field'"))
        }
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_missing_channel_kind_error() {
    let path = write_temp(
        r#"version: 1
channel:
  id: no-kind
"#,
        "missing_kind",
    );
    let err = build_channel(&path).expect_err("expected missing channel.kind error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.kind required")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_non_integer_version_error() {
    let path = write_temp(
        r#"version: "1"
channel:
  kind: in_memory
"#,
        "non_int_version",
    );
    let err = build_channel(&path).expect_err("expected non-integer version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("'version' must be integer")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_non_string_kind_error() {
    let path = write_temp(
        r#"version: 1
channel:
  kind: 123
"#,
        "non_string_kind",
    );
    let err = build_channel(&path).expect_err("expected non-string kind error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.kind must be string")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_channel_not_mapping_error() {
    let path = write_temp(
        r#"version: 1
channel: 42
"#,
        "channel_not_map",
    );
    let err = build_channel(&path).expect_err("expected channel not mapping error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("'channel' must be a mapping")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}

#[test]
fn spec_build_unsupported_version_error() {
    let path = write_temp(
        r#"version: 2
channel:
  kind: in_memory
"#,
        "unsupported_version",
    );
    let err = build_channel(&path).expect_err("expected unsupported version error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("unsupported version")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}
