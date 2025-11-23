use allora_runtime::spec::HttpOutboundAdapterSpecYamlParser;

#[test]
fn http_outbound_adapter_spec_parse_success_full() {
    let raw = r#"version: 1
http-outbound-adapter:
  id: http.outboundEcho
  host: 127.0.0.1
  port: 8080
  base-path: /api
  path: /echo
  method: POST
  use-out-msg: false
"#;
    let spec = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapter spec full");
    assert_eq!(spec.id(), Some("http.outboundEcho"));
    assert_eq!(spec.host(), "127.0.0.1");
    assert_eq!(spec.port(), 8080);
    assert_eq!(spec.base_path(), "/api");
    assert_eq!(spec.path(), Some("/echo"));
    assert_eq!(spec.method(), Some("POST"));
    assert_eq!(spec.use_out_msg(), false); // overridden
}

#[test]
fn http_outbound_adapter_spec_parse_success_minimal_defaults() {
    let raw = r#"version: 1
http-outbound-adapter:
  host: 10.0.0.5
  port: 443
"#;
    let spec = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapter spec minimal");
    assert!(spec.id().is_none());
    assert_eq!(spec.base_path(), "/"); // default
    assert!(spec.path().is_none());
    assert!(spec.method().is_none());
    assert_eq!(spec.use_out_msg(), true); // default
}

#[test]
fn http_outbound_adapter_spec_missing_root_error() {
    let raw = "version: 1";
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected missing root error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("missing 'http-outbound-adapter'"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_empty_host_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  host: ""
  port: 8080
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty host error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("host must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_zero_port_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  host: 127.0.0.1
  port: 0
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected zero port error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("port out of range")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_invalid_base_path_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  host: 127.0.0.1
  port: 8080
  base-path: api
"#;
    let err = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected invalid base-path error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("base-path must start with '/'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_invalid_path_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  host: 127.0.0.1
  port: 8080
  path: echo
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected invalid path error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("path must start with '/'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_empty_id_error() {
    let raw = r#"version: 1
http-outbound-adapter:
  id: ""
  host: 127.0.0.1
  port: 8080
"#;
    let err =
        HttpOutboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty id error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("id must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapter_spec_unsupported_version_error() {
    let raw = r#"version: 2
http-outbound-adapter:
  host: 127.0.0.1
  port: 8080
"#;
    let err = HttpOutboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected unsupported version error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("unsupported version")),
        other => panic!("unexpected error: {other:?}"),
    }
}
