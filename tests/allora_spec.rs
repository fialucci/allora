use allora::{build, Channel};
use std::fs;
use std::path::{Path, PathBuf};

// Simple RAII helper that creates a temp YAML file and removes it on drop.
struct TempFile {
    path: PathBuf,
}
impl TempFile {
    fn new(prefix: &str, contents: &str) -> Self {
        let mut p = std::env::temp_dir();
        p.push(format!("{}_{}.yml", prefix, uuid::Uuid::new_v4()));
        fs::write(&p, contents).expect("write temp yaml");
        TempFile { path: p }
    }
    fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn allora_spec_build_from_str_success() {
    let raw = r#"version: 1
channels:
  - kind: in_memory
    id: inbound.orders
  - kind: in_memory
    id: processed.orders
"#;
    let tf = TempFile::new("allora_top", raw);
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
    let tf = TempFile::new("allora_missing_channels", raw);
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
    let tf = TempFile::new("allora_wrong_version", raw);
    let err = build(tf.path()).expect_err("expected version error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("unsupported version")),
        other => panic!("unexpected error: {other:?}"),
    }
}
