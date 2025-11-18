use allora_runtime::{Result, Runtime};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_spec(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, body).expect("write spec");
    p
}

fn minimal_spec() -> &'static str {
    "version: 1\nchannels:\n  - id: c1\n"
}

fn multi_spec() -> &'static str {
    "version: 1\nchannels:\n  - id: c1\n    kind: direct\n  - id: c2\n    kind: direct\n  - id: c3\n    kind: direct\n"
}

#[test]
fn builds_with_explicit_path() -> Result<()> {
    let td = TempDir::new().expect("tempdir");
    let spec_path = write_spec(td.path(), "custom.yml", multi_spec());
    let runtime = Runtime::new().with_config_file(&spec_path).run()?;
    assert_eq!(runtime.channel_count(), 3);
    Ok(())
}

#[test]
fn missing_config_errors() {
    let td = TempDir::new().expect("tempdir");
    // Use a randomized file name to avoid coupling test expectations to a hard-coded literal.
    let missing = td
        .path()
        .join(format!("missing_{}.yml", uuid::Uuid::new_v4()));
    let err = Runtime::new().with_config_file(&missing).run().unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.to_lowercase().contains("config file"),
        "expected generic config file mention, got: {msg}"
    );
    // Avoid asserting on the exact file name to keep test resilient to formatting changes.
}

#[test]
fn explicit_path_finds_nested_config() -> Result<()> {
    // Test using explicit path for nested configs (doesn't rely on CWD or exe parent search).
    let td = TempDir::new().expect("tempdir");

    // Create nested structure
    let level1 = td.path().join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).expect("create nested");

    // Place config deep in the tree
    let config_path = write_spec(&level2, "deep.yml", minimal_spec());

    // Should work with explicit path regardless of CWD
    let runtime = Runtime::new().with_config_file(&config_path).run()?;
    assert_eq!(runtime.channel_count(), 1);
    Ok(())
}

#[test]
fn explicit_path_with_multiple_configs() -> Result<()> {
    // Test that explicit path picks the right config when multiple exist.
    let td = TempDir::new().expect("tempdir");

    // Create multiple configs
    let config1 = write_spec(td.path(), "config1.yml", minimal_spec()); // 1 channel
    let config2 = write_spec(td.path(), "config2.yml", multi_spec()); // 3 channels

    // Should load the specified config
    let runtime1 = Runtime::new().with_config_file(&config1).run()?;
    assert_eq!(runtime1.channel_count(), 1);

    let runtime2 = Runtime::new().with_config_file(&config2).run()?;
    assert_eq!(runtime2.channel_count(), 3);

    Ok(())
}

#[test]
fn canonical_path_computed_when_file_exists() -> Result<()> {
    // Test that canonical path is computed when the file exists,
    // addressing the earlier canonicalize() inconsistency fix.
    let td = TempDir::new().expect("tempdir");
    let spec_path = write_spec(td.path(), "test.yml", minimal_spec());

    // The config should exist and be canonicalizable
    assert!(spec_path.exists(), "Config file should exist");

    let runtime = Runtime::new().with_config_file(&spec_path).run()?;
    assert_eq!(runtime.channel_count(), 1);
    Ok(())
}

#[test]
fn non_canonical_path_handled_correctly() {
    // Test that non-existent paths are handled gracefully without
    // attempting canonicalization.
    let td = TempDir::new().expect("tempdir");
    let missing = td.path().join("non_existent.yml");

    // Should not panic on canonicalize attempt
    let result = Runtime::new().with_config_file(&missing).run();
    assert!(result.is_err(), "Should error for non-existent config");
}
