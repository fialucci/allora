use std::io::Write;
use tempfile::{Builder, NamedTempFile};

/// Create a temporary YAML file (with `.yml` suffix) containing `contents`.
/// The file is removed automatically when the returned `NamedTempFile` is dropped.
pub fn temp_yaml(contents: &str) -> NamedTempFile {
    let mut tmp = Builder::new()
        .suffix(".yml")
        .tempfile()
        .expect("temp yaml file");
    tmp.write_all(contents.as_bytes()).expect("write yaml temp");
    tmp
}

/// Create a temporary file with arbitrary extension `ext` (without leading dot).
/// The file is removed automatically when dropped.
pub fn temp_with_ext(contents: &str, ext: &str) -> NamedTempFile {
    let mut tmp = Builder::new()
        .suffix(&format!(".{ext}"))
        .tempfile()
        .expect("temp file");
    tmp.write_all(contents.as_bytes()).expect("write temp");
    tmp
}
