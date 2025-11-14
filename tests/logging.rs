use allora::logging::load_logging_settings;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write(dir: &Path, contents: &str) {
    fs::write(dir.join("logging.yml"), contents).unwrap();
}

#[test]
fn defaults_without_file() {
    let td = TempDir::new().unwrap();
    let s = load_logging_settings(td.path());
    assert_eq!(s.filter, "info");
    assert!(s.ansi);
    assert!(s.with_timestamp);
}

#[test]
fn filter_precedence_over_level() {
    let td = TempDir::new().unwrap();
    write(
        td.path(),
        "level: warn\nfilter: trace,foo=debug\nansi: false\nformat:\n  with_timestamp: false\n",
    );
    let s = load_logging_settings(td.path());
    assert_eq!(s.filter, "trace,foo=debug");
    assert!(!s.ansi);
    assert!(!s.with_timestamp);
}

#[test]
fn level_used_when_filter_missing() {
    let td = TempDir::new().unwrap();
    write(td.path(), "level: error\nansi: true\n");
    let s = load_logging_settings(td.path());
    assert_eq!(s.filter, "error");
    assert!(s.ansi);
    assert!(s.with_timestamp); // default timestamp
}

#[test]
fn parse_error_falls_back_to_defaults() {
    let td = TempDir::new().unwrap();
    write(td.path(), "::this is not valid yaml:::\n");
    let s = load_logging_settings(td.path());
    assert_eq!(s.filter, "info");
    assert!(s.ansi);
    assert!(s.with_timestamp);
}

#[test]
#[cfg(unix)] // Permission tests only work reliably on Unix-like systems
fn read_error_reports_descriptive_source() {
    use std::os::unix::fs::PermissionsExt;

    let td = TempDir::new().unwrap();
    let log_path = td.path().join("logging.yml");

    // Create file with content
    fs::write(&log_path, "filter: debug\n").unwrap();

    // Make file unreadable (chmod 000)
    let mut perms = fs::metadata(&log_path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&log_path, perms).unwrap();

    // Load settings - should fall back to defaults
    let s = load_logging_settings(td.path());

    // Verify defaults are used
    assert_eq!(s.filter, "info");
    assert!(s.ansi);
    assert!(s.with_timestamp);

    // Verify source string indicates read error, not just "default"
    assert!(
        s.source.contains("read error"),
        "Expected source to mention 'read error', got: {}",
        s.source
    );
    assert!(
        s.source.contains("logging.yml"),
        "Expected source to mention file path, got: {}",
        s.source
    );
    assert!(
        s.source.contains("using defaults"),
        "Expected source to mention 'using defaults', got: {}",
        s.source
    );
}
