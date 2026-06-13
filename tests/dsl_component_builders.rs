use allora::dsl::component_builders::{
    build_channel_from_spec, build_http_outbound_adapter_from_spec,
    build_http_outbound_adapters_from_spec,
};
use allora::spec::{HttpOutboundAdapterSpec, HttpOutboundAdaptersSpec};
use allora::{spec::ChannelSpec, Error};

#[test]
/// GIVEN a ChannelSpec with an explicit non-empty id
/// WHEN build_channel_from_spec is invoked
/// THEN the resulting channel id matches the spec
fn build_channel_from_spec_explicit_id_success() {
    let spec = ChannelSpec::queue().id("explicit-chan");
    let channel = build_channel_from_spec(spec).expect("channel builds");
    assert_eq!(channel.id(), "explicit-chan");
}

#[test]
/// GIVEN a ChannelSpec without an id
/// WHEN build_channel_from_spec is invoked
/// THEN an auto-generated id is assigned (non-empty and prefixed with "queue:" or "direct:")
fn build_channel_from_spec_auto_id_success() {
    let spec = ChannelSpec::queue();
    let channel = build_channel_from_spec(spec).expect("channel builds");
    let id = channel.id();
    assert!(!id.is_empty(), "auto id should not be empty");
    assert!(
        id.starts_with("queue:") || id.starts_with("direct:"),
        "unexpected auto id prefix {id}"
    );
}

#[test]
/// GIVEN two ChannelSpecs without ids
/// WHEN each is built separately
/// THEN the generated ids are distinct (low collision probability)
fn build_channel_from_spec_auto_id_uniqueness() {
    let chan_a = build_channel_from_spec(ChannelSpec::queue()).unwrap();
    let chan_b = build_channel_from_spec(ChannelSpec::queue()).unwrap();
    assert_ne!(chan_a.id(), chan_b.id(), "auto-generated ids should differ");
}

#[test]
/// GIVEN a ChannelSpec with an empty id string
/// WHEN build_channel_from_spec is invoked
/// THEN a serialization error is returned complaining about empty id
fn build_channel_from_spec_empty_id_error() {
    let spec = ChannelSpec::queue().id("");
    let err = build_channel_from_spec(spec).expect_err("expected empty id error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("channel.id must not be empty")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
/// GIVEN an HTTP outbound adapter spec with explicit id and method
/// WHEN build_http_outbound_adapter_from_spec is invoked
/// THEN the resulting adapter has matching id and target URL
fn build_http_outbound_adapter_from_spec_explicit_id_success() {
    let spec = HttpOutboundAdapterSpec::with_id(
        "http.out.echo",
        "http://127.0.0.1:8080/api/echo",
        Some("POST"),
        true,
    );
    let adapter = build_http_outbound_adapter_from_spec(spec).expect("adapter builds");
    assert_eq!(adapter.id(), "http.out.echo");
    assert_eq!(adapter.url().scheme(), "http");
    assert_eq!(adapter.url().port(), Some(8080));
}

#[test]
/// GIVEN an HTTP outbound adapter spec without id/method
/// WHEN build_http_outbound_adapter_from_spec is invoked
/// THEN a non-empty id is auto-generated and defaults are applied
fn build_http_outbound_adapter_from_spec_auto_id_defaults() {
    let spec = HttpOutboundAdapterSpec::new("http://127.0.0.1:9090/", None, None, true);
    let adapter = build_http_outbound_adapter_from_spec(spec).expect("adapter builds");
    assert!(!adapter.id().is_empty(), "auto id should not be empty");
    assert_eq!(adapter.url().port(), Some(9090));
}

#[test]
/// GIVEN two outbound adapter specs without ids
/// WHEN each is built separately
/// THEN the generated ids are distinct
fn build_http_outbound_adapter_from_spec_auto_id_uniqueness() {
    let spec_a = HttpOutboundAdapterSpec::new("http://127.0.0.1:7001/", None, None, true);
    let spec_b = HttpOutboundAdapterSpec::new("http://127.0.0.1:7001/", None, None, true);
    let a = build_http_outbound_adapter_from_spec(spec_a).unwrap();
    let b = build_http_outbound_adapter_from_spec(spec_b).unwrap();
    assert_ne!(a.id(), b.id(), "auto ids should differ");
}

#[test]
/// GIVEN an HTTP outbound adapter spec with empty id string
/// WHEN build_http_outbound_adapter_from_spec is invoked
/// THEN a serialization error complaining about empty id is returned
fn build_http_outbound_adapter_from_spec_empty_id_error() {
    let spec =
        HttpOutboundAdapterSpec::with_id("", "http://127.0.0.1:8080/", None, true);
    let err = build_http_outbound_adapter_from_spec(spec).expect_err("expected empty id error");
    match err {
        Error::Serialization(msg) => assert!(msg.contains("id must not be empty")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
/// GIVEN a collection spec with two adapters (one with id, one without)
/// WHEN build_http_outbound_adapters_from_spec is invoked
/// THEN both adapters build and order is preserved
fn build_http_outbound_adapters_from_spec_two_success() {
    let mut col = HttpOutboundAdaptersSpec::new(1);
    col.push(HttpOutboundAdapterSpec::with_id(
        "first",
        "http://127.0.0.1:9500/a",
        None,
        true,
    ));
    col.push(HttpOutboundAdapterSpec::new(
        "http://127.0.0.1:9501/b",
        Some("GET"),
        None,
        true,
    ));
    let adapters = build_http_outbound_adapters_from_spec(col).expect("collection builds");
    assert_eq!(adapters.len(), 2);
    assert_eq!(adapters[0].id(), "first");
    assert_eq!(adapters[0].adapter().url().port(), Some(9500));
    assert_eq!(adapters[1].adapter().url().port(), Some(9501));
}

#[test]
/// GIVEN two outbound adapter specs with the same explicit id
/// WHEN build_http_outbound_adapters_from_spec is invoked
/// THEN a duplicate id serialization error is returned
fn build_http_outbound_adapters_duplicate_id_error() {
    let mut col = HttpOutboundAdaptersSpec::new(1);
    col.push(HttpOutboundAdapterSpec::with_id(
        "dup",
        "http://127.0.0.1:8001/",
        Some("POST"),
        true,
    ));
    col.push(HttpOutboundAdapterSpec::with_id(
        "dup",
        "http://127.0.0.1:8002/",
        Some("POST"),
        true,
    ));
    let err = build_http_outbound_adapters_from_spec(col).expect_err("expected duplicate id error");
    match err {
        Error::Serialization(msg) => {
            assert!(msg.contains("duplicate http-outbound-adapter.id 'dup'"))
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
/// GIVEN one explicit outbound adapter id using reserved auto prefix and two adapters without ids
/// WHEN build_http_outbound_adapters_from_spec is invoked
/// THEN missing ids are generated deterministically starting after the highest explicit suffix
fn build_http_outbound_adapters_auto_id_generation_sequence() {
    let mut col = HttpOutboundAdaptersSpec::new(1);
    col.push(HttpOutboundAdapterSpec::with_id(
        "http-outbound-adapter:auto.5",
        "http://127.0.0.1:8100/",
        Some("POST"),
        true,
    ));
    col.push(HttpOutboundAdapterSpec::new(
        "http://127.0.0.1:8101/",
        Some("GET"),
        None,
        true,
    ));
    col.push(HttpOutboundAdapterSpec::new(
        "http://127.0.0.1:8102/",
        Some("DELETE"),
        None,
        true,
    ));
    let adapters = build_http_outbound_adapters_from_spec(col).expect("build outbound adapters");
    assert_eq!(adapters.len(), 3);
    assert_eq!(adapters[0].id(), "http-outbound-adapter:auto.5");
    assert_eq!(adapters[1].id(), "http-outbound-adapter:auto.6");
    assert_eq!(adapters[2].id(), "http-outbound-adapter:auto.7");
}
