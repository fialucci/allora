#[path = "common/temp.rs"]
mod temp;
use allora::{build_filter, Exchange, Filter, Message};
use temp::temp_yaml;

fn apply(filter: &Filter, body: &str) -> bool {
    let ex = Exchange::new(Message::from_text(body));
    filter.accepts(&ex)
}

#[test]
fn dsl_build_filter_from_file() {
    let f = build_filter("tests/fixtures/filter.yml").expect("build filter from fixture");
    // Without Trace-Id header should reject even if body contains KEEP
    assert!(!apply(&f, "A KEEP message"));
    // With header present accept
    let mut ex = Exchange::new(Message::from_text("A KEEP message"));
    ex.in_msg.headers.insert("Trace-Id".into(), "abc".into());
    assert!(f.accepts(&ex));
    // Non-matching body still reject
    let mut ex2 = Exchange::new(Message::from_text("A DROP message"));
    ex2.in_msg.headers.insert("Trace-Id".into(), "abc".into());
    assert!(!f.accepts(&ex2));
}

#[test]
fn dsl_build_filter_from_str_body_contains() {
    let raw = "version: 1\nfilter:\n  from: any\n  when: body.contains(\"ABC\")";
    let tmp = temp_yaml(raw);
    let f = build_filter(tmp.path()).expect("build filter from yaml str");
    assert!(apply(&f, "XYZABC123"));
    assert!(!apply(&f, "XYZAB123"));
}

#[test]
fn dsl_build_filter_from_str_header_equals() {
    let raw = "version: 1\nfilter:\n  from: any\n  when: header(\"Priority\") == \"high\"";
    let tmp = temp_yaml(raw);
    let f = build_filter(tmp.path()).expect("build filter header equals");
    let mut ex = Exchange::new(Message::from_text("irrelevant"));
    ex.in_msg.headers.insert("Priority".into(), "high".into());
    assert!(f.accepts(&ex));
    ex.in_msg.headers.insert("Priority".into(), "low".into());
    assert!(!f.accepts(&ex));
}

#[test]
fn dsl_build_filter_from_str_exists_header() {
    let raw = "version: 1\nfilter:\n  from: any\n  when: exists(header(\"Trace-Id\"))";
    let tmp = temp_yaml(raw);
    let f = build_filter(tmp.path()).expect("build filter exists header");
    let mut ex = Exchange::new(Message::from_text("body"));
    assert!(!f.accepts(&ex));
    ex.in_msg.headers.insert("Trace-Id".into(), "abc".into());
    assert!(f.accepts(&ex));
}

#[test]
fn dsl_build_filter_from_str_fallback_exact_body_match() {
    let raw = "version: 1\nfilter:\n  from: any\n  when: EXACT";
    let tmp = temp_yaml(raw);
    let f = build_filter(tmp.path()).expect("build filter fallback exact body");
    assert!(apply(&f, "EXACT"));
    assert!(!apply(&f, "NOT_EXACT"));
}

#[test]
fn dsl_build_filter_from_file_truth_table() {
    let f = build_filter("tests/fixtures/filter.yml").expect("build filter from fixture");
    // (true, true)
    let mut ex_tt = Exchange::new(Message::from_text("KEEP inside"));
    ex_tt.in_msg.headers.insert("Trace-Id".into(), "abc".into());
    assert!(f.accepts(&ex_tt));
    // (true, false)
    let ex_tf = Exchange::new(Message::from_text("KEEP inside"));
    assert!(!f.accepts(&ex_tf));
    // (false, true)
    let mut ex_ft = Exchange::new(Message::from_text("DROP inside"));
    ex_ft.in_msg.headers.insert("Trace-Id".into(), "abc".into());
    assert!(!f.accepts(&ex_ft));
    // (false, false)
    let ex_ff = Exchange::new(Message::from_text("DROP inside"));
    assert!(!f.accepts(&ex_ff));
}
