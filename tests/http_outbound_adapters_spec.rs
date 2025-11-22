use allora_runtime::spec::{
    HttpOutboundAdaptersSpecYamlParser, HttpOutboundAdaptersSpec, HttpOutboundAdapterSpec,
};

#[test]
fn http_outbound_adapters_spec_yaml_parse_two() {
    let raw = r#"version: 1
http-outbound-adapters:
  - id: first
    host: 127.0.0.1
    port: 8080
    base-path: /api
    path: /echo
    method: POST
  - host: 10.0.0.2
    port: 443
"#;
    let spec = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect("parse outbound adapters collection");
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.adapters().len(), 2);
    assert_eq!(spec.adapters()[0].id(), Some("first"));
    assert!(spec.adapters()[1].id().is_none());
}

#[test]
fn http_outbound_adapters_spec_programmatic_add_and_push() {
    let mut spec = HttpOutboundAdaptersSpec::new(1);
    // Programmatic add/push using single adapter specs constructed manually.
    let a1 = HttpOutboundAdapterSpec::new(
        // host
        "127.0.0.1",
        // port
        18080,
        // base_path
        "/alpha",
        // path
        Some("/ping".into()),
        // method
        Some("POST".into()),
        // id
        None,
        // use_out_msg
        true,
    );
    let a2 = HttpOutboundAdapterSpec::new(
        "127.0.0.1",
        18081,
        "/beta",
        Some("/pong".into()),
        Some("GET".into()),
        Some("second".into()),
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
fn http_outbound_adapters_spec_yaml_missing_root_error() {
    let raw = "version: 1";
    let err = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected missing root error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'http-outbound-adapters'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn http_outbound_adapters_spec_yaml_empty_sequence_error() {
    let raw = r#"version: 1
http-outbound-adapters: []"#;
    let err = HttpOutboundAdaptersSpecYamlParser::parse_str(raw)
        .expect_err("expected empty sequence error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("sequence must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}
