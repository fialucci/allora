use allora_runtime::spec::{HttpInboundAdapterSpec, HttpInboundAdaptersSpec};

#[test]
fn http_inbound_adapters_spec_programmatic_push() {
    let mut spec = HttpInboundAdaptersSpec::new(1);
    spec.push(HttpInboundAdapterSpec::new(
        "127.0.0.1",
        8080,
        "/alpha",
        vec!["GET".into()],
        "inbound.alpha",
    ));
    spec.push(HttpInboundAdapterSpec::with_id(
        "beta.id",
        "127.0.0.1",
        8081,
        "/beta",
        vec!["POST".into()],
        "inbound.beta",
    ));
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.adapters().len(), 2);
    assert!(spec.adapters()[0].id().is_none());
    assert_eq!(spec.adapters()[1].id(), Some("beta.id"));
    let vec_adapters = spec.clone().into_adapters();
    assert_eq!(vec_adapters.len(), 2);
}
