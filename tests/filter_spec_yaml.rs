use allora::spec::FilterSpecYamlParser;

#[test]
fn filter_spec_yaml_parse_with_to_success() {
    let raw = r#"version: 1
filter:
  from: inbound.orders
  to: vetted.orders
  when: body.contains("KEEP") && exists(header("Trace-Id"))
"#;
    let spec = FilterSpecYamlParser::parse_str(raw).expect("parse filter spec");
    assert_eq!(spec.from(), "inbound.orders");
    assert_eq!(spec.to(), Some("vetted.orders"));
    assert_eq!(
        spec.when(),
        "body.contains(\"KEEP\") && exists(header(\"Trace-Id\"))"
    );
}

#[test]
fn filter_spec_yaml_parse_without_to_success() {
    let raw = r#"version: 1
filter:
  from: inbound.audit
  when: exists(header("Audit-Flag")) && header("Audit-Flag") == "true"
"#;
    let spec = FilterSpecYamlParser::parse_str(raw).expect("parse filter spec without to");
    assert_eq!(spec.from(), "inbound.audit");
    assert!(spec.to().is_none());
}

#[test]
fn filter_spec_yaml_missing_filter_error() {
    let raw = "version: 1";
    let err = FilterSpecYamlParser::parse_str(raw).expect_err("expected missing filter error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'filter'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn filter_spec_yaml_empty_from_error() {
    let raw = r#"version: 1
filter:
  from: ""
  when: header(\"X\") == \"Y\"
"#;
    let err = FilterSpecYamlParser::parse_str(raw).expect_err("expected empty from error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("filter.from must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn filter_spec_yaml_empty_when_error() {
    let raw = r#"version: 1
filter:
  from: inbound.x
  when: ""
"#;
    let err = FilterSpecYamlParser::parse_str(raw).expect_err("expected empty when error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("filter.when must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn filter_spec_yaml_empty_to_error() {
    let raw = r#"version: 1
filter:
  from: inbound.x
  to: ""
  when: header(\"X\") == \"Y\"
"#;
    let err = FilterSpecYamlParser::parse_str(raw).expect_err("expected empty to error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("filter.to must not be empty")),
        other => panic!("unexpected error: {other:?}"),
    }
}
