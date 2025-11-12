use allora::dsl::component_builders::build_filters_from_spec;
// internal builder reuse
use allora::spec::FiltersSpecYamlParser;
use allora::{build_filter, Exchange, Filter, Message};
#[path = "common/filter.rs"]
mod filter_helpers;
use filter_helpers::apply;

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
    // First filter expects KEEP + Trace-Id header
    let mut ex1 = Exchange::new(Message::from_text("KEEP this"));
    ex1.in_msg.set_header("Trace-Id", "abc");
    assert!(filters[0].accepts(&ex1));
    let ex1_fail = Exchange::new(Message::from_text("DROP this"));
    assert!(!filters[0].accepts(&ex1_fail));
    // Second filter expects Audit-Flag == true
    let mut ex2 = Exchange::new(Message::from_text("body"));
    ex2.in_msg.set_header("Audit-Flag", "true");
    assert!(filters[1].accepts(&ex2));
    ex2.in_msg.set_header("Audit-Flag", "false");
    assert!(!filters[1].accepts(&ex2));
}
