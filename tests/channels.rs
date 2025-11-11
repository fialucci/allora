use allora::dsl::component_builders::build_channels_from_spec;
use allora::spec::ChannelsSpecYamlParser;
use allora::Channel;
use std::collections::HashSet;

#[test]
fn channels_fixture_builds_expected_ids() {
    let raw =
        std::fs::read_to_string("tests/fixtures/channels.yml").expect("read channels fixture");
    let spec = ChannelsSpecYamlParser::parse_str(&raw).expect("parse channels spec");
    let channels = build_channels_from_spec(spec).expect("build channels");
    assert_eq!(channels.len(), 3, "expected exactly 3 channels");
    let ids: HashSet<&str> = channels.iter().map(|c| c.id()).collect();
    let expected: HashSet<&str> = ["inbound.orders", "processed.orders", "error.deadletter"]
        .into_iter()
        .collect();
    assert_eq!(ids, expected, "channel ids mismatch");
}

#[test]
fn channels_fixture_no_behavior_checks_only_creation() {
    // Affirm we do not test queue behavior here (intentional minimal scope)
    // Re-run build to show isolation of creation logic.
    let raw =
        std::fs::read_to_string("tests/fixtures/channels.yml").expect("read channels fixture");
    let spec = ChannelsSpecYamlParser::parse_str(&raw).expect("parse channels spec");
    let channels = build_channels_from_spec(spec).expect("build channels");
    // Only assert count again; no send/receive operations.
    assert_eq!(channels.len(), 3);
}

#[test]
fn channels_missing_ids_are_auto_generated_and_unique() {
    let raw = r#"version: 1
channels:
  - kind: in_memory
    id: provided.one
  - kind: in_memory
  - kind: in_memory
  - kind: in_memory
"#;
    let spec = ChannelsSpecYamlParser::parse_str(raw).expect("parse mixed id spec");
    let channels = build_channels_from_spec(spec).expect("build channels with auto IDs");
    assert_eq!(channels.len(), 4);
    let mut seen = std::collections::HashSet::new();
    for c in &channels {
        assert!(
            seen.insert(c.id().to_string()),
            "duplicate generated id detected"
        );
    }
    assert!(seen.contains("provided.one"));
    // Extract auto-generated IDs (those starting with the auto prefix)
    let auto_ids: Vec<String> = seen
        .iter()
        .filter(|id| id.starts_with("channel:auto."))
        .cloned()
        .collect();
    // Expect exactly 3 auto IDs (since 3 specs lacked an id)
    assert_eq!(auto_ids.len(), 3, "expected 3 auto-generated IDs");
    // Verify each follows the pattern channel:auto.<number> and numbers are unique
    let mut numeric_parts: Vec<u64> = auto_ids
        .iter()
        .map(|id| {
            id.trim_start_matches("channel:auto.")
                .parse::<u64>()
                .expect("numeric suffix")
        })
        .collect();
    numeric_parts.sort_unstable();
    assert_eq!(
        numeric_parts,
        vec![1, 2, 3],
        "expected consecutive numeric suffixes starting at 1"
    );
}

#[test]
fn channels_duplicate_id_error_via_builder() {
    let raw = r#"version: 1
channels:
  - kind: in_memory
    id: dup
  - kind: in_memory
    id: dup
"#;
    let spec = ChannelsSpecYamlParser::parse_str(raw).expect("parse duplicate id spec");
    let err = build_channels_from_spec(spec).expect_err("expected builder duplicate error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("duplicate channel.id 'dup'")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}
