use allora::{build_channel, Channel, Error};
#[path = "common/temp.rs"]
mod temp;
use temp::{temp_with_ext, temp_yaml};

/// GIVEN a valid YAML channel spec file (version 1, in_memory, id = path-yml)
/// WHEN we build the channel from the file path (format inferred from .yml extension)
/// THEN the resulting channel id matches the spec
#[test]
fn dsl_build_channel_from_path_infer_yaml_success() {
    let tmp = temp_yaml("version: 1\nchannel:\n  kind: in_memory\n  id: path-yml");
    let ch = build_channel(tmp.path()).expect("channel builds from path");
    assert_eq!(ch.id(), "path-yml");
}

/// GIVEN a YAML-like channel spec saved with an unsupported extension (.conf)
/// WHEN we attempt to build the channel from the file path
/// THEN we receive a serialization error indicating format inference failure
#[test]
fn dsl_build_channel_from_path_infer_error() {
    let tmp = temp_with_ext(
        "version: 1\nchannel:\n  kind: in_memory\n  id: no-ext",
        "conf",
    );
    let err = build_channel(tmp.path()).expect_err("expected inference error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("cannot infer DSL format")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}
