use allora::{build_channel, Channel, Error};
use std::fs;
use std::path::PathBuf;

fn write_temp_with_ext(contents: &str, name_hint: &str, ext: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("allora-dsl-tests");
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{}_{}.{}", name_hint, uuid::Uuid::new_v4(), ext));
    fs::write(&file, contents).unwrap();
    file
}

/// GIVEN a valid YAML channel spec file (version 1, in_memory, id = path-yml)
/// WHEN we build the channel from the file path (format inferred from .yml extension)
/// THEN the resulting channel id matches the spec
#[test]
fn dsl_build_channel_from_path_infer_yaml_success() {
    let path = write_temp_with_ext(
        "version: 1\nchannel:\n  kind: in_memory\n  id: path-yml",
        "dsl_path",
        "yml",
    );
    let ch = build_channel(&path).expect("channel builds from path");
    assert_eq!(ch.id(), "path-yml");
}

/// GIVEN a YAML-like channel spec saved with an unsupported extension (.conf)
/// WHEN we attempt to build the channel from the file path
/// THEN we receive a serialization error indicating format inference failure
#[test]
fn dsl_build_channel_from_path_infer_error() {
    let path = write_temp_with_ext(
        "version: 1\nchannel:\n  kind: in_memory\n  id: no-ext",
        "dsl_path_no_ext",
        "conf",
    );
    let err = build_channel(&path).expect_err("expected inference error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("cannot infer DSL format")),
        _ => panic!("unexpected error variant: {err:?}"),
    }
}
