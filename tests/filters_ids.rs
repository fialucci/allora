use allora::dsl::component_builders::build_filters_from_spec;
use allora::spec::FiltersSpecYamlParser;
use allora::Error;

#[test]
fn filters_auto_id_generation_and_uniqueness() {
    // Two filters missing id + one duplicate id to trigger error on duplicate.
    let raw = r#"version: 1
filters:
  - from: a
    when: body.contains("X")
  - from: b
    when: body.contains("Y")
  - id: dup
    from: c
    when: body.contains("Z")
  - id: dup
    from: d
    when: body.contains("Z")
"#;
    let spec = FiltersSpecYamlParser::parse_str(raw).expect("parse");
    // Build should fail due to duplicate explicit id 'dup'
    let err = build_filters_from_spec(spec).expect_err("expected duplicate error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("duplicate filter.id")),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn filters_auto_id_generation_success() {
    let raw = r#"version: 1
filters:
  - from: a
    when: body.contains("X")
  - from: b
    when: body.contains("Y")
"#;
    let spec = FiltersSpecYamlParser::parse_str(raw).expect("parse");
    let built = build_filters_from_spec(spec).expect("build filters");
    assert_eq!(built.len(), 2);

    // Indirectly test by adding a third build with additional missing ids; generation must remain deterministic within a single build call.
    let raw2 = r#"version: 1
filters:
  - from: a
    when: body.contains("X")
"#;
    let spec2 = FiltersSpecYamlParser::parse_str(raw2).expect("parse2");
    let built2 = build_filters_from_spec(spec2).expect("build filters2");
    assert_eq!(built2.len(), 1);
}
