use allora::dsl::component_builders::{
    build_service_activators_from_spec, build_service_from_spec,
};
use allora::spec::{ServiceActivatorSpec, ServiceActivatorsSpec};
use allora::{Exchange, Message};

#[tokio::test]
async fn service_builder_basic() {
    let spec = allora::spec::ServiceActivatorSpec::new("impl/a.rs", "in.a", "out.a");
    let proc = allora::dsl::component_builders::build_service_from_spec(spec).unwrap();
    let mut exchange = allora::Exchange::new(allora::Message::from_text("msg"));
    proc.process(&mut exchange).await.unwrap();
    assert_eq!(
        exchange.in_msg.header("service-activator.ref-name"),
        Some("impl/a.rs")
    );
}

#[tokio::test]
async fn service_builders_sequence_ids() {
    let spec = allora::spec::ServiceActivatorsSpec::new(1)
        .add(allora::spec::ServiceActivatorSpec::new(
            "impl/a.rs",
            "in.a",
            "out.a",
        ))
        .add(allora::spec::ServiceActivatorSpec::new(
            "impl/b.rs",
            "in.b",
            "out.b",
        ));
    let procs = allora::dsl::component_builders::build_service_activators_from_spec(spec).unwrap();
    let mut ex1 = allora::Exchange::new(allora::Message::from_text("one"));
    procs[0].process(&mut ex1).await.unwrap();
    let mut ex2 = allora::Exchange::new(allora::Message::from_text("two"));
    procs[1].process(&mut ex2).await.unwrap();
    assert_eq!(
        ex1.in_msg.header("service-activator.id"),
        Some("service:auto.1")
    );
    assert_eq!(
        ex2.in_msg.header("service-activator.id"),
        Some("service:auto.2")
    );
}
