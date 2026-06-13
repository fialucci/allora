//! Integration tests for the http-outbound-adapters collection spec.
//!
//! Validates the 0.0.9 `url:` schema for the collection form (the form a
//! real `allora.yaml` uses).

use allora_runtime::error::Error as RuntimeError;
use allora_runtime::spec::{
    HttpOutboundAdapterSpec, HttpOutboundAdaptersSpec, HttpOutboundAdaptersSpecYamlParser,
};

#[test]
fn parse_two_adapters() {
    let raw = r#"version: 1
http-outbound-adapters:
  - id: first
    url: http://127.0.0.1:8080/api/echo
    method: POST
  - url: https://10.0.0.2/
"#;
    let spec = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapters collection");
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.adapters().len(), 2);
    assert_eq!(spec.adapters()[0].id(), Some("first"));
    assert_eq!(spec.adapters()[0].url(), "http://127.0.0.1:8080/api/echo");
    assert!(spec.adapters()[1].id().is_none());
    assert_eq!(spec.adapters()[1].url(), "https://10.0.0.2/");
}

#[test]
fn programmatic_add_and_push() {
    let mut spec = HttpOutboundAdaptersSpec::new(1);
    let a1 = HttpOutboundAdapterSpec::new(
        "http://127.0.0.1:18080/alpha/ping",
        Some("POST"),
        None,
        true,
    );
    let a2 = HttpOutboundAdapterSpec::new(
        "http://127.0.0.1:18081/beta/pong",
        Some("GET"),
        Some("second"),
        false,
    );
    spec.push(a1);
    spec.push(a2);
    assert_eq!(spec.adapters().len(), 2);
    assert!(spec.adapters()[0].id().is_none());
    assert_eq!(spec.adapters()[1].id(), Some("second"));
    let owned = spec.clone().into_adapters();
    assert_eq!(owned.len(), 2);
}

#[test]
fn missing_root_error() {
    let raw = "version: 1";
    let err = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected missing root error");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("missing 'http-outbound-adapters'"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn empty_sequence_error() {
    let raw = r#"version: 1
http-outbound-adapters: []"#;
    let err = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected empty sequence error");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("sequence must not be empty"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_url_in_collection_error() {
    let raw = r#"version: 1
http-outbound-adapters:
  - id: bad
    url: ":::not a url"
"#;
    let err = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected invalid url error inside collection");
    match err {
        RuntimeError::Serialization(msg) => {
            assert!(msg.contains("url invalid"), "got: {msg}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
