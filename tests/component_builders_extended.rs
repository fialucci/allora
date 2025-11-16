use allora::dsl::component_builders::{
    build_channels_from_spec, build_filter_from_spec, build_filters_from_spec,
    build_service_activators_from_spec, build_service_from_spec,
};
use allora::spec::{
    ChannelSpec, ChannelsSpec, FilterSpec, FiltersSpec, ServiceActivatorSpec, ServiceActivatorsSpec,
};
use allora::{Exchange, Message, SyncProcessor};

#[test]
fn single_filter_builder_preserves_explicit_id() {
    let spec = FilterSpec::with_id("custom.filter", "body.contains(\"X\")", "in.a");
    let filter = build_filter_from_spec(spec).expect("build single filter");
    assert_eq!(filter.id(), "custom.filter");
}

#[test]
fn single_filter_builder_generates_random_id_when_missing() {
    let spec = FilterSpec::new("body.contains(\"X\")", "in.a");
    let filter = build_filter_from_spec(spec).expect("build single filter");
    assert!(!filter.id().is_empty());
}

#[test]
fn filters_explicit_reserved_numeric_advances_sequence() {
    let spec = FiltersSpec::new(1)
        .add(FilterSpec::with_id(
            "filter:auto.5",
            "body.contains(\"X\")",
            "a",
        ))
        .add(FilterSpec::new("body.contains(\"Y\")", "b"));
    let built = build_filters_from_spec(spec).expect("build filters");
    assert_eq!(built.len(), 2);
    assert_eq!(built[0].id(), "filter:auto.5");
    assert_eq!(built[1].id(), "filter:auto.6");
}

#[test]
fn filters_malformed_reserved_prefix_ignored_for_sequence() {
    let spec = FiltersSpec::new(1)
        .add(FilterSpec::with_id(
            "filter:auto.bad",
            "body.contains(\"X\")",
            "a",
        ))
        .add(FilterSpec::new("body.contains(\"Y\")", "b"));
    let built = build_filters_from_spec(spec).expect("build filters");
    assert_eq!(built.len(), 2);
    assert_eq!(built[0].id(), "filter:auto.bad");
    assert_eq!(built[1].id(), "filter:auto.1");
}

#[test]
fn channels_explicit_reserved_like_prefix_does_not_shift_sequence() {
    let spec = ChannelsSpec::new(1)
        .add(ChannelSpec::in_memory().id("channel:auto.3"))
        .add(ChannelSpec::in_memory())
        .add(ChannelSpec::direct());
    let built = build_channels_from_spec(spec).expect("build channels");
    let ids: Vec<String> = built.iter().map(|c| c.id().to_string()).collect();
    assert!(ids.contains(&"channel:auto.1".to_string()));
    assert!(ids.contains(&"channel:auto.2".to_string()));
    assert!(ids.contains(&"channel:auto.3".to_string()));
}

#[test]
fn services_explicit_reserved_numeric_advances_sequence() {
    let spec = ServiceActivatorsSpec::new(1)
        .add(ServiceActivatorSpec::with_id(
            "service:auto.3",
            "impl/a.rs",
            "in.a",
            "out.a",
        ))
        .add(ServiceActivatorSpec::new("impl/b.rs", "in.b", "out.b"));
    let built = build_service_activators_from_spec(spec).expect("build services");
    assert_eq!(built.len(), 2);
    let mut ex0 = Exchange::new(Message::from_text("x"));
    built[0].process_sync(&mut ex0).unwrap();
    assert_eq!(
        ex0.in_msg.header("service-activator.id"),
        Some("service:auto.3")
    );
    let mut ex1 = Exchange::new(Message::from_text("y"));
    built[1].process_sync(&mut ex1).unwrap();
    assert_eq!(
        ex1.in_msg.header("service-activator.id"),
        Some("service:auto.4")
    );
}

#[test]
fn services_malformed_reserved_prefix_ignored_for_sequence() {
    let spec = ServiceActivatorsSpec::new(1)
        .add(ServiceActivatorSpec::with_id(
            "service:auto.bad",
            "impl/a.rs",
            "in.a",
            "out.a",
        ))
        .add(ServiceActivatorSpec::new("impl/b.rs", "in.b", "out.b"));
    let built = build_service_activators_from_spec(spec).expect("build services");
    assert_eq!(built.len(), 2);
    let mut ex0 = Exchange::new(Message::from_text("x"));
    built[0].process_sync(&mut ex0).unwrap();
    assert_eq!(
        ex0.in_msg.header("service-activator.id"),
        Some("service:auto.bad")
    );
    let mut ex1 = Exchange::new(Message::from_text("y"));
    built[1].process_sync(&mut ex1).unwrap();
    assert_eq!(
        ex1.in_msg.header("service-activator.id"),
        Some("service:auto.1")
    );
}

#[test]
fn build_service_from_spec_validation_positive_path() {
    let spec = ServiceActivatorSpec::new("impl/x.rs", "in.x", "out.x");
    let proc = build_service_from_spec(spec).expect("valid build");
    let mut exchange = Exchange::new(Message::from_text("ping"));
    proc.process_sync(&mut exchange).unwrap();
    assert_eq!(
        exchange.in_msg.header("service-activator.ref-name"),
        Some("impl/x.rs")
    );
}
