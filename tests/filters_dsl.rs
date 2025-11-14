use allora::dsl::component_builders::build_filters_from_spec;
// internal builder reuse
use allora::spec::FiltersSpecYamlParser;
use allora::{Exchange, Message};
#[path = "common/filter.rs"]
mod filter_helpers;

#[test]
fn filters_dsl_build_multiple_from_fixture() {
    let raw = include_str!("fixtures/filters.yml");
    let spec = FiltersSpecYamlParser::parse_str(raw).expect("parse filters collection");
    let filters = build_filters_from_spec(spec).expect("build filters");
    assert_eq!(filters.len(), 2);
}

#[test]
fn filters_dsl_individual_application() {
    let raw = include_str!("fixtures/filters.yml");
    let spec = FiltersSpecYamlParser::parse_str(raw).expect("parse filters collection");
    let filters = build_filters_from_spec(spec).expect("build filters");
    // The first filter expects KEEP + Trace-Id header
    let mut ex1 = Exchange::new(Message::from_text("KEEP this"));
    ex1.in_msg.set_header("Trace-Id", "abc");
    assert!(filters[0].accepts(&ex1));
    let ex1_fail = Exchange::new(Message::from_text("DROP this"));
    assert!(!filters[0].accepts(&ex1_fail));
    // The second filter expects Audit-Flag == true
    let mut ex2 = Exchange::new(Message::from_text("body"));
    ex2.in_msg.set_header("Audit-Flag", "true");
    assert!(filters[1].accepts(&ex2));
    ex2.in_msg.set_header("Audit-Flag", "false");
    assert!(!filters[1].accepts(&ex2));
}
