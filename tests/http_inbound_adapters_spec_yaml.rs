use allora_runtime::spec::HttpInboundAdaptersSpecYamlParser;

#[test]
fn http_inbound_adapters_spec_yaml_parse_success_two_entries() {
    let raw = r#"version: 1
http-inbound-adapters:
  - id: http.receiveGateway
    host: 127.0.0.1
    port: 8080
    path: /receiveGateway
    methods: [ POST ]
    request-channel: receiveChannel
    reply-channel: replyChannel
  - host: 0.0.0.0
    port: 8081
    path: /orders
    methods: [ POST, GET ]
    request-channel: inbound.orders
"#;
    let spec =
        HttpInboundAdaptersSpecYamlParser::parse_str(raw).expect("parse adapters collection");
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.adapters().len(), 2);
    assert_eq!(spec.adapters()[0].id(), Some("http.receiveGateway"));
    assert_eq!(spec.adapters()[1].methods(), &["POST".into(), "GET".into()]);
}

#[test]
fn http_inbound_adapters_spec_yaml_missing_root_error() {
    let raw = "version: 1";
    let err =
        HttpInboundAdaptersSpecYamlParser::parse_str(raw).expect_err("expected missing root error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("missing 'http-inbound-adapters'"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapters_spec_yaml_non_sequence_error() {
    let raw = r#"version: 1
http-inbound-adapters: {}
"#;
    let err =
        HttpInboundAdaptersSpecYamlParser::parse_str(raw).expect_err("expected non-sequence error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("must be a sequence")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapters_spec_yaml_empty_sequence_error() {
    let raw = r#"version: 1
http-inbound-adapters: []
"#;
    let err = HttpInboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected empty sequence error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("sequence must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapters_spec_yaml_entry_not_mapping_error() {
    let raw = r#"version: 1
http-inbound-adapters:
  - 42
"#;
    let err = HttpInboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected entry mapping error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("entry must be a mapping")),
        other => panic!("unexpected error: {other:?}"),
    }
}
