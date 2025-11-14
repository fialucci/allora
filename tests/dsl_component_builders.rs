use allora::dsl::component_builders::build_channel_from_spec;
use allora::{spec::ChannelSpec, Error};

#[test]
/// GIVEN a ChannelSpec with an explicit non-empty id
/// WHEN build_channel_from_spec is invoked
/// THEN the resulting channel id matches the spec
fn build_channel_from_spec_explicit_id_success() {
    let spec = ChannelSpec::in_memory().id("explicit-chan");
    let channel = build_channel_from_spec(spec).expect("channel builds");
    assert_eq!(channel.id(), "explicit-chan");
}

#[test]
/// GIVEN a ChannelSpec without an id
/// WHEN build_channel_from_spec is invoked
/// THEN an auto-generated id is assigned (non-empty and prefixed with "channel:")
fn build_channel_from_spec_auto_id_success() {
    let spec = ChannelSpec::in_memory();
    let channel = build_channel_from_spec(spec).expect("channel builds");
    let id = channel.id();
    assert!(!id.is_empty(), "auto id should not be empty");
    assert!(
        id.starts_with("channel:") || id.starts_with("direct:"),
        "unexpected auto id prefix {id}"
    );
}

#[test]
/// GIVEN two ChannelSpecs without ids
/// WHEN each is built separately
/// THEN the generated ids are distinct (low collision probability)
fn build_channel_from_spec_auto_id_uniqueness() {
    let chan_a = build_channel_from_spec(ChannelSpec::in_memory()).unwrap();
    let chan_b = build_channel_from_spec(ChannelSpec::in_memory()).unwrap();
    assert_ne!(chan_a.id(), chan_b.id(), "auto-generated ids should differ");
}

#[test]
/// GIVEN a ChannelSpec with an empty id string
/// WHEN build_channel_from_spec is invoked
/// THEN a serialization error is returned complaining about empty id
fn build_channel_from_spec_empty_id_error() {
    let spec = ChannelSpec::in_memory().id("");
    let err = build_channel_from_spec(spec).expect_err("expected empty id error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.id must not be empty")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}
