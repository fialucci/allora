use allora::dsl::component_builders::{
    build_service_activators_from_spec, build_service_from_spec,
};
use allora::processor::SyncProcessor;
use allora::spec::{ServiceActivatorSpec, ServiceActivatorsSpec};
use allora::{Exchange, Message};

#[test]
fn build_single_service_sets_name_header() {
    let spec = ServiceActivatorSpec::new("src/hello_world.rs", "inbound.orders", "vetted.orders");
    let proc = build_service_from_spec(spec).expect("build service");
    let mut exchange = Exchange::new(Message::from_text("ping"));
    proc.process_sync(&mut exchange).expect("process");
    assert_eq!(
        exchange.in_msg.header("service-activator.ref-name"),
        Some("src/hello_world.rs")
    );
}

#[test]
fn build_multiple_services_generates_auto_ids() {
    let spec = ServiceActivatorsSpec::new(1)
        .add(ServiceActivatorSpec::with_id(
            "svc.alpha",
            "src/a.rs",
            "in.a",
            "out.a",
        ))
        .add(ServiceActivatorSpec::new("src/b.rs", "in.b", "out.b"))
        .add(ServiceActivatorSpec::new("src/c.rs", "in.c", "out.c"));
    let procs = build_service_activators_from_spec(spec).expect("build services");
    assert_eq!(procs.len(), 3);
    let mut exchange = Exchange::new(Message::from_text("x"));
    procs[0].process_sync(&mut exchange).unwrap();
    assert_eq!(
        exchange.in_msg.header("service-activator.id"),
        Some("svc.alpha")
    );
    // process the auto-id services
    for p in &procs[1..] {
        let mut ex2 = Exchange::new(Message::from_text("y"));
        p.process_sync(&mut ex2).unwrap();
        let sid = ex2
            .in_msg
            .header("service-activator.id")
            .expect("auto id header");
        assert!(sid.starts_with("service:auto."));
    }
}

#[test]
fn build_services_duplicate_id_error() {
    let spec = ServiceActivatorsSpec::new(1)
        .add(ServiceActivatorSpec::with_id(
            "dup", "src/a.rs", "in.a", "out.a",
        ))
        .add(ServiceActivatorSpec::with_id(
            "dup", "src/b.rs", "in.b", "out.b",
        ));
    let err = build_service_activators_from_spec(spec).expect_err("expect duplicate id error");
    match err {
        allora::Error::Serialization(msg) => assert!(msg.contains("duplicate service.id")),
        other => panic!("unexpected error: {other:?}"),
    }
}
