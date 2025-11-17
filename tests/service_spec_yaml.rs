use allora::spec::ServiceSpecYamlParser;

#[test]
fn service_spec_yaml_parse_success_with_id() {
    let raw = r#"version: 1
service-activator:
  id: hello_world
  ref-name: src/hello_world.rs
  from: inbound.orders
  to: vetted.orders
"#;
    let spec = ServiceSpecYamlParser::parse_str(raw).expect("parse service spec");
    assert_eq!(spec.id(), Some("hello_world"));
    assert_eq!(spec.ref_name(), "src/hello_world.rs");
    assert_eq!(spec.from(), "inbound.orders");
    assert_eq!(spec.to(), "vetted.orders");
}

#[test]
fn service_spec_yaml_parse_success_without_id() {
    let raw = r#"version: 1
service-activator:
  ref-name: src/hello_world.rs
  from: inbound.orders
  to: vetted.orders
"#;
    let spec = ServiceSpecYamlParser::parse_str(raw).expect("parse service spec without id");
    assert!(spec.id().is_none());
    assert_eq!(spec.ref_name(), "src/hello_world.rs");
}

#[test]
fn service_spec_yaml_missing_service_error() {
    let raw = "version: 1";
    let err = ServiceSpecYamlParser::parse_str(raw).expect_err("expected missing service error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'service-activator'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn service_spec_yaml_empty_name_error() {
    let raw = r#"version: 1
service-activator:
  ref-name: ""
  from: inbound.orders
  to: vetted.orders
"#;
    let err = ServiceSpecYamlParser::parse_str(raw).expect_err("expected empty impl error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("service-activator.ref-name must not be empty"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn service_spec_yaml_empty_from_error() {
    let raw = r#"version: 1
service-activator:
  ref-name: src/hello_world.rs
  from: ""
  to: vetted.orders
"#;
    let err = ServiceSpecYamlParser::parse_str(raw).expect_err("expected empty from error");
    match err {
        allora::Error::Serialization(msg) => {
            assert!(msg.contains("service-activator.from must not be empty"))
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn service_spec_yaml_empty_to_error() {
    let raw = r#"version: 1
service-activator:
  ref-name: src/hello_world.rs
  from: inbound.orders
  to: ""
"#;
    let err = ServiceSpecYamlParser::parse_str(raw).expect_err("expected empty to error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("service-activator.to must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn service_spec_yaml_empty_id_error() {
    let raw = r#"version: 1
service-activator:
  id: ""
  ref-name: src/hello_world.rs
  from: inbound.orders
  to: vetted.orders
"#;
    let err = ServiceSpecYamlParser::parse_str(raw).expect_err("expected empty id error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("service-activator.id must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}
