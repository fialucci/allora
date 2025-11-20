use allora_runtime::spec::HttpInboundAdaptersSpecYamlParser;

fn parse_err(yaml: &str, needle: &str) {
    let err = HttpInboundAdaptersSpecYamlParser::parse_str(yaml).expect_err("expected error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains(needle), "msg='{}' needle='{}'", msg, needle)
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapters_yaml_invalid_yaml_error() {
    let err = HttpInboundAdaptersSpecYamlParser::parse_str("::bad yaml")
        .expect_err("invalid YAML should error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("yaml parse error")),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn http_inbound_adapters_yaml_missing_version_error() {
    parse_err("http-inbound-adapters: []", "missing 'version'");
}

#[test]
fn http_inbound_adapters_yaml_version_non_integer_error() {
    parse_err(
        "version: '1'\nhttp-inbound-adapters: []",
        "'version' must be integer",
    );
}

#[test]
fn http_inbound_adapters_yaml_version_unsupported_error() {
    parse_err(
        "version: 2\nhttp-inbound-adapters: []",
        "unsupported version",
    );
}

#[test]
fn http_inbound_adapters_yaml_missing_root_error() {
    parse_err("version: 1", "missing 'http-inbound-adapters'");
}

#[test]
fn http_inbound_adapters_yaml_root_not_sequence_error() {
    parse_err(
        "version: 1\nhttp-inbound-adapters: {}",
        "must be a sequence",
    );
}

#[test]
fn http_inbound_adapters_yaml_empty_sequence_error() {
    parse_err(
        "version: 1\nhttp-inbound-adapters: []",
        "sequence must not be empty",
    );
}

#[test]
fn http_inbound_adapters_yaml_entry_not_mapping_error() {
    parse_err(
        "version: 1\nhttp-inbound-adapters:\n  - 42",
        "entry must be a mapping",
    );
}

#[test]
fn http_inbound_adapters_yaml_bubbled_invalid_entry_error() {
    // Second entry invalid (empty host) should bubble host empty message from single parser.
    let raw = r#"version: 1
http-inbound-adapters:
  - host: 127.0.0.1
    port: 8080
    path: /ok
    methods: [GET]
    request-channel: ch.ok
  - host: ''
    port: 8081
    path: /bad
    methods: [POST]
    request-channel: ch.bad
"#;
    let err = HttpInboundAdaptersSpecYamlParser::parse_str(raw).expect_err("second entry invalid");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("host must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapters_yaml_bubbled_unsupported_method_error() {
    let raw = r#"version: 1
http-inbound-adapters:
  - host: 127.0.0.1
    port: 8080
    path: /ok
    methods: [GET]
    request-channel: ch.ok
  - host: 127.0.0.1
    port: 8082
    path: /badmethod
    methods: [FOO]
    request-channel: ch.bad
"#;
    let err = HttpInboundAdaptersSpecYamlParser::parse_str(raw).expect_err("unsupported method");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("unsupported http-inbound-adapter.method"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
