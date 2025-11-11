use allora::spec::FiltersSpecYamlParser;

#[test]
fn filters_spec_yaml_parse_success() {
    let raw = include_str!("fixtures/filters.yml");
    let spec = FiltersSpecYamlParser::parse_str(raw).expect("parse filters spec");
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.filters().len(), 2);
    assert_eq!(spec.filters()[0].from(), "inbound.orders");
    assert_eq!(spec.filters()[1].from(), "inbound.audit");
    assert_eq!(spec.filters()[0].id(), Some("filt.orders"));
    assert_eq!(spec.filters()[1].id(), Some("filt.audit"));
}

#[test]
fn filters_spec_yaml_missing_filters_error() {
    let raw = "version: 1";
    let err = FiltersSpecYamlParser::parse_str(raw).expect_err("expected missing filters error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("missing 'filters'")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn filters_spec_yaml_empty_sequence_error() {
    let raw = "version: 1\nfilters: {}"; // object instead of sequence
    let err = FiltersSpecYamlParser::parse_str(raw).expect_err("expected sequence error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("must be a sequence")),
        other => panic!("unexpected error: {other:?}"),
    }
}
