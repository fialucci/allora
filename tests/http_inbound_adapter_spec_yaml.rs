use allora_runtime::spec::HttpInboundAdapterSpecYamlParser;

#[test]
fn http_inbound_adapter_spec_yaml_parse_success_with_id_reply() {
    let raw = r#"version: 1
http-inbound-adapter:
  id: http.receiveGateway
  host: 127.0.0.1
  port: 8080
  path: /receiveGateway
  methods: [ POST ]
  request-channel: receiveChannel
  reply-channel: replyChannel
"#;
    let spec =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect("parse http inbound adapter spec");
    assert_eq!(spec.id(), Some("http.receiveGateway"));
    assert_eq!(spec.host(), "127.0.0.1");
    assert_eq!(spec.port(), 8080);
    assert_eq!(spec.path(), "/receiveGateway");
    assert_eq!(spec.methods(), &["POST".to_string()]);
    assert_eq!(spec.request_channel(), "receiveChannel");
    assert_eq!(spec.reply_channel(), Some("replyChannel"));
}

#[test]
fn http_inbound_adapter_spec_yaml_parse_success_without_id_or_reply() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: 0.0.0.0
  port: 8081
  path: /orders
  methods: [ POST, GET ]
  request-channel: inbound.orders
"#;
    let spec =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect("parse adapter spec no id/reply");
    assert!(spec.id().is_none());
    assert_eq!(spec.reply_channel(), None);
    assert_eq!(
        spec.methods(),
        &["POST", "GET"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn http_inbound_adapter_spec_yaml_missing_root_error() {
    let raw = "version: 1";
    let err =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected missing root error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("missing 'http-inbound-adapter'"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_spec_yaml_empty_host_error() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: ""
  port: 8080
  path: /p
  methods: [ POST ]
  request-channel: ch
"#;
    let err =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty host error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("host must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_spec_yaml_port_out_of_range_error() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: 127.0.0.1
  port: 70000
  path: /p
  methods: [ POST ]
  request-channel: ch
"#;
    let err =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected port range error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("port out of range")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_spec_yaml_methods_empty_error() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: 127.0.0.1
  port: 8080
  path: /p
  methods: []
  request-channel: ch
"#;
    let err =
        HttpInboundAdapterSpecYamlParser::parse_str(raw).expect_err("expected empty methods error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("methods sequence must not be empty"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_spec_yaml_unsupported_method_error() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: 127.0.0.1
  port: 8080
  path: /p
  methods: [ FOO ]
  request-channel: ch
"#;
    let err = HttpInboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected unsupported method error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("unsupported http-inbound-adapter.method"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_inbound_adapter_spec_yaml_empty_request_channel_error() {
    let raw = r#"version: 1
http-inbound-adapter:
  host: 127.0.0.1
  port: 8080
  path: /p
  methods: [ POST ]
  request-channel: ""
"#;
    let err = HttpInboundAdapterSpecYamlParser::parse_str(raw)
        .expect_err("expected empty request-channel error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("request-channel must not be empty"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
