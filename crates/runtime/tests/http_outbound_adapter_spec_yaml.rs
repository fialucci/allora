//! Integration tests for the http-outbound-adapter YAML spec parser.
//!
//! Validates the 0.0.9 `url:` schema: both happy-path deserialization and
//! the eager URL validation that surfaces invalid configuration at config
//! load time (not at first dispatch).

use allora_runtime::error::Error as RuntimeError;
use allora_runtime::spec::HttpOutboundAdapterSpecYamlParser;

#[test]
fn parse_success_full() {
    let raw = r#"version: 1
http-outbound-adapter:
  id: http.outboundEcho
  url: http://127.0.0.1:8080/api/echo
  method: POST
  use-out-msg: false
"#;
    let spec = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapter spec full");
    assert_eq!(spec.id(), Some("http.outboundEcho"));
    assert_eq!(spec.url(), "http://127.0.0.1:8080/api/echo");
    assert_eq!(spec.method(), Some("POST"));
    assert!(!spec.use_out_msg(), "use-out-msg override should be false");
}

#[test]
fn parse_success_minimal_defaults() {
    let raw = r#"version: 1
http-outbound-adapter:
  url: https://10.0.0.5/
"#;
    let spec = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapter spec minimal");
    assert!(spec.id().is_none());
    assert_eq!(spec.url(), "https://10.0.0.5/");
    assert!(spec.method().is_none());
    assert!(spec.use_out_msg(), "default use-out-msg should be true");
}

#[test]
fn parse_success_https() {
    // Confirms the `https://` scheme is accepted by the parser. The actual
    // HTTPS dispatch path is exercised in
    // `allora-http`'s `http_outbound_dispatches_over_https` integration test.
    let raw = r#"version: 1
http-outbound-adapter:
  id: chain.submit
  url: https://devnet.fialucci.org/oracle/submissions
  method: POST
"#;
    let spec =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect("parse https outbound spec");
    assert_eq!(spec.url(), "https://devnet.fialucci.org/oracle/submissions");
}

#[test]
fn missing_root_error() {
    let raw = "version: 1";
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected missing root error");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("missing 'http-outbound-adapter'"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn missing_url_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  method: POST
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected missing url error");
    match err {
        // Serde reports the missing field at the deserialization stage.
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("url"), "error should mention url, got: {msg}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn empty_url_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  url: ""
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty url error");
    match err {
        RuntimeError::Serialization(msg) => assert!(msg.contains("url must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_url_error() {
    // Garbage URL; parser should reject it at config-load time rather than
    // failing later at dispatch.
    let raw = r#"version: 1
http-outbound-adapter:
  url: ":::not a url"
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected invalid url error");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("url invalid"), "got: {msg}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unsupported_scheme_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  url: ftp://example.com/file
"#;
    let err = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected unsupported scheme error");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("unsupported scheme"), "got: {msg}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn empty_id_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  id: ""
  url: http://127.0.0.1:8080/
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty id error");
    match err {
        RuntimeError::Serialization(msg) => assert!(msg.contains("id must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unsupported_version_error() {
    let raw = r#"version: 2
http-outbound-adapter:
  url: http://127.0.0.1:8080/
"#;
    let err = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected unsupported version error");
    match err {
        RuntimeError::Serialization(msg) => assert!(msg.contains("unsupported version")),
        other => panic!("unexpected error: {other:?}"),
    }
}
